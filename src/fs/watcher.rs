//! Watcher — only compiled with `watch` feature (notify is optional)
//! Fixes review: notify was documented but missing from Cargo.toml default deps.
#![cfg(feature = "watch")]
// stub: notify::RecommendedWatcher live reload
