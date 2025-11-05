use super::errors::LayoutError;
use clap::Args;
use std::collections::{hash_map::Entry, HashMap};

#[derive(Debug, Clone)]
pub struct BlockNames {
    pub name: String,
    pub file: String,
}

#[derive(Debug, Clone)]
pub enum BlockSpecifier {
    Specific(BlockNames),
    All(String),
}

pub fn parse_block_specifier(input: &str) -> Result<BlockSpecifier, LayoutError> {
    let parts: Vec<&str> = input.split('@').collect();

    match parts.len() {
        1 => {
            let file = parts[0];
            if file.is_empty() {
                Err(LayoutError::InvalidBlockArgument(
                    "Layout file must not be empty".to_string(),
                ))
            } else {
                Ok(BlockSpecifier::All(file.to_string()))
            }
        }
        2 => {
            let name = parts[0];
            let file = parts[1];
            if name.is_empty() || file.is_empty() {
                Err(LayoutError::InvalidBlockArgument(format!(
                    "Failed to unpack block {}",
                    input
                )))
            } else {
                Ok(BlockSpecifier::Specific(BlockNames {
                    name: name.to_string(),
                    file: file.to_string(),
                }))
            }
        }
        _ => Err(LayoutError::InvalidBlockArgument(format!(
            "Failed to unpack block {}",
            input
        ))),
    }
}

#[derive(Args, Debug)]
pub struct LayoutArgs {
    #[arg(
        value_name = "BLOCK@FILE|FILE",
        num_args = 1..,
        value_parser = parse_block_specifier,
        help = "One or more blocks in the form name@layout_file (toml/yaml/json), or provide a layout file to build all blocks"
    )]
    pub specifiers: Vec<BlockSpecifier>,

    #[arg(skip)]
    pub blocks: Vec<BlockNames>,

    #[arg(
        long,
        help = "Enable strict type conversions; disallow lossy casts during bytestream assembly",
        default_value_t = false
    )]
    pub strict: bool,
}

impl LayoutArgs {
    pub fn resolve_blocks(&mut self) -> Result<&[BlockNames], LayoutError> {
        if self.blocks.is_empty() {
            let resolved = Self::expand_specifiers(&self.specifiers)?;

            if resolved.is_empty() {
                return Err(LayoutError::NoBlocksProvided);
            }

            self.blocks = resolved;
        }

        Ok(&self.blocks)
    }

    fn expand_specifiers(specifiers: &[BlockSpecifier]) -> Result<Vec<BlockNames>, LayoutError> {
        let mut resolved = Vec::new();
        let mut cache: HashMap<String, Vec<String>> = HashMap::new();

        for specifier in specifiers {
            match specifier {
                BlockSpecifier::Specific(block) => {
                    resolved.push(block.clone());
                }
                BlockSpecifier::All(file) => {
                    let file_key = file.clone();
                    let names_ref = match cache.entry(file_key.clone()) {
                        Entry::Occupied(entry) => entry.into_mut(),
                        Entry::Vacant(entry) => {
                            let layout = super::load_layout(&file_key)?;
                            let names: Vec<String> = layout.blocks.keys().cloned().collect();

                            if names.is_empty() {
                                return Err(LayoutError::NoBlocksInLayout(file_key.clone()));
                            }

                            entry.insert(names)
                        }
                    };

                    for name in names_ref.iter() {
                        resolved.push(BlockNames {
                            name: name.clone(),
                            file: file_key.clone(),
                        });
                    }
                }
            }
        }

        Ok(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn write_temp_layout(contents: &str) -> String {
        let mut path = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        path.push(format!("mint_layout_test_{}.toml", unique));
        fs::write(&path, contents).expect("write temp layout");
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn resolves_mixed_specifiers_in_order() {
        let layout_path = "examples/block.toml".to_string();
        let mut args = LayoutArgs {
            specifiers: vec![
                BlockSpecifier::Specific(BlockNames {
                    name: "block2".to_string(),
                    file: layout_path.clone(),
                }),
                BlockSpecifier::All(layout_path.clone()),
            ],
            blocks: Vec::new(),
            strict: false,
        };

        let resolved = args
            .resolve_blocks()
            .expect("should resolve specifiers");

        let names: Vec<_> = resolved.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(names, vec!["block2", "block", "block2", "block3"]);

        // Second call uses cached result
        let resolved_again = args
            .resolve_blocks()
            .expect("should reuse resolved blocks");
        assert_eq!(resolved_again.len(), 4);
    }

    #[test]
    fn errors_when_layout_contains_no_blocks() {
        let layout_contents = r#"
[settings]
endianness = "little"
virtual_offset = 0
byte_swap = false
pad_to_end = false

[settings.crc]
polynomial = 0x1
start = 0
xor_out = 0
ref_in = false
ref_out = false
area = "data"
"#;

        let layout_path = write_temp_layout(layout_contents);

        let mut args = LayoutArgs {
            specifiers: vec![BlockSpecifier::All(layout_path.clone())],
            blocks: Vec::new(),
            strict: false,
        };

        let err = args
            .resolve_blocks()
            .expect_err("should error on empty layout");

        match err {
            LayoutError::NoBlocksInLayout(path) => assert_eq!(path, layout_path),
            other => panic!("unexpected error: {}", other),
        }

        fs::remove_file(layout_path).ok();
    }
}
