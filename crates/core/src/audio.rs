//! What the machine can do with sound. A call needs a microphone and a speaker; when there is none,
//! the ui says so before ringing anyone (WhatsApp Web does the same).

use std::fs;
use std::path::Path;

/// Setting this to anything forces "no audio", to try the ui's message box on a machine that has it.
pub const NO_AUDIO_VARIABLE: &str = "ZAPZAP_NO_AUDIO";

/// Whether some ALSA device with the given suffix (`c` capture, `p` playback) can be opened.
fn device_openable(directory: &Path, suffix: char) -> bool {
    let Ok(entries) = fs::read_dir(directory) else { return false };
    entries.flatten().any(|entry| {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        name.starts_with("pcmC") && name.ends_with(suffix) && fs::File::open(entry.path()).is_ok()
    })
}

pub fn microphone_available() -> bool {
    std::env::var_os(NO_AUDIO_VARIABLE).is_none() && device_openable(Path::new("/dev/snd"), 'c')
}

pub fn speaker_available() -> bool {
    std::env::var_os(NO_AUDIO_VARIABLE).is_none() && device_openable(Path::new("/dev/snd"), 'p')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_directory_has_no_devices() {
        assert!(!device_openable(Path::new("/definitely/not/here"), 'c'));
    }

    #[test]
    fn only_matching_device_files_count() {
        let directory = std::env::temp_dir().join(format!("zapzap-audio-test-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("controlC0"), b"").unwrap();
        fs::write(directory.join("pcmC0D0p"), b"").unwrap();
        assert!(device_openable(&directory, 'p'));
        assert!(!device_openable(&directory, 'c'));
        fs::remove_dir_all(&directory).unwrap();
    }
}
