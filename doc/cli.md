# Command Line Interface

mint builds flash blocks from layout files and data sources, then hands off output to HexView-compatible processing (h3xy).

```
mint [OPTIONS] BLOCK@FILE...
```

## Positional Arguments

### `BLOCK@FILE...`

Specifies which blocks to build. Each argument must be a block name paired with a layout file.

Order is preserved and referenced as `@1`, `@2`, ... in the output string.

**Examples:**

```bash
# Build single block
mint config@layout.toml --xlsx data.xlsx -v Default -o "@1 /XI -o config.hex"

# Build multiple specific blocks
mint config@layout.toml calibration@layout.toml --xlsx data.xlsx -v Default \
  -o "@1 /MO:@2 /XI -o firmware.hex"
```

---

## Data Source Options

You can specify exactly one data source (`--xlsx`, `--postgres`, `--http`, or `--json`) along with a variant (`-v`).

### `--xlsx <FILE>`

Path to Excel workbook containing variant data.

```bash
mint block@layout.toml --xlsx data.xlsx -v Default -o "@1 /XI -o output.hex"
```

### `--main-sheet <NAME>`

Override the default main sheet name (`Main`) for the excel data source.

```bash
mint block@layout.toml --xlsx data.xlsx --main-sheet Config -v Default -o "@1 /XI -o output.hex"
```

### `--postgres <PATH or JSON>`

Use PostgreSQL as the data source. Accepts a JSON file path or inline JSON string.

```bash
# Using a config file
mint block@layout.toml --postgres pg_config.json -v Default -o "@1 /XI -o output.hex"

# Using inline JSON
mint block@layout.toml --postgres '{"url":"...","query_template":"..."}' -v Default -o "@1 /XI -o output.hex"
```

