use mint_core::output::error::OutputError;
use std::path::{Component, Path, PathBuf};

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

    let resolved = resolve_existing_ancestor(&absolute);
    Ok(resolve_existing_ancestor(&resolved))
}

fn resolve_existing_ancestor(path: &Path) -> PathBuf {
    for ancestor in path.ancestors() {
        if let Ok(resolved) = ancestor.canonicalize()
            && let Ok(suffix) = path.strip_prefix(ancestor)
        {
            return suffix.components().fold(resolved, |mut path, component| {
                match component {
                    Component::ParentDir => {
                        path.pop();
                    }
                    Component::Normal(part) => path.push(part),
                    _ => {}
                }
                path
            });
        }
    }

    path.to_owned()
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
