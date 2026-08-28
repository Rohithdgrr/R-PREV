//! Two-tier cache — mem LRU + disk 500MB, quantized keys, disk png thumbs + PreviewResult serialization via bincode? Phase 2: store rendered preview as in-memory Arc, disk only for image thumbs.
pub mod key;

use std::collections::hash_map::DefaultHasher;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use lru::LruCache;
use std::num::NonZeroUsize;

use crate::preview::PreviewResult;

pub struct Cache {
    mem: Mutex<LruCache<String, Arc<PreviewResult>>>,
    dir: PathBuf,
    max_disk_bytes: u64,
    max_mem_entries: usize,
}

impl Cache {
    pub fn new() -> Self {
        let dir = directories::ProjectDirs::from("com", "tui-preview", "tui-preview")
            .map(|d| d.cache_dir().join("tui-preview").join("thumbs"))
            .unwrap_or_else(|| PathBuf::from(".cache").join("tui-preview").join("thumbs"));
        Self::with_dir(dir, 500 * 1024 * 1024, 100)
    }
    pub fn with_dir(dir: PathBuf, max_disk_bytes: u64, max_mem_entries: usize) -> Self {
        let _ = std::fs::create_dir_all(&dir);
        let cap = NonZeroUsize::new(max_mem_entries.max(1)).unwrap();
        Self { mem: Mutex::new(LruCache::new(cap)), dir, max_disk_bytes, max_mem_entries }
    }
    pub fn quantized(area: ratatui::layout::Rect) -> ratatui::layout::Rect {
        key::quantized(area)
    }
    pub fn worker_threads() -> usize {
        (num_cpus::get() / 2).clamp(2, 6)
    }
    pub fn mem_get(&self, k: &str) -> Option<Arc<PreviewResult>> {
        let mut g = self.mem.lock().ok()?;
        g.get(k).cloned()
    }
    pub fn mem_put(&self, k: String, v: Arc<PreviewResult>) {
        if let Ok(mut g) = self.mem.lock() { g.put(k, v); }
    }
    pub fn disk_path(&self, key: &str, ext: &str) -> PathBuf {
        self.dir.join(format!("{}.{}", key, ext))
    }
    /// Try disk thumb (only for images in Phase 2) — returns None if not image cache
    pub fn disk_get_image(&self, key: &str) -> Option<Arc<PreviewResult>> {
        let p = self.disk_path(key, "png");
        if !p.exists() { return None; }
        // Load png via image crate if still valid (mtime already in key, so always valid if exists)
        let img = image::open(&p).ok()?;
        let meta = crate::preview::meta::file_meta(Path::new("disk_cache"));
        Some(Arc::new(PreviewResult::Image { rgba: img, meta }))
    }
    pub fn disk_put_image(&self, key: &str, img: &image::DynamicImage) {
        let p = self.disk_path(key, "png");
        let _ = std::fs::create_dir_all(self.dir.clone());
        let _ = img.save(&p);
        self.evict_if_needed();
    }
    pub fn get(&self, key: &str) -> Option<Arc<PreviewResult>> {
        if let Some(v) = self.mem_get(key) { return Some(v); }
        if let Some(v) = self.disk_get_image(key) {
            self.mem_put(key.to_string(), v.clone());
            return Some(v);
        }
        None
    }
    pub fn put(&self, key: String, v: Arc<PreviewResult>) {
        // If image, also write disk thumb async via spawn_blocking caller
        if let PreviewResult::Image { rgba, .. } = &*v {
            self.disk_put_image(&key, rgba);
        }
        self.mem_put(key, v);
    }
    fn evict_if_needed(&self) {
        // Remove oldest files until under cap
        let total: u64 = walkdir::WalkDir::new(&self.dir).into_iter().filter_map(|e| e.ok()).filter_map(|e| e.metadata().ok()).map(|m| m.len()).sum();
        if total <= self.max_disk_bytes { return; }
        let mut entries: Vec<_> = walkdir::WalkDir::new(&self.dir).into_iter().filter_map(|e| e.ok()).filter(|e| e.file_type().is_file()).collect();
        entries.sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()).unwrap_or(std::time::SystemTime::UNIX_EPOCH));
        let mut cur = total;
        for e in entries {
            if cur <= self.max_disk_bytes { break; }
            let sz = e.metadata().map(|m| m.len()).unwrap_or(0);
            let _ = std::fs::remove_file(e.path());
            cur = cur.saturating_sub(sz);
        }
    }
    pub fn cache_dir(&self) -> &Path { &self.dir }
}
impl Default for Cache { fn default() -> Self { Self::new() } }

pub fn clear_disk_cache() -> anyhow::Result<()> {
    let dir = Cache::new().dir;
    if dir.exists() { std::fs::remove_dir_all(&dir)?; }
    Ok(())
}
