pub mod args;
pub mod block;
mod conversions;
mod entry;
pub mod errors;
pub mod header;
pub mod settings;
pub mod value;

use block::Config;
use errors::LayoutError;
use std::path::Path;

pub fn load_layout(filename: &str) -> Result<Config, LayoutError> {
    let text = std::fs::read_to_string(filename)
        .map_err(|_| LayoutError::FileError(format!("failed to open file: {}", filename)))?;

    let ext = Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    let cfg: Config = match ext.as_str() {
        "toml" => toml::from_str(&text).map_err(|e| {
            LayoutError::FileError(format!("failed to parse file {}: {}", filename, e))
        })?,
        "yaml" | "yml" => serde_yaml::from_str(&text).map_err(|e| {
            LayoutError::FileError(format!("failed to parse file {}: {}", filename, e))
        })?,
        "json" => serde_json::from_str(&text).map_err(|e| {
            LayoutError::FileError(format!("failed to parse file {}: {}", filename, e))
        })?,
        _ => {
            return Err(LayoutError::FileError(
                "Unsupported file format".to_string(),
            ));
        }
    };

    Ok(cfg)
}
