use mint_cli::commands;

#[path = "common/mod.rs"]
mod common;

#[test]
fn smoke_build_examples_all_formats_and_options() {
    common::ensure_out_dir();

    let layouts = [
        "examples/block.toml",
        "examples/block.yaml",
        "examples/block.json",
    ];
    let blocks = ["block", "block2", "block3"];

    for layout_path in layouts {
        let Some(ds) = common::find_working_datasheet() else {
            continue;
        };

        let cfg = mint_cli::layout::load_layout(layout_path).expect("layout loads");

        for &blk in &blocks {
            if !cfg.blocks.contains_key(blk) {
                continue;
            }

            // Hex
            let args_hex =
                common::build_args(layout_path, blk, mint_cli::output::args::OutputFormat::Hex);
            let input = mint_cli::layout::args::BlockNames {
                name: blk.to_string(),
                file: layout_path.to_string(),
            };
            commands::generate::build_block_single(&input, Some(&ds), &args_hex)
                .expect("build hex");
            common::assert_out_file_exists(blk, mint_cli::output::args::OutputFormat::Hex);

            // Mot
            let args_mot =
                common::build_args(layout_path, blk, mint_cli::output::args::OutputFormat::Mot);
            commands::generate::build_block_single(&input, Some(&ds), &args_mot)
                .expect("build mot");
            common::assert_out_file_exists(blk, mint_cli::output::args::OutputFormat::Mot);
        }

        let block_inputs = cfg
            .blocks
            .keys()
            .map(|name| mint_cli::layout::args::BlockNames {
                name: name.clone(),
                file: layout_path.to_string(),
            })
            .collect::<Vec<_>>();

        if !block_inputs.is_empty() {
            let args_single = common::build_args_for_layouts(
                block_inputs,
                mint_cli::output::args::OutputFormat::Hex,
            );

            commands::build_single_file(&args_single, Some(&ds)).expect("build combined");
            common::assert_out_file_exists("combined", mint_cli::output::args::OutputFormat::Hex);
        }
    }
}
