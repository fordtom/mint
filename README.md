## mint

Build flash blocks from a layout file (TOML/YAML/JSON) and a data source (currently Excel or Postgres), then emit hex files.

![img](img.png)

Install with `cargo install mint-cli` or via nix flakes.

Run `mint --help` for all available options, or see the [CLI documentation](doc/cli.md). Example layouts and data are in [`doc/examples/`](doc/examples/).

### Input formats

Brief TOML illustrating common patterns (see [`doc/examples/block.toml`](doc/examples/block.toml)):

```toml
[block.data]
# single numeric from Excel by name
device.info.version.major = { name = "FWVersionMajor", type = "u16" }

# fixed-size string from Excel (padded)
device.info.name = { name = "DeviceName", type = "u8", size = 16 }

# 1D numeric array from Excel sheet reference
calibration.coefficients = { name = "Coefficients1D", type = "f32", size = 8 }

# 2D numeric array from Excel sheet reference
calibration.matrix = { name = "CalibrationMatrix", type = "i16", size = [3, 3] }

# literals
message = { value = "Hello", type = "u8", size = 16 }
net.ip  = { value = [192, 168, 1, 100], type = "u8", size = 4 }
```

Datasheet view (how the Excel workbook is interpreted). See workbook in [`doc/examples/data.xlsx`](doc/examples/data.xlsx).

Main sheet (first row is headers):

| Name              | Default            | Debug | VarA |
| ----------------- | ------------------ | ----- | ---- |
| DeviceName        | MyDevice           |       |      |
| FWVersionMajor    | 3                  |       | 4    |
| Coefficients1D    | #Coefficients1D    |       |      |
| CalibrationMatrix | #CalibrationMatrix |       |      |

Notes:

- Precedence follows the order passed to `-v`/`--variant`. Supply columns separated by `/` (for example `-v Debug/VarA`). The first non-empty entry wins, falling back to `Default`.
- Cell starting with `#` is a sheet reference (for arrays); otherwise a string cell is literal bytes (for u8 strings).

Arrays from Excel sheets:

- First row is headers; width of the header row defines a 2D array's expected width.
- Values are taken only as complete rows until an empty cell is encountered.

| C1  | C2  | C3  |
| --- | --- | --- |
| 1   | 2   | 3   |
| 4   | 5   | 6   |
| 7   | 8   | 9   |
| ... | ... | ... |

Strings and undersized arrays are padded to their expected size by default. If you want to enforce strict length, use the `SIZE` in place of `size` in the layout file (e.g. `SIZE = 8`).

### Postgres data source

As an alternative to Excel, you can use a Postgres database via `-p`/`--postgres`:

```bash
mint layout.toml -p config.json -v Debug/Default
# or inline JSON:
mint layout.toml -p '{"database":{"url":"..."},"query":{"template":"..."}}' -v Debug/Default
```

Config format (JSON file or inline string):

```json
{
  "database": { "url": "postgres://user:pass@host/db" },
  "query": {
    "template": "SELECT json_object_agg(name, value)::text FROM config WHERE variant = $1"
  }
}
```

The query is executed once per variant (passed as `$1`) and must return a single row with column 0 containing a JSON object mapping names to values. Native JSON arrays are supported for 1D/2D arrays; space/comma/semicolon-delimited strings are also parsed as numeric arrays.
