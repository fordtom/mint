use serde_json::Value;

use crate::output::error::OutputError;

/// Render a used values JSON report.
pub fn render_used_values_json(report: &Value) -> Result<String, OutputError> {
    serde_json::to_string_pretty(report)
        .map_err(|e| OutputError::FileError(format!("failed to serialize JSON report: {}", e)))
}
