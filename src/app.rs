//! App state + event loop — see docs/ARCHITECTURE.md §2/6 and docs/WORKING.md §3
//! Review fixes: quantized cache key, sized pool, centralized timeout via Router::dispatch, unwind kept.

use std::path::PathBuf;
use std::sync::Arc;

use ratatui::prelude::Rect;

use crate::cache::Cache;
use crate::config::Config;
use crate::preview::{PreviewResult, Router};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Search,
    Help,
    FullscreenPreview,
}

pub struct App {
    pub files: Vec<crate::fs::Entry>,
    pub filtered: Vec<usize>, // indices into files when searching
    pub selected: usize,
    pub mode: Mode,
    pub preview: Option<PreviewResult>,
    pub config: Arc<Config>,
    pub cache: Arc<Cache>,
    pub router: Arc<Router>,
    pub dirty: bool,
    pub current_dir: PathBuf,
}

impl App {
    pub fn new(
        path: PathBuf,
        config: Arc<Config>,
        cache: Arc<Cache>,
        router: Arc<Router>,
    ) -> anyhow::Result<Self> {
        let current_dir =
            if path.is_file() { path.parent().unwrap().to_path_buf() } else { path.clone() };
        let files = crate::fs::list_dir(&current_dir, config.general.show_hidden)?;
        Ok(Self {
            files,
            filtered: vec![],
            selected: 0,
            mode: Mode::Normal,
            preview: None,
            config,
            cache,
            router,
            dirty: true,
            current_dir,
        })
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        // stub: j/k, /, f, ?, q, etc. — moves selected, toggles mode, marks dirty
        // enqueues async preview via router.dispatch with centralized 5s timeout
        let _ = key;
        self.dirty = true;
    }

    pub fn quantized_area(area: Rect) -> Rect {
        // 8 cols x 4 rows quantization — see docs/PERFORMANCE.md §4
        Rect { x: area.x, y: area.y, width: (area.width / 8) * 8, height: (area.height / 4) * 4 }
    }
}

pub async fn run(path: PathBuf, cfg: Config) -> anyhow::Result<()> {
    let _ = (path, cfg);
    // stub: init terminal, event_stream, JoinSet with sized pool (num_cpus/2).clamp(2,6),
    // Router::dispatch with catch_unwind + 5s timeout, render loop
    anyhow::bail!("src skeleton — implement per docs/TODO.md Phase 0")
}
