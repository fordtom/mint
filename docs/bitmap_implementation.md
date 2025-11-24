# Bitmap Leaf Implementation Plan (v3)

## Syntax
- Any scalar leaf may declare `bitmap = [ ... ]` instead of `name`/`value`.
- Storage `type` must be integer (`u8/u16/u32/u64/i8/i16/i32/i64`). Storage signedness drives interpretation of every bitmap field (all fields inherit the storage’s signed/two’s-complement behavior; no per-field override).
- Example:
  ```
  flags = { type = "i16", bitmap = [
      { bits = 1, name = "AllowDebug" },
      { bits = 2, name = "ModeSelect" },
      { bits = 1, value = true },
      { bits = 4, name = "RegionCode" },
      { bits = 8, value = 0 }, # explicit padding
  ] }
  ```
- `bits` is a positive integer count per element. The sum of all `bits` **must equal** the storage width (bytes * 8). Users must add padding entries themselves (no implicit zero-fill).
- Each entry is mutually exclusive between `name = "ExcelRow"` and `value = <literal>`, mirroring the existing leaf sources. Arrays are invalid.

## Parsing (`src/layout/entry.rs`)
- `LeafEntry` gains `bitmap: Option<Vec<BitmapField>>`; retains `#[serde(deny_unknown_fields)]`.
- `BitmapField` struct:
  - `bits: usize` (serde key `bits`).
  - `source: EntrySource` via `#[serde(flatten)]` (so `name` / `value` reused).
- Validation rules:
  - Exactly one of `{name,value,bitmap}` per leaf.
  - `size`/`SIZE` keys forbidden when `bitmap` is present.
  - `bits > 0`.
  - Sum of `bits` must equal `self.scalar_type.size_bytes() * 8`; otherwise emit `LayoutError` (independent of strict flag).
  - Storage type must not be floating-point.

## DataValue + Conversion Enhancements
- Extend `DataValue` with `Bool(bool)` to capture serde booleans.
- When a bitmap field uses literal strings `"true"`/`"false"` (case-insensitive) or Excel bool cells, convert to `DataValue::Bool`.
- Introduce helper in `layout` module:  
  `fn clamp_bitfield_value(value: &DataValue, bits: usize, signed: bool, strict: bool) -> Result<i128, LayoutError>`
  - Accept bools (`true` → 1, `false` → 0).
  - Accept integers/floats (floats must be whole numbers, reusing `TryFromStrict` when strict).
  - Accept strings `"0"` / `"1"` / `"true"` / `"false"` (case-insensitive).
  - Range:
    - Unsigned: `0 ..= 2^bits - 1`.
    - Signed: `-(2^(bits-1)) ..= 2^(bits-1) - 1` (two's complement).
  - If outside range:
    - `strict == true` ⇒ error.
    - `strict == false` ⇒ saturate to nearest representable value.
  - Returns `i128` so the caller can mask/pack easily.

## Excel Retrieval (`src/variant/mod.rs`)
- `retrieve_single_value`:
  - Accept `Data::Bool` → `DataValue::Bool`.
  - Accept string `true/false` as bool (case-insensitive).
- `retrieve_1d_array_or_string` / `retrieve_2d_array` conversions should do the same for bool cells, ensuring bitmaps sourced from sheets behave consistently.

## Bitmap Emission Flow
- `LeafEntry::emit_bytes` order:
  1. If `bitmap.is_some()` → call `emit_bitmap`.
  2. Else follow existing scalar / array logic.
- `emit_bitmap` steps:
  1. Assert `size_keys` empty and `bitmap` present.
  2. Determine storage width in bits and signedness.
  3. Ensure total bits already validated (panic if not).
  4. Iterate bitmap fields in order, tracking current bit offset (LSB-first).
     - Resolve `DataValue` via existing source mechanisms (`name` uses `data_sheet.retrieve_single_value`, `value` expects `ValueSource::Single`).
     - Call `clamp_bitfield_value` with `(bits, signed, config.strict)`.
     - Convert signed result into two's-complement bit pattern limited to `bits` and insert into `u128` accumulator (shift by current offset, mask).
  5. After packing all fields, emit bytes by casting accumulator to the requested scalar type and running existing endian serialization utilities.

## Strict Mode Interpretation
- `config.strict` now only affects conversion tolerance inside `clamp_bitfield_value` (range overflow and float/integer exactness). Length enforcement is unconditional (total bits must match storage bits regardless of strict flag).

## Error Cases to Cover
- Using bitmap with float storage.
- Missing `name`/`value` inside a bitmap field.
- `bits = 0`.
- Sum of bits != storage width.
- Arrays as bitmap sources.
- Value outside representable range under strict mode.

## Testing
- Add tests verifying:
  - Happy path packing for unsigned + signed storage.
  - Saturation vs error based on strict flag.
  - Bool literals and Excel bools map to correct bit patterns.
  - Detection of mis-sized bitmaps and zero-bit fields.
  - Example snippet in `examples/block.toml` for documentation.
