## Project Overview

mint is an embedded development tool that works with layout files (toml/yaml/json) and excel sheets to assemble, diff, export, sign (and more) static hex files for flashing to microcontrollers.

## Architecture & Codebase

### Core Concepts

- **Layouts**: TOML/YAML/JSON files defining memory blocks (`src/layout`).
- **DataSheet**: Excel workbook (`.xlsx`) serving as the data source (`src/variant`).
  - Uses `Name` column for lookups.
  - Supports variants via columns (e.g., `Debug`, `VarA`).
  - Arrays are referenced by sheet name (prefixed with `#`).
- **Output**: Generates hex files, handling block overlaps and CRC calculations (`src/output`).

### Build Flow

1. **Parse Args**: `clap` defines arguments in `src/args.rs`.
2. **Resolve Blocks**: Parallel loading of layout files (`rayon`).
3. **Build Bytestreams**: Each block is built by combining layout config with Excel data.
4. **Output**: Hex files are generated (either per-block or combined).

### Key Directories

- `src/commands/`: Command implementations (e.g., `build`).
- `src/layout/`: Layout parsing and block configuration.
- `src/variant/`: Excel interaction and value retrieval.
- `src/output/`: Hex generation and data ranges.

## Development Environment

- **Nix**: Use `nix develop` for the environment.
- **Commands**:
  - Build: `cargo build`
  - Test: `cargo test` (Always run after changes)
  - Format: `cargo fmt` (Run before submitting)
  - Clippy: `cargo clippy` (Run before submitting)

## Working Guidelines

- **Minimal Changes**: Do only what is asked.
- **Planning**: Allocate time to plan superior solutions.
- **Clarification**: Ask if goals are unclear.
- **Comments**: No "history" comments (e.g., "changed x to y"). Document current state only.
- **Compatibility**: Do not maintain backwards compatibility unless required. Focus on better functionality.
