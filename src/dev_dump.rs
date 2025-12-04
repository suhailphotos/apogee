// src/dev_dump.rs

use crate::config::Config;

/// Dev-only full config dump.
/// You can delete this file and its call in main.rs whenever you’re done.
pub fn dump_full_config(cfg: &Config) {
    println!("\n========== FULL CONFIG DUMP ==========\n");
    println!("{:#?}", cfg);
    println!("\n======================================\n");
}
