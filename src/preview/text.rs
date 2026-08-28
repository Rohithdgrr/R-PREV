//! TextHandler — syntect highlight + encoding_rs + binary guard + markdown + limit 2MB/5000 lines
use crate::error::PreviewError;
use crate::preview::{meta, PreviewCtx, PreviewHandler, PreviewResult};
use content_inspector::{inspect, ContentType};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::path::Path;

const MAX_BYTES: usize = 2 * 1024 * 1024;
const MAX_LINES: usize = 5000;

pub struct TextHandler;
impl PreviewHandler for TextHandler {
    fn name(&self) -> &'static str {
        "text"
    }
    fn priority(&self, path: &Path, mime: &str, _magic: &[u8]) -> u8 {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        match ext.as_str() {
            "txt" | "md" | "markdown" | "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "json"
            | "toml" | "yaml" | "yml" | "sh" | "bash" | "zsh" | "ps1" | "log" | "ini" | "conf"
            | "css" | "html" | "xml" | "c" | "cpp" | "h" | "hpp" | "go" | "java" | "kt" | "rb"
            | "php" | "swift" | "dart" | "r" | "sql" | "env" => 70,
            _ if mime.starts_with("text/") => 60,
            _ => 5,
        }
    }
    fn file_size_limit(&self) -> u64 {
        MAX_BYTES as u64 * 2
    }

    fn preview_blocking(&self, ctx: PreviewCtx) -> Result<PreviewResult, PreviewError> {
        let path = &ctx.path;
        let bytes = read_limited(path, MAX_BYTES)?;
        if bytes.is_empty() {
            return Ok(PreviewResult::Text {
                lines: vec![Line::from(Span::styled(
                    "(empty file)",
                    Style::default().fg(Color::DarkGray),
                ))],
                title: path.display().to_string(),
                meta: meta::file_meta(path),
            });
        }
        let inspector = inspect(&bytes[..bytes.len().min(1024)]);
        if inspector == ContentType::BINARY {
            return Err(PreviewError::Decode(format!(
                "binary file \u{2022} {} \u{2022} not text previewable",
                meta::human_size(bytes.len() as u64)
            )));
        }
        let text = String::from_utf8(bytes.clone()).unwrap_or_else(|_| {
            let (cow, _, had_errors) = encoding_rs::WINDOWS_1252.decode(&bytes);
            if had_errors {
                String::from_utf8_lossy(&bytes).into_owned()
            } else {
                cow.into_owned()
            }
        });
        let truncated_bytes =
            std::fs::metadata(path).map(|m| m.len()).unwrap_or(0) > MAX_BYTES as u64;
        let mut lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();
        let total_lines = lines.len();
        let truncated_lines = total_lines > MAX_LINES;
        if truncated_lines {
            lines.truncate(MAX_LINES);
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if ext == "md" || ext == "markdown" {
            return Ok(PreviewResult::Text {
                lines: render_markdown(&lines.join("\n")),
                title: path.display().to_string(),
                meta: meta::file_meta(path),
            });
        }

        let highlighted = highlight(&lines, &ext);
        let mut out = highlighted;
        if truncated_lines || truncated_bytes {
            let hint = format!(
                "\u{2026} truncated ({} lines total, {} shown{})",
                total_lines,
                out.len(),
                if truncated_bytes { ", file >2MB" } else { "" }
            );
            out.push(Line::from(Span::styled(
                hint,
                Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC),
            )));
        }
        Ok(PreviewResult::Text {
            lines: out,
            title: path.display().to_string(),
            meta: meta::file_meta(path),
        })
    }
}

fn read_limited(path: &Path, limit: usize) -> Result<Vec<u8>, PreviewError> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).map_err(PreviewError::Io)?;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    let mut total = 0usize;
    loop {
        let n = f.read(&mut chunk).map_err(PreviewError::Io)?;
        if n == 0 {
            break;
        }
        if total + n > limit {
            buf.extend_from_slice(&chunk[..limit - total]);
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        total += n;
    }
    Ok(buf)
}

