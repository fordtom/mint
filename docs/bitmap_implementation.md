# Bitmap Leaf Implementation Plan (Updated)

## Syntax
- Leaf stores bitmap via `bitmap = [ ... ]` alongside `type = "u8|u16|u32|u64|i8|i16|i32|i64"`.
- Example:
  ```
  flags = { type = "u16", bitmap = [
      { size = 1, name = "AllowDebug" },
      { size = 2, name = "ModeSelect" },
      { size = 1, value = true },
      { size = 4, name = "Region" },
      { size = 8, value = 0 },
  ] }
  ```
- `size` = number of bits (integer >0). No per-field `type`.
- Signed handling:
  - Fields default to unsigned when storage is unsigned.
  - When storage scalar is signed (`i8/i16/i32/i64`), fields default to signed interpretation (two's complement) unless `signed = false`.
  - Optional `signed = true/false` overrides inheritance for individual fields.
- Field values use same mutually exclusive sources as standard leaves: `name = "ExcelRow"` or `value = <literal>`.
- Order defines bit packing (LSB-first). Remaining bits auto-zero-padded unless strict mode demands exact coverage.

## Parsing (`src/layout/entry.rs`)
- Extend `LeafEntry` with `bitmap: Option<Vec<BitmapField>>`; keep `#[serde(deny_unknown_fields)]`.
- `BitmapField`:
  - `size_bits: u8` (`#[serde(rename = "size")]`).
  - `signed: Option<bool>`.
  - `source: EntrySource` via `#[serde(flatten)]`.
- Validation:
  - Exactly one of `{name,value,bitmap}`.
  - Reject `size`/`SIZE` on bitmap leaves for now.
  - Storage `scalar_type` must be integer (no floats). Allow signed+unsigned.
  - Each field size ≤ storage bits; sum ≤ storage bits (error if overflow).

## Boolean + Literal Support (`src/layout/value.rs`, `src/layout/conversions.rs`)
- Introduce `DataValue::Bool(bool)` to represent serde booleans (`true/false`) from layout or Excel.
- Map string literals `"true"/"false"` (case-insensitive) to bool during bitmap field conversion.
- Helper `fn extract_bitmap_value(value: &DataValue, width: u8, signed: bool, strict: bool) -> Result<i128, LayoutError>`:
  - Accept `Bool`, ints, floats (integer-valued), and strings `0/1/true/false`.
  - Clamp to range: unsigned fields `[0, 2^width-1]`, signed fields `[ -2^(width-1), 2^(width-1)-1 ]`.
  - Returns `i128`; caller reinterprets as unsigned when needed before packing.

## Excel Retrieval (`src/variant/mod.rs`)
- `retrieve_single_value`:
  - Accept `Data::Bool` → `DataValue::Bool`.
  - Accept strings `true/false` → `Bool`.
  - Keep numeric behavior as-is.
- `retrieve_1d_array_or_string` / `retrieve_2d_array`:
  - When reading cells, convert bool cells or bool-like strings into `DataValue::Bool`.
  - Numeric conversions unchanged.

## Bitmap Emission
- New `LeafEntry::emit_bitmap`.
- Steps:
  1. Ensure no array sizing keys set.
  2. Determine storage bit width and confirm scalar integer type; capture if storage signed for default inheritance.
  3. Iterate bitmap fields:
     - Resolve `DataValue`: `EntrySource::Name` uses `data_sheet.retrieve_single_value`, `EntrySource::Value` requires single value.
     - Determine signedness: `field.signed.unwrap_or(storage_is_signed)`.
     - Convert via `extract_bitmap_value`.
     - For signed fields, convert to two's complement representation within `width` bits before packing.
     - Accumulate into `u128` buffer at current bit offset (LSB-first).
  4. After loop: if total bits < storage bits → pad zeros unless `config.strict`, where mismatch triggers error.
  5. Emit bytes by narrowing accumulator to requested scalar type and using existing endian conversion.

## Strictness & Padding
- Default: zero-fill remaining high bits.
- `config.strict` (from CLI `--strict`) → require cumulative field size = storage bits.
- Future: optional `BITMAP` uppercase key for per-layout strict enforcement (not scoped now).

## Error Cases
- Missing data sheet for `name` field.
- Arrays inside bitmap field source.
- Field value out of range for declared width & signedness.
- Storage type float, or bitmap plus `size/SIZE`.

## Testing
- Add tests covering:
  - Packing order, zero padding, strict failure.
  - Signed fields (negative values) with signed storage and overrides.
  - Bool literal + Excel bool retrieval.
  - Overflow detection for widths.
  - Error on using bitmap with float storage or arrays.
