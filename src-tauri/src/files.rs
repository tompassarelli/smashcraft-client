//! The file helpers the client's stores share: atomic writes, modification
//! times in milliseconds, numbered file names and the `TextFile` the pages read.

use serde::Serialize;
use std::fs::Metadata;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// A text file in one of the folders the client reads, with when it was written.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextFile {
    pub folder: String,
    pub name: String,
    pub text: String,
    /// When the file was last written, in milliseconds since 1970; 0 when unknown.
    pub modified: u64,
}

/// `time` in milliseconds since 1970; 0 before the epoch.
pub fn millis(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |since| since.as_millis() as u64)
}

/// The milliseconds of a file's modified time, from its metadata; none when it can't be read.
pub fn modified_ms(metadata: std::io::Result<Metadata>) -> Option<u64> {
    metadata.and_then(|meta| meta.modified()).ok().map(millis)
}

/// The digits between `prefix` and `suffix` in `name`; none unless there are some and they are all digits.
pub fn serial_of<'a>(name: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
    let serial = name.strip_prefix(prefix)?.strip_suffix(suffix)?;
    (!serial.is_empty() && serial.bytes().all(|b| b.is_ascii_digit())).then_some(serial)
}

/// Writes `contents` to `path` through a `.tmp` sibling and a rename, so a reader sees the old file or the new one; creates the folder first.
pub fn write_atomic(path: &Path, contents: impl AsRef<[u8]>) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    std::fs::write(&tmp, contents).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}
