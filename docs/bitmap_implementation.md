# Bitmap Leaf (Bitmap = [...]) Implementation Plan

## Syntax
- Leaf shape (single scalar storage): `flags = { type = "u16", bitmap = [ { type = "u1", name = "AllowDebug" }, { type = "u2", value = 3 }, ... ] }`.
- `bitmap` list order defines packing order (LSB-first, matching existing little-endian scalar serialization).
- Each bitmap item mirrors current leaf sources: it may have `name = "ExcelRow"` **or** `value = 7` (mutually exclusive). No nested arrays.
- `type` on the bitmap entry uses the existing scalar storage enum. Allowed: `u8|u16|u32|u64`. Reject signed/float.
- Field widths: parsed from `type = "u1" .. "u64"` (alias `width_bits`). Runtime validates `0 < width ≤ storage_bits`.
- Padding: if cumulative width < storage bits, low/high-order remainder is zero-filled. If `config.strict` (from CLI `--strict`) is enabled, emit error instead of padding.

## Parsing Changes (`src/layout/entry.rs`)
- Extend `LeafEntry` with `bitmap: Option<Vec<BitmapField>>`. Keep `#[serde(deny_unknown_fields)]`.
- Ensure exactly one of `{name,value,bitmap}` is set; emit `LayoutError` otherwise.
- `BitmapField` struct:
  - `bit_type: BitWidth` (deserialized from strings like `"u3"`).
  - `source: EntrySource` reused via `#[serde(flatten)]` to keep `name`/`value` semantics.
- Add helper `ScalarType::is_unsigned()` and call when `bitmap.is_some()`.
- `LeafEntry::emit_bytes` branch order:
  1. If `bitmap` present → `emit_bitmap`.
  2. Else existing scalar/array logic.
- Disallow `size`/`SIZE` for bitmaps (error) until array semantics are defined.

## DataValue + Conversion (`src/layout/value.rs`, `src/layout/conversions.rs`)
- Add `DataValue::Bool(bool)` to support literal `true/false` in layout.
- Update `convert_value_to_bytes` to reject bool unless scalar type is `u1`? Instead, leave existing path unchanged and handle bool in bitmap helper.
- Provide helper `fn as_bitmap_int(value: &DataValue, width_bits: u32, strict: bool) -> Result<u128, LayoutError>`:
  - Accept `Bool`, numeric (`U64/I64/F64`) via existing `TryFrom{Strict}` with bounds check `< 2^width`.
  - Accept `Str` when matches `(?i)true|false|0|1`; map accordingly; else error.
- Reuse this helper when consuming inline literal fields.

## Excel Retrieval (`src/variant/mod.rs`)
- `retrieve_single_value` currently rejects non-numeric. Extend:
  - Accept `Data::Bool(b)` → `DataValue::Bool(*b)`.
  - Accept `Data::String` when string equals `true` or `false` (case-insensitive) → `DataValue::Bool`.
  - Keep existing numeric behavior; other string forms remain errors.
- `retrieve_1d_array_or_string` & `retrieve_2d_array` should convert sheet cells into `DataValue::Bool` when `Data::Bool` or `true/false` strings appear. (Enables future bitmap arrays and general bool support.)

## Bitmap Emission (`src/layout/entry.rs`)
- New method `fn emit_bitmap(&self, data_sheet: Option<&DataSheet>, config: &BuildConfig) -> Result<Vec<u8>, LayoutError>`.
- Steps:
  1. Assert `bitmap` exists and `size_keys` unset.
  2. Determine storage bit width via `self.scalar_type.size_bytes() * 8`.
  3. Iterate bitmap fields accumulating `(bit_offset, width)`; ensure total ≤ storage bits.
  4. For each field:
     - Resolve `DataValue`:
       - `EntrySource::Name` → call `data_sheet.retrieve_single_value`.
       - `EntrySource::Value(ValueSource::Single(_))` → reuse existing `ValueSource`.
       - Reject arrays (mirrors scalar behavior).
     - Convert to integer with `as_bitmap_int`.
     - Mask to width, then shift into `u128 accumulator` at current offset (LSB-first).
  5. If strict mode and total bits != storage bits, error; else zero-fill remaining bits.
  6. Convert accumulator to requested scalar bytes via helper using `ScalarType` (u8/u16/u32/u64) and existing endian conversion.

## Strictness / Padding
- Default: unused high bits zero-padded.
- `--strict` (existing global) → require exact fill; throw `LayoutError::DataValueExportFailed("bitmap fields do not cover storage width")`.
- Future extension: allow `BITMAP = [...]` uppercase analog if per-field strictness needed (not in initial implementation).

## Boolean Acceptance Rules
- Layout literals: `true/false` (serde bool) stored as `DataValue::Bool`.
- Layout strings: `"true"`/`"FALSE"` etc convert to bool only when consumed as bitmap field (using lowercase comparison).
- Excel cells:
  - `TRUE/FALSE` typed as bool → direct.
  - String `"TRUE"` etc → bool.
  - Numeric `0/1` → allowed via integer path.

## Testing Outline
- Unit tests in `tests/`:
  - Ensure bitmap packing order & zero padding.
  - Strict mode error when underspecified.
  - Reject signed storage types.
  - Accept bools from layout literal and Excel.
  - Overflow detection when value ≥ 2^width.
- Add example snippet in `examples/block.toml` demonstrating `bitmap`.
