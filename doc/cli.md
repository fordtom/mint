# Command Line Interface

mint builds flash blocks from layout files and data sources, emitting Intel HEX or Motorola S-Record files.

```
mint [OPTIONS] [BLOCK@FILE | FILE]...
```

## Positional Arguments

### `[BLOCK@FILE | FILE]...`

Specifies which blocks to build. Two formats are supported:

| Format              | Description                             |
| ------------------- | --------------------------------------- |
| `block@layout.toml` | Build specific block from layout file   |
| `layout.toml`       | Build all blocks defined in layout file |

**Examples:**

```bash
# Build single block
mint config@layout.toml --xlsx data.xlsx -v Default

# Build multiple specific blocks
mint config@layout.toml calibration@layout.toml --xlsx data.xlsx -v Default

# Build all blocks from a file
mint layout.toml --xlsx data.xlsx -v Default

# Mix both styles
mint header@layout.toml calibration.toml --xlsx data.xlsx -v Default
```

---

## Data Source Options

You can specify exactly one data source (`--xlsx`, `--postgres`, `--rest`, or `--json`) along with a variant (`-v`).

### `--xlsx <FILE>`

Path to Excel workbook containing variant data.

```bash
mint layout.toml --xlsx data.xlsx -v Default
```

### `--main-sheet <NAME>`

Override the default main sheet name (`Main`) for the excel data source.

```bash
mint layout.toml --xlsx data.xlsx --main-sheet Config -v Default
```

### `--postgres <PATH or JSON>`

Use PostgreSQL as the data source. Accepts a JSON file path or inline JSON string.

```bash
# Using a config file
mint layout.toml --postgres pg_config.json -v Default

# Using inline JSON
mint layout.toml --postgres '{"url":"...","query_template":"..."}' -v Default
```

See [Data Sources](sources.md#postgres--p---postgres) for config format details.

### `--rest <PATH or JSON>`

Use REST API as the data source. Accepts a JSON file path or inline JSON string.

```bash
# Using a config file
mint layout.toml --rest rest_config.json -v Default

# Using inline JSON
mint layout.toml --rest '{"url":"...","headers":{...}}' -v Default
```

See [Data Sources](sources.md#rest--r---rest) for config format details.

### `--json <PATH or JSON>`

Use raw JSON as the data source. Accepts a JSON file path or inline JSON string.

The JSON format is an object with variant names as top-level keys. Each variant contains an object with name:value pairs.

```bash
# Using a JSON file
mint layout.toml --json data.json -v Debug/Default

# Using inline JSON
mint layout.toml --json '{"Default":{"key1":123,"key2":"value"},"Debug":{"key1":456}}' -v Debug/Default
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
mint layout.toml --xlsx data.xlsx -v Default

# Fallback chain: try Debug first, then Default
mint layout.toml --xlsx data.xlsx -v Debug/Default

# Three-level fallback
mint layout.toml --xlsx data.xlsx -v Production/Debug/Default
```

---

## Output Options

### `-o, --out <DIR>`

Output directory for generated files. Created if it doesn't exist.

**Default:** `out`

```bash
mint layout.toml --xlsx data.xlsx -v Default -o build/hex
```

### `--prefix <STR>`

String prepended to output filenames.

**Default:** empty

```bash
# Produces: out/FW_config.hex
mint config@layout.toml --xlsx data.xlsx -v Default --prefix FW_
```

### `--suffix <STR>`

String appended to output filenames (before extension).

**Default:** empty

```bash
# Produces: out/config_v2.hex
mint config@layout.toml --xlsx data.xlsx -v Default --suffix _v2
```

### `--format <FORMAT>`

Output file format.

| Value | Description         | Extension |
| ----- | ------------------- | --------- |
| `hex` | Intel HEX (default) | `.hex`    |
| `mot` | Motorola S-Record   | `.mot`    |

```bash
# Intel HEX (default)
mint layout.toml --xlsx data.xlsx -v Default --format hex

# Motorola S-Record
mint layout.toml --xlsx data.xlsx -v Default --format mot
```

### `--record-width <N>`

Bytes per data record in output file. Range: 1-64.

**Default:** `32`

```bash
# 16 bytes per record (shorter lines)
mint layout.toml --xlsx data.xlsx -v Default --record-width 16

# 64 bytes per record (longer lines)
mint layout.toml --xlsx data.xlsx -v Default --record-width 64
```

### `--combined`

Emit a single output file containing all blocks instead of one file per block.

```bash
# Without --combined: out/config.hex, out/calibration.hex
mint config@layout.toml calibration@layout.toml --xlsx data.xlsx -v Default

# With --combined: out/combined.hex
mint config@layout.toml calibration@layout.toml --xlsx data.xlsx -v Default --combined
```

### `--pad-to-end`

Pad output to full block length. When enabled, the output file will contain padding bytes up to the block's defined length. Bear in mind that other features (such as CRC checking across the whole block) can/will already have padded the block to the full length, rendering this option useless.

```bash
# Output padded to full block length
mint layout.toml --xlsx data.xlsx -v Default --pad-to-end
```

---

## Build Options

### `--strict`

Enable strict type conversions. Errors on lossy casts instead of saturating/truncating.

```bash
mint layout.toml --xlsx data.xlsx -v Default --strict
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
mint layout.toml --xlsx data.xlsx -v Default --stats
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
mint layout.toml --xlsx data.xlsx -v Default --quiet
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
mint layout.toml --xlsx data.xlsx -v Default
```

### Production build with all options

```bash
mint \
  config@layout.toml \
  calibration@layout.toml \
  --xlsx data.xlsx \
  -v Production/Default \
  -o release/firmware \
  --prefix FW_ \
  --suffix _v1.2.3 \
  --format mot \
  --record-width 32 \
  --strict \
  --stats
```

### Build with Postgres backend

```bash
mint layout.toml \
  --postgres pg_config.json \
  -v Production/Default \
  --combined
```

See [Data Sources](sources.md#postgres--p---postgres) for config format.

### Build with REST backend

```bash
mint layout.toml \
  --rest rest_config.json \
  -v Production/Default \
  --combined
```

See [Data Sources](sources.md#rest--r---rest) for config format.

### Build with JSON data source

```bash
mint layout.toml \
  --json data.json \
  -v Debug/Default \
  --combined
```

See [Data Sources](sources.md#json---json) for format details.
