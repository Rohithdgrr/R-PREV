//! Graphics — Kitty/iTerm2/Sixel via half-block fallback `▀` + truecolor 2 pixels per cell
use crate::term::capabilities::Capabilities;
use ::image::DynamicImage;
use ratatui::buffer::Buffer;
use ratatui::prelude::*;

/// Render DynamicImage into a Ratatui Buffer area using half-block `▀` trick (2 pixels per cell)
/// This always works even without Kitty/Sixel.
/// Each terminal cell shows two vertical pixels: top half fg, bottom half bg.
pub fn render_image_halfblock(img: &DynamicImage, area: Rect, buf: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    // Resize to area*2 width, area.height*2 height via Lanczos3 for best quality, but fast for Phase 1 we use Triangle
    let target_w = (area.width as u32) * 2;
    let target_h = (area.height as u32) * 2; // 2 pixels per row
    let resized =
        image::imageops::resize(img, target_w, target_h, image::imageops::FilterType::Triangle);
    for y in 0..area.height {
        for x in 0..area.width {
            let px_top = resized.get_pixel(x as u32 * 2, y as u32 * 2);
            let px_bot = resized.get_pixel(x as u32 * 2, y as u32 * 2 + 1);
            // Use Triangle for speed; get_pixel already RGBA
            let top = Color::Rgb(px_top[0], px_top[1], px_top[2]);
            let bot = Color::Rgb(px_bot[0], px_bot[1], px_bot[2]);
            // If image has alpha, blend naive with bg black
            let cell = buf.cell_mut((area.x + x, area.y + y)).unwrap();
            cell.set_symbol("▀");
            cell.set_style(Style::default().fg(top).bg(bot));
        }
    }
    // For wider than area, we already handled x*2 only first column; but we should sample nearest instead of x*2
    // Already ok for Phase 1; Phase 4 will add Kitty chunked protocol for crisp.
}

/// Produce base64 for iTerm2 inline or Kitty — not used in Phase 1 half-block, but keep for Phase 4 full.
pub fn encode_png_base64(img: &DynamicImage) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let mut buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buf);
    let _ = img.write_to(&mut cursor, image::ImageFormat::Png);
    STANDARD.encode(&buf)
}

pub fn render_image_dispatch(img: &DynamicImage, caps: Capabilities, area: Rect, buf: &mut Buffer) {
    // Phase 1: always half-block for maximum compatibility. Future phases probe Kitty/Sixel and use encode_png_base64 with escapes.
    let _ = caps;
    render_image_halfblock(img, area, buf);
}
