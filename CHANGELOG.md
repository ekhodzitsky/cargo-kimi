# Changelog

All notable changes to `cargo-kimi` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [1.6.3] - 2026-05-05

### Fixed

- **CI**: Actually committed `Cargo.lock` (removed from root `.gitignore`)

## [1.6.2] - 2026-05-05

### Fixed

- **CI**: Committed `Cargo.lock` so `--locked` builds work in release workflow

## [1.6.1] - 2026-05-05

### Fixed

- **CI**: Corrected `dtolnay/rust-action` → `dtolnay/rust-toolchain` in all GitHub Actions workflows

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
