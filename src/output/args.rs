use std::path::PathBuf;

use clap::{Args, ValueEnum};

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Hex,
    Mot,
}

/// Output configuration for the build command.
#[derive(Args, Debug, Clone)]
pub struct OutputArgs {
    /// HexView-compatible CLI string (include -o <file> for output path).
    /// Use @1, @2, ... to reference input blocks in the order given.
    #[arg(
        short = 'o',
        long,
        value_name = "HEXVIEW",
        help = "HexView-compatible CLI string (include -o <file>)"
    )]
    pub hexview: String,

    /// Export used values as a JSON report.
    #[arg(long, value_name = "FILE", help = "Export used values as JSON")]
    pub export_json: Option<PathBuf>,

    /// Show detailed build statistics.
    #[arg(long, help = "Show detailed build statistics")]
    pub stats: bool,

    /// Suppress all output except errors.
    #[arg(long, help = "Suppress all output except errors")]
    pub quiet: bool,
}
