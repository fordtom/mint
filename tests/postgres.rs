//! Integration tests for PostgresDataSource.
//!
//! Requires a running Postgres server. Skip with: cargo test --test postgres -- --ignored
//! Or run specifically: cargo test --test postgres -- --include-ignored

use mint_cli::layout::value::{DataValue, ValueSource};
use mint_cli::variant::args::VariantArgs;
use mint_cli::variant::create_data_source;

const TEST_DB_URL: &str = "postgres://localhost/mint_test";

fn setup_test_data() {
    use postgres::{Client, NoTls};

    let mut client = Client::connect(TEST_DB_URL, NoTls).expect("connect to test db");

    client
        .batch_execute(
            r#"
            DROP TABLE IF EXISTS config CASCADE;
            CREATE TABLE config (
                variant TEXT NOT NULL,
                name TEXT NOT NULL,
                value JSONB NOT NULL,
                PRIMARY KEY (variant, name)
            );

            INSERT INTO config (variant, name, value) VALUES
                ('Default', 'TemperatureMax', '50'),
                ('Default', 'Value2', '2'),
                ('Default', 'enabled', 'true'),
                ('Default', 'arraySpaces', '"0 100 200 300"'),
                ('Default', 'arrayCommas', '"1,2,3,4"'),
                ('Default', 'arraySemicolons', '"10; 20; 30"'),
                ('Default', 'arrayMixed', '"5, 10; 15 20"'),
                ('Default', 'arraySingle', '"42"'),
                ('Default', 'arrayFloats', '"1.5 2.5 3.5"'),
                ('Default', 'arrayNegative', '"-1 -2 -3"'),
                ('Default', 'literalString', '"hello world"'),
                ('Default', 'nativeArray1d', '[10, 20, 30]'),
                ('Default', 'nativeArray2d', '[[1, 2], [3, 4], [5, 6]]'),
                ('Debug', 'TemperatureMax', '60'),
                ('Debug', 'debugMode', 'true'),
                ('VarA', 'TemperatureMax', '55'),
                ('VarA', 'enabled', 'false');
            "#,
        )
        .expect("setup test data");
}

fn build_pg_args(variant: &str) -> VariantArgs {
    let config = format!(
        r#"{{
            "url": "{}",
            "query_template": "SELECT json_object_agg(name, value)::text FROM config WHERE variant = $1"
        }}"#,
        TEST_DB_URL
    );

    VariantArgs {
        postgres: Some(config),
        variant: Some(variant.to_string()),
        ..Default::default()
    }
}

#[test]
#[ignore = "requires running postgres server"]
fn postgres_retrieve_single_value_priority_order() {
    setup_test_data();

    let args = build_pg_args("VarA/Debug/Default");
    let ds = create_data_source(&args)
        .expect("datasource load")
        .expect("datasource exists");

    // VarA has TemperatureMax=55, should take priority
    let value = ds.retrieve_single_value("TemperatureMax").unwrap();
    println!("TemperatureMax (VarA/Debug/Default): {:?}", value);
    assert!(matches!(value, DataValue::U64(55)));

    // VarA has enabled=false
    let value = ds.retrieve_single_value("enabled").unwrap();
    println!("enabled (VarA/Debug/Default): {:?}", value);
    assert!(matches!(value, DataValue::Bool(false)));

    // debugMode only in Debug
    let value = ds.retrieve_single_value("debugMode").unwrap();
    println!("debugMode (VarA/Debug/Default): {:?}", value);
    assert!(matches!(value, DataValue::Bool(true)));

    // Value2 only in Default
    let value = ds.retrieve_single_value("Value2").unwrap();
    println!("Value2 (VarA/Debug/Default): {:?}", value);
    assert!(matches!(value, DataValue::U64(2)));
}

#[test]
#[ignore = "requires running postgres server"]
fn postgres_retrieve_single_value_fallback() {
    setup_test_data();

    let args = build_pg_args("Debug/Default");
    let ds = create_data_source(&args)
        .expect("datasource load")
        .expect("datasource exists");

    // Debug has TemperatureMax=60, should take priority over Default's 50
    let value = ds.retrieve_single_value("TemperatureMax").unwrap();
    println!("TemperatureMax (Debug/Default): {:?}", value);
    assert!(matches!(value, DataValue::U64(60)));

    // enabled not in Debug, falls back to Default's true
    let value = ds.retrieve_single_value("enabled").unwrap();
    println!("enabled (Debug/Default): {:?}", value);
    assert!(matches!(value, DataValue::Bool(true)));
}

#[test]
#[ignore = "requires running postgres server"]
fn postgres_retrieve_missing_key_errors() {
    setup_test_data();

    let args = build_pg_args("Default");
    let ds = create_data_source(&args)
        .expect("datasource load")
        .expect("datasource exists");

    let result = ds.retrieve_single_value("NonExistent");
    assert!(result.is_err());
    println!("Missing key error: {:?}", result.unwrap_err());
}

