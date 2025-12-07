use mint_cli::commands;
use mint_cli::version::create_data_source;

#[path = "common/mod.rs"]
mod common;

#[test]
fn test_build_without_excel() {
    common::ensure_out_dir();

    let layout_path = "tests/data/blocks.toml";

    // Build simple_block which has all inline values (no Excel dependency)
    let args = mint_cli::args::Args {
        layout: mint_cli::layout::args::LayoutArgs {
            blocks: vec![mint_cli::layout::args::BlockNames {
                name: "simple_block".to_string(),
                file: layout_path.to_string(),
            }],
            strict: false,
        },
        version: Default::default(),
        output: mint_cli::output::args::OutputArgs {
            out: "out".to_string(),
            prefix: "TEST".to_string(),
            suffix: "NOEXCEL".to_string(),
            record_width: 32,
            format: mint_cli::output::args::OutputFormat::Hex,
            combined: false,
            stats: false,
            quiet: true,
        },
    };

    // This should succeed since all values are inline
    let stats = commands::build(&args, None).expect("build should succeed without Excel file");

    assert!(
        stats.blocks_processed > 0,
        "Should build at least one block"
    );

    common::assert_out_file_exists_custom(
        "simple_block",
        "TEST",
        "NOEXCEL",
        mint_cli::output::args::OutputFormat::Hex,
    );
}

#[test]
fn test_error_when_name_without_excel() {
    common::ensure_out_dir();

    // Use a block that references names from Excel
    let layout_path = "tests/data/blocks.toml";

    let input = mint_cli::layout::args::BlockNames {
        name: "block".to_string(),
        file: layout_path.to_string(),
    };

    let args = mint_cli::args::Args {
        layout: mint_cli::layout::args::LayoutArgs {
            blocks: vec![input.clone()],
            strict: false,
        },
        version: Default::default(),
        output: mint_cli::output::args::OutputArgs {
            out: "out".to_string(),
            prefix: "TEST".to_string(),
            suffix: "ERROR".to_string(),
            record_width: 32,
            format: mint_cli::output::args::OutputFormat::Hex,
            combined: false,
            stats: false,
            quiet: true,
        },
    };

    // This should fail with MissingDataSheet error
    let result = commands::build(&args, None);
    assert!(
        result.is_err(),
        "Expected error when using 'name' without Excel file"
    );

    let err = result.unwrap_err();
    let err_str = format!("{}", err);
    assert!(
        err_str.contains("Missing datasheet")
            || err_str.contains("requires a value from a data source"),
        "Error should mention missing data source, got: {}",
        err_str
    );
}

#[test]
fn test_factory_returns_none_without_xlsx() {
    // Test that create_data_source returns None when no xlsx is provided
    let args_no_excel = mint_cli::version::args::VersionArgs::default();

    let result = create_data_source(&args_no_excel).expect("should return Ok(None)");
    assert!(
        result.is_none(),
        "create_data_source should return None when no xlsx provided"
    );

    // Test with deprecated variant flag (would produce warning)
    let args_variant_no_excel = mint_cli::version::args::VersionArgs {
        variant: Some("VarA".to_string()),
        ..Default::default()
    };

    let result = create_data_source(&args_variant_no_excel).expect("should return Ok(None)");
    assert!(
        result.is_none(),
        "create_data_source should return None when no xlsx provided, even with deprecated variant flag"
    );

    // Test with version flag
    let args_version_no_excel = mint_cli::version::args::VersionArgs::default();

    let result = create_data_source(&args_version_no_excel).expect("should return Ok(None)");
    assert!(
        result.is_none(),
        "create_data_source should return None when no xlsx provided, even with version flag"
    );
}
