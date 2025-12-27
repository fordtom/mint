use postgres::{Client, NoTls};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

use super::DataSource;
use super::args::DataArgs;
use super::errors::DataError;
use crate::layout::value::{DataValue, ValueSource};

fn load_json_string_or_file(input: &str) -> Result<String, DataError> {
    if input.ends_with(".json") {
        std::fs::read_to_string(input)
            .map_err(|_| DataError::FileError(format!("failed to open file: {}", input)))
    } else {
        Ok(input.to_string())
    }
}

/// Navigates into nested JSON objects using a path of keys.
/// Returns the value at the specified path, or the original value if path is empty.
fn extract_nested_value<'a>(value: &'a Value, path: &[String]) -> Result<&'a Value, DataError> {
    let mut current = value;
    for key in path {
        current = current.get(key).ok_or_else(|| {
            DataError::RetrievalError(format!("nested key '{}' not found in response", key))
        })?;
    }
    Ok(current)
}

#[derive(Debug, Deserialize)]
struct PostgresConfig {
    url: String,
    query_template: String,
    /// Path of keys to navigate into nested response objects.
    #[serde(default)]
    data_path: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RestConfig {
    url: String,
    #[serde(default)]
    headers: HashMap<String, String>,
    /// Path of keys to navigate into nested response objects.
    #[serde(default)]
    data_path: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GraphQLConfig {
    url: String,
    query: String,
    version_variable: String,
    #[serde(default)]
    variables: HashMap<String, Value>,
    #[serde(default)]
    headers: HashMap<String, String>,
    /// Path of keys to navigate into nested data objects (applied after extracting `data` field).
    #[serde(default)]
    data_path: Vec<String>,
}

/// Shared JSON-based data source that reads version data from JSON objects.
/// Result: `Vec<HashMap<String, Value>>` in version priority order.
pub struct JsonDataSource {
    version_columns: Vec<HashMap<String, Value>>,
}

impl JsonDataSource {
    fn new(version_columns: Vec<HashMap<String, Value>>) -> Self {
        JsonDataSource { version_columns }
    }

    /// Creates a JSON data source from Postgres queries.
    pub(crate) fn from_postgres(args: &DataArgs) -> Result<Self, DataError> {
        let pg_config_str = args
            .postgres
            .as_ref()
            .ok_or_else(|| DataError::MiscError("missing postgres config".to_string()))?;

        let json_str = load_json_string_or_file(pg_config_str)?;
        let config: PostgresConfig = serde_json::from_str(&json_str)
            .map_err(|e| DataError::FileError(format!("failed to parse JSON: {}", e)))?;

        let mut client = Client::connect(&config.url, NoTls)
            .map_err(|e| DataError::MiscError(format!("failed to connect to Postgres: {}", e)))?;

        let versions = args.get_version_list();
        let mut version_columns = Vec::with_capacity(versions.len());

        for version in &versions {
            let row = client
                .query_one(&config.query_template, &[version])
                .map_err(|e| {
                    DataError::RetrievalError(format!(
                        "query failed for version '{}': {}",
                        version, e
                    ))
                })?;

            let json_str: String = row.try_get(0).map_err(|e| {
                DataError::RetrievalError(format!(
                    "failed to get JSON column for version '{}': {}",
                    version, e
                ))
            })?;

            let response_value: Value = serde_json::from_str(&json_str).map_err(|e| {
                DataError::RetrievalError(format!(
                    "failed to parse JSON for version '{}': {}",
                    version, e
                ))
            })?;

            // Navigate into nested objects if data_path is specified
            let target_value =
                extract_nested_value(&response_value, &config.data_path).map_err(|e| {
                    DataError::RetrievalError(format!(
                        "failed to extract nested data for version '{}': {}",
                        version, e
                    ))
                })?;

            let map: HashMap<String, Value> = target_value
                .as_object()
                .ok_or_else(|| {
                    DataError::RetrievalError(format!(
                        "expected object at data_path for version '{}'",
                        version
                    ))
                })?
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            version_columns.push(map);
        }

        Ok(Self::new(version_columns))
    }

    /// Creates a JSON data source from REST API calls.
    pub(crate) fn from_rest(args: &DataArgs) -> Result<Self, DataError> {
        let rest_config_str = args
            .rest
            .as_ref()
            .ok_or_else(|| DataError::MiscError("missing rest config".to_string()))?;

        let json_str = load_json_string_or_file(rest_config_str)?;
        let config: RestConfig = serde_json::from_str(&json_str)
            .map_err(|e| DataError::FileError(format!("failed to parse JSON: {}", e)))?;

        let versions = args.get_version_list();
        let mut version_columns = Vec::with_capacity(versions.len());

        for version in &versions {
            let encoded_version =
                percent_encoding::utf8_percent_encode(version, percent_encoding::NON_ALPHANUMERIC);
            let url = config.url.replace("$1", &encoded_version.to_string());

            let mut request = ureq::get(&url);
            for (key, value) in &config.headers {
                request = request.header(key, value);
            }

            let response = request.call().map_err(|e| {
                DataError::RetrievalError(format!(
                    "REST request failed for version '{}': {}",
                    version, e
                ))
            })?;

            let json_str = response.into_body().read_to_string().map_err(|e| {
                DataError::RetrievalError(format!(
                    "failed to read response body for version '{}': {}",
                    version, e
                ))
            })?;

            let response_value: Value = serde_json::from_str(&json_str).map_err(|e| {
                DataError::RetrievalError(format!(
                    "failed to parse JSON for version '{}': {}",
                    version, e
                ))
            })?;

            // Navigate into nested objects if data_path is specified
            let target_value =
                extract_nested_value(&response_value, &config.data_path).map_err(|e| {
                    DataError::RetrievalError(format!(
                        "failed to extract nested data for version '{}': {}",
                        version, e
                    ))
                })?;

            let map: HashMap<String, Value> = target_value
                .as_object()
                .ok_or_else(|| {
                    DataError::RetrievalError(format!(
                        "expected object at data_path for version '{}'",
                        version
                    ))
                })?
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            version_columns.push(map);
        }

        Ok(Self::new(version_columns))
    }

    /// Creates a JSON data source from GraphQL API calls.
    pub(crate) fn from_graphql(args: &DataArgs) -> Result<Self, DataError> {
        let graphql_config_str = args
            .graphql
            .as_ref()
            .ok_or_else(|| DataError::MiscError("missing graphql config".to_string()))?;

        let json_str = load_json_string_or_file(graphql_config_str)?;
        let config: GraphQLConfig = serde_json::from_str(&json_str)
            .map_err(|e| DataError::FileError(format!("failed to parse JSON: {}", e)))?;

        let versions = args.get_version_list();
        let mut version_columns = Vec::with_capacity(versions.len());

        for version in &versions {
            let mut variables = serde_json::Map::new();
            // Add any static variables from config first
            for (key, value) in &config.variables {
                variables.insert(key.clone(), value.clone());
            }
            // Override/add the dynamic version variable
            variables.insert(
                config.version_variable.clone(),
                serde_json::Value::String(version.clone()),
            );

            let request_body = serde_json::json!({
                "query": config.query,
                "variables": variables
            });

            let mut request = ureq::post(&config.url).header("Content-Type", "application/json");
            for (key, value) in &config.headers {
                request = request.header(key, value);
            }

            let body = serde_json::to_string(&request_body).map_err(|e| {
                DataError::RetrievalError(format!("failed to serialize GraphQL request: {}", e))
            })?;

            let response = request.send(body.as_bytes()).map_err(|e| {
                DataError::RetrievalError(format!(
                    "GraphQL request failed for version '{}': {}",
                    version, e
                ))
            })?;

            let json_str = response.into_body().read_to_string().map_err(|e| {
                DataError::RetrievalError(format!(
                    "failed to read response body for version '{}': {}",
                    version, e
                ))
            })?;

            let response_value: Value = serde_json::from_str(&json_str).map_err(|e| {
                DataError::RetrievalError(format!(
                    "failed to parse JSON response for version '{}': {}",
                    version, e
                ))
            })?;

            // Check for GraphQL errors
            if let Some(errors) = response_value.get("errors") {
                return Err(DataError::RetrievalError(format!(
                    "GraphQL errors for version '{}': {}",
                    version,
                    serde_json::to_string(errors).unwrap_or_else(|_| "unknown error".to_string())
                )));
            }

            // GraphQL responses wrap data in { "data": { ... } }
            let data_value = response_value.get("data").ok_or_else(|| {
                DataError::RetrievalError(format!(
                    "GraphQL response missing 'data' field for version '{}'",
                    version
                ))
            })?;

            // Navigate into nested objects if data_path is specified
            let target_value =
                extract_nested_value(data_value, &config.data_path).map_err(|e| {
                    DataError::RetrievalError(format!(
                        "failed to extract nested data for version '{}': {}",
                        version, e
                    ))
                })?;

            let map: HashMap<String, Value> = target_value
                .as_object()
                .ok_or_else(|| {
                    DataError::RetrievalError(format!(
                        "expected object at data_path for version '{}'",
                        version
                    ))
                })?
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            version_columns.push(map);
        }

        Ok(Self::new(version_columns))
    }

    /// Creates a JSON data source from a JSON object.
    /// Expected format: `{ "VersionName": { "key1": value1, "key2": value2, ... }, ... }`
    pub(crate) fn from_json(args: &DataArgs) -> Result<Self, DataError> {
        let json_str = args
            .json
            .as_ref()
            .ok_or_else(|| DataError::MiscError("missing json config".to_string()))?;

        let json_content = load_json_string_or_file(json_str)?;
        let data: HashMap<String, HashMap<String, Value>> = serde_json::from_str(&json_content)
            .map_err(|e| DataError::FileError(format!("failed to parse JSON: {}", e)))?;

        let versions = args.get_version_list();
        let mut version_columns = Vec::with_capacity(versions.len());

        for version in &versions {
            let map = data
                .get(version)
                .ok_or_else(|| {
                    DataError::RetrievalError(format!(
                        "version '{}' not found in JSON data",
                        version
                    ))
                })?
                .clone();
            version_columns.push(map);
        }

        Ok(Self::new(version_columns))
    }

    fn lookup(&self, name: &str) -> Option<&Value> {
        self.version_columns
            .iter()
            .find_map(|map| map.get(name).filter(|v| !v.is_null()))
    }

    fn value_to_data_value(value: &Value) -> Result<DataValue, DataError> {
        match value {
            Value::Bool(b) => Ok(DataValue::Bool(*b)),
            Value::Number(n) => {
                if let Some(u) = n.as_u64() {
                    Ok(DataValue::U64(u))
                } else if let Some(i) = n.as_i64() {
                    Ok(DataValue::I64(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(DataValue::F64(f))
                } else {
                    Err(DataError::RetrievalError(
                        "unsupported numeric type".to_string(),
                    ))
                }
            }
            Value::String(s) => Ok(DataValue::Str(s.clone())),
            _ => Err(DataError::RetrievalError(
                "expected scalar value".to_string(),
            )),
        }
    }

    fn parse_delimited_numbers(s: &str) -> Option<Vec<DataValue>> {
        s.split(|c: char| c.is_whitespace() || c == ',' || c == ';')
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .map(|p| {
                p.parse::<u64>()
                    .map(DataValue::U64)
                    .ok()
                    .or_else(|| p.parse::<i64>().map(DataValue::I64).ok())
                    .or_else(|| p.parse::<f64>().map(DataValue::F64).ok())
            })
            .collect()
    }
}

impl DataSource for JsonDataSource {
    fn retrieve_single_value(&self, name: &str) -> Result<DataValue, DataError> {
        let result = (|| {
            let value = self
                .lookup(name)
                .ok_or_else(|| DataError::RetrievalError("key not found in any version".into()))?;

            let dv = Self::value_to_data_value(value)?;
            match dv {
                DataValue::Str(_) => Err(DataError::RetrievalError(
                    "Found non-numeric single value".to_string(),
                )),
                _ => Ok(dv),
            }
        })();

        result.map_err(|e| DataError::WhileRetrieving {
            name: name.to_string(),
            source: Box::new(e),
        })
    }

    fn retrieve_1d_array_or_string(&self, name: &str) -> Result<ValueSource, DataError> {
        let result = (|| {
            let value = self
                .lookup(name)
                .ok_or_else(|| DataError::RetrievalError("key not found in any version".into()))?;

            match value {
                Value::Array(arr) => {
                    let items: Result<Vec<_>, _> =
                        arr.iter().map(Self::value_to_data_value).collect();
                    Ok(ValueSource::Array(items?))
                }
                Value::String(s) => match Self::parse_delimited_numbers(s) {
                    Some(arr) if !arr.is_empty() => Ok(ValueSource::Array(arr)),
                    _ => Ok(ValueSource::Single(DataValue::Str(s.clone()))),
                },
                _ => Err(DataError::RetrievalError(
                    "expected array or string for 1D array".to_string(),
                )),
            }
        })();

        result.map_err(|e| DataError::WhileRetrieving {
            name: name.to_string(),
            source: Box::new(e),
        })
    }

    fn retrieve_2d_array(&self, name: &str) -> Result<Vec<Vec<DataValue>>, DataError> {
        let result = (|| {
            let value = self
                .lookup(name)
                .ok_or_else(|| DataError::RetrievalError("key not found in any version".into()))?;

            let Value::Array(outer) = value else {
                return Err(DataError::RetrievalError(
                    "expected 2D array (array of arrays)".to_string(),
                ));
            };

            outer
                .iter()
                .map(|row_val| {
                    let Value::Array(inner) = row_val else {
                        return Err(DataError::RetrievalError(
                            "expected array for 2D array row".to_string(),
                        ));
                    };
                    inner.iter().map(Self::value_to_data_value).collect()
                })
                .collect()
        })();

        result.map_err(|e| DataError::WhileRetrieving {
            name: name.to_string(),
            source: Box::new(e),
        })
    }
}
