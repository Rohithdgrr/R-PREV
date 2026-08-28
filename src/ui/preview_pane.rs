//! Preview pane — match PreviewResult -> Paragraph/Table/Image via term::graphics
use crate::preview::{meta, PreviewResult};
use crate::term::{capabilities, graphics};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Widget, Wrap},
};

pub struct PreviewPaneWidget<'a> {
    pub result: &'a Option<PreviewResult>,
    pub selected_path: Option<&'a std::path::Path>,
}
impl<'a> ratatui::widgets::Widget for PreviewPaneWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        render_preview(area, buf, self.result, self.selected_path)
    }
}
pub fn render_preview(
    area: Rect,
    buf: &mut Buffer,
    result: &Option<PreviewResult>,
    selected_path: Option<&std::path::Path>,
) {
    let block = Block::default()
        .title(" Preview (Phase 1) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    // render block border first
    block.render(area, buf);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    match result {
        None => {
            let txt = "No preview yet.\nSelect a file.";
            Paragraph::new(txt)
                .style(Style::default().fg(Color::DarkGray))
                .wrap(Wrap { trim: false })
                .render(inner, buf);
        }
        Some(PreviewResult::Error { msg, fallback }) => {
            // red error + fallback if present
            let mut lines = vec![Line::from(Span::styled(
                format!("⚠ {}", msg),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ))];
            if let Some(fb) = fallback {
                lines.push(Line::from(""));
                match &**fb {
                    PreviewResult::Text { lines: fl, .. } => lines.extend(fl.clone()),
                    _ => lines.push(Line::from(Span::styled(
                        "no fallback",
                        Style::default().fg(Color::DarkGray),
                    ))),
                }
            } else if let Some(p) = selected_path {
                // show metadata as fallback
                lines.push(Line::from(""));
                let m = crate::preview::meta::file_meta(p);
                lines.push(Line::from(Span::styled(
                    format!("File: {}", p.display()),
                    Style::default().fg(Color::White),
                )));
                lines.push(Line::from(Span::styled(
                    format!("Size: {}", meta::human_size(m.size)),
                    Style::default().fg(Color::Cyan),
                )));
                lines.push(Line::from(Span::styled(
                    format!("MIME: {}", m.mime),
                    Style::default().fg(Color::Cyan),
                )));
            }
            Paragraph::new(lines).wrap(Wrap { trim: false }).render(inner, buf);
        }
        Some(PreviewResult::Text { lines, title, meta: m }) => {
            let header = Line::from(vec![
                Span::styled(
                    format!(" {} ", title),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" • {} • {}", meta::human_size(m.size), m.mime),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            // header row occupies one line, rest scrollable — Phase 1 just shows first inner.height-1 lines
            let mut all = vec![header, Line::from("")];
            let avail = inner.height.saturating_sub(2) as usize;
            all.extend(lines.iter().take(avail).cloned());
            Paragraph::new(all).wrap(Wrap { trim: false }).render(inner, buf);
        }
        Some(PreviewResult::Table { headers, rows, meta: m }) => {
            // Build header row
            let header_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
            let header_row =
                Row::new(headers.iter().map(|h| Cell::from(h.as_str()).style(header_style)))
                    .height(1);
            let table_rows: Vec<Row> =
                rows.iter().map(|r| Row::new(r.iter().map(|c| Cell::from(c.as_str())))).collect();
            let widths =
                headers.iter().map(|_| ratatui::layout::Constraint::Length(20)).collect::<Vec<_>>();
            let title =
                format!(" Table {} rows • {} • {} ", rows.len(), meta::human_size(m.size), m.mime);
            let block2 = Block::default().borders(Borders::NONE);
            Table::new(table_rows, widths).header(header_row).block(block2).render(inner, buf);
            // Note: title above already rendered via border; keep simple for Phase1
        }
        Some(PreviewResult::Image { rgba, meta: m }) => {
            // Show dims header + half-block image below
            let dims = m.dims.map(|(w, h)| format!("{}×{}", w, h)).unwrap_or_else(|| "?".into());
            let header = format!(" {} • {} • {} ", dims, meta::human_size(m.size), m.mime);
            // Render header as one line paragraph, then image below
            let header_area = Rect { x: inner.x, y: inner.y, width: inner.width, height: 1 };
            Paragraph::new(Line::from(Span::styled(header, Style::default().fg(Color::Cyan))))
                .render(header_area, buf);
            let img_area = Rect {
                x: inner.x,
                y: inner.y + 1,
                width: inner.width,
                height: inner.height.saturating_sub(1),
            };
            let caps = capabilities::detect();
            graphics::render_image_dispatch(rgba, caps, img_area, buf);
        }
        Some(PreviewResult::Directory { entries }) => {
            let mut lines = vec![Line::from(Span::styled(
                format!("Directory • {} entries", entries.len()),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ))];
            lines.push(Line::from(""));
            for e in entries.iter().take(inner.height as usize - 2) {
                let icon = if e.is_dir { "📁 " } else { "📄 " };
                lines.push(Line::from(Span::styled(
                    format!("{}{}", icon, e.name),
                    Style::default().fg(if e.is_dir { Color::Cyan } else { Color::White }),
                )));
            }
            Paragraph::new(lines).render(inner, buf);
        }
        Some(PreviewResult::Archive { entries, meta: m }) => {
            let title = format!(" Archive • {} entries • {} • {} ", entries.len(), meta::human_size(m.size), m.mime);
            let header = Line::from(Span::styled(title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
            let mut lines = vec![header, Line::from("")];
            for e in entries.iter().take(inner.height.saturating_sub(3) as usize) {
                lines.push(Line::from(Span::styled(format!("  {}  {} KB", e.name, e.size/1024), Style::default().fg(Color::White))));
            }
            Paragraph::new(lines).wrap(Wrap{trim:false}).render(inner, buf);
        }
        Some(PreviewResult::Audio { meta: am, waveform, duration }) => {
            let title = format!("♪ {} — {} • {:02}:{:02} • {} bars", am.title.clone().unwrap_or_else(|| "Audio".into()), am.artist.clone().unwrap_or_else(|| "Unknown".into()), duration.as_secs()/60, duration.as_secs()%60, waveform.len());
            let mut lines = vec![
                Line::from(Span::styled(title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled("Space: play/pause  s: stop  (rodio + symphonia)", Style::default().fg(Color::DarkGray))),
                Line::from(""),
            ];
            if !waveform.is_empty() {
                // Sparkline-like waveform as block characters
                let max = *waveform.iter().max().unwrap_or(&1) as f32;
                let spark: String = waveform.iter().map(|&v| {
                    let lvl = (v as f32 / max * 7.0) as usize;
                    match lvl { 0=>" ",1=>"_",2=>".",3=>"-",4=>"*",5=>"#",6=>"█",7=>"█", _=>" " }
                }).collect();
                lines.push(Line::from(Span::styled(spark, Style::default().fg(Color::Magenta))));
            } else {
                lines.push(Line::from(Span::styled("(no waveform — empty or unsupported codec)", Style::default().fg(Color::DarkGray))));
            }
            lines.push(Line::from(""));
            // Also embed fallback file meta as last line
            lines.push(Line::from(Span::styled("Tip: navigate away auto-stops audio", Style::default().fg(Color::DarkGray))));
            Paragraph::new(lines).wrap(Wrap{trim:false}).render(inner, buf);
        }
    }
}
