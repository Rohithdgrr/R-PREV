//! File metadata helpers
use super::FileMeta;
use std::path::Path;

pub fn file_meta(path: &Path) -> FileMeta {
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let mtime = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
    let mime = mime_guess::from_path(path).first_or_octet_stream().to_string();
    let dims = None; // image dims filled by image handler
    FileMeta { size, mtime, mime, dims }
}

pub fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut s = bytes as f64;
    let mut u = 0;
    while s >= 1024.0 && u < UNITS.len() - 1 {
        s /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{} {}", bytes, UNITS[u])
    } else {
        format!("{:.1} {}", s, UNITS[u])
    }
}
