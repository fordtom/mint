use mint_cli::layout::value::DataValue;
use mint_cli::variant::DataSheet;
use mint_cli::variant::args::VariantArgs;

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
        DataValue::Str(s) => panic!("expected numeric value, got {}", s),
    }
}

#[test]
fn stacked_variants_respect_order() {
    let args = build_args(Some("VarA/Debug"), false);
    let sheet = DataSheet::new(&args)
        .expect("datasheet load")
        .expect("datasheet exists");

    let value = sheet
        .retrieve_single_value("TemperatureMax")
        .expect("value present");

    assert_eq!(value_as_i64(value), 55);
}

#[test]
fn stacked_variants_fall_back_when_empty() {
    let args = build_args(Some(" VarA / Debug "), false);
    let sheet = DataSheet::new(&args)
        .expect("datasheet load")
        .expect("datasheet exists");

    let value = sheet
        .retrieve_single_value("Value 2")
        .expect("value present");

    // VarA is empty for this row, so Debug should supply the value.
    assert_eq!(value_as_i64(value), 2);
}

#[test]
fn legacy_debug_flag_still_applies_first() {
    let args = build_args(Some("VarA"), true);
    let sheet = DataSheet::new(&args)
        .expect("datasheet load")
        .expect("datasheet exists");

    let value = sheet
        .retrieve_single_value("TemperatureMax")
        .expect("value present");

    assert_eq!(value_as_i64(value), 60);
}
