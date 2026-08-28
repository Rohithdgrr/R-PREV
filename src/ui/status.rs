//! Status bar helper
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

pub fn render_status_line(left: &str, right: &str) -> Paragraph<'static> {
    let line = Line::from(vec![
        Span::styled(left.to_string(), Style::default().fg(Color::White).bg(Color::DarkGray)),
        Span::styled(right.to_string(), Style::default().fg(Color::Cyan).bg(Color::DarkGray)),
    ]);
    Paragraph::new(line).style(Style::default().bg(Color::DarkGray))
}
