use clap::Args;

#[derive(Args, Debug, Clone, Default)]
pub struct VariantArgs {
    #[arg(
        short = 'x',
        long,
        value_name = "FILE",
        group = "datasource",
        requires = "variant",
        help = "Path to the Excel variants file"
    )]
    pub xlsx: Option<String>,

    #[arg(long, value_name = "NAME", help = "Main sheet name in Excel")]
    pub main_sheet: Option<String>,

    #[arg(
        short = 'p',
        long,
        value_name = "PATH or json string",
        group = "datasource",
        requires = "variant",
        help = "Path to the JSON file or a JSON string containing the postgres configuration options and template"
    )]
    pub postgres: Option<String>,

    #[arg(
        short = 'v',
        long,
        value_name = "NAME[/NAME...]",
        requires = "datasource",
        help = "Variant columns to use in priority order (separate with '/')"
    )]
    pub variant: Option<String>,
}
