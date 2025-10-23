use clap::{Args, ValueEnum};

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Hex,
    Mot,
}

#[derive(Args, Debug, Clone)]
pub struct OutputArgs {
    #[arg(
        short = 'o',
        long,
        value_name = "DIR",
        default_value = "out",
        help = "Output directory"
    )]
    pub out: String,

    #[arg(
        long,
        value_name = "STR",
        default_value = "",
        help = "Optional prefix to prepend to each block name in output filename"
    )]
    pub prefix: String,

    #[arg(
        long,
        value_name = "STR",
        default_value = "",
        help = "Optional suffix to append to each block name in output filename"
    )]
    pub suffix: String,

    #[arg(
        long,
        value_name = "N",
        default_value_t = 32u16,
        value_parser = clap::value_parser!(u16).range(1..=64),
        help = "Number of bytes per HEX data record (1..=64)",
    )]
    pub record_width: u16,

    #[arg(
        long,
        value_enum,
        default_value_t = OutputFormat::Hex,
        help = "Output format: hex or mot",
    )]
    pub format: OutputFormat,

    #[arg(long, help = "Emit a single combined file instead of one per block")]
    pub combined: bool,

    #[arg(long, help = "Show detailed build statistics")]
    pub stats: bool,

    #[arg(long, help = "Suppress all output except errors")]
    pub quiet: bool,
}
