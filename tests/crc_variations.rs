use mint_cli::commands;

#[path = "common/mod.rs"]
mod common;

fn build_checksum_output(hexview: &str, out_path: &str) -> h3xy::HexFile {
    common::ensure_out_dir();

    let layout = r#"
[settings]
endianness = "little"

[block.header]
start_address = 0x1000
length = 0x20
padding = 0xFF

[block.data]
payload = { value = [1, 2, 3, 4], type = "u8", size = 4 }
"#;

    let layout_path = common::write_layout_file("checksum_layout", layout);

    let args = mint_cli::args::Args {
        layout: mint_cli::layout::args::LayoutArgs {
            blocks: vec![mint_cli::layout::args::BlockNames {
                name: "block".to_string(),
                file: layout_path,
            }],
            strict: false,
        },
        data: Default::default(),
        output: mint_cli::output::args::OutputArgs {
            hexview: hexview.to_string(),
            export_json: None,
            stats: false,
            quiet: true,
        },
    };

    commands::build(&args, None).expect("build should succeed");

    let content = std::fs::read(out_path).expect("read output");
    h3xy::parse_intel_hex(&content).expect("parse intel hex")
}

#[test]
fn checksum_append_bytesum_be() {
    let out_path = "out/checksum_append_be.hex";
    let hexview = format!("@1 /CS0:@append /XI -o {out_path}");
    let hexfile = build_checksum_output(&hexview, out_path);

    // bytes [1,2,3,4] -> sum 10 (0x000A) big endian => 00 0A
    assert_eq!(hexfile.read_byte(0x1004), Some(0x00));
    assert_eq!(hexfile.read_byte(0x1005), Some(0x0A));
}

#[test]
fn checksum_append_bytesum_le() {
    let out_path = "out/checksum_append_le.hex";
    let hexview = format!("@1 /CSR0:@append /XI -o {out_path}");
    let hexfile = build_checksum_output(&hexview, out_path);

    // bytes [1,2,3,4] -> sum 10 (0x000A) little endian => 0A 00
    assert_eq!(hexfile.read_byte(0x1004), Some(0x0A));
    assert_eq!(hexfile.read_byte(0x1005), Some(0x00));
}
