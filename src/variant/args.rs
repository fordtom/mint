use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct VariantArgs {
    #[arg(
        short = 'x',
        long,
        required = false,
        value_name = "FILE",
        help = "Path to the Excel variants file"
    )]
    pub xlsx: Option<String>,

    #[arg(long, value_name = "NAME", help = "Main sheet name in Excel")]
    pub main_sheet: Option<String>,

    #[arg(
        short = 'v',
        long,
        value_name = "NAME[/NAME...]",
        help = "Variant columns to use in priority order (separate with '/')"
    )]
    pub variant: Option<String>,
}
