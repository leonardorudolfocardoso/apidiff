# CLAUDE.md

## Project Overview

**apidiff** is a Rust CLI tool for comparing OpenAPI 3.0.x specifications and detecting breaking changes.

## Build & Run

```bash
cargo build              # debug build
cargo build --release    # release build
cargo run -- <OLD> <NEW> # compare two OpenAPI specs
cargo run -- --help      # show usage
cargo test               # run all tests (34 total)
```

Exit codes: 0 = no breaking changes, 1 = breaking changes found, 2 = error.

## Project Structure

```
src/
  main.rs          # CLI parsing, output formatting, orchestration
  changes.rs       # Data model: Severity, Location, Change
  loader.rs        # OpenAPI spec loading (file I/O + parsing)
  loader/tests.rs  # Loader unit tests
  diff.rs          # Comparison logic (7-layer algorithm)
  diff/tests.rs    # Diff unit tests
tests/fixtures/    # Sample OpenAPI specs for testing
```

## Key Dependencies

- **clap** (4.5) — CLI argument parsing
- **openapiv3** (2.2) — OpenAPI 3.0.x type definitions
- **serde_json** / **serde_yml** — JSON/YAML parsing
- **indexmap** — Ordered maps (used by openapiv3)
- **tempfile** (dev) — Temp files for tests

## Architecture

### Loader (`loader.rs`)
- `load_file(path)` — reads file, detects format, parses OpenAPI spec
- `LoadError` — file-level errors (I/O or parse) with path context
- `ParseError` — content-level errors (JSON/YAML) without path

### Diff (`diff.rs`)
7-layer comparison algorithm:
1. Paths — endpoints added/removed
2. Operations — HTTP methods added/removed
3. Parameters — added/removed/required changes
4. Request bodies — added/removed/required changes
5. Responses — status codes added/removed
6. Content — media types with direction-aware schema comparison
7. Schemas — type changes, properties, enums, $ref resolution

Direction matters: request vs response schemas have opposite breaking rules.

### Changes (`changes.rs`)
- `Severity::Breaking` / `Severity::NonBreaking`
- `Location` — path + HTTP method
- `Change` — severity + location + message

## Code Conventions

- Clap derive pattern for CLI (struct-based)
- Standard Rust formatting (snake_case, CamelCase)
- Minimal style — no unnecessary abstractions
- Error handling: `?` propagation in `run()`, handle at `main()` boundary
- No panics — errors go to stderr with non-zero exit
- Tests in submodules (`mod tests;`)
