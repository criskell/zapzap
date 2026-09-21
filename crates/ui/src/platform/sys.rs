//! Linux x86_64 system calls, no libc. Failures are fatal: there is nothing useful to recover to.

use core::arch::asm;

const SYS_READ: isize = 0;
const SYS_WRITE: isize = 1;
const SYS_WRITEV: isize = 20;
const SYS_OPEN: isize = 2;
const SYS_CLOSE: isize = 3;
const SYS_POLL: isize = 7;
const SYS_MADVISE: isize = 28;
const MADV_DONTNEED: usize = 4;
const SYS_SOCKET: isize = 41;
const SYS_CLOCK_GETTIME: isize = 228;
const SYS_CONNECT: isize = 42;
const SYS_EXIT_GROUP: isize = 231;

const EINTR: isize = -4;
const AF_UNIX: usize = 1;
const SOCK_STREAM: usize = 1;
const POLLIN: i16 = 1;

#[repr(C)]
pub struct SockAddrUn {
    pub family: u16,
    pub path: [u8; 108],
}

#[repr(C)]
struct IoVec {
    base: usize,
    len: usize,
}

#[repr(C)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}

fn syscall3(number: isize, a: usize, b: usize, c: usize) -> isize {
    let result: isize;
    unsafe {
        asm!("syscall", inlateout("rax") number => result, in("rdi") a, in("rsi") b, in("rdx") c,
             lateout("rcx") _, lateout("r11") _, options(nostack));
    }
    result
}

pub fn exit(code: usize) -> ! {
    syscall3(SYS_EXIT_GROUP, code, 0, 0);
    loop {}
}

pub fn die(message: &str) -> ! {
    syscall3(SYS_WRITE, 2, message.as_ptr() as usize, message.len());
    exit(1)
}

pub fn write_all(fd: i32, mut data: &[u8]) {
    while !data.is_empty() {
        let n = syscall3(SYS_WRITE, fd as usize, data.as_ptr() as usize, data.len());
        if n == EINTR {
            continue;
        }
        if n <= 0 {
            die("write failed\n");
        }
        data = &data[n as usize..];
    }
}

/// Sends `head` then `body` in one syscall; a short write is finished with plain writes.
pub fn write_all_pair(fd: i32, head: &[u8], body: &[u8]) {
    let iov = [IoVec { base: head.as_ptr() as usize, len: head.len() }, IoVec { base: body.as_ptr() as usize, len: body.len() }];
    let sent = syscall3(SYS_WRITEV, fd as usize, iov.as_ptr() as usize, iov.len());
    if sent < 0 {
        write_all(fd, head);
        write_all(fd, body);
        return;
    }
    let sent = sent as usize;
    if sent < head.len() {
        write_all(fd, &head[sent..]);
        write_all(fd, body);
    } else {
        write_all(fd, &body[sent - head.len()..]);
    }
}

/// Exits quietly when the peer closes the connection.
pub fn read_exact(fd: i32, mut buf: &mut [u8]) {
    while !buf.is_empty() {
        let n = syscall3(SYS_READ, fd as usize, buf.as_mut_ptr() as usize, buf.len());
        if n == EINTR {
            continue;
        }
        if n <= 0 {
            exit(0);
        }
        buf = &mut buf[n as usize..];
    }
}

/// Reads until the buffer is full or the file ends; returns the bytes read. `path` must end in NUL.
pub fn read_file(path: &[u8], buf: &mut [u8]) -> usize {
    let fd = syscall3(SYS_OPEN, path.as_ptr() as usize, 0, 0);
    if fd < 0 {
        return 0;
    }
    let mut filled = 0;
    while filled < buf.len() {
        let n = syscall3(SYS_READ, fd as usize, buf[filled..].as_mut_ptr() as usize, buf.len() - filled);
        if n <= 0 {
            break;
        }
        filled += n as usize;
    }
    syscall3(SYS_CLOSE, fd as usize, 0, 0);
    filled
}

pub fn connect_unix(addr: &SockAddrUn) -> i32 {
    let fd = syscall3(SYS_SOCKET, AF_UNIX, SOCK_STREAM, 0);
    if fd < 0 {
        die("socket failed\n");
    }
    if syscall3(SYS_CONNECT, fd as usize, addr as *const SockAddrUn as usize, size_of::<SockAddrUn>()) < 0 {
        die("cannot connect to the X server\n");
    }
    fd as i32
}

