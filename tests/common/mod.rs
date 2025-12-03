#![allow(dead_code)]

use std::fs;
use std::path::Path;

use mint_cli::args::Args;
use mint_cli::layout::args::{BlockNames, LayoutArgs};
use mint_cli::output::args::{OutputArgs, OutputFormat};
use mint_cli::variant::{self, DataSource};

pub fn ensure_out_dir() {
    fs::create_dir_all("out").unwrap();
}

pub fn write_layout_file(file_stem: &str, contents: &str) -> String {
    ensure_out_dir();
    let path = format!("out/{}.toml", file_stem);
    std::fs::write(&path, contents).expect("write layout file");
    path
}

pub fn build_args(layout_path: &str, block_name: &str, format: OutputFormat) -> Args {
    Args {
        layout: LayoutArgs {
            blocks: vec![BlockNames {
                name: block_name.to_string(),
                file: layout_path.to_string(),
            }],
            strict: false,
        },
        variant: variant::args::VariantArgs {
            xlsx: Some("examples/data.xlsx".to_string()),
            variant: Some("Default".to_string()),
            ..Default::default()
        },
        output: OutputArgs {
            out: "out".to_string(),
            prefix: "PRE".to_string(),
            suffix: "SUF".to_string(),
            record_width: 32,
            format,
            combined: false,
            stats: false,
            quiet: false,
        },
    }
}

pub fn find_working_datasource() -> Option<Box<dyn DataSource>> {
    let variant_candidates: [&str; 2] = ["Default", "VarA/Default"];

    for var in &variant_candidates {
        let var_args = variant::args::VariantArgs {
            xlsx: Some("examples/data.xlsx".to_string()),
            variant: Some(var.to_string()),
            ..Default::default()
        };
        if let Ok(Some(ds)) = variant::create_data_source(&var_args) {
            return Some(ds);
        }
    }
    None
}

pub fn assert_out_file_exists(block_name: &str, format: OutputFormat) {
    let ext = match format {
        OutputFormat::Hex => "hex",
        OutputFormat::Mot => "mot",
    };
    let expected = format!("{}_{}_{}.{}", "PRE", block_name, "SUF", ext);
    assert!(Path::new("out").join(expected).exists());
}

pub fn assert_out_file_exists_custom(
    block_name: &str,
    prefix: &str,
    suffix: &str,
    format: OutputFormat,
) {
    let ext = match format {
        OutputFormat::Hex => "hex",
        OutputFormat::Mot => "mot",
    };
    let expected = format!("{}_{}_{}.{}", prefix, block_name, suffix, ext);
    assert!(Path::new("out").join(expected).exists());
}

pub fn build_args_for_layouts(layouts: Vec<BlockNames>, format: OutputFormat) -> Args {
    Args {
        layout: LayoutArgs {
            blocks: layouts,
            strict: false,
        },
        variant: variant::args::VariantArgs {
            xlsx: Some("examples/data.xlsx".to_string()),
            variant: Some("Default".to_string()),
            ..Default::default()
        },
        output: OutputArgs {
            out: "out".to_string(),
            prefix: "PRE".to_string(),
            suffix: "SUF".to_string(),
            record_width: 32,
            format,
            combined: true,
            stats: false,
            quiet: false,
        },
    }
}
