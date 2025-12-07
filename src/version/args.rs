use clap::Args;

#[derive(Args, Debug, Clone, Default)]
pub struct VersionArgs {
    #[arg(
        long,
        value_name = "FILE",
        group = "datasource",
        requires_any = ["version", "variant"],
        help = "Path to the Excel versions file"
    )]
    pub xlsx: Option<String>,

    #[arg(long, value_name = "NAME", help = "Main sheet name in Excel")]
    pub main_sheet: Option<String>,

    #[arg(
        long,
        value_name = "PATH or json string",
        group = "datasource",
        requires_any = ["version", "variant"],
        help = "Path to the JSON file or a JSON string containing the postgres configuration options and template"
    )]
    pub postgres: Option<String>,

    #[arg(
        long,
        value_name = "PATH or json string",
        group = "datasource",
        requires_any = ["version", "variant"],
        help = "Path to the JSON file or a JSON string containing the REST API configuration options and template"
    )]
    pub rest: Option<String>,

    #[arg(
        long,
        value_name = "PATH or json string",
        group = "datasource",
        requires_any = ["version", "variant"],
        help = "Path to JSON file or JSON string. Format: object with version names as keys, each containing an object with name:value pairs (e.g., {\"VersionName\": {\"key1\": value1, \"key2\": value2}})"
    )]
    pub json: Option<String>,

    #[arg(
        short = 'v',
        long,
        value_name = "NAME[/NAME...]",
        requires = "datasource",
        help = "Version columns to use in priority order (separate with '/')"
    )]
    pub version: Option<String>,

    #[arg(
        long,
        value_name = "NAME[/NAME...]",
        requires = "datasource",
        help = "[DEPRECATED] Use --version instead. Version columns to use in priority order (separate with '/')"
    )]
    pub variant: Option<String>,
}

impl VersionArgs {
    /// Parses the version stack from the raw slash-separated string.
    /// Handles fallback from deprecated --variant flag.
    pub fn get_version_list(&self) -> Vec<String> {
        let raw = self.version.as_deref().or(self.variant.as_deref());
        raw.map(|r| {
            r.split('/')
                .map(|name| name.trim())
                .filter(|name| !name.is_empty())
                .map(|name| name.to_string())
                .collect()
        })
        .unwrap_or_default()
    }
}
