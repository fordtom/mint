use std::path::Path;

use mint_cli::commands;
use mint_cli::layout::args::BlockNames;

#[path = "common/mod.rs"]
mod common;

#[test]
fn test_missing_block_name_errors() {
    common::ensure_out_dir();

    let layout_path = "tests/data/blocks.toml";

    let args = mint_cli::args::Args {
        layout: mint_cli::layout::args::LayoutArgs {
            blocks: vec![BlockNames {
                name: String::new(),
                file: layout_path.to_string(),
            }],
            strict: false,
        },
        data: Default::default(),
        output: mint_cli::output::args::OutputArgs {
            hexview: "@1 /XI -o out/invalid.hex".to_string(),
            export_json: None,
            stats: false,
            quiet: true,
        },
    };

    let result = commands::build(&args, None);
    assert!(result.is_err(), "expected error for missing block name");
}

#[test]
fn test_duplicate_blocks_are_built_in_order() {
    common::ensure_out_dir();

    let layout_path = "tests/data/blocks.toml";
    let Some(ds) = common::find_working_datasource() else {
        return;
    };

    let blocks = vec![
        BlockNames {
            name: "block".to_string(),
            file: layout_path.to_string(),
        },
        BlockNames {
            name: "block".to_string(),
            file: layout_path.to_string(),
        },
    ];

    let args = common::build_args_for_layouts(
        blocks,
        mint_cli::output::args::OutputFormat::Hex,
        "out/dup_blocks.hex",
    );

    let stats = commands::build(&args, Some(ds.as_ref())).expect("build should succeed");
    assert_eq!(stats.blocks_processed, 2, "duplicates should be built");
    common::assert_out_file_exists(Path::new("out/dup_blocks.hex"));
}
