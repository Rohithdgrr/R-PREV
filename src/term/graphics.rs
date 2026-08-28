//! Graphics — Phase 4: Kitty chunked 4096, Sixel ESC P q, iTerm2 1337, fallback half-block `▀`
use crate::term::capabilities::Capabilities;
use ::image::DynamicImage;
use ratatui::buffer::Buffer;
use ratatui::prelude::*;

/// Half-block fallback: 2 vertical pixels per cell `▀` truecolor
pub fn render_image_halfblock(img: &DynamicImage, area: Rect, buf: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let target_w = (area.width as u32) * 2;
    let target_h = (area.height as u32) * 2;
    let resized =
        image::imageops::resize(img, target_w, target_h, image::imageops::FilterType::Triangle);
    for y in 0..area.height {
        for x in 0..area.width {
            let px_top = resized.get_pixel(x as u32 * 2, y as u32 * 2);
            let px_bot = resized.get_pixel(x as u32 * 2, y as u32 * 2 + 1);
            let top = Color::Rgb(px_top[0], px_top[1], px_top[2]);
            let bot = Color::Rgb(px_bot[0], px_bot[1], px_bot[2]);
            let cell = buf.cell_mut((area.x + x, area.y + y)).unwrap();
            cell.set_symbol("▀");
            cell.set_style(Style::default().fg(top).bg(bot));
        }
    }
}

/// Encode PNG base64 for Kitty / iTerm2
pub fn encode_png_base64(img: &DynamicImage) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let mut buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buf);
    let _ = img.write_to(&mut cursor, image::ImageFormat::Png);
    STANDARD.encode(&buf)
}

/// Build Kitty graphics protocol chunk (4096 byte chunks) — ESC _ G a=T,f=32,s=w,v=h,m=1;BASE64 ESC \  ... ESC _ G m=0; ESC \
pub fn kitty_escape(img: &DynamicImage, _area: Rect) -> String {
    let b64 = encode_png_base64(img);
    let (w, h) = (img.width(), img.height());
    // Kitty: a=T transmitter, f=32 RGBA, s/v original dims, m flag for continuing
    let chunk_size = 4096;
    let mut out = String::new();
    let mut first = true;
    for chunk in b64.as_bytes().chunks(chunk_size) {
        let m = if chunk.as_ptr().addr() + chunk.len() < b64.as_bytes().as_ptr().addr() + b64.len() { 1 } else { 0 };
        let payload = String::from_utf8_lossy(chunk);
        if first {
            out.push_str(&format!("\x1b_Ga=T,f=32,s={},v={},m={};{}\x1b\\", w, h, m, payload));
            first = false;
        } else {
            out.push_str(&format!("\x1b_Gm={};{}\x1b\\", m, payload));
        }
    }
    out
}

/// iTerm2 inline File= ESC ]1337;File=inline=1;width=w;height=h:BASE64 BEL
pub fn iterm2_escape(img: &DynamicImage) -> String {
    let b64 = encode_png_base64(img);
    let (w, h) = (img.width(), img.height());
    format!("\x1b]1337;File=inline=1;width={};height={}:{}\x07", w, h, b64)
}

/// Sixel placeholder: ESC P q "1;1;w;h ... data ESC \  — full Sixel encoding is large; we fallback to half-block for Sixel in Phase4 and delegate to viuer in future.
pub fn sixel_escape(_img: &DynamicImage) -> String {
    // Not implementing full Sixel byte encoding here; fallback to half-block via buffer cells.
    String::new()
}

pub fn render_image_dispatch(img: &DynamicImage, caps: Capabilities, area: Rect, buf: &mut Buffer) {
    // Phase 4: dispatch by capability, but still render via buffer cells for half-block path.
    // Kitty/Sixel/iTerm2 require writing escapes directly to stdout, not via Ratatui Buffer.
    // Ratatui Buffer cannot emit kitty escapes, so Phase 4 still uses half-block for in-pane TUI.
    // True Kitty/Sixel will be used in fullscreen `f` mode via direct stdout write (future), half-block here is correct.
    // We probe caps to choose future path, but fallback is half-block verified in Windows Terminal + WezTerm.
    let _ = caps;
    // If we were in fullscreen Kitty, we would do: print!("{}", kitty_escape(img, area)); return
    render_image_halfblock(img, area, buf);
}
