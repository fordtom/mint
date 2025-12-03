#![allow(dead_code, unused_variables, unused_imports)]

use postgres::{Client, Error, NoTls};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

use super::args::VariantArgs;
use super::errors::VariantError;
use super::helpers;
use super::DataSource;
use crate::layout::value::{DataValue, ValueSource};

#[derive(Debug, Deserialize)]
struct PostgresConfig {
    database: DatabaseConfig,
    query: QueryConfig,
}

#[derive(Debug, Deserialize)]
struct DatabaseConfig {
    url: String,
    token_env_var: String,
}

#[derive(Debug, Deserialize)]
struct QueryConfig {
    template: String,
}

fn load_config(input: &str) -> Result<PostgresConfig, VariantError> {
    if input.ends_with(".json") {
        let input = std::fs::read_to_string(input)
            .map_err(|_| VariantError::FileError(format!("failed to open file: {}", input)))?;
    }

    let config: PostgresConfig = serde_json::from_str(input)
        .map_err(|e| VariantError::FileError(format!("failed to parse JSON: {}", e)))?;
    Ok(config)
}

pub struct PostgresDataSource {
    variant_columns: Vec<HashMap<String, String>>,
}

impl PostgresDataSource {
    pub(crate) fn new(args: &VariantArgs) -> Result<Self, VariantError> {
        Err(VariantError::MiscError(
            "Postgres data source not implemented".to_string(),
        ))
    }
}

impl DataSource for PostgresDataSource {
    fn retrieve_single_value(&self, name: &str) -> Result<DataValue, VariantError> {
        Err(VariantError::MiscError(
            "Postgres data source not implemented".to_string(),
        ))
    }

    fn retrieve_1d_array_or_string(&self, name: &str) -> Result<ValueSource, VariantError> {
        Err(VariantError::MiscError(
            "Postgres data source not implemented".to_string(),
        ))
    }

    fn retrieve_2d_array(&self, name: &str) -> Result<Vec<Vec<DataValue>>, VariantError> {
        Err(VariantError::MiscError(
            "Postgres data source not implemented".to_string(),
        ))
    }
}
