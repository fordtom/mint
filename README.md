## mint

Build flash blocks from a layout file (TOML/YAML/JSON) and a data source (Excel, Postgres, HTTP, or JSON), then emit hex files.

![img](doc/img.png)

Install with `cargo install mint-cli` or via nix flakes.

### Documentation

- [CLI reference](doc/cli.md)
- [Layout files](doc/layout.md)
- [Data sources](doc/sources.md)
- [Example layouts & data](doc/examples/)

### Quick Start

```bash
# Excel data source
mint config@layout.toml --xlsx data.xlsx -v Default -o "@1 /XI -o out.hex"

# Postgres data source
mint config@layout.toml --postgres config.json -v Debug/Default -o "@1 /XI -o out.hex"

# HTTP data source
mint config@layout.toml --http config.json -v Debug/Default -o "@1 /XI -o out.hex"

# JSON data source
mint config@layout.toml --json data.json -v Debug/Default -o "@1 /XI -o out.hex"

# Multiple blocks with options
mint config@layout.toml calibration@layout.toml --xlsx data.xlsx -v Production/Default \
  -o "@1 /MO:@2 /XI -o out/combined.hex" --stats
```

### Layout Example

```toml
[block.data]
device.info.version.major = { name = "FWVersionMajor", type = "u16" }
device.info.name = { name = "DeviceName", type = "u8", size = 16 }
calibration.coefficients = { name = "Coefficients1D", type = "f32", size = 8 }
calibration.matrix = { name = "CalibrationMatrix", type = "i16", size = [3, 3] }
message = { value = "Hello", type = "u8", size = 16 }
```

See [`doc/examples/block.toml`](doc/examples/block.toml) for full examples.
