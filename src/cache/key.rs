//! Cache key — quantized area + mtime + size + version
use std::path::Path;
use sha2::{Sha256, Digest};
use ratatui::layout::Rect;

const CACHE_VERSION: u8 = 1;

pub fn quantized(area: Rect) -> Rect {
    Rect { x: area.x, y: area.y, width: (area.width / 8) * 8, height: (area.height / 4) * 4 }
}

pub fn cache_key(path: &Path, area: Rect, extra: &str) -> String {
    let q = quantized(area);
    let meta = std::fs::metadata(path);
    let (mtime, size) = meta.map(|m| (
        m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0),
        m.len()
    )).unwrap_or((0,0));
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let input = format!("{}:{}:{}:{}:{}:{}:{}", canon.display(), mtime, size, q.width, q.height, CACHE_VERSION, extra);
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quantize_rounds() {
        let r = Rect { x:0,y:0,width: 15,height: 7 };
        let q = quantized(r);
        assert_eq!(q.width, 8);
        assert_eq!(q.height, 4);
    }
    #[test]
    fn same_pixel_same_key() {
        let p = std::path::Path::new("fixtures/sample.txt");
        std::fs::write(p, b"hello").ok();
        let k1 = cache_key(p, Rect{x:0,y:0,width:80,height:24}, "");
        let k2 = cache_key(p, Rect{x:0,y:0,width:81,height:24}, ""); // quantized same
        assert_eq!(k1,k2);
    }
}
