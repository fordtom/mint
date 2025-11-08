use std::path::Path;

use crate::output::args::{OutputArgs, OutputFormat};
use crate::output::errors::OutputError;

pub fn write_output(
    args: &OutputArgs,
    block_name: &str,
    contents: &str,
) -> Result<(), OutputError> {
    let mut name_parts: Vec<String> = Vec::new();
    if !args.prefix.is_empty() {
        name_parts.push(args.prefix.clone());
    }
    name_parts.push(block_name.to_string());
    if !args.suffix.is_empty() {
        name_parts.push(args.suffix.clone());
    }
    let ext = match args.format {
        OutputFormat::Hex => "hex",
        OutputFormat::Mot => "mot",
    };
    let out_filename = format!("{}.{}", name_parts.join("_"), ext);
    let out_path = Path::new(&args.out).join(out_filename);
    std::fs::write(out_path, contents).map_err(|e| {
        OutputError::FileError(format!("failed to write block {}: {}", block_name, e))
    })?;
    Ok(())
}
