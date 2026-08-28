//! Terminal capabilities detect — Kitty/Sixel/iTerm2/truecolor + probe via CSI ? query (Phase 4)
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
        || term_program.contains("wezterm")
        || term_program.contains("kitty");
    let iterm2 = term_program.contains("iterm") || term.contains("iterm");
    // Heuristic Sixel: foot, xterm-sixel, Windows Terminal 1.22+ sets WT_SESSION or WT_SIXEL
    let sixel_env = env::var("WT_SIXEL").is_ok() || env::var("WT_SESSION").is_ok() || term.contains("sixel") || term.contains("foot");
    let sixel = (term.contains("xterm") && !kitty && !iterm2) || sixel_env;
    let truecolor = colorterm.contains("truecolor")
        || colorterm.contains("24bit")
        || term.contains("truecolor")
        || env::var("COLORTERM").unwrap_or_default().contains("truecolor");
    let unicode = true;
    let caps = Capabilities { kitty, sixel, iterm2, truecolor, unicode };
    // Active CSI probe disabled in Phase 4 to avoid TTY blocking; heuristic above is sufficient.
    // Full viuer-style probe (DA1) will be added later via proper term probe crate.
    caps
}

/// Try active CSI probe: ESC[?1;2c via DECTEC query — not fully reliable, so timeout 50ms and fallback to env.
/// We probe Sixel via DA1 response containing `4` and Kitty via `__kitty` in response to graphics query.
/// For Phase 4 we keep it simple: try to read response within deadline, else heuristic.
fn probe_sixel_kitty() -> Option<Capabilities> {
    // Save raw mode not needed for probe; we just try non-blocking read
    // We send CSI c (DA1) and check for Sixel bit (4) — but limited without raw mode we skip actual read
    // To avoid blocking TUI startup, we just return None and rely on env heuristic above.
    // Full viuer-style probe will be done via `term::probe` crate in future.
    None
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
