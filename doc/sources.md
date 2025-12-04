# Data Sources

mint supports three data source types: Excel workbooks, Postgres databases, and REST APIs.

## Excel (`-x, --xlsx`)

```bash
mint layout.toml -x data.xlsx -v Default
```

### Main Sheet Structure

The first sheet (or one specified via `--main-sheet`) contains variant data:

| Name              | Default            | Debug | VarA |
| ----------------- | ------------------ | ----- | ---- |
| DeviceName        | MyDevice           |       |      |
| FWVersionMajor    | 3                  |       | 4    |
| Coefficients1D    | #Coefficients1D    |       |      |
| CalibrationMatrix | #CalibrationMatrix |       |      |

- **Name column**: lookup key used by layout files
- **Variant columns**: values for each variant (e.g., Default, Debug, VarA)
- **Precedence**: follows `-v` order; first non-empty wins, falls back to Default
- **Sheet references**: cells starting with `#` reference array sheets (e.g., `#Coefficients1D`)

### Array Sheets

For 1D/2D arrays, reference a sheet by name with `#` prefix:

| C1  | C2  | C3  |
| --- | --- | --- |
| 1   | 2   | 3   |
| 4   | 5   | 6   |
| 7   | 8   | 9   |

- First row defines headers (and width for 2D arrays)
- Values read row-by-row until an empty cell is encountered
- Strings and undersized arrays are padded by default; use `SIZE` (uppercase) in layout to enforce strict length

---

## Postgres (`-p, --postgres`)

```bash
mint layout.toml -p config.json -v Debug/Default
# or inline:
mint layout.toml -p '{"url":"...","query_template":"..."}' -v Debug/Default
```

### Config Format

```json
{
  "url": "postgres://user:pass@host/db",
  "query_template": "SELECT json_object_agg(name, value)::text FROM config WHERE variant = $1"
}
```

### Query Requirements

- Executed once per variant (passed as `$1`)
- Must return a single row with column 0 containing a JSON object mapping names to values
- Native JSON arrays are supported for 1D/2D arrays
- Space/comma/semicolon-delimited strings are also parsed as numeric arrays

---

## REST (`-r, --rest`)

```bash
mint layout.toml -r config.json -v Debug/Default
# or inline:
mint layout.toml -r '{"url":"...","headers":{...}}' -v Debug/Default
```

### Config Format

```json
{
  "url": "https://api.example.com/config?variant=$1",
  "headers": {
    "Authorization": "Bearer token123"
  }
}
```

- **url**: HTTP endpoint URL template using `$1` as placeholder for the variant string (URL-encoded)
- **headers**: Optional HTTP headers map

### Response Requirements

- Must return a JSON object mapping names to values
- Native JSON arrays are supported for 1D/2D arrays
- Space/comma/semicolon-delimited strings are also parsed as numeric arrays
- Request is made once per variant with `$1` replaced by the URL-encoded variant string
