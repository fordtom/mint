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
        short = 'r',
        long,
        value_name = "PATH or json string",
        group = "datasource",
        requires = "variant",
        help = "Path to the JSON file or a JSON string containing the REST API configuration options and template"
    )]
    pub rest: Option<String>,

    #[arg(
        short = 'j',
        long,
        value_name = "PATH or json string",
        group = "datasource",
        requires = "variant",
        help = "Path to the JSON file or a JSON string containing variant data as an object with variant names as keys"
    )]
    pub json: Option<String>,

    #[arg(
        short = 'v',
        long,
        value_name = "NAME[/NAME...]",
        requires = "datasource",
        help = "Variant columns to use in priority order (separate with '/')"
    )]
    pub variant: Option<String>,
}

impl VariantArgs {
    /// Parses the variant stack from the raw slash-separated string.
    pub fn get_variant_list(&self) -> Vec<String> {
        self.variant
            .as_deref()
            .map(|raw| {
                raw.split('/')
                    .map(|name| name.trim())
                    .filter(|name| !name.is_empty())
                    .map(|name| name.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }
}
