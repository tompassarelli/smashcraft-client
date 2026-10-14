use serde::Serialize;
use std::fs::Metadata;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextFile {
    pub folder: String,
    pub name: String,
    pub text: String,
    pub modified: u64,
}

pub fn millis(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |since| since.as_millis() as u64)
}

pub fn modified_ms(metadata: std::io::Result<Metadata>) -> Option<u64> {
    metadata.and_then(|meta| meta.modified()).ok().map(millis)
}

pub fn serial_of<'a>(name: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
    let serial = name.strip_prefix(prefix)?.strip_suffix(suffix)?;
    (!serial.is_empty() && serial.bytes().all(|b| b.is_ascii_digit())).then_some(serial)
}

pub fn write_atomic(path: &Path, contents: impl AsRef<[u8]>) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    std::fs::write(&tmp, contents).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serials_are_digits_between_the_prefix_and_suffix() {
        assert_eq!(serial_of("smashcraft-match-12.txt", "smashcraft-match-", ".txt"), Some("12"));
        assert_eq!(serial_of("smashcraft-match-.txt", "smashcraft-match-", ".txt"), None);
        assert_eq!(serial_of("smashcraft-match-1a.txt", "smashcraft-match-", ".txt"), None);
        assert_eq!(serial_of("smashcraft-match-index.pld", "smashcraft-match-", ".txt"), None);
    }

    #[test]
    fn an_atomic_write_creates_its_folder_and_leaves_no_temporary_file() {
        let dir = std::env::temp_dir().join(format!("smashcraft-files-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("nested/games.json");
        write_atomic(&path, "first").unwrap();
        write_atomic(&path, "second").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
        assert!(!dir.join("nested/games.json.tmp").exists());
        assert_eq!(millis(UNIX_EPOCH + std::time::Duration::from_millis(1500)), 1500);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
