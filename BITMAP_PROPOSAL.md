# Bitmap Support Proposal

## Requirements
- User must specify storage type (u8, u16, u32, u64)
- Only single-bit values (0/1, true/false) supported initially
- Should work with both Excel data and literal values

## Proposed Methods

### Method 1: Explicit Bit Map (Recommended)
Define each bit position explicitly with boolean values.

**Layout syntax:**
```toml
flags = { type = "u32", bitmap = { 0 = true, 5 = true, 7 = false } }
```

**Excel syntax:**
- Excel cell contains comma-separated bit positions: `"0,5,7"` (sets bits 0, 5, 7 to 1)
- Or use a sheet reference `#FlagBits` with columns: `Bit`, `Value` (0/1 or true/false)

**Pros:**
- Explicit and clear
- Can set bits to 0 explicitly
- Works well for small to medium bitmaps

**Cons:**
- Verbose for large bitmaps
- Excel format needs parsing

---

### Method 2: Array of Bit Positions (Set Bits Only)
Specify only which bits should be set to 1; all others default to 0.

**Layout syntax:**
```toml
flags = { type = "u32", bitmap = [0, 5, 7] }
```

**Excel syntax:**
- Excel cell contains comma-separated bit positions: `"0,5,7"`
- Or use sheet reference `#FlagBits` with single column of bit positions

**Pros:**
- Concise for sparse bitmaps
- Simple Excel format

**Cons:**
- Cannot explicitly set bits to 0 (only 1 or default 0)
- Less flexible than Method 1

---

### Method 3: Excel Sheet with Bit/Value Pairs
Use Excel sheet with explicit bit positions and values.

**Layout syntax:**
```toml
flags = { name = "FlagBits", type = "u32", bitmap = true }
```

**Excel sheet structure (`#FlagBits`):**
```
Bit | Value
----|------
0   | 1
5   | 1
7   | 0
```

**Pros:**
- Good for complex bitmaps managed in Excel
- Clear separation of bit positions and values
- Easy to maintain in Excel

**Cons:**
- Requires Excel file
- More complex parsing

---

### Method 4: Hybrid Approach (Most Flexible)
Support multiple formats based on context:
- Literal map: `bitmap = { 0 = true, 5 = true }`
- Array of positions: `bitmap = [0, 5, 7]` (sets to 1)
- Excel single value: `name = "FlagBits"` with `bitmap = true` and cell value `"0,5,7"`
- Excel sheet: `name = "#FlagBits"` with columns `Bit`, `Value`

**Implementation:**
- Add `bitmap` field to `LeafEntry` (optional)
- When present, `type` becomes the storage type
- Parse bitmap data based on source (literal vs Excel)

---

## Recommended Approach: Method 4 (Hybrid)

This provides maximum flexibility:
1. **Literal map** for explicit control: `bitmap = { 0 = true, 5 = false }`
2. **Literal array** for simple cases: `bitmap = [0, 5, 7]`
3. **Excel single value** for simple Excel-driven bitmaps: cell `"0,5,7"`
4. **Excel sheet** for complex bitmaps: sheet with `Bit` and `Value` columns

## Implementation Notes

- Add `bitmap: Option<BitmapSource>` to `LeafEntry`
- `BitmapSource` enum: `Map(HashMap<u32, bool>)`, `Array(Vec<u32>)`
- When `bitmap` is present, validate:
  - `type` must be unsigned integer (u8, u16, u32, u64)
  - Bit positions must fit within storage type size
  - No `size` field allowed (bitmaps are always single values)
- Excel parsing:
  - Single value: parse comma-separated integers
  - Sheet reference: parse `Bit`/`Value` columns
