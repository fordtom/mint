# Command Line Interface

mint builds flash blocks from layout files and data sources, emitting Intel HEX or Motorola S-Record files.

```
mint [OPTIONS] [BLOCK@FILE | FILE]...
```

## Positional Arguments

### `[BLOCK@FILE | FILE]...`

Specifies which blocks to build. Two formats are supported:

| Format | Description |
|--------|-------------|
| `block@layout.toml` | Build specific block from layout file |
| `layout.toml` | Build all blocks defined in layout file |

**Examples:**

```bash
# Build single block
mint config@layout.toml -x data.xlsx -v Default

# Build multiple specific blocks
mint config@layout.toml calibration@layout.toml -x data.xlsx -v Default

# Build all blocks from a file
mint layout.toml -x data.xlsx -v Default

# Mix both styles
mint header@layout.toml calibration.toml -x data.xlsx -v Default
```

---

## Data Source Options

You must specify exactly one data source (`-x` or `-p`) along with a variant (`-v`).

### `-x, --xlsx <FILE>`

Path to Excel workbook containing variant data.

```bash
mint layout.toml -x data.xlsx -v Default
```

The workbook should have:
- A main sheet with columns: `Name`, `Default`, and optional variant columns
- Optional array sheets referenced with `#SheetName` syntax

### `--main-sheet <NAME>`

Override the default main sheet name (first sheet) in the Excel workbook.

```bash
mint layout.toml -x data.xlsx --main-sheet Config -v Default
```

### `-p, --postgres <PATH or JSON>`

Use PostgreSQL as the data source. Accepts a JSON file path or inline JSON string.

```bash
# Using a config file
mint layout.toml -p pg_config.json -v Default

# Using inline JSON
mint layout.toml -p '{"database":{"url":"postgres://localhost/db"},"query":{"template":"SELECT json_object_agg(name,value)::text FROM config WHERE variant=$1"}}' -v Default
```

### `-v, --variant <NAME[/NAME...]>`

Variant columns to query, in priority order. The first non-empty value found wins.

```bash
# Single variant
mint layout.toml -x data.xlsx -v Default

# Fallback chain: try Debug first, then Default
mint layout.toml -x data.xlsx -v Debug/Default

# Three-level fallback
mint layout.toml -x data.xlsx -v Production/Debug/Default
```

---

## Output Options

### `-o, --out <DIR>`

Output directory for generated files. Created if it doesn't exist.

**Default:** `out`

```bash
mint layout.toml -x data.xlsx -v Default -o build/hex
```

### `--prefix <STR>`

String prepended to output filenames.

**Default:** empty

```bash
# Produces: out/FW_config.hex
mint config@layout.toml -x data.xlsx -v Default --prefix FW_
```

### `--suffix <STR>`

String appended to output filenames (before extension).

**Default:** empty

```bash
# Produces: out/config_v2.hex
mint config@layout.toml -x data.xlsx -v Default --suffix _v2
```

### `--format <FORMAT>`

Output file format.

| Value | Description | Extension |
|-------|-------------|-----------|
| `hex` | Intel HEX (default) | `.hex` |
| `mot` | Motorola S-Record | `.mot` |

```bash
# Intel HEX (default)
mint layout.toml -x data.xlsx -v Default --format hex

# Motorola S-Record
mint layout.toml -x data.xlsx -v Default --format mot
```

### `--record-width <N>`

Bytes per data record in output file. Range: 1-64.

**Default:** `32`

```bash
# 16 bytes per record (shorter lines)
mint layout.toml -x data.xlsx -v Default --record-width 16

# 64 bytes per record (longer lines)
mint layout.toml -x data.xlsx -v Default --record-width 64
```

**Effect on output:**

```
# --record-width 16
:10000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00
:10001000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00

# --record-width 32
:20000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00
```

### `--combined`

Emit a single output file containing all blocks instead of one file per block.

```bash
# Without --combined: out/config.hex, out/calibration.hex
mint config@layout.toml calibration@layout.toml -x data.xlsx -v Default

# With --combined: out/combined.hex
mint config@layout.toml calibration@layout.toml -x data.xlsx -v Default --combined
```

---

## Build Options

### `--strict`

Enable strict type conversions. Errors on lossy casts instead of saturating/truncating.

```bash
mint layout.toml -x data.xlsx -v Default --strict
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
mint layout.toml -x data.xlsx -v Default --stats
```

**Example output:**

```
┌────────────┬───────────┬───────────┬──────────┬────────────┐
│ Block      │ Address   │ Allocated │ Used     │ CRC        │
├────────────┼───────────┼───────────┼──────────┼────────────┤
│ config     │ 0x08B000  │ 4096      │ 312      │ 0xA1B2C3D4 │
│ calibration│ 0x08C000  │ 4096      │ 1024     │ 0xE5F6A7B8 │
├────────────┼───────────┼───────────┼──────────┼────────────┤
│ Total      │           │ 8192      │ 1336     │            │
└────────────┴───────────┴───────────┴──────────┴────────────┘
Space efficiency: 16.3%
```

### `--quiet`

Suppress all output except errors.

```bash
mint layout.toml -x data.xlsx -v Default --quiet
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
mint layout.toml -x data.xlsx -v Default
```

### Production build with all options

```bash
mint \
  config@layout.toml \
  calibration@layout.toml \
  -x data.xlsx \
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
  -p '{"database":{"url":"postgres://user:pass@localhost/config"},"query":{"template":"SELECT json_object_agg(name,value)::text FROM settings WHERE variant=$1"}}' \
  -v Production/Default \
  --combined
```
