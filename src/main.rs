use clap::Parser;

use mint_cli::args::Args;
use mint_cli::commands;
use mint_cli::error::*;
use mint_cli::layout;
use mint_cli::data;
use mint_cli::visuals;

fn main() -> Result<(), NvmError> {
    let args = Args::parse();

    let data_source = data::create_data_source(&args.data)?;

    // Check if blocks are provided
    args.layout
        .blocks
        .first()
        .ok_or(layout::errors::LayoutError::NoBlocksProvided)?;

    std::fs::create_dir_all(&args.output.out).map_err(|e| {
        NvmError::Output(mint_cli::output::errors::OutputError::FileError(format!(
            "failed to create output directory: {}",
            e
        )))
    })?;

    let stats = commands::build(&args, data_source.as_deref())?;

    if !args.output.quiet {
        if args.output.stats {
            visuals::print_detailed(&stats);
        } else {
            visuals::print_summary(&stats);
        }
    }

    Ok(())
}
