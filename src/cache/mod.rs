//! Two-tier cache — mem LRU + disk, quantized keys, sized pool
//! Quantize: 8 cols x 4 rows. Pool: (num_cpus/2).clamp(2,6).

use ratatui::prelude::Rect;
use std::path::PathBuf;

pub struct Cache {
    // stub per docs/BACKEND.md §4
}

impl Cache {
    pub fn quantized(area: Rect) -> Rect {
        Rect { x: area.x, y: area.y, width: (area.width / 8) * 8, height: (area.height / 4) * 4 }
    }
    pub fn worker_threads() -> usize {
        (num_cpus::get() / 2).clamp(2, 6)
    }
}

pub fn clear_disk_cache() -> anyhow::Result<()> {
    Ok(())
}
