use mint_core::output::error::OutputError;
use std::path::{Path, PathBuf};

pub(super) fn same_destination(left: &Path, right: &Path) -> Result<bool, OutputError> {
    let left = absolute_destination(left)?;
    let right = absolute_destination(right)?;
    if left == right {
        return Ok(true);
    }

    Ok(matches!(
        (left.canonicalize(), right.canonicalize()),
        (Ok(left), Ok(right)) if left == right
    ))
}

fn absolute_destination(path: &Path) -> Result<PathBuf, OutputError> {
    std::path::absolute(path).map_err(|e| {
        OutputError::FileError(format!(
            "failed to resolve output path {}: {}",
            path.display(),
            e
        ))
    })
}

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
