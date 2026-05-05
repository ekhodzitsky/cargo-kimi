# Changelog

All notable changes to `cargo-kimi` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added

- **Rich terminal output**: `cargo kimi check` now displays colorized tables, severity emojis, and per-file score badges using `comfy-table` and `colored`
- **`cargo kimi badge`**: Generate SVG score badges for READMEs (`kimi-score.svg`)
- **`.kimi.toml` configuration**: Project-level config for `strictness`, `fail-on-drop`, `ignore` paths, and default `format`
- **Auto-badge generation**: `cargo kimi check` automatically writes `kimi-score.svg` after each text run
- **Integration test** for badge generation

### Changed

- **CI**: Run tests with `RUST_TEST_THREADS=1` to avoid `cargo metadata` lock contention in integration tests
- **Dogfood workflow**: Auto-commit generated `kimi-score.svg` on every push

### Fixed

- **docs.rs build**: Added `src/lib.rs` to provide a library target, fixing `error: no library targets found` on docs.rs
- **Rustdoc warnings**: Wrapped generic return types in backticks to resolve unclosed HTML tag warnings

## [1.6.6] - 2026-05-05

### Changed

- **Repository**: `cargo-kimi` now lives in its own repository at `github.com/ekhodzitsky/cargo-kimi`
- **CI workflows**: Updated paths for standalone repository layout (no `cargo-kimi/` subdirectory)
- **Documentation**: All install URLs and GitHub Action references point to the new repository

## [1.6.5] - 2026-05-05

### Added

- **`cargo kimi watch`**: Continuous filesystem watcher that re-runs contract checks on every `.rs` save
- **`--format sarif`**: SARIF 2.1.0 output for GitHub Code Scanning integration
- **Unit test coverage** for `unsafe` block auto-fix (`fix_missing_safety_inserts_comment`)
- **Integration tests** for SARIF validity and `watch --help`

### Fixed

- **CI deprecation warnings**: Added `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24=true` to all GitHub Actions workflows

## [1.6.4] - 2026-05-05

### Added

- **`cargo kimi watch`**: Continuous file-system watching mode that re-runs contract checks on every `.rs` save
- **`--format sarif`**: SARIF output for native GitHub Code Scanning integration
- **Auto-fix `unsafe` blocks**: `cargo kimi fix` now inserts `// SAFETY: TODO` stubs before unannotated `unsafe` blocks

### Fixed

- **CI release workflow**: Removed `--locked` from cross-compilation build steps (stale lockfile failures)
- **CI publish dry-run**: Added `--allow-dirty` so `Cargo.lock` refresh does not block the dry-run
- **CI**: `dtolnay/rust-action` → `dtolnay/rust-toolchain@stable`
- **CI**: Committed `Cargo.lock` and removed it from `.gitignore` for reproducible binary builds
- **CI**: Corrected release artifact paths (`cargo-kimi/target/` → `target/` after `working-directory` fix)

## [1.6.0] - 2026-05-05

### Added

- **GitHub Action**: Reusable `cargo-kimi` GitHub Action for CI pipelines
- **Kimi skills integration**: Built-in skill definitions for `kimi-check` and `kimi-fix`
- **Score exemptions**: Allow per-file or per-rule score exemptions via configuration
- **Property test overflow fix**: Corrected arithmetic overflow edge cases in generated property tests
- **Smarter unwrap→? conversion**: Improved heuristics for suggesting `?` over `unwrap()`
- **Security fixes**:
  - Path traversal validation hardened
  - TOCTOU race conditions mitigated in file operations
  - Graceful handling of non-UTF8 paths and file contents
- **Lazy-static regexes**: Compiled regular expressions now use `std::sync::LazyLock` (requires Rust 1.80+)
- **`IssueCategory` enum**: Structured categorization of all reported issues

## [1.5.0] - 2026-05-04

### Added

- **`cargo kimi fix`**: Automated fixing of common contract violations
- **`cargo kimi trend`**: Track score trends across commits
- **Per-file scoring**: Individual contract scores for each source file
- **`--format json`**: Machine-readable JSON output for integrations
- **MCP server**: Model Context Protocol server for IDE integration
