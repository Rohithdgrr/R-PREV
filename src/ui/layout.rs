//! Layout — Phase 2: 30/70 + footer + search bar + fullscreen + help
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Mode};

pub fn draw(f: &mut Frame, app: &mut App) {
    // Search bar takes one extra row when active
    let (main_area, status_area, search_area) = if app.mode == Mode::Search {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1), Constraint::Length(3)])
            .split(f.area());
        (chunks[0], chunks[1], Some(chunks[2]))
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(f.area());
        (chunks[0], chunks[1], None)
    };

    // Fullscreen preview: preview fills main_area
    if app.mode == Mode::FullscreenPreview || app.fullscreen {
        draw_preview(f, app, main_area);
        draw_status(f, app, status_area);
        if let Some(area) = search_area { draw_search(f, app, area); }
        if app.mode == Mode::Help { draw_help(f); }
        if app.mode == Mode::InputNewFile || app.mode == Mode::InputNewFolder { draw_input(f, app); }
        return;
    }

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(main_area);

    draw_file_list(f, app, main_chunks[0]);
    draw_preview(f, app, main_chunks[1]);
    draw_status(f, app, status_area);
    if let Some(area) = search_area { draw_search(f, app, area); }

    if app.mode == Mode::Help {
        draw_help(f);
    }
    if app.mode == Mode::InputNewFile || app.mode == Mode::InputNewFolder {
        draw_input(f, app);
    }
}

fn draw_file_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let visible = app.visible_indices();
    let items: Vec<ListItem> = visible.iter().enumerate().map(|(vis_idx, &file_idx)| {
        let e = &app.files[file_idx];
        let is_selected = vis_idx == app.selected;
        let dir_marker = if e.is_dir { "/" } else { "" };
        let symlink = if e.is_symlink { " →" } else { "" };
        let content = format!("{}{}{}", e.name, dir_marker, symlink);
        let style = if is_selected {
            Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
        } else if e.is_dir {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else if e.is_symlink {
            Style::default().fg(Color::Magenta)
        } else {
            Style::default().fg(Color::White)
        };
        let prefix = if is_selected { "▶ " } else { "  " };
        ListItem::new(Line::from(vec![Span::styled(format!("{}{}", prefix, content), style)]))
    }).collect();

    let title = if app.filtered.is_some() {
        format!(" Files [{} filtered / {} total] ", visible.len(), app.files.len())
    } else {
        format!(" Files [{}] ", app.files.len())
    };
    let block = Block::default().title(title).borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray));
    let list = List::new(items).block(block).highlight_style(Style::default().bg(Color::Blue));
    f.render_widget(list, area);
}

fn draw_preview(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    // Show loading indicator if preview_loading
    if app.preview_loading && app.preview.is_none() {
        let block = Block::default().title(" Preview (loading…) ").borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow));
        let p = Paragraph::new(Line::from(Span::styled("Loading…", Style::default().fg(Color::Yellow)))).block(block);
        f.render_widget(p, area);
        return;
    }
    let selected_path = app.selected_entry().map(|e| e.path.as_path());
    f.render_widget(
        crate::ui::preview_pane::PreviewPaneWidget { result: &app.preview, selected_path },
        area,
    );
}

fn draw_status(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let cur = app.selected_entry().map(|e| e.name.as_str()).unwrap_or("-");
    let visible_len = app.visible_indices().len();
    let pos = if visible_len==0 { "0/0".to_string() } else { format!("{}/{}", app.selected+1, visible_len) };
    let dir = app.current_dir.display().to_string();
    let mode_str = match app.mode {
        Mode::Normal => "NORMAL",
        Mode::Search => "SEARCH",
        Mode::Help => "HELP",
        Mode::FullscreenPreview => "FULLSCREEN",
        Mode::InputNewFile => "NEW FILE",
        Mode::InputNewFolder => "NEW FOLDER",
    };
    let hidden_str = if app.show_hidden { "hidden:ON" } else { "hidden:OFF" };
    let filter_str = if app.filter_formats_only { "filter:MEDIA" } else { "filter:ALL" };
    let audio_str = if app.audio_sink.is_some() { "♪ playing" } else { "" };
    let loading_str = if app.preview_loading { " loading…" } else { "" };
    let txt = format!(" {} | {} | {} | {} | {} | {}{} | f:fullscreen /:search a:new file A:new folder m:toggle filter q:quit ", mode_str, pos, cur, hidden_str, filter_str, audio_str, loading_str);
    let dir_line = format!(" Dir: {} ", dir);
    let status = crate::ui::status::render_status_line(&txt, &dir_line);
    f.render_widget(status, area);
}

fn draw_search(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    f.render_widget(crate::ui::search::SearchBarWidget { query: &app.search.query }, area);
}

fn draw_help(f: &mut Frame) {
    let area = centered_rect(75, 70, f.area());
    f.render_widget(ratatui::widgets::Clear, area);
    let text = vec![
        Line::from(Span::styled(" tui-preview — Help (Phase 2) ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled("Navigation:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from("  j / ↓        Next    k / ↑  Prev   g Top  G Bottom"),
        Line::from("  Enter        Enter dir  Backspace Parent"),
        Line::from("  h            Toggle hidden  f Toggle fullscreen  q/Esc/Ctrl+C Quit"),
        Line::from(""),
        Line::from(Span::styled("Search & Preview:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from("  /            Fuzzy search — type to filter, Enter select, Esc clear"),
        Line::from("  Enter on TooLarge → force preview"),
        Line::from("  Space        Play/pause audio  s Stop audio"),
        Line::from("  a            New file  A/N  New folder  o Open external  m Toggle format filter"),
        Line::from("  Only media/doc formats shown by default (.jpg .png .pdf .doc etc) + dirs"),
        Line::from(""),
        Line::from(Span::styled("Phase 2:", Style::default().fg(Color::Yellow))),
        Line::from("  • Cache mem LRU 100 + disk 500MB quantized 8×4  sized pool"),
        Line::from("  • Async preview JoinSet + abort on scroll  5s timeout"),
        Line::from("  • Audio mp3/flac/wav via rodio + waveform sparkline"),
        Line::from("  • Tracing to ~/.cache/tui-preview/debug.log (RUST_LOG=debug)"),
        Line::from(""),
        Line::from(Span::styled("Press ? or Esc or q to close", Style::default().fg(Color::DarkGray))),
    ];
    let block = Block::default().borders(Borders::ALL).title(" Help ").border_style(Style::default().fg(Color::Yellow));
    let p = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn draw_input(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.area());
    f.render_widget(ratatui::widgets::Clear, area);
    let is_folder = app.mode == Mode::InputNewFolder;
    let title = if is_folder { " New Folder (Enter to create, Esc to cancel) " } else { " New File (Enter to create, Esc to cancel) " };
    let block = Block::default().title(title).borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(area);
    let prompt = if is_folder { "Folder name: " } else { "File name: " };
    let input_line = format!("{}{}█", prompt, app.input_buffer);
    let mut lines = vec![
        Line::from(Span::styled(input_line, Style::default().fg(Color::White))),
    ];
    if let Some(err) = &app.input_error {
        lines.push(Line::from(Span::styled(format!("Error: {}", err), Style::default().fg(Color::Red))));
    } else {
        lines.push(Line::from(Span::styled("Tip: name without path separator", Style::default().fg(Color::DarkGray))));
    }
    let p = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ]).split(r);
    Layout::default().direction(Direction::Horizontal).constraints([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ]).split(popup_layout[1])[1]
}
