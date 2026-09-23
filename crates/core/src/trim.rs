//! Gives back the file pages the process is not using. Executable mappings and the first read-only
//! mapping of a file (its headers and constants) are never written, so dropping their pages costs only a
//! page fault (served from the page cache) if they are used again; what stays resident is what actually
//! keeps running. Writable mappings and the later read-only one (RELRO) are left alone: they hold
//! relocated, modified pages that cannot be recovered from the file.

#[cfg(target_os = "linux")]
use std::fs;

#[cfg(target_os = "linux")]
const MADV_DONTNEED: i32 = 4;

#[cfg(target_os = "linux")]
extern "C" {
    fn madvise(address: *mut core::ffi::c_void, length: usize, advice: i32) -> i32;
}

/// Drops the resident pages of every file mapping that was never written: code, constants and the
/// libraries' read-only data. Returns how many bytes of address space were advised.
///
/// A mapping counts as never written when it is a read-only or executable file mapping whose
/// `Anonymous` size in smaps is zero: relocated data (RELRO, the GOT) has been copied on write and
/// would lose its contents.
#[cfg(not(target_os = "linux"))]
pub fn drop_idle_code() -> usize {
    0
}

#[cfg(target_os = "linux")]
pub fn drop_idle_code() -> usize {
    let Ok(smaps) = fs::read_to_string("/proc/self/smaps") else { return 0 };
    let mut advised = 0;
    let mut current: Option<(usize, usize)> = None;
    let mut candidate = false;
    for line in smaps.lines() {
        let first = line.as_bytes().first().copied().unwrap_or(b' ');
        if first.is_ascii_hexdigit() && line.contains('-') && !line.starts_with("Rss") {
            // A new mapping: "start-end perms offset dev inode path".
            let mut fields = line.split_whitespace();
            let (Some(range), Some(permissions)) = (fields.next(), fields.next()) else { continue };
            let is_file = fields.nth(3).is_some_and(|path| path.starts_with('/'));
            candidate = is_file && (permissions == "r-xp" || permissions == "r--p");
            current = range.split_once('-').and_then(|(start, end)| Some((usize::from_str_radix(start, 16).ok()?, usize::from_str_radix(end, 16).ok()?)));
        } else if candidate && line.starts_with("Anonymous:") {
            if line.split_whitespace().nth(1) == Some("0") {
                if let Some((start, end)) = current {
                    // SAFETY: the range is a mapping of this process that no one ever wrote to (no
                    // anonymous pages); MADV_DONTNEED discards clean pages that the kernel reloads
                    // from the file on demand.
                    if unsafe { madvise(start as *mut core::ffi::c_void, end - start, MADV_DONTNEED) } == 0 {
                        advised += end - start;
                    }
                }
            }
            candidate = false;
        }
    }
    advised
}

/// Starts a thread that lets go of idle file pages all the time: every quarter of a second while the
/// process starts up (that is when most of the binary gets touched), every half second afterwards. Each
/// page fault maps sixteen neighbouring pages, so a burst of activity leaves megabytes resident that
/// the next pass takes back.
pub fn keep_trimmed() {
    let _ = std::thread::Builder::new().name("trim".into()).stack_size(64 * 1024).spawn(|| {
        let started = std::time::Instant::now();
        loop {
            let wait = if started.elapsed().as_secs() < 30 { 250 } else { 500 };
            std::thread::sleep(std::time::Duration::from_millis(wait));
            drop_idle_code();
        }
    });
}
