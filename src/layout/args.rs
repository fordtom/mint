use super::errors::LayoutError;
use clap::Args;

#[derive(Debug, Clone)]
pub struct BlockNames {
    pub name: String,
    pub file: String,
}

pub fn parse_block_arg(block: &str) -> Result<BlockNames, LayoutError> {
    let parts: Vec<&str> = block.split('@').collect();

    if parts.len() != 2 {
        Err(LayoutError::InvalidBlockArgument(format!(
            "Failed to unpack block {}",
            block
        )))
    } else {
        Ok(BlockNames {
            name: parts[0].to_string(),
            file: parts[1].to_string(),
        })
    }
}

#[derive(Args, Debug)]
pub struct LayoutArgs {
    #[arg(value_name = "BLOCK@FILE", num_args = 1.., value_parser = parse_block_arg, help = "One or more blocks in the form name@layout_file (toml/yaml/json)")]
    pub blocks: Vec<BlockNames>,

    #[arg(
        long,
        help = "Enable strict type conversions; disallow lossy casts during bytestream assembly",
        default_value_t = false
    )]
    pub strict: bool,
}
