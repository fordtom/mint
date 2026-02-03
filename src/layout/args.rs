use super::errors::LayoutError;
use clap::Args;

#[derive(Debug, Clone)]
pub struct BlockNames {
    pub name: String,
    pub file: String,
}

pub fn parse_block_arg(block: &str) -> Result<BlockNames, LayoutError> {
    let Some((name, file)) = block.split_once('@') else {
        return Err(LayoutError::InvalidBlockArgument(format!(
            "Expected BLOCK@FILE, got '{}'",
            block
        )));
    };
    if name.is_empty() || file.is_empty() {
        return Err(LayoutError::InvalidBlockArgument(format!(
            "Expected BLOCK@FILE, got '{}'",
            block
        )));
    }

    Ok(BlockNames {
        name: name.to_string(),
        file: file.to_string(),
    })
}

#[derive(Args, Debug)]
pub struct LayoutArgs {
    #[arg(
        value_name = "BLOCK@FILE",
        num_args = 1..,
        value_parser = parse_block_arg,
        help = "One or more blocks as name@layout_file (toml/yaml/json)"
    )]
    pub blocks: Vec<BlockNames>,

    #[arg(
        long,
        help = "Enable strict type conversions; disallow lossy casts during bytestream assembly",
        default_value_t = false
    )]
    pub strict: bool,
}
