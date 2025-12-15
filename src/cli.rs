use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "apogee", version, about)]
pub struct Args {
    /// Path to config.toml (defaults to XDG path)
    #[arg(long)]
    pub config: Option<std::path::PathBuf>,

    /// Print a debug report (requires --features report at compile time)
    #[arg(long, value_enum, default_value_t = ReportMode::Off)]
    pub report: ReportMode,

    /// Write report to a file instead of stdout (requires --features report)
    #[arg(long)]
    pub report_out: Option<std::path::PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ReportMode {
    Off,
    Summary,
    Full,
}
