use mint_core::output::error::OutputError;
use std::path::Path;

pub fn write_text(path: &Path, contents: &str) -> Result<(), OutputError> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| {
            OutputError::FileError(format!(
                "failed to create directory {}: {}",
                parent.display(),
                e
            ))
        })?;
    }

    std::fs::write(path, contents).map_err(|e| {
        OutputError::FileError(format!("failed to write {}: {}", path.display(), e))
    })?;
    Ok(())
}
