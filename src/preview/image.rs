//! ImageHandler — image crate, 50MB guard, 10000x10000 guard
use crate::error::PreviewError;
use crate::preview::{meta, FileMeta, PreviewCtx, PreviewHandler, PreviewResult};
use ::image::GenericImageView;
use std::path::Path;

pub struct ImageHandler;
impl PreviewHandler for ImageHandler {
    fn name(&self) -> &'static str {
        "image"
    }
    fn priority(&self, path: &Path, mime: &str, magic: &[u8]) -> u8 {
        let is_magic_image = matches!(magic.get(0..4), Some([0x89, 0x50, 0x4E, 0x47]))
            || matches!(magic.get(0..3), Some([0xFF, 0xD8, 0xFF]))
            || magic.starts_with(b"GIF8")
            || magic.starts_with(b"BM")
            || mime.starts_with("image/");
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let is_ext_image = matches!(
            ext.as_str(),
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" | "ico" | "tiff"
        );
        if is_magic_image || is_ext_image {
            100
        } else {
            2
        }
    }
    fn file_size_limit(&self) -> u64 {
        50 * 1024 * 1024
    }

    fn preview_blocking(&self, ctx: PreviewCtx) -> Result<PreviewResult, PreviewError> {
        let path = &ctx.path;
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if size > self.file_size_limit() && !crate::preview::is_force_preview() {
            return Err(PreviewError::TooLarge(size, self.file_size_limit()));
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let meta = meta::file_meta(path);
        if ext == "svg" {
            // Phase 1 SVG: render to png via resvg if possible, otherwise fallback to text preview of SVG source
            // Try resvg path with tiny-skia brought in via resvg
            // For simplicity in Phase 1, return Text fallback if resvg fails — still useful
            match render_svg(path) {
                Ok((w, h, img)) => {
                    let meta2 = FileMeta { dims: Some((w, h)), ..meta };
                    return Ok(PreviewResult::Image { rgba: img, meta: meta2 });
                }
                Err(e) => {
                    // Fallback to text mode so user at least sees SVG source
                    let txt = std::fs::read_to_string(path)
                        .unwrap_or_else(|_| format!("svg read error: {}", e));
                    let truncated: String = txt.chars().take(2000).collect();
                    let line = ratatui::text::Line::from(ratatui::text::Span::styled(
                        format!(
                            "SVG preview failed: {} — showing source (first 2000 chars):\n{}",
                            e, truncated
                        ),
                        ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
                    ));
                    return Ok(PreviewResult::Text {
                        lines: vec![line],
                        title: path.display().to_string(),
                        meta,
                    });
                }
            }
        }
        // raster via image crate
        let img =
            image::open(path).map_err(|e| PreviewError::Decode(format!("image open: {}", e)))?;
        let (w, h) = img.dimensions();
        if w > 10000 || h > 10000 {
            return Err(PreviewError::Decode(format!("image dims too large {}x{}", w, h)));
        }
        let meta2 = FileMeta { dims: Some((w, h)), ..meta };
        Ok(PreviewResult::Image { rgba: img, meta: meta2 })
    }
}

fn render_svg(path: &Path) -> Result<(u32, u32, image::DynamicImage), String> {
    let svg_str = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    if svg_str.len() > 2 * 1024 * 1024 {
        return Err("SVG too large (>2MB)".into());
    }
    if svg_str.matches('<').count() > 50_000 {
        return Err("SVG too many nodes".into());
    }
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(&svg_str, &opt).map_err(|e| e.to_string())?;
    let w = tree.size().width().ceil() as u32;
    let h = tree.size().height().ceil() as u32;
    if w == 0 || h == 0 {
        return Err("SVG zero size".into());
    }
    if w > 10000 || h > 10000 {
        return Err(format!("SVG dims {}x{} too large", w, h));
    }
    // resvg 0.43 uses tiny-skia via resvg crate re-export? Use tiny-skia directly if available as transitive dep
    // Fallback: try to allocate via tiny-skia if present, else error
    // We attempt dynamic: create pixmap via `resvg::tiny_skia`
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).ok_or("pixmap alloc fail".to_string())?;
    resvg::render(&tree, resvg::tiny_skia::Transform::default(), &mut pixmap.as_mut());
    let img = image::DynamicImage::ImageRgba8(
        image::RgbaImage::from_raw(w, h, pixmap.data().to_vec())
            .ok_or("rgba convert fail".to_string())?,
    );
    Ok((w, h, img))
}
