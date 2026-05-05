# Changelog

All notable changes to `cargo-kimi` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

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
