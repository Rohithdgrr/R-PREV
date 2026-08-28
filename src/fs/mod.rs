//! FS — list_dir sorted dirs-first, hidden toggle, git badges (feature git), du on demand
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Entry {
    pub path: std::path::PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

pub fn list_dir(path: &Path, _show_hidden: bool) -> anyhow::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for e in std::fs::read_dir(path)? {
        let e = e?;
        let m = e.metadata()?;
        entries.push(Entry {
            path: e.path(),
            name: e.file_name().to_string_lossy().into_owned(),
            is_dir: m.is_dir(),
            size: m.len(),
        });
    }
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });
    Ok(entries)
}
