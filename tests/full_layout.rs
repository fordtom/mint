use mint_cli::args::Args;
use mint_cli::commands;
use mint_cli::layout::args::{BlockSpecifier, LayoutArgs};
use mint_cli::output::args::{OutputArgs, OutputFormat};
use mint_cli::variant;

#[path = "common/mod.rs"]
mod common;

#[test]
fn builds_all_blocks_from_example_layout() {
    common::ensure_out_dir();

    let layout_path = "examples/block.toml".to_string();

    let mut args = Args {
        layout: LayoutArgs {
            specifiers: vec![BlockSpecifier::All(layout_path.clone())],
            blocks: Vec::new(),
            strict: false,
        },
        variant: variant::args::VariantArgs {
            xlsx: Some("examples/data.xlsx".to_string()),
            variant: None,
            debug: false,
            main_sheet: "Main".to_string(),
        },
        output: OutputArgs {
            out: "out".to_string(),
            prefix: "FULL".to_string(),
            suffix: "LAYOUT".to_string(),
            record_width: 32,
            format: OutputFormat::Hex,
            combined: false,
            stats: false,
            quiet: true,
        },
    };

    args.layout.resolve_blocks().expect("resolve blocks");

    let datasheet = common
        .find_working_datasheet()
        .expect("datasheet should exist for example");

    let stats = commands::build_separate_blocks(&args, Some(&datasheet))
        .expect("build full layout");

    assert_eq!(stats.blocks_processed, 3);

    for block_name in ["block", "block2", "block3"] {
        common::assert_out_file_exists_custom(
            block_name,
            "FULL",
            "LAYOUT",
            OutputFormat::Hex,
        );
    }
}
