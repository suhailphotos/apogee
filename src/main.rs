mod cli;
mod config;
mod context;

use anyhow::{Context as _, Result};
use clap::Parser;

#[cfg(feature = "report")]
mod report;

fn main() -> Result<()> {
    let args = cli::Args::parse();

    // Build minimal context (XDG_CONFIG_HOME + hostname + platform + shell)
    let mut ctx = context::ContextEnv::new()?;

    // Locate (or create default) config path
    let config_path = ctx.locate_config(args.config.as_ref())?;

    // Load config
    let cfg = config::Config::load_from_path(&config_path)
        .with_context(|| format!("failed to load config at {}", config_path.display()))?;

    // Locate (or create default) secrets path
    let secrets_path = ctx.locate_secrets()?;

    // Load secrets and fill missing env entries
    let secrets = context::load_secrets_env(&secrets_path)?;
    let secrets_loaded = secrets.len();
    ctx.merge_secrets_fill_missing(&secrets);

    // Optional runtime context dump
    if args.dump_context {
        let text = ctx.debug_dump(args.effective_redact());
        if let Some(out_path) = args.dump_context_out.as_ref() {
            std::fs::write(out_path, text)?;
            println!("wrote context dump: {}", out_path.display());
        } else {
            print!("{text}");
        }
    }

    println!(
        "loaded apogee config: schema_version={} default_shell={:?} platform={} host={} secrets_loaded={}",
        cfg.apogee.schema_version,
        cfg.apogee.default_shell,
        // use the method so it isn't dead code
        match ctx.platform() {
            config::Platform::Mac => "mac",
            config::Platform::Linux => "linux",
            config::Platform::Windows => "windows",
            config::Platform::Wsl => "wsl",
            config::Platform::Other => "other",
        },
        ctx.host(),
        secrets_loaded
    );

    // Optional report (requires --features report)
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
