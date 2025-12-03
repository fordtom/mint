use mint_cli::commands;
use mint_cli::layout::args::BlockNames;

#[path = "common/mod.rs"]
mod common;

#[test]
fn test_file_expands_all_blocks() {
    common::ensure_out_dir();

    let layout_path = "examples/block_no_excel.toml";

    let Some(_ds) = common::find_working_datasource() else {
        return;
    };

    let args = mint_cli::args::Args {
        layout: mint_cli::layout::args::LayoutArgs {
            blocks: vec![BlockNames {
                name: String::new(),
                file: layout_path.to_string(),
            }],
            strict: false,
        },
        variant: mint_cli::variant::args::VariantArgs {
            xlsx: None,
            variant: None,
            main_sheet: "Main".to_string(),
        },
        output: mint_cli::output::args::OutputArgs {
            out: "out".to_string(),
            prefix: "EXPAND".to_string(),
            suffix: "TEST".to_string(),
            record_width: 32,
            format: mint_cli::output::args::OutputFormat::Hex,
            combined: false,
            stats: false,
            quiet: true,
        },
    };

    let stats = commands::build(&args, None).expect("build should succeed");

    let cfg = mint_cli::layout::load_layout(layout_path).expect("layout loads");
    assert_eq!(
        stats.blocks_processed,
        cfg.blocks.len(),
        "Should build all blocks in the file"
    );

    for block_name in cfg.blocks.keys() {
        common::assert_out_file_exists_custom(
            block_name,
            "EXPAND",
            "TEST",
            mint_cli::output::args::OutputFormat::Hex,
        );
    }
}

#[test]
fn test_deduplication_file_and_specific() {
    common::ensure_out_dir();

    let layout_path = "examples/block_no_excel.toml";

    let args = mint_cli::args::Args {
        layout: mint_cli::layout::args::LayoutArgs {
            blocks: vec![
                BlockNames {
                    name: String::new(),
                    file: layout_path.to_string(),
                },
                BlockNames {
                    name: "simple_block".to_string(),
                    file: layout_path.to_string(),
                },
            ],
            strict: false,
        },
        variant: mint_cli::variant::args::VariantArgs {
            xlsx: None,
            variant: None,
            main_sheet: "Main".to_string(),
        },
        output: mint_cli::output::args::OutputArgs {
            out: "out".to_string(),
            prefix: "DEDUP".to_string(),
            suffix: "TEST".to_string(),
            record_width: 32,
            format: mint_cli::output::args::OutputFormat::Hex,
            combined: false,
            stats: false,
            quiet: true,
        },
    };

    let stats = commands::build(&args, None).expect("build should succeed");

    let cfg = mint_cli::layout::load_layout(layout_path).expect("layout loads");
    assert_eq!(
        stats.blocks_processed,
        cfg.blocks.len(),
        "Should deduplicate and only build each block once"
    );
}

#[test]
fn test_file_expansion_with_combined() {
    common::ensure_out_dir();

    let layout_path = "examples/block_no_excel.toml";

    let args = mint_cli::args::Args {
        layout: mint_cli::layout::args::LayoutArgs {
            blocks: vec![BlockNames {
                name: String::new(),
                file: layout_path.to_string(),
            }],
            strict: false,
        },
        variant: mint_cli::variant::args::VariantArgs {
            xlsx: None,
            variant: None,
            main_sheet: "Main".to_string(),
        },
        output: mint_cli::output::args::OutputArgs {
            out: "out".to_string(),
            prefix: "COMBINED".to_string(),
            suffix: "EXPAND".to_string(),
            record_width: 32,
            format: mint_cli::output::args::OutputFormat::Hex,
            combined: true,
            stats: false,
            quiet: true,
        },
    };

    let stats = commands::build(&args, None).expect("build should succeed");

    let cfg = mint_cli::layout::load_layout(layout_path).expect("layout loads");
    assert_eq!(
        stats.blocks_processed,
        cfg.blocks.len(),
        "Should build all blocks in combined mode"
    );

    common::assert_out_file_exists_custom(
        "combined",
        "COMBINED",
        "EXPAND",
        mint_cli::output::args::OutputFormat::Hex,
    );
}
