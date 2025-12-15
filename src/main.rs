mod cli;
mod config;

use anyhow::{Context as _, Result};
use clap::Parser;

#[cfg(feature = "report")]
mod report;

fn main() -> Result<()> {
    let args = cli::Args::parse();

    let path = args.config.unwrap_or_else(config::default_config_path);
    let cfg = config::Config::load_from_path(&path)
        .with_context(|| format!("failed to load config at {}", path.display()))?;

    // Default behavior: minimal confirmation (no config dump)
    println!(
        "loaded apogee config: schema_version={} default_shell={:?}",
        cfg.apogee.schema_version, cfg.apogee.default_shell
    );

    // Optional report: only compiled when the "report" feature is enabled
    #[cfg(feature = "report")]
    {
        if args.report != cli::ReportMode::Off {
            let text = report::build_report(&cfg, args.report);
            if let Some(out_path) = args.report_out {
                std::fs::write(&out_path, text)?;
                println!("wrote report: {}", out_path.display());
            } else {
                print!("{text}");
            }
        }
    }

    Ok(())
}
