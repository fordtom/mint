//! Tests for optional CRC - verifies builds work when settings.crc and header.crc_location are omitted.

use mint_cli::commands;

#[path = "common/mod.rs"]
mod common;

#[test]
fn test_build_without_crc_settings() {
    common::ensure_out_dir();

    let layout_path = "tests/data/no_crc.toml";
    let args = common::build_args(layout_path, "nocrc_block", mint_cli::output::args::OutputFormat::Hex);

    // Build should succeed without CRC settings
    let stats = commands::build(&args, None).expect("build should succeed without CRC");

    assert_eq!(stats.blocks_processed, 1);
    let block_stat = &stats.block_stats[0];
    assert_eq!(block_stat.name, "nocrc_block");
    assert!(block_stat.crc_value.is_none(), "CRC value should be None when CRC is disabled");
    assert!(block_stat.used_size > 0);
    assert!(block_stat.allocated_size > 0);

    common::assert_out_file_exists("nocrc_block", mint_cli::output::args::OutputFormat::Hex);
}

#[test]
fn test_mixed_crc_and_no_crc_blocks() {
    common::ensure_out_dir();

    // Use simple_block which has CRC settings
    let layout_with_crc = "tests/data/blocks.toml";
    let args_crc = common::build_args(layout_with_crc, "simple_block", mint_cli::output::args::OutputFormat::Hex);

    let stats_crc = commands::build(&args_crc, None).expect("build with CRC should succeed");
    assert!(stats_crc.block_stats[0].crc_value.is_some(), "CRC value should be present");

    // Use nocrc_block which has no CRC settings
    let layout_no_crc = "tests/data/no_crc.toml";
    let args_no_crc = common::build_args(layout_no_crc, "nocrc_block", mint_cli::output::args::OutputFormat::Hex);

    let stats_no_crc = commands::build(&args_no_crc, None).expect("build without CRC should succeed");
    assert!(stats_no_crc.block_stats[0].crc_value.is_none(), "CRC value should be None");
}
