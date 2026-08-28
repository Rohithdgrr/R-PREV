//! CsvHandler — delimiter sniff ,;|tab + comfy-table
use crate::error::PreviewError;
use crate::preview::{meta, FileMeta, PreviewCtx, PreviewHandler, PreviewResult};
use std::path::Path;

pub struct CsvHandler;
impl PreviewHandler for CsvHandler {
    fn name(&self) -> &'static str {
        "csv"
    }
    fn priority(&self, path: &Path, _mime: &str, _magic: &[u8]) -> u8 {
        match path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase().as_str() {
            "csv" => 85,
            "tsv" => 85,
            "psv" => 80,
            _ => 2,
        }
    }
    fn file_size_limit(&self) -> u64 {
        10 * 1024 * 1024
    }

    fn preview_blocking(&self, ctx: PreviewCtx) -> Result<PreviewResult, PreviewError> {
        let path = &ctx.path;
        let delim = sniff_delimiter(path);
        let file = std::fs::File::open(path).map_err(PreviewError::Io)?;
        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(delim)
            .flexible(true)
            .trim(csv::Trim::All)
            .has_headers(true)
            .from_reader(file);
        let headers: Vec<String> =
            rdr.headers().map(|h| h.iter().map(|s| s.to_string()).collect()).unwrap_or_default();
        let mut rows: Vec<Vec<String>> = Vec::new();
        for rec in rdr.records().take(100) {
            let rec = rec.map_err(|e| PreviewError::Decode(format!("csv parse: {}", e)))?;
            rows.push(rec.iter().map(|s| s.to_string()).collect());
        }
        let total_note = if rows.len() == 100 { " (first 100 rows)" } else { "" };
        let meta = meta::file_meta(path);
        // If no headers/rows, fallback to text error
        if headers.is_empty() && rows.is_empty() {
            return Err(PreviewError::Decode("empty csv".into()));
        }
        // Attach hint as first row? We'll encode via fallback text not needed; return Table
        Ok(PreviewResult::Table { headers, rows, meta })
    }
}

fn sniff_delimiter(path: &Path) -> u8 {
    use std::io::Read;
    let mut buf = [0u8; 1024];
    let n = std::fs::File::open(path).and_then(|mut f| f.read(&mut buf)).unwrap_or(0);
    let snippet = &buf[..n];
    let mut counts = [(b',', 0), (b';', 0), (b'\t', 0), (b'|', 0)];
    for &b in snippet {
        for (c, cnt) in counts.iter_mut() {
            if b == *c {
                *cnt += 1;
            }
        }
    }
    counts.sort_by_key(|b| std::cmp::Reverse(b.1));
    if counts[0].1 == 0 {
        b','
    } else {
        counts[0].0
    }
}
