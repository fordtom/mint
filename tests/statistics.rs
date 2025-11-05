use mint_cli::commands::{
    self,
    stats::{BlockStat, BuildStats},
};

#[path = "common/mod.rs"]
mod common;

#[test]
fn test_block_stat_collection() {
    common::ensure_out_dir();

    let layout_path = "examples/block.toml";

    let Some(ds) = common::find_working_datasheet() else {
        return;
    };

    let args = common::build_args(
        layout_path,
        "block",
        mint_cli::output::args::OutputFormat::Hex,
    );
    let input = mint_cli::layout::args::BlockNames {
        name: "block".to_string(),
        file: layout_path.to_string(),
    };

    let block_stat = commands::generate::build_block_single(&input, Some(&ds), &args)
        .expect("build should succeed");

    assert_eq!(block_stat.name, "block");
    assert!(block_stat.allocated_size > 0);
    assert!(block_stat.used_size > 0);
    assert!(block_stat.used_size <= block_stat.allocated_size);
}

#[test]
fn test_build_stats_aggregation() {
    common::ensure_out_dir();

    let layout_path = "examples/block.toml";

    let Some(ds) = common::find_working_datasheet() else {
        return;
    };

    let cfg = mint_cli::layout::load_layout(layout_path).expect("layout loads");
    let block_inputs = cfg
        .blocks
        .keys()
        .take(2)
        .map(|name| mint_cli::layout::args::BlockNames {
            name: name.clone(),
            file: layout_path.to_string(),
        })
        .collect::<Vec<_>>();

    if block_inputs.is_empty() {
        return;
    }

    let blocks = block_inputs.clone();
    let args = common::build_args_for_layouts(&blocks, mint_cli::output::args::OutputFormat::Hex);

    let stats = commands::build_single_file(&args, &blocks, Some(&ds))
        .expect("build should succeed");

    assert_eq!(stats.blocks_processed, block_inputs.len());
    assert!(stats.total_allocated > 0);
    assert!(stats.total_used > 0);
    assert!(stats.total_used <= stats.total_allocated);
    assert_eq!(stats.block_stats.len(), block_inputs.len());

    let manual_total_allocated: usize = stats
        .block_stats
        .iter()
        .map(|b| b.allocated_size as usize)
        .sum();
    let manual_total_used: usize = stats.block_stats.iter().map(|b| b.used_size as usize).sum();

    assert_eq!(stats.total_allocated, manual_total_allocated);
    assert_eq!(stats.total_used, manual_total_used);
}

#[test]
fn test_space_efficiency_calculation() {
    let mut stats = BuildStats::new();

    stats.add_block(BlockStat {
        name: "test1".to_string(),
        start_address: 0x1000,
        allocated_size: 100,
        used_size: 80,
        crc_value: 0x12345678,
    });

    stats.add_block(BlockStat {
        name: "test2".to_string(),
        start_address: 0x2000,
        allocated_size: 200,
        used_size: 120,
        crc_value: 0x9ABCDEF0,
    });

    assert_eq!(stats.blocks_processed, 2);
    assert_eq!(stats.total_allocated, 300);
    assert_eq!(stats.total_used, 200);

    let efficiency = stats.space_efficiency();
    let expected = (200.0 / 300.0) * 100.0;
    assert!((efficiency - expected).abs() < 0.01);
}

#[test]
fn test_combined_mode_stats() {
    common::ensure_out_dir();

    let layout_path = "examples/block.toml";

    let Some(ds) = common::find_working_datasheet() else {
        return;
    };

    let cfg = mint_cli::layout::load_layout(layout_path).expect("layout loads");
    let block_inputs = cfg
        .blocks
        .keys()
        .map(|name| mint_cli::layout::args::BlockNames {
            name: name.clone(),
            file: layout_path.to_string(),
        })
        .collect::<Vec<_>>();

    if block_inputs.is_empty() {
        return;
    }

    let blocks = block_inputs.clone();
    let args = common::build_args_for_layouts(&blocks, mint_cli::output::args::OutputFormat::Hex);

    let stats = commands::build_single_file(&args, &blocks, Some(&ds))
        .expect("build should succeed");

    assert_eq!(stats.blocks_processed, block_inputs.len());

    for block_stat in &stats.block_stats {
        assert!(block_stat.allocated_size > 0);
        assert!(block_stat.used_size > 0);
        assert!(block_stat.used_size <= block_stat.allocated_size);
    }
}

#[test]
fn test_space_efficiency_edge_cases() {
    let mut stats = BuildStats::new();
    assert_eq!(stats.space_efficiency(), 0.0);

    stats.add_block(BlockStat {
        name: "full".to_string(),
        start_address: 0x1000,
        allocated_size: 100,
        used_size: 100,
        crc_value: 0x12345678,
    });

    let efficiency = stats.space_efficiency();
    assert!((efficiency - 100.0).abs() < 0.01);
}
