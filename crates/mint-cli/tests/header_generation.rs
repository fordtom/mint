use std::fs;
use std::process::Command;

#[path = "common/mod.rs"]
mod common;

fn mint_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mint"));
    command.current_dir(env!("CARGO_MANIFEST_DIR"));
    command
}

#[test]
fn generated_example_header_is_checked_in() {
    let header_path = common::unique_out_path("generated-blocks", "h");
    let output = mint_command()
        .args(["header", "../../doc/examples/block.toml", "-o"])
        .arg(&header_path)
        .output()
        .expect("mint header should run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let generated = fs::read_to_string(&header_path).expect("generated header is readable");
    let checked_in = fs::read_to_string("../../doc/examples/blocks.h")
        .expect("checked-in generated header is readable")
        .replace("\r\n", "\n");
    assert_eq!(generated, checked_in);
}

#[test]
fn validation_failure_does_not_touch_output() {
    let layout = common::write_layout_file(
        "invalid-header",
        r#"
[mint]
abi = "generic-le"
[block.header]
start_address = 0
length = 16
[block.data]
for = { value = 1, type = "u8" }
"#,
    );
    let output_path = common::unique_out_path("preserved-header", "h");
    fs::write(&output_path, "preserve me\n").expect("sentinel header writes");

    let output = mint_command()
        .args(["header", &layout, "-o"])
        .arg(&output_path)
        .output()
        .expect("mint header should run");
    assert!(!output.status.success());
    assert_eq!(
        fs::read_to_string(output_path).expect("sentinel header remains readable"),
        "preserve me\n"
    );
}
