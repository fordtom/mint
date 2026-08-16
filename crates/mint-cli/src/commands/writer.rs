use mint_core::output::error::OutputError;
use std::path::{Path, PathBuf};

pub(super) fn same_destination(left: &Path, right: &Path) -> Result<bool, OutputError> {
    Ok(normalize_destination(left)? == normalize_destination(right)?)
}

fn normalize_destination(path: &Path) -> Result<PathBuf, OutputError> {
    let absolute = std::path::absolute(path).map_err(|e| {
        OutputError::FileError(format!(
            "failed to resolve output path {}: {}",
            path.display(),
            e
        ))
    })?;

    // Resolve the deepest existing ancestor while preserving any missing suffix.
    for ancestor in absolute.ancestors() {
        if let Ok(resolved) = ancestor.canonicalize()
            && let Ok(suffix) = absolute.strip_prefix(ancestor)
        {
            return Ok(resolved.join(suffix));
        }
    }

    Ok(absolute)
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
