//! Event mapping — crossterm key -> Action (Phase 2: search, fullscreen, audio)
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Up,
    Down,
    Top,
    Bottom,
    Enter,
    Parent,
    ToggleHidden,
    ToggleHelp,
    ToggleFullscreen,
    Quit,
    Search,
    CopyPath,
    PlayPause,
    Stop,
    Char(char),
    BackspaceChar,
    Esc,
    NextPage,      // n
    PrevPage,      // p
    NextSheet,     // Tab
    PrevSheet,     // Shift+Tab
    OpenExternal,  // o
    NewFile,       // a
    NewFolder,     // A / N
    ToggleFormatFilter, // m
}

pub fn key_to_action(key: KeyEvent) -> Option<Action> {
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), KeyModifiers::NONE) => Some(Action::Quit),
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(Action::Quit),
        (KeyCode::Esc, _) => Some(Action::Esc),
        (KeyCode::Char('k'), KeyModifiers::NONE) | (KeyCode::Up, _) => Some(Action::Up),
        (KeyCode::Char('j'), KeyModifiers::NONE) | (KeyCode::Down, _) => Some(Action::Down),
        (KeyCode::Char('g'), KeyModifiers::NONE) => Some(Action::Top),
        (KeyCode::Char('G'), _) if key.code == KeyCode::Char('G') => Some(Action::Bottom),
        (KeyCode::Char('h'), KeyModifiers::NONE) => Some(Action::ToggleHidden),
        (KeyCode::Char('?'), _) => Some(Action::ToggleHelp),
        (KeyCode::Char('/'), _) => Some(Action::Search),
        (KeyCode::Char('f'), KeyModifiers::NONE) => Some(Action::ToggleFullscreen),
        (KeyCode::Char('n'), KeyModifiers::NONE) => Some(Action::NextPage),
        (KeyCode::Char('p'), KeyModifiers::NONE) => Some(Action::PrevPage),
        (KeyCode::Tab, KeyModifiers::NONE) => Some(Action::NextSheet),
        (KeyCode::BackTab, _) => Some(Action::PrevSheet),
        (KeyCode::Char('o'), KeyModifiers::NONE) => Some(Action::OpenExternal),
        (KeyCode::Char('a'), KeyModifiers::NONE) => Some(Action::NewFile),
        (KeyCode::Char('A'), _) => Some(Action::NewFolder),
        (KeyCode::Char('N'), _) => Some(Action::NewFolder),
        (KeyCode::Char('m'), KeyModifiers::NONE) => Some(Action::ToggleFormatFilter),
        (KeyCode::Char(' '), _) => Some(Action::PlayPause),
        (KeyCode::Char('s'), KeyModifiers::NONE) => Some(Action::Stop),
        (KeyCode::Enter, _) => Some(Action::Enter),
        (KeyCode::Backspace, _) => Some(Action::BackspaceChar),
        (KeyCode::Char(c), KeyModifiers::NONE) if c != 'q' && c != 'j' && c != 'k' && c != 'g' && c != 'h' && c != 'f' && c != 's' && c != 'n' && c != 'p' && c != 'o' && c != 'a' && c != 'm' => Some(Action::Char(c)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    fn k(code: KeyCode) -> KeyEvent {
        KeyEvent { code, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: KeyEventState::NONE }
    }
    #[test]
    fn j_maps_down() { assert_eq!(key_to_action(k(KeyCode::Char('j'))), Some(Action::Down)); }
    #[test]
    fn k_maps_up() { assert_eq!(key_to_action(k(KeyCode::Char('k'))), Some(Action::Up)); }
    #[test]
    fn f_maps_fullscreen() { assert_eq!(key_to_action(k(KeyCode::Char('f'))), Some(Action::ToggleFullscreen)); }
    #[test]
    fn q_maps_quit() { assert_eq!(key_to_action(k(KeyCode::Char('q'))), Some(Action::Quit)); }
}
