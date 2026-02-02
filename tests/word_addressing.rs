use mint_cli::commands;
use mint_cli::layout::args::BlockNames;
use mint_cli::output::args::OutputArgs;

#[path = "common/mod.rs"]
mod common;

#[test]
fn swapword_swaps_bytes() {
    let layout = r#"
[settings]
endianness = "little"

[block.header]
start_address = 0x1000
length = 0x20
padding = 0xFF

[block.data]
val1 = { value = 0x1234, type = "u16" }
val2 = { value = 0x5678, type = "u16" }
"#;

    let path = common::write_layout_file("swapword_basic", layout);

    let args = mint_cli::args::Args {
        layout: mint_cli::layout::args::LayoutArgs {
            blocks: vec![BlockNames {
                name: "block".to_string(),
                file: path,
            }],
            strict: false,
        },
        data: mint_cli::data::args::DataArgs::default(),
        output: OutputArgs {
            hexview: "@1 /SWAPWORD /XI -o out/swapword.hex".to_string(),
            export_json: None,
            stats: false,
            quiet: false,
        },
    };

    commands::build(&args, None).expect("build should succeed");

    let content = std::fs::read("out/swapword.hex").expect("read hex file");
    let hexfile = h3xy::parse_intel_hex(&content).expect("parse intel hex");

    assert_eq!(hexfile.read_byte(0x1000), Some(0x12));
    assert_eq!(hexfile.read_byte(0x1001), Some(0x34));
    assert_eq!(hexfile.read_byte(0x1002), Some(0x56));
    assert_eq!(hexfile.read_byte(0x1003), Some(0x78));
}

#[test]
fn remap_offsets_addresses() {
    let layout = r#"
[settings]
endianness = "little"

[block.header]
start_address = 0x1000
length = 0x20
padding = 0xFF

[block.data]
val = { value = 0xAA, type = "u8" }
"#;

    let path = common::write_layout_file("remap_basic", layout);

    let args = mint_cli::args::Args {
        layout: mint_cli::layout::args::LayoutArgs {
            blocks: vec![BlockNames {
                name: "block".to_string(),
                file: path,
            }],
            strict: false,
        },
        data: mint_cli::data::args::DataArgs::default(),
        output: OutputArgs {
            hexview: "@1 /REMAP:0x1000-0x10FF,0x2000,0x100,0x100 /XI -o out/remap.hex".to_string(),
            export_json: None,
            stats: false,
            quiet: false,
        },
    };

    commands::build(&args, None).expect("build should succeed");

    let content = std::fs::read("out/remap.hex").expect("read hex file");
    let hexfile = h3xy::parse_intel_hex(&content).expect("parse intel hex");

    assert_eq!(hexfile.read_byte(0x1000), None);
    assert_eq!(hexfile.read_byte(0x2000), Some(0xAA));
}
