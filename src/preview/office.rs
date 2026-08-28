//! OfficeHandler — docx-rs + calamine + in-house pptx via zip+quick-xml
use std::path::Path;
use calamine::Reader as _;
use crate::preview::{FileMeta, PreviewResult, PreviewHandler, PreviewCtx, meta};
use crate::error::PreviewError;
use ratatui::style::{Color, Style, Modifier};
use ratatui::text::{Line, Span};

pub struct OfficeHandler;
impl PreviewHandler for OfficeHandler {
    fn name(&self) -> &'static str { "office" }
    fn priority(&self, path: &Path, _mime: &str, _magic: &[u8]) -> u8 {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        match ext.as_str() {
            "docx" => 90,
            "xlsx"|"xls"|"ods" => 90,
            "pptx"|"ppt" => 90,
            _ => 2,
        }
    }
    fn file_size_limit(&self) -> u64 { 100 * 1024 * 1024 }

    fn preview_blocking(&self, ctx: PreviewCtx) -> Result<PreviewResult, PreviewError> {
        let path = &ctx.path;
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if size > self.file_size_limit() && !crate::preview::is_force_preview() {
            return Err(PreviewError::TooLarge(size, self.file_size_limit()));
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        match ext.as_str() {
            "docx" => preview_docx(path),
            "xlsx"|"xls"|"ods" => preview_xlsx_with_sheet(path, 0),
            "pptx"|"ppt" => preview_pptx(path, 0),
            _ => Err(PreviewError::Unsupported(format!("office ext {}", ext))),
        }
    }
}

// --- DOCX ---
fn preview_docx(path: &Path) -> Result<PreviewResult, PreviewError> {
    let docx = docx_rs::read_docx(&std::fs::read(path).map_err(PreviewError::Io)?)
        .map_err(|e| PreviewError::Decode(format!("docx read: {}", e)))?;
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(format!("DOCX • {} • {}", path.display(), meta::human_size(std::fs::metadata(path).map(|m| m.len()).unwrap_or(0))), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))));
    lines.push(Line::from(""));
    let mut para_count = 0usize;
    for child in docx.document.children.iter() {
        match child {
            docx_rs::DocumentChild::Paragraph(p) => {
                let mut text = String::new();
                for run_child in &p.children {
                    if let docx_rs::ParagraphChild::Run(r) = run_child {
                        for rc in &r.children {
                            if let docx_rs::RunChild::Text(t) = rc { text.push_str(&t.text); }
                        }
                    }
                }
                let trimmed = text.trim().to_string();
                if trimmed.is_empty() { continue; }
                // Heading detection: style "Heading1" etc
                let is_heading = p.property.style.as_ref().map(|s| s.val.contains("Heading")).unwrap_or(false);
                let style = if is_heading { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::White) };
                lines.push(Line::from(Span::styled(trimmed, style)));
                para_count += 1;
                if para_count > 400 { lines.push(Line::from(Span::styled("… truncated (400 paras)", Style::default().fg(Color::Yellow)))); break; }
            }
            docx_rs::DocumentChild::Table(t) => {
                let row_count = t.rows.len();
                lines.push(Line::from(Span::styled(format!("  [Table {} rows]", row_count), Style::default().fg(Color::DarkGray))));
                for row_child in &t.rows {
                    let docx_rs::TableChild::TableRow(row) = row_child;
                    let mut row_text: Vec<String> = Vec::new();
                    for cell_child in &row.cells {
                        let docx_rs::TableRowChild::TableCell(cell) = cell_child;
                        let mut cell_text = String::new();
                        for content in &cell.children {
                            if let docx_rs::TableCellContent::Paragraph(paragraph) = content {
                                for pc in &paragraph.children {
                                    if let docx_rs::ParagraphChild::Run(r) = pc {
                                        for rc in &r.children { if let docx_rs::RunChild::Text(t) = rc { cell_text.push_str(&t.text); } }
                                    }
                                }
                                cell_text.push(' ');
                            }
                        }
                        row_text.push(cell_text.trim().to_string());
                    }
                    if !row_text.iter().all(|s| s.is_empty()) {
                        lines.push(Line::from(Span::styled(format!("  | {} |", row_text.join(" | ")), Style::default().fg(Color::White))));
                    }
                }
            }
            _ => {}
        }
    }
    if para_count == 0 && lines.len() <= 2 {
        lines.push(Line::from(Span::styled("(empty document or no paragraphs found)", Style::default().fg(Color::DarkGray))));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Tab: sheets N/A  n/p: disabled", Style::default().fg(Color::DarkGray))));
    Ok(PreviewResult::Text { lines, title: path.display().to_string(), meta: meta::file_meta(path) })
}

