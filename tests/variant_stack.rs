use mint_cli::layout::value::DataValue;
use mint_cli::variant::args::VariantArgs;
use mint_cli::variant::create_data_source;

fn build_args(variant: Option<&str>, debug: bool) -> VariantArgs {
    VariantArgs {
        xlsx: Some("examples/data.xlsx".to_string()),
        main_sheet: "Main".to_string(),
        variant: variant.map(|v| v.to_string()),
        debug,
    }
}

fn value_as_i64(value: DataValue) -> i64 {
    match value {
        DataValue::I64(v) => v,
        DataValue::U64(v) => v as i64,
        DataValue::F64(v) => v as i64,
        DataValue::Bool(v) => i64::from(v),
        DataValue::Str(s) => panic!("expected numeric value, got {}", s),
    }
}

#[test]
fn stacked_variants_respect_order() {
    let args = build_args(Some("VarA/Debug"), false);
    let ds = create_data_source(&args)
        .expect("datasource load")
        .expect("datasource exists");

    let value = ds
        .retrieve_single_value("TemperatureMax")
        .expect("value present");

    assert_eq!(value_as_i64(value), 55);
}

#[test]
fn stacked_variants_fall_back_when_empty() {
    let args = build_args(Some(" VarA / Debug "), false);
    let ds = create_data_source(&args)
        .expect("datasource load")
        .expect("datasource exists");

    let value = ds.retrieve_single_value("Value 2").expect("value present");

    assert_eq!(value_as_i64(value), 2);
}

#[test]
fn legacy_debug_flag_still_applies_first() {
    let args = build_args(Some("VarA"), true);
    let ds = create_data_source(&args)
        .expect("datasource load")
        .expect("datasource exists");

    let value = ds
        .retrieve_single_value("TemperatureMax")
        .expect("value present");

    assert_eq!(value_as_i64(value), 60);
}

#[test]
fn boolean_cell_retrieves_default_true() {
    let args = build_args(None, false);
    let ds = create_data_source(&args)
        .expect("datasource load")
        .expect("datasource exists");

    let value = ds
        .retrieve_single_value("boolean")
        .expect("boolean present");

    assert!(matches!(value, DataValue::Bool(true)));
}

#[test]
fn boolean_cell_retrieves_debug_true() {
    let args = build_args(Some("Debug"), false);
    let ds = create_data_source(&args)
        .expect("datasource load")
        .expect("datasource exists");

    let value = ds
        .retrieve_single_value("boolean")
        .expect("boolean present");

    assert!(matches!(value, DataValue::Bool(true)));
}

#[test]
fn boolean_cell_retrieves_vara_false() {
    let args = build_args(Some("VarA"), false);
    let ds = create_data_source(&args)
        .expect("datasource load")
        .expect("datasource exists");

    let value = ds
        .retrieve_single_value("boolean")
        .expect("boolean present");

    assert!(matches!(value, DataValue::Bool(false)));
}
