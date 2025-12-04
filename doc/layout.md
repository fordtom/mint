# Layout Files

Layout files define memory blocks and their data fields. Supported formats: TOML, YAML, JSON. The data in the layout file helps mint understand the structure of the data, and how you want to represent the data in the output. Each block represents a contiguous region of memory (typically a single struct stored in a known location in flash). For an example of a block, see [`doc/examples/blocks.h`](doc/examples/blocks.h) and compare it to the layout file [`doc/examples/block.toml`](doc/examples/block.toml).

## Structure

```toml
[settings]          # Global settings (optional)
# ...

[blockname.header]  # Block header (required per block)
# ...

[blockname.data]    # Block data fields (required per block)
# ...
```

---

## Settings

Global settings apply to all blocks. All fields are optional.

```toml
[settings]
endianness = "little"      # "little" (default) or "big"
virtual_offset = 0x0       # Offset added to all addresses
byte_swap = false          # Swap byte pairs across the block (used to emulate word-addressable memory)
pad_to_end = false         # Pad outputted block to full length

[settings.crc]
polynomial = 0x04C11DB7    # CRC polynomial
start = 0xFFFFFFFF         # Initial CRC value
xor_out = 0xFFFFFFFF       # XOR applied to final CRC
ref_in = true              # Reflect input bytes
ref_out = true             # Reflect output CRC
area = "data"              # CRC coverage: "data" (padded to 4 byte boundary) or "all" (padded to block boundary)
```

---

## Block Header

Each block requires a header section defining memory layout.

```toml
[blockname.header]
start_address = 0x8B000    # Start address in memory (required)
length = 0x1000            # Block size in bytes (required)
crc_location = "end"       # CRC placement: "end" to append to data struct as a u32 value, or absolute address (e.g. 0x8BFF0)
padding = 0xFF             # Padding byte value (default: 0xFF)
```

## Block Data

Data fields are key-value pairs where the key is a dotted path (matching C struct hierarchy) and the value defines the field.

### Field Attributes

| Attribute     | Description                                                                   |
| ------------- | ----------------------------------------------------------------------------- |
| `type`        | Data type (required)                                                          |
| `value`       | Literal value (mutually exclusive with `name`)                                |
| `name`        | Data source lookup key (mutually exclusive with `value`)                      |
| `size`/`SIZE` | Array size; `size` pads if data is shorter, `SIZE` errors if data is shorter. |
| `bitmap`      | Bitmap field definitions (see below)                                          |

---

## Field Examples

### Scalar Values

```toml
[block.data]
# Literal numeric
device.id = { value = 0x1234, type = "u32" }

# From data source
device.serial = { name = "SerialNumber", type = "u32" }

# Boolean (stored as integer)
config.enable = { value = true, type = "u8" }
```

### Strings

Strings use `u8` type with `size` for fixed-length fields.

```toml
[block.data]
# Literal string (padded to size)
message = { value = "Hello", type = "u8", size = 16 }

# From data source
device.name = { name = "DeviceName", type = "u8", size = 32 }
```

### Arrays

```toml
[block.data]
# 1D literal array
network.ip = { value = [192, 168, 1, 100], type = "u8", size = 4 }

# 1D from data source
calibration.coeffs = { name = "Coefficients1D", type = "f32", size = 8 }

# 2D array (e.g., 3x3 matrix)
calibration.matrix = { name = "CalibrationMatrix", type = "i16", size = [3, 3] }

# Strict size (error if data source has fewer elements)
strict.array = { name = "SomeArray", type = "f32", SIZE = 8 }
```

### Bitmaps

Pack multiple values into a single integer.

```toml
[block.data]
config.flags = { type = "u16", bitmap = [
    { bits = 1, name = "EnableDebug" },   # 1 bit from data source
    { bits = 3, name = "ModeSelect" },    # 3 bits from data source
    { bits = 1, value = true },           # 1 bit literal
    { bits = 4, name = "RegionCode" },    # 4 bits from data source
    { bits = 7, value = 0 },              # 7 bits padding
] }
```

Bitmap fields are packed LSB-first into the specified type. signedness of fields match the type. Negative values are represented as two's complement. The sum of the bits in the bitmap must match the type size.

---

## Multiple Blocks

A single layout file can define multiple blocks:

```toml
[settings]
endianness = "little"

[config.header]
start_address = 0x8000
length = 0x1000
crc_location = "end"

[config.data]
version = { value = 1, type = "u16" }

[calibration.header]
start_address = 0x9000
length = 0x1000
crc_location = "end"

[calibration.data]
coefficients = { name = "Coefficients", type = "f32", size = 16 }
```

Build specific blocks with `blockname@file.toml` syntax:

```bash
mint config@layout.toml --xlsx data.xlsx -v Default
```

---

## Format Examples

### TOML

```toml
[block.header]
start_address = 0x8000
length = 0x100

[block.data]
device.id = { value = 0x1234, type = "u32" }
device.name = { name = "DeviceName", type = "u8", size = 16 }
```

### YAML

```yaml
block:
  header:
    start_address: 0x8000
    length: 0x100
  data:
    device.id:
      value: 0x1234
      type: "u32"
    device.name:
      name: "DeviceName"
      type: "u8"
      size: 16
```

### JSON

```json
{
  "block": {
    "header": {
      "start_address": 32768,
      "length": 256
    },
    "data": {
      "device.id": { "value": 4660, "type": "u32" },
      "device.name": { "name": "DeviceName", "type": "u8", "size": 16 }
    }
  }
}
```