fn highlight(lines: &[String], ext: &str) -> Vec<Line<'static>> {
    let syntax = syntect::parsing::SyntaxSet::load_defaults_newlines();
    let theme_set = syntect::highlighting::ThemeSet::load_defaults();
    let syn = syntax
        .find_syntax_by_extension(ext)
        .or_else(|| syntax.find_syntax_by_extension("txt"))
        .unwrap_or_else(|| syntax.find_syntax_plain_text());
    let theme = &theme_set.themes["base16-ocean.dark"];
    let mut h = syntect::easy::HighlightLines::new(syn, theme);
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        let ranges: Vec<(syntect::highlighting::Style, &str)> =
            h.highlight_line(line, &syntax).unwrap_or_default();
        if ranges.is_empty() {
            out.push(Line::from(Span::raw(line.clone())));
        } else {
            let spans: Vec<Span> = ranges
                .into_iter()
                .map(|(style, txt)| {
                    let fg = style.foreground;
                    let color = Color::Rgb(fg.r, fg.g, fg.b);
                    let mut s = Style::default().fg(color);
                    if style.font_style.contains(syntect::highlighting::FontStyle::BOLD) {
                        s = s.add_modifier(Modifier::BOLD);
                    }
                    if style.font_style.contains(syntect::highlighting::FontStyle::ITALIC) {
                        s = s.add_modifier(Modifier::ITALIC);
                    }
                    Span::styled(txt.to_string(), s)
                })
                .collect();
            out.push(Line::from(spans));
        }
    }
    out
}

fn render_markdown(md: &str) -> Vec<Line<'static>> {
    use pulldown_cmark::{Event as MdEvent, Parser, Tag, TagEnd};
    let parser = Parser::new(md);
    let mut lines: Vec<Line> = Vec::new();
    let mut cur_spans: Vec<Span> = Vec::new();
    let mut is_heading = false;
    let mut is_code = false;
    let mut list_depth = 0usize;
    for ev in parser {
        match ev {
            MdEvent::Start(Tag::Heading { level, .. }) => {
                is_heading = true;
                cur_spans.push(Span::styled(
                    format!("{} ", "#".repeat(level as usize)),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ));
            }
            MdEvent::End(TagEnd::Heading(_)) => {
                is_heading = false;
                lines.push(Line::from(std::mem::take(&mut cur_spans)));
            }
            MdEvent::Start(Tag::CodeBlock(_)) => {
                is_code = true;
            }
            MdEvent::End(TagEnd::CodeBlock) => {
                is_code = false;
                lines.push(Line::from(std::mem::take(&mut cur_spans)));
            }
            MdEvent::Start(Tag::List(_)) => {
                list_depth += 1;
            }
            MdEvent::End(TagEnd::List(_)) => {
                list_depth = list_depth.saturating_sub(1);
            }
            MdEvent::Start(Tag::Item) => {
                cur_spans.push(Span::styled(
                    "\u{2022} ".to_string(),
                    Style::default().fg(Color::Yellow),
                ));
            }
            MdEvent::End(TagEnd::Item) => {
                lines.push(Line::from(std::mem::take(&mut cur_spans)));
            }
            MdEvent::Start(Tag::Paragraph) => {}
            MdEvent::End(TagEnd::Paragraph) => {
                if !cur_spans.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut cur_spans)));
                    lines.push(Line::from(""));
                }
            }
            MdEvent::Text(t) => {
                let style = if is_heading {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else if is_code {
                    Style::default().fg(Color::Green).bg(Color::Rgb(30, 30, 30))
                } else {
                    Style::default().fg(Color::White)
                };
                cur_spans.push(Span::styled(t.to_string(), style));
            }
            MdEvent::Code(c) => {
                cur_spans.push(Span::styled(format!("`{}`", c), Style::default().fg(Color::Green)));
            }
            MdEvent::SoftBreak | MdEvent::HardBreak => {
                lines.push(Line::from(std::mem::take(&mut cur_spans)));
            }
            _ => {}
        }
    }
    if !cur_spans.is_empty() {
        lines.push(Line::from(cur_spans));
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::styled(md.to_string(), Style::default().fg(Color::White))));
    }
    lines
}
