use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "apogee", version, about)]
pub struct Args {
    /// Path to config.toml (overrides APOGEE_CONFIG and XDG default)
    #[arg(long)]
    pub config: Option<std::path::PathBuf>,

    /// Print a debug report (requires --features report at compile time)
    #[arg(long, value_enum, default_value_t = ReportMode::Off)]
    pub report: ReportMode,

    /// Write report to a file instead of stdout (requires --features report)
    #[arg(long)]
    pub report_out: Option<std::path::PathBuf>,

    /// Dump runtime context (resolved config/secrets paths + env map)
    #[arg(long, default_value_t = false)]
    pub dump_context: bool,

    /// Write context dump to a file instead of stdout
    #[arg(long)]
    pub dump_context_out: Option<std::path::PathBuf>,

    /// Redact secret-like values in context dump (default on)
    #[arg(long, default_value_t = true)]
    pub redact: bool,

    /// Disable redaction in context dump
    #[arg(long = "no-redact", default_value_t = false)]
    pub no_redact: bool,
}

impl Args {
    pub fn effective_redact(&self) -> bool {
        if self.no_redact { false } else { self.redact }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ReportMode {
    Off,
    Summary,
    Full,
}
