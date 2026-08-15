# mint

mint builds static binary flash blocks from TOML layout files and Excel or JSON data sources. It also generates matching C headers and ABI fingerprints from those layouts.

[Upgrading from v1](doc/migration-v2.md)

![img](https://raw.githubusercontent.com/tomrford/mint/main/doc/img.png)

## Install

```bash
cargo add mint-core
cargo install mint-cli
```

From a checkout, use `cargo install --path crates/mint-cli`.

## Documentation

- [CLI reference](https://github.com/tomrford/mint/blob/main/doc/cli.md)
- [Layout files](https://github.com/tomrford/mint/blob/main/doc/layout.md)
- [Data sources](https://github.com/tomrford/mint/blob/main/doc/sources.md)
- [Example layouts & data](https://github.com/tomrford/mint/tree/main/doc/examples)

## Quick start

```bash
mint build block.toml --xlsx data.xlsx --variants Default --stats
mint build layout.toml -j data.json --variants Debug/Default
mint header layout.toml -o layout.h
mint fingerprint layout.toml
mint abi list
```

See [`doc/examples/block.toml`](https://github.com/tomrford/mint/blob/main/doc/examples/block.toml) and its [generated header](https://github.com/tomrford/mint/blob/main/doc/examples/blocks.h) for a complete example.
