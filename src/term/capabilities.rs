//! Terminal capabilities detect — Kitty/Sixel/iTerm2/truecolor
use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    pub kitty: bool,
    pub sixel: bool,
    pub iterm2: bool,
    pub truecolor: bool,
    pub unicode: bool,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self { kitty: false, sixel: false, iterm2: false, truecolor: false, unicode: true }
    }
}

pub fn detect() -> Capabilities {
    let term = env::var("TERM").unwrap_or_default().to_lowercase();
    let term_program = env::var("TERM_PROGRAM").unwrap_or_default().to_lowercase();
    let colorterm = env::var("COLORTERM").unwrap_or_default().to_lowercase();
    let kitty = env::var("KITTY_WINDOW_ID").is_ok()
        || term_program.contains("kitty")
        || term_program.contains("ghostty")
        || term_program.contains("wezterm");
    let iterm2 = term_program.contains("iterm") || term.contains("iterm");
    // Windows Terminal 1.22+ has Sixel, Foot, etc. We probe conservatively: env hints
    let sixel = term.contains("xterm") && !kitty && !iterm2 || env::var("WT_SIXEL").is_ok();
    let truecolor = colorterm.contains("truecolor")
        || colorterm.contains("24bit")
        || term.contains("truecolor");
    let unicode = true; // assume unicode unless proven
    Capabilities { kitty, sixel, iterm2, truecolor, unicode }
}

pub fn describe(caps: Capabilities) -> String {
    format!(
        "Kitty:{} Sixel:{} iTerm2:{} TrueColor:{} Unicode:{}",
        if caps.kitty { "yes" } else { "no" },
        if caps.sixel { "yes" } else { "no" },
        if caps.iterm2 { "yes" } else { "no" },
        if caps.truecolor { "yes" } else { "no" },
        if caps.unicode { "yes" } else { "no" }
    )
}
