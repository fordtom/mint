use crate::layout::args::LayoutArgs;
use crate::output::args::OutputArgs;
use crate::data::args::DataArgs;
use clap::Parser;

// Top-level CLI parser. Sub-sections are flattened from sub-Args structs.
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Build flash blocks from layout files and data sources (Excel, Postgres, or REST)",
    after_help = "For more information, visit https://crates.io/crates/mint-cli"
)]
pub struct Args {
    #[command(flatten)]
    pub layout: LayoutArgs,

    #[command(flatten)]
    pub data: DataArgs,

    #[command(flatten)]
    pub output: OutputArgs,
}
