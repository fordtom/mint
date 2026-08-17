---
name: mint
description: "Guide for working with mint, an embedded development tool that assembles flash memory hex files from TOML layout files and data sources (Excel/JSON). Use this skill whenever a project uses or mentions mint / mint-cli, when you encounter .toml layout files that define memory blocks for firmware or flash, when you need to create or modify flash block definitions, set up mint in a build system or CI pipeline, or work with Excel/JSON data sources for embedded device configuration. Also trigger when you see references to building Intel HEX or Motorola S-Record files from structured layout definitions, or when a user mentions replacing a custom hex-generation script with a declarative tool."
---

# mint

mint builds binary flash images (Intel HEX or Motorola S-Record) from a declarative TOML layout file and an optional Excel or JSON data source. Use it when a project needs flash configuration blocks, matching C headers, or ABI fingerprints from a layout instead of a hand-written hex generator.

Install the CLI with `cargo install mint-cli` or the repository nix flake, then run `mint skill` to load the current documentation before you write or change a layout.