/// Blocks until one of the two descriptors has something to read (or hung up), or `timeout_ms`
/// passes (negative: never). A negative descriptor is ignored by the kernel, which is how a closed
/// stdin is switched off.
pub fn wait_readable(first: i32, second: i32, timeout_ms: i32) -> (bool, bool) {
    let mut fds = [PollFd { fd: first, events: POLLIN, revents: 0 }, PollFd { fd: second, events: POLLIN, revents: 0 }];
    while syscall3(SYS_POLL, fds.as_mut_ptr() as usize, fds.len(), timeout_ms as isize as usize) == EINTR {}
    (fds[0].revents != 0, fds[1].revents != 0)
}

/// Reads whatever is available (at most `buf.len()` bytes); 0 means end of input.
pub fn read_some(fd: i32, buf: &mut [u8]) -> usize {
    loop {
        let n = syscall3(SYS_READ, fd as usize, buf.as_mut_ptr() as usize, buf.len());
        if n != EINTR {
            return n.max(0) as usize;
        }
    }
}

pub fn has_pending(fd: i32) -> bool {
    let mut pollfd = PollFd { fd, events: POLLIN, revents: 0 };
    syscall3(SYS_POLL, &mut pollfd as *mut PollFd as usize, 1, 0) > 0
}

/// # Safety
/// `envp` must be the NULL-terminated environment block the kernel passed at process start.
pub unsafe fn env(envp: *const *const u8, name: &[u8]) -> Option<&'static [u8]> {
    let mut entry = envp;
    while !(*entry).is_null() {
        let bytes = cstr(*entry);
        if bytes.len() > name.len() && bytes.starts_with(name) && bytes[name.len()] == b'=' {
            return Some(&bytes[name.len() + 1..]);
        }
        entry = entry.add(1);
    }
    None
}

/// # Safety
/// `text` must point to a NUL-terminated string that lives for the whole process.
pub unsafe fn cstr(text: *const u8) -> &'static [u8] {
    let mut len = 0;
    while *text.add(len) != 0 {
        len += 1;
    }
    core::slice::from_raw_parts(text, len)
}

/// Seconds since the Unix epoch.
pub fn unix_seconds() -> i64 {
    let mut timespec = [0i64; 2];
    syscall3(SYS_CLOCK_GETTIME, 0, timespec.as_mut_ptr() as usize, 0);
    timespec[0]
}

/// Milliseconds on a clock that only moves forward (for timing double clicks).
pub fn monotonic_ms() -> i64 {
    let mut timespec = [0i64; 2];
    syscall3(SYS_CLOCK_GETTIME, 1, timespec.as_mut_ptr() as usize, 0);
    timespec[0] * 1000 + timespec[1] / 1_000_000
}

/// Gives back the resident pages of the file-backed mappings that are never written (the fonts and
/// icons in the read-only data, and the code). They come back from the page cache on the next use, one
/// fault-around window at a time, so what stays resident between frames is what a frame really touches.
/// The page cache keeps the file (shared, evictable); this only stops the process from holding it.
pub fn drop_idle_file_pages() {
    let mut maps = [0u8; 4096];
    let fd = syscall3(SYS_OPEN, b"/proc/self/maps\0".as_ptr() as usize, 0, 0);
    if fd < 0 {
        return;
    }
    let mut filled = 0;
    loop {
        let n = syscall3(SYS_READ, fd as usize, maps[filled..].as_mut_ptr() as usize, maps.len() - filled);
        if n <= 0 {
            break;
        }
        filled += n as usize;
    }
    syscall3(SYS_CLOSE, fd as usize, 0, 0);
    for line in maps[..filled].split(|&b| b == b'\n') {
        let mut fields = line.split(|&b| b == b' ').filter(|field| !field.is_empty());
        let (Some(range), Some(permissions), Some(offset)) = (fields.next(), fields.next(), fields.next()) else { continue };
        // File-backed (the sixth field is a path), and code or the untouched start of the file.
        let is_file = fields.nth(2).is_some_and(|path| path.first() == Some(&b'/'));
        let untouched = permissions == b"r-xp" || (permissions == b"r--p" && hex(offset) == Some(0));
        if !is_file || !untouched {
            continue;
        }
        let mut ends = range.split(|&b| b == b'-');
        let (Some(start), Some(end)) = (ends.next().and_then(hex), ends.next().and_then(hex)) else { continue };
        syscall3(SYS_MADVISE, start, end - start, MADV_DONTNEED);
    }
}

fn hex(digits: &[u8]) -> Option<usize> {
    digits.iter().try_fold(0usize, |total, &digit| Some(total * 16 + (digit as char).to_digit(16)? as usize))
}
