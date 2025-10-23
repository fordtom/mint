use crate::layout::args::LayoutArgs;
use crate::output::args::OutputArgs;
use crate::variant::args::VariantArgs;
use clap::Parser;

// Top-level CLI parser. Sub-sections are flattened from sub-Args structs.
#[derive(Parser, Debug)]
#[command(author, version, about = "Build flash blocks from layout + Excel data")]
pub struct Args {
    #[command(flatten)]
    pub layout: LayoutArgs,

    #[command(flatten)]
    pub variant: VariantArgs,

    #[command(flatten)]
    pub output: OutputArgs,
}