See [Data Sources](sources.md#postgres--p---postgres) for config format details.

### `--http <PATH or JSON>`

Use HTTP API as the data source. Accepts a JSON file path or inline JSON string.

```bash
# Using a config file
mint block@layout.toml --http http_config.json -v Default -o "@1 /XI -o output.hex"

# Using inline JSON
mint block@layout.toml --http '{"url":"...","headers":{...}}' -v Default -o "@1 /XI -o output.hex"
```

See [Data Sources](sources.md#http---http) for config format details.

### `--json <PATH or JSON>`

Use raw JSON as the data source. Accepts a JSON file path or inline JSON string.

The JSON format is an object with variant names as top-level keys. Each variant contains an object with name:value pairs.

```bash
# Using a JSON file
mint block@layout.toml --json data.json -v Debug/Default -o "@1 /XI -o output.hex"

# Using inline JSON
mint block@layout.toml --json '{"Default":{"key1":123,"key2":"value"},"Debug":{"key1":456}}' -v Debug/Default -o "@1 /XI -o output.hex"
```

**Example JSON format:**

```json
{
  "Default": {
    "DeviceName": "MyDevice",
    "FWVersionMajor": 3,
    "Coefficients1D": [1.0, 2.0, 3.0]
  },
  "Debug": {
    "DeviceName": "DebugDevice",
    "FWVersionMajor": 4
  }
}
```

See [Data Sources](sources.md#json---json) for format details.

### `-v, --variant <NAME[/NAME...]>`

Variant columns to query, in priority order. The first non-empty value found wins.

```bash
# Single variant
mint block@layout.toml --xlsx data.xlsx -v Default -o "@1 /XI -o output.hex"

# Fallback chain: try Debug first, then Default
mint block@layout.toml --xlsx data.xlsx -v Debug/Default -o "@1 /XI -o output.hex"

# Three-level fallback
mint block@layout.toml --xlsx data.xlsx -v Production/Debug/Default -o "@1 /XI -o output.hex"
```

---

## Output Options

### `-o, --out <HEXVIEW>`

HexView-compatible CLI string (h3xy). Include the output file with `-o <file>` inside the string.
Use `@1`, `@2`, ... to reference input blocks in the order provided.

```bash
# Intel HEX output (default bytes per line)
mint block@layout.toml --xlsx data.xlsx -v Default -o "@1 /XI -o build/firmware.hex"

# Motorola S-Record with 16 bytes per line
mint block@layout.toml --xlsx data.xlsx -v Default -o "@1 /XS:16 -o build/firmware.mot"

# Merge two blocks (opaque) into a single HEX file
mint config@layout.toml calibration@layout.toml --xlsx data.xlsx -v Default \
  -o "@1 /MO:@2 /XI -o combined.hex"
```

### `--export-json <FILE>`

Export used `block.data` values as JSON. Report is nested by layout file, then block name.

```bash
mint block@layout.toml --xlsx data.xlsx -v Default \
  -o "@1 /XI -o output.hex" \
  --export-json build/report.json
```

---

## Build Options

### `--strict`

Enable strict type conversions. Errors on lossy casts instead of saturating/truncating.

```bash
mint block@layout.toml --xlsx data.xlsx -v Default -o "@1 /XI -o output.hex" --strict
```

**Without `--strict`:**

- Float `1.5` → `u8` becomes `1` (truncated)
- Value `300` → `u8` becomes `255` (saturated)

**With `--strict`:**

- Float `1.5` → `u8` produces an error
- Value `300` → `u8` produces an error

---

## Display Options

### `--stats`

Show detailed build statistics after completion.

```bash
mint block@layout.toml --xlsx data.xlsx -v Default -o "@1 /XI -o output.hex" --stats
```

**Example output:**

```
+------------------+--------------+
| Build Summary    |              |
+=================================+
| Build Time       | 4.878ms      |
|------------------+--------------|
| Blocks Processed | 6            |
|------------------+--------------|
| Total Allocated  | 13,056 bytes |
|------------------+--------------|
| Total Used       | 627 bytes    |
|------------------+--------------|
| Space Efficiency | 4.8%         |
+------------------+--------------+

+--------------+-----------------------+-----------------------+------------+------------+
| Block        | Address Range         | Used/Alloc            | Efficiency | CRC Value  |
+========================================================================================+
| block        | 0x0008B000-0x0008BFFF | 308 bytes/4,096 bytes | 7.5%       | 0xB1FAC7CA |
|--------------+-----------------------+-----------------------+------------+------------|
| block2       | 0x0008C000-0x0008CFFF | 80 bytes/4,096 bytes  | 2.0%       | 0x8CF01930 |
|--------------+-----------------------+-----------------------+------------+------------|
| block3       | 0x0008D000-0x0008DFFF | 160 bytes/4,096 bytes | 3.9%       | 0x0E8D6A3D |
|--------------+-----------------------+-----------------------+------------+------------|
| block_bitmap | 0x0008E000-0x0008E0FF | 19 bytes/256 bytes    | 7.4%       | 0x54A08471 |
|--------------+-----------------------+-----------------------+------------+------------|
| simple_block | 0x00008000-0x000080FF | 49 bytes/256 bytes    | 19.1%      | 0xFEBB07BD |
|--------------+-----------------------+-----------------------+------------+------------|
| pg_block     | 0x00001000-0x000010FF | 11 bytes/256 bytes    | 4.3%       | 0x5F67F442 |
+--------------+-----------------------+-----------------------+------------+------------+
```

### `--quiet`

Suppress all output except errors.

```bash
mint block@layout.toml --xlsx data.xlsx -v Default -o "@1 /XI -o output.hex" --quiet
```

---

## Help & Version

### `-h, --help`

Print help information.

```bash
mint --help
```

### `-V, --version`

Print version information.

```bash
mint --version
```

---

## Complete Examples

### Basic build with Excel data

```bash
mint block@layout.toml --xlsx data.xlsx -v Default -o "@1 /XI -o firmware.hex"
```

### Production build with all options

```bash
mint \
  config@layout.toml \
  calibration@layout.toml \
  --xlsx data.xlsx \
  -v Production/Default \
  -o "@1 /MO:@2 /XS:32 -o release/FW_v1.2.3.mot" \
  --strict \
  --stats
```

### Build with Postgres backend

```bash
mint block@layout.toml \
  --postgres pg_config.json \
  -v Production/Default \
  -o "@1 /XI -o firmware.hex"
```

See [Data Sources](sources.md#postgres--p---postgres) for config format.

### Build with HTTP backend

```bash
mint block@layout.toml \
  --http http_config.json \
  -v Production/Default \
  -o "@1 /XI -o firmware.hex"
```

See [Data Sources](sources.md#http---http) for config format.

### Build with JSON data source

```bash
mint block@layout.toml \
  --json data.json \
  -v Debug/Default \
  -o "@1 /XI -o firmware.hex"
```

See [Data Sources](sources.md#json---json) for format details.