// --- XLSX / XLS / ODS via calamine ---
pub fn preview_xlsx_with_sheet(path: &Path, sheet_idx: usize) -> Result<PreviewResult, PreviewError> {
    let mut workbook = calamine::open_workbook_auto(path).map_err(|e| PreviewError::Decode(format!("calamine open: {}", e)))?;
    let sheets = workbook.sheet_names().to_vec();
    if sheets.is_empty() { return Err(PreviewError::Decode("no sheets".into())); }
    let idx = sheet_idx.min(sheets.len()-1);
    let sheet_name = sheets[idx].clone();
    let range = workbook.worksheet_range(&sheet_name).map_err(|e| PreviewError::Decode(format!("worksheet_range: {}", e)))?;
    // Collect headers (first row) + up to 100 rows after
    let mut rows: Vec<Vec<String>> = Vec::new();
    for row in range.rows().take(101) {
        rows.push(row.iter().map(|c| c.to_string()).collect());
    }
    let headers = if !rows.is_empty() { rows.remove(0) } else { Vec::new() };
    // Prepend sheet selector line: keep for UI to show Tab hint
    let meta2 = meta::file_meta(path);
    // Store sheet info in meta.mime suffix hack? Instead we encode sheets via title: we'll return Table with headers/rows; app.rs will keep sheet_idx state and show title with sheet name
    // Add hint as first header if needed? For Phase3 we just return Table; UI will render sheet name in title via app's state
    Ok(PreviewResult::Table { headers, rows, meta: meta2 })
}

pub fn xlsx_sheet_names(path: &Path) -> Vec<String> {
    calamine::open_workbook_auto(path).map(|wb| wb.sheet_names().to_vec()).unwrap_or_default()
}

// --- PPTX in-house via zip + quick-xml ---
pub fn preview_pptx(path: &Path, slide_idx: usize) -> Result<PreviewResult, PreviewError> {
    let data = std::fs::read(path).map_err(PreviewError::Io)?;
    let reader = std::io::Cursor::new(data);
    let mut zip = zip::ZipArchive::new(reader).map_err(|e| PreviewError::Decode(format!("zip open pptx: {}", e)))?;
    // Collect slide files ppt/slides/slideN.xml
    let mut slide_names: Vec<String> = Vec::new();
    for i in 0..zip.len() {
        if let Ok(file) = zip.by_index(i) {
            let name = file.name().to_string();
            if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                slide_names.push(name);
            }
        }
    }
    slide_names.sort();
    if slide_names.is_empty() {
        return Err(PreviewError::Decode("no slides found in pptx".into()));
    }
    let total = slide_names.len();
    let idx = slide_idx.min(total-1);
    let slide_name = slide_names[idx].clone();
    // Re-open zip to get file content
    let data2 = std::fs::read(path).map_err(PreviewError::Io)?;
    let mut zip2 = zip::ZipArchive::new(std::io::Cursor::new(data2)).map_err(|e| PreviewError::Decode(format!("zip open 2: {}", e)))?;
    let mut xml_str = String::new();
    {
        let mut f = zip2.by_name(&slide_name).map_err(|e| PreviewError::Decode(format!("slide {}: {}", slide_name, e)))?;
        use std::io::Read;
        f.read_to_string(&mut xml_str).map_err(|e| PreviewError::Decode(format!("read slide: {}", e)))?;
    }
    let texts = extract_a_t(&xml_str);
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(format!("PPTX • {} • Slide {}/{} • n/p to paginate", path.display(), idx+1, total), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))));
    lines.push(Line::from(""));
    if texts.is_empty() {
        lines.push(Line::from(Span::styled("(no text runs <a:t> in slide)", Style::default().fg(Color::DarkGray))));
    } else {
        for (i, t) in texts.iter().enumerate() {
            let text = t.trim();
            if text.is_empty() { continue; }
            // Heading heuristic: first item bold cyan
            let style = if i==0 { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::White) };
            lines.push(Line::from(Span::styled(text.to_string(), style)));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(format!("— End slide {} — Tab: N/A  n/p to navigate", idx+1), Style::default().fg(Color::DarkGray))));
    Ok(PreviewResult::Text { lines, title: format!("{} [Slide {}/{}]", path.display(), idx+1, total), meta: meta::file_meta(path) })
}

pub fn pptx_slide_count(path: &Path) -> usize {
    let data = match std::fs::read(path) { Ok(d)=>d, Err(_)=>return 0 };
    let mut zip = match zip::ZipArchive::new(std::io::Cursor::new(data)) { Ok(z)=>z, Err(_)=>return 0 };
    let mut count = 0usize;
    for i in 0..zip.len() {
        if let Ok(f) = zip.by_index(i) { if f.name().starts_with("ppt/slides/slide") && f.name().ends_with(".xml") { count+=1; } }
    }
    count
}

fn extract_a_t(xml: &str) -> Vec<String> {
    // quick-xml extract <a:t> text nodes
    let mut reader = quick_xml::Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut texts = Vec::new();
    let mut buf = Vec::new();
    let mut in_a_t = false;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(e)) if e.name().as_ref()==b"a:t" => { in_a_t = true; }
            Ok(quick_xml::events::Event::End(e)) if e.name().as_ref()==b"a:t" => { in_a_t = false; }
            Ok(quick_xml::events::Event::Text(e)) if in_a_t => {
                if let Ok(s) = e.unescape() { texts.push(s.into_owned()); }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    texts
}
