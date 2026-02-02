#![allow(dead_code)]

use std::fs;
use std::path::Path;

use mint_cli::args::Args;
use mint_cli::data::{self, DataSource};
use mint_cli::layout::args::{BlockNames, LayoutArgs};
use mint_cli::output::args::{OutputArgs, OutputFormat};

pub fn ensure_out_dir() {
    fs::create_dir_all("out").unwrap();
}

pub fn write_layout_file(file_stem: &str, contents: &str) -> String {
    ensure_out_dir();
    let path = format!("out/{}.toml", file_stem);
    std::fs::write(&path, contents).expect("write layout file");
    path
}

/// Build test args with output written to out/{block_name}.{ext}
pub fn build_args(layout_path: &str, block_name: &str, format: OutputFormat) -> Args {
    let ext = match format {
        OutputFormat::Hex => "hex",
        OutputFormat::Mot => "mot",
    };
    let out_path = format!("out/{}.{}", block_name, ext);
    Args {
        layout: LayoutArgs {
            blocks: vec![BlockNames {
                name: block_name.to_string(),
                file: layout_path.to_string(),
            }],
            strict: false,
        },
        data: data::args::DataArgs {
            xlsx: Some("tests/data/data.xlsx".to_string()),
            version: Some("Default".to_string()),
            ..Default::default()
        },
        output: OutputArgs {
            hexview: build_hexview(1, &out_path, format),
            export_json: None,
            stats: false,
            quiet: false,
        },
    }
}

pub fn find_working_datasource() -> Option<Box<dyn DataSource>> {
    let version_candidates: [&str; 2] = ["Default", "VarA/Default"];

    for ver in &version_candidates {
        let ver_args = data::args::DataArgs {
            xlsx: Some("tests/data/data.xlsx".to_string()),
            version: Some(ver.to_string()),
            ..Default::default()
        };
        if let Ok(Some(ds)) = data::create_data_source(&ver_args) {
            return Some(ds);
        }
    }
    None
}

/// Assert that the output file exists at the given path
pub fn assert_out_file_exists(out_path: &Path) {
    assert!(
        out_path.exists(),
        "expected output file to exist: {}",
        out_path.display()
    );
}

/// Build test args for multiple layouts, output to the specified path
pub fn build_args_for_layouts(
    layouts: Vec<BlockNames>,
    format: OutputFormat,
    out_path: &str,
) -> Args {
    let block_count = layouts.len();
    Args {
        layout: LayoutArgs {
            blocks: layouts,
            strict: false,
        },
        data: data::args::DataArgs {
            xlsx: Some("tests/data/data.xlsx".to_string()),
            version: Some("Default".to_string()),
            ..Default::default()
        },
        output: OutputArgs {
            hexview: build_hexview(block_count, out_path, format),
            export_json: None,
            stats: false,
            quiet: false,
        },
    }
}

fn build_hexview(block_count: usize, out_path: &str, format: OutputFormat) -> String {
    let output_flag = match format {
        OutputFormat::Hex => "/XI:32",
        OutputFormat::Mot => "/XS:32",
    };

    let mut parts = Vec::with_capacity(2 + block_count.saturating_sub(1));
    if block_count > 0 {
        parts.push("@1".to_string());
    }
    for idx in 2..=block_count {
        parts.push(format!("/MO:@{idx}"));
    }
    parts.push(output_flag.to_string());
    parts.push("-o".to_string());
    parts.push(out_path.to_string());
    parts.join(" ")
}
