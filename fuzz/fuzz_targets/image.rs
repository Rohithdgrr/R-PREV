//! cargo-fuzz target for image handler — seed with fixtures/*.png
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| { let _ = image::load_from_memory(data); });
