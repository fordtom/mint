# Migrating to v2

Mint v2 makes layout and command syntax explicit and treats the emitted binary as an ABI artefact. Update automation and regenerate every output before deploying v2.

## Required changes

- Replace `[mint].endianness = "little"` with `abi = "generic-le"`, or `"big"` with `"generic-be"`. Use a target profile such as `arm-aapcs32-le` when its layout rules apply. `mint abi list` and `mint abi show ABI` report the supported rules.
- Replace `--versions` with `--variants`. The `-v` short form remains available, and slash-separated fallback order is unchanged.
- Add the required `build` subcommand to build invocations.
- Convert YAML layouts to TOML. Layout files must have a `.toml` extension; JSON remains supported as a data source, not as a layout format.

Before (v1):

```toml
[mint]
endianness = "little"
[config.header]
start_address = 0x8000
length = 0x100
[config.data]
version = { name = "Version", type = "u16" }
```

```bash
mint config.toml --xlsx data.xlsx --versions Default
```

After (v2):

```toml
[mint]
abi = "generic-le"
[config.header]
start_address = 0x8000
length = 0x100
[config.data]
version = { name = "Version", type = "u16" }
```

```bash
mint build config.toml --xlsx data.xlsx --variants Default
```

## Binary and fingerprint compatibility

V2 aligns each nested aggregate to its strictest child and adds tail padding to nested and root aggregates. This can change field offsets, refs, checksum inputs, reserved sizes, and emitted bytes. Select the correct ABI, regenerate all binaries and headers, compile the generated header with the target compiler, and validate the result before flashing.

Current v2 uses the `mint block ABI fingerprint v2` hash domain and intentionally invalidates values made by the earlier v2 pre-release fingerprint v1 schema. Regenerate stored values and firmware constants together. A fingerprint field uses `type = "u64"` with `fingerprint = true` for its own block or a block name for another block in the same file; `mint fingerprint` prints values without a data source.

## New layout tools and address fields

`mint header` generates C11 typedefs, dimension and bitmap macros, fingerprint constants, and static assertions for offsets and structure sizes. Named ABI profiles now define byte order, scalar storage, alignment, array stride, and target address units; the output container remains a separate choice.

A scalar `ref` accepts a same-block field path or an unsigned absolute address and stores it in `u16`, `u32`, or `u64` using the selected ABI's address units. A reflist uses an array of path and address targets plus one-dimensional `size` or `SIZE`; lowercase `size` zero-fills unused slots, while uppercase `SIZE` requires an exact count.

V2 provides no compatibility aliases for removed forms. Invocations without `build`, `--versions`, `[mint].endianness`, YAML layouts, unknown configuration keys, and invalid or ambiguous field-source combinations are rejected instead of being accepted or inferred.