#[test]
#[ignore = "requires running postgres server"]
fn postgres_retrieve_1d_array_space_delimited() {
    setup_test_data();

    let args = build_pg_args("Default");
    let ds = create_data_source(&args).unwrap().unwrap();

    let value = ds.retrieve_1d_array_or_string("arraySpaces").unwrap();
    println!("arraySpaces: {:?}", value);
    let ValueSource::Array(arr) = value else {
        panic!("expected array");
    };
    assert_eq!(arr.len(), 4);
    assert!(matches!(arr[0], DataValue::U64(0)));
    assert!(matches!(arr[3], DataValue::U64(300)));
}

#[test]
#[ignore = "requires running postgres server"]
fn postgres_retrieve_1d_array_comma_delimited() {
    setup_test_data();

    let args = build_pg_args("Default");
    let ds = create_data_source(&args).unwrap().unwrap();

    let value = ds.retrieve_1d_array_or_string("arrayCommas").unwrap();
    println!("arrayCommas: {:?}", value);
    let ValueSource::Array(arr) = value else {
        panic!("expected array");
    };
    assert_eq!(arr.len(), 4);
}

#[test]
#[ignore = "requires running postgres server"]
fn postgres_retrieve_1d_array_mixed_delimiters() {
    setup_test_data();

    let args = build_pg_args("Default");
    let ds = create_data_source(&args).unwrap().unwrap();

    let value = ds.retrieve_1d_array_or_string("arrayMixed").unwrap();
    println!("arrayMixed: {:?}", value);
    let ValueSource::Array(arr) = value else {
        panic!("expected array");
    };
    assert_eq!(arr.len(), 4);
}

#[test]
#[ignore = "requires running postgres server"]
fn postgres_retrieve_1d_array_single_value() {
    setup_test_data();

    let args = build_pg_args("Default");
    let ds = create_data_source(&args).unwrap().unwrap();

    let value = ds.retrieve_1d_array_or_string("arraySingle").unwrap();
    println!("arraySingle: {:?}", value);
    let ValueSource::Array(arr) = value else {
        panic!("expected array");
    };
    assert_eq!(arr.len(), 1);
    assert!(matches!(arr[0], DataValue::U64(42)));
}

#[test]
#[ignore = "requires running postgres server"]
fn postgres_retrieve_1d_array_floats() {
    setup_test_data();

    let args = build_pg_args("Default");
    let ds = create_data_source(&args).unwrap().unwrap();

    let value = ds.retrieve_1d_array_or_string("arrayFloats").unwrap();
    println!("arrayFloats: {:?}", value);
    let ValueSource::Array(arr) = value else {
        panic!("expected array");
    };
    assert_eq!(arr.len(), 3);
    assert!(matches!(arr[0], DataValue::F64(f) if (f - 1.5).abs() < 0.001));
}

#[test]
#[ignore = "requires running postgres server"]
fn postgres_retrieve_1d_array_negative() {
    setup_test_data();

    let args = build_pg_args("Default");
    let ds = create_data_source(&args).unwrap().unwrap();

    let value = ds.retrieve_1d_array_or_string("arrayNegative").unwrap();
    println!("arrayNegative: {:?}", value);
    let ValueSource::Array(arr) = value else {
        panic!("expected array");
    };
    assert_eq!(arr.len(), 3);
    assert!(matches!(arr[0], DataValue::I64(-1)));
}

#[test]
#[ignore = "requires running postgres server"]
fn postgres_retrieve_1d_literal_string() {
    setup_test_data();

    let args = build_pg_args("Default");
    let ds = create_data_source(&args).unwrap().unwrap();

    let value = ds.retrieve_1d_array_or_string("literalString").unwrap();
    println!("literalString: {:?}", value);
    let ValueSource::Single(DataValue::Str(s)) = value else {
        panic!("expected single string");
    };
    assert_eq!(s, "hello world");
}

#[test]
#[ignore = "requires running postgres server"]
fn postgres_retrieve_1d_native_json_array() {
    setup_test_data();

    let args = build_pg_args("Default");
    let ds = create_data_source(&args).unwrap().unwrap();

    let value = ds.retrieve_1d_array_or_string("nativeArray1d").unwrap();
    println!("nativeArray1d: {:?}", value);
    let ValueSource::Array(arr) = value else {
        panic!("expected array");
    };
    assert_eq!(arr.len(), 3);
    assert!(matches!(arr[0], DataValue::U64(10)));
    assert!(matches!(arr[1], DataValue::U64(20)));
    assert!(matches!(arr[2], DataValue::U64(30)));
}

#[test]
#[ignore = "requires running postgres server"]
fn postgres_retrieve_2d_native_json_array() {
    setup_test_data();

    let args = build_pg_args("Default");
    let ds = create_data_source(&args).unwrap().unwrap();

    let value = ds.retrieve_2d_array("nativeArray2d").unwrap();
    println!("nativeArray2d: {:?}", value);
    assert_eq!(value.len(), 3);
    assert_eq!(value[0].len(), 2);
    assert!(matches!(value[0][0], DataValue::U64(1)));
    assert!(matches!(value[0][1], DataValue::U64(2)));
    assert!(matches!(value[2][0], DataValue::U64(5)));
    assert!(matches!(value[2][1], DataValue::U64(6)));
}
