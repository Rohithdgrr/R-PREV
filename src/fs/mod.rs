//! FS — list_dir sorted dirs-first, hidden toggle, symlink depth guard
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub is_symlink: bool,
    pub symlink_target: Option<PathBuf>,
}

impl Entry {
    pub fn display_prefix(&self) -> &'static str {
        if self.is_symlink {
            "🔗 "
        } else if self.is_dir {
            "📁 "
        } else {
            "📄 "
        }
    }
    pub fn human_size(&self) -> String {
        human_size(self.size)
    }
}

fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{:.1} {}", size, UNITS[unit])
    }
}

/// Whitelisted media/doc formats to show (per user request: only .jpg/.png/.pdf/.doc etc)
// Directories are always shown; files only if extension whitelisted
pub const ALLOWED_FORMATS: &[&str] = &[
    // images
    "jpg","jpeg","png","gif","webp","bmp","svg","ico","tiff","heic","heif",
    // docs
    "pdf","doc","docx","xls","xlsx","ods","ppt","pptx","odt","rtf","txt","md","csv","tsv",
    // archives (previewable)
    "zip","tar","gz","tgz","7z","rar",
    // audio/video
    "mp3","flac","wav","ogg","m4a","aac","opus","wma","mp4","mkv","webm","avi","mov","m4v","flv",
];

pub fn is_allowed_format(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let lower = ext.to_lowercase();
        ALLOWED_FORMATS.contains(&lower.as_str())
    } else {
        false
    }
}

pub fn list_dir_filtered(path: &Path, show_hidden: bool, filter_formats: bool) -> anyhow::Result<Vec<Entry>> {
    let mut all = list_dir_raw(path, show_hidden)?;
    if filter_formats {
        all.retain(|e| e.is_dir || is_allowed_format(&e.path));
    }
    Ok(all)
}

fn list_dir_raw(path: &Path, show_hidden: bool) -> anyhow::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    let read = std::fs::read_dir(path)
        .map_err(|e| anyhow::anyhow!("cannot read dir {}: {}", path.display(), e))?;
    for e in read {
        let e = e?;
        let name = e.file_name().to_string_lossy().into_owned();
        if !show_hidden && name.starts_with('.') {
            continue;
        }
        let entry_path = e.path();
        let md = e.file_type()?;
        let is_symlink = md.is_symlink();
        let (is_dir, size, target) = if is_symlink {
            match resolve_symlink(&entry_path, 10) {
                Ok((p, md)) => (md.is_dir(), md.len(), Some(p)),
                Err(_) => (false, 0, None), // broken symlink
            }
        } else {
            let m = e.metadata()?;
            (m.is_dir(), m.len(), None)
        };
        entries.push(Entry {
            path: entry_path,
            name,
            is_dir,
            size,
            is_symlink,
            symlink_target: target,
        });
    }
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    Ok(entries)
}

/// Backward-compatible alias: raw list now filtered via wrapper
pub fn list_dir(path: &Path, show_hidden: bool) -> anyhow::Result<Vec<Entry>> {
    // By default (per user request) only show whitelisted formats + dirs
    list_dir_filtered(path, show_hidden, true)
}

fn resolve_symlink(path: &Path, max_depth: usize) -> anyhow::Result<(PathBuf, std::fs::Metadata)> {
    let mut current = path.to_path_buf();
    let mut depth = 0;
    loop {
        if depth > max_depth {
            anyhow::bail!("symlink depth exceeded {}", max_depth);
        }
        let target = std::fs::read_link(&current)?;
        let next = if target.is_absolute() {
            target
        } else {
            current.parent().unwrap_or_else(|| Path::new(".")).join(target)
        };
        current = next;
        // Check if still symlink
        let ft = std::fs::symlink_metadata(&current)?;
        if !ft.is_symlink() {
            let md = std::fs::metadata(&current)?;
            return Ok((current, md));
        }
        depth += 1;
    }
}

/// Human-readable size for status bar
pub fn human_size_public(bytes: u64) -> String {
    human_size(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[test]
    fn test_list_dir_sorting() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        fs::create_dir(dir.join("b_dir")).unwrap();
        fs::create_dir(dir.join("a_dir")).unwrap();
        fs::write(dir.join("z_file.png"), b"hello").unwrap();
        fs::write(dir.join("a_file.jpg"), b"hello").unwrap();
        let entries = list_dir(dir, false).unwrap();
        assert!(entries[0].is_dir);
        assert_eq!(entries[0].name, "a_dir");
        assert_eq!(entries[1].name, "b_dir");
        assert_eq!(entries[2].name, "a_file.jpg");
        assert_eq!(entries[3].name, "z_file.png");
    }
    #[test]
    fn test_hidden_filter() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        fs::write(dir.join(".hidden.png"), b"x").unwrap();
        fs::write(dir.join("visible.png"), b"x").unwrap();
        let no_hidden = list_dir(dir, false).unwrap();
        assert_eq!(no_hidden.len(), 1);
        assert_eq!(no_hidden[0].name, "visible.png");
        let with_hidden = list_dir(dir, true).unwrap();
        assert_eq!(with_hidden.len(), 2);
    }
    #[test]
    fn test_allowed_formats_filter() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        fs::write(dir.join("photo.jpg"), b"x").unwrap();
        fs::write(dir.join("doc.pdf"), b"x").unwrap();
        fs::write(dir.join("script.exe"), b"x").unwrap();
        fs::write(dir.join("archive.zip"), b"x").unwrap();
        let entries = list_dir(dir, false).unwrap();
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"photo.jpg"));
        assert!(names.contains(&"doc.pdf"));
        assert!(names.contains(&"archive.zip"));
        assert!(!names.contains(&"script.exe"), "exe should be filtered");
        // raw should still see exe
        let raw = list_dir_raw(dir, false).unwrap();
        assert!(raw.iter().any(|e| e.name=="script.exe"));
        assert!(is_allowed_format(std::path::Path::new("file.PDF")));
        assert!(!is_allowed_format(std::path::Path::new("file.exe")));
    }
}
