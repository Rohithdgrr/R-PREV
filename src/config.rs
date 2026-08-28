//! Config — TOML load, worker_threads = (num_cpus/2).clamp
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug, Default, Clone)]
pub struct Config {
    #[serde(default)]
    pub general: General,
    #[serde(default)]
    pub cache: CacheCfg,
    #[serde(default)]
    pub preview: PreviewCfg,
}

#[derive(Deserialize, Debug, Clone)]
pub struct General {
    pub theme: String,
    pub show_hidden: bool,
    pub preview_delay_ms: u64,
}
impl Default for General {
    fn default() -> Self {
        Self { theme: "dark".into(), show_hidden: false, preview_delay_ms: 50 }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct CacheCfg {
    pub max_disk_mb: u64,
    pub mem_entries: usize,
    pub worker_threads: usize,
}
impl Default for CacheCfg {
    fn default() -> Self {
        Self {
            max_disk_mb: 500,
            mem_entries: 100,
            worker_threads: (num_cpus::get() / 2).clamp(2, 6),
        }
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct PreviewCfg {
    #[serde(default)]
    pub max_image_mb: u64,
}

impl Config {
    fn from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let cfg: Self = toml::from_str(&content)?;
        Ok(cfg)
    }
}

pub fn config_path() -> PathBuf {
    if let Some(proj) = directories::ProjectDirs::from("com", "tui-preview", "tui-preview") {
        proj.config_dir().join("config.toml")
    } else {
        PathBuf::from("config.toml")
    }
}

pub fn load(theme_override: Option<&str>) -> anyhow::Result<Config> {
    let mut cfg = if config_path().exists() {
        Config::from_file(&config_path()).unwrap_or_default()
    } else {
        Config::default()
    };
    if let Some(t) = theme_override {
        cfg.general.theme = t.into();
    }
    if cfg.cache.worker_threads == 0 {
        cfg.cache.worker_threads = (num_cpus::get() / 2).clamp(2, 6);
    }
    Ok(cfg)
}

pub fn init_default_config() -> anyhow::Result<PathBuf> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if !path.exists() {
        let default_toml = r#"[general]
theme = "dark"
show_hidden = false
preview_delay_ms = 50

[cache]
max_disk_mb = 500
mem_entries = 100
# worker_threads will be auto-sized if omitted: (num_cpus/2).clamp(2,6)

[preview]
max_image_mb = 50
"#;
        std::fs::write(&path, default_toml)?;
    }
    Ok(path)
}
