//! SearchBar — fuzzy live filter (simple substring Phase 2, nucleo-matcher in future)
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    buffer::Buffer,
    widgets::Widget,
};

pub struct SearchState {
    pub query: String,
    pub active: bool,
    pub selected_idx: usize,
}

impl SearchState {
    pub fn new() -> Self { Self { query: String::new(), active: false, selected_idx: 0 } }
    pub fn enter(&mut self) { self.active = true; self.query.clear(); self.selected_idx = 0; }
    pub fn exit(&mut self) { self.active = false; self.query.clear(); }
    pub fn push(&mut self, c: char) { self.query.push(c); }
    pub fn pop(&mut self) { self.query.pop(); }
}

/// Returns indices of files matching query (Phase 2 simple case-insensitive substring, fast)
pub fn filter_files(files: &[crate::fs::Entry], query: &str) -> Vec<usize> {
    if query.is_empty() { return (0..files.len()).collect(); }
    let q = query.to_lowercase();
    let mut out = Vec::new();
    for (idx, e) in files.iter().enumerate() {
        if e.name.to_lowercase().contains(&q) {
            out.push(idx);
        }
    }
    out
}

pub struct SearchBarWidget<'a> {
    pub query: &'a str,
}

impl<'a> Widget for SearchBarWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::ALL).title(" Search (type, Esc to clear) ").border_style(Style::default().fg(Color::Yellow));
        let inner = block.inner(area);
        block.render(area, buf);
        let line = Line::from(vec![
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::styled(self.query.to_string(), Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::White)),
        ]);
        Paragraph::new(line).render(inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::Entry;
    use std::path::PathBuf;
    fn entry(name: &str) -> Entry { Entry { path: PathBuf::from(name), name: name.to_string(), is_dir: false, size: 0, is_symlink: false, symlink_target: None } }
    #[test]
    fn filter_substring() {
        let files = vec![entry("ARCHITECTURE.md"), entry("BACKEND.md"), entry("README.md")];
        let res = filter_files(&files, "arch");
        assert_eq!(res, vec![0]);
    }
    #[test]
    fn empty_query_all() {
        let files = vec![entry("a"), entry("b")];
        let res = filter_files(&files, "");
        assert_eq!(res.len(), 2);
    }
}
