//! Gives back the file pages the process is not using. Executable mappings and the first read-only
//! mapping of a file (its headers and constants) are never written, so dropping their pages costs only a
//! page fault (served from the page cache) if they are used again; what stays resident is what actually
//! keeps running. Writable mappings and the later read-only one (RELRO) are left alone: they hold
//! relocated, modified pages that cannot be recovered from the file.

use std::fs;

const MADV_DONTNEED: i32 = 4;

extern "C" {
    fn madvise(address: *mut core::ffi::c_void, length: usize, advice: i32) -> i32;
}

/// Drops the resident pages of every executable mapping (the program and its libraries).
/// Returns how many bytes of address space were advised.
pub fn drop_idle_code() -> usize {
    let Ok(maps) = fs::read_to_string("/proc/self/maps") else { return 0 };
    let mut advised = 0;
    for line in maps.lines() {
        let mut fields = line.split_whitespace();
        let (Some(range), Some(permissions), Some(offset)) = (fields.next(), fields.next(), fields.next()) else { continue };
        // Backed by a file (the sixth field is a path), and either code or the untouched start of the file.
        let is_file = fields.nth(2).is_some_and(|path| path.starts_with('/'));
        let untouched = permissions == "r-xp" || (permissions == "r--p" && usize::from_str_radix(offset, 16) == Ok(0));
        if !is_file || !untouched {
            continue;
        }
        let Some((start, end)) = range.split_once('-') else { continue };
        let (Ok(start), Ok(end)) = (usize::from_str_radix(start, 16), usize::from_str_radix(end, 16)) else { continue };
        // SAFETY: the range is a mapping of this process that is only ever read and executed; advising
        // MADV_DONTNEED on it discards clean pages that the kernel reloads from the file on demand.
        if unsafe { madvise(start as *mut core::ffi::c_void, end - start, MADV_DONTNEED) } == 0 {
            advised += end - start;
        }
    }
    advised
}
