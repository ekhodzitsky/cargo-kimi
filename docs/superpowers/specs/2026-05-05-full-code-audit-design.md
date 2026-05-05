# Full Code Audit — cargo-kimi v1.6.7

**Date:** 2026-05-05
**Scope:** All source code (~3800 LOC, 17 modules), tests, CI/CD
**Approach:** Module-by-module, core to periphery

---

## 1. Audit Structure

Four echelons ordered by dependency:

| Echelon | Modules | LOC | Priority |
|---------|---------|-----|----------|
| 1 — Core | `contracts.rs`, `config.rs` | 921 | Critical |
| 2 — Core consumers | `fix.rs`, `testgen.rs`, `verify.rs` | 1254 | High |
| 3 — Interfaces | `lsp.rs`, `mcp.rs`, `cli.rs`, `lib.rs` | 916 | Medium |
| 4 — Infrastructure | `watch.rs`, `trend.rs`, `badge.rs`, `init.rs`, `skills.rs`, `workspace.rs`, `util.rs`, tests, CI/CD | 695+ | Standard |

## 2. Audit Dimensions

Every module is evaluated across five dimensions:

1. **Correctness** — logic bugs, edge cases, false positives/negatives
2. **Security** — input validation, path traversal, injection, ReDoS
3. **Code quality** — idiomatic Rust, duplication, naming, module size
4. **Performance** — unnecessary allocations, O(n²) patterns, regex compilation
5. **Test coverage** — missing scenarios, untested paths

## 3. Echelon 1 — Core

### 3.1 `contracts.rs` (830 LOC)

**Correctness (critical):**
- Regex patterns for `pub fn`, `unwrap()`, `expect()`, `panic!()`, `unsafe` — verify no false positives from comments, macros, string literals
- Hoare triple recognition (`/// {`) — correct precondition/postcondition detection above `pub fn`
- Scoring arithmetic — weight calculations, boundary values (0, 100), no overflow
- Exemptions (`// kimi:score-ignore=...`) — parsing, application, edge cases
- `mod tests` detection — correctly excludes test blocks from checks
- Newtype detection — accuracy of `pub struct Foo(Bar)` pattern matching
- Function length calculation — correct line counting (ignores blanks? comments?)

**Security:**
- File I/O — symlink handling, invalid paths, binary files
- Regex DoS (ReDoS) — catastrophic backtracking analysis for all patterns

**Quality:**
- 830 lines — candidate for splitting into submodules (parsing, scoring, reporting)
- Regex pattern duplication across functions
- Self-consistency — does the contract checker use `unwrap()` itself?

**Performance:**
- `LazyLock` regex compilation — verify all patterns are lazy-static, not recompiled per file
- File reading strategy — full file in memory vs streaming

### 3.2 `config.rs` (91 LOC)

- Invalid `.kimi.toml` values — error messages, fallback behavior
- Unknown strictness levels — handled or silently ignored?
- Missing config file — correct defaults applied
- Default values — documented and consistent with README

## 4. Echelon 2 — Core Consumers

### 4.1 `fix.rs` (597 LOC)

**Correctness (critical):**
- Hoare triple insertion — preserves existing doc comments, attributes (`#[derive]`, `#[cfg]`)
- `unwrap()` → `?` replacement — correctly identifies function return type for `?` validity
- `// SAFETY` comment positioning relative to `unsafe` block
- Idempotency — repeated `fix` runs must not duplicate fixes
- Multi-line function signatures — correct insertion point

**Quality:**
- Parsing duplication with `contracts.rs` — shared logic opportunity
- String manipulation vs regex — consistency of approach

### 4.2 `testgen.rs` (472 LOC)

**Correctness:**
- Newtype pattern parsing — edge cases: generics (`Foo<T>`), lifetimes (`Foo<'a>`), visibility modifiers
- Arithmetic property tests — overflow/underflow for `u8`/`i8`, division by zero
- Generated code validity — compiles and passes tests

**Security:**
- Code injection — can a type/field name inject arbitrary code into generated tests?

### 4.3 `verify.rs` (185 LOC)

**Correctness:**
- Kani output parsing — correct extraction of coverage data
- Kani not installed — graceful error handling

**Reliability:**
- Process timeouts — Kani can run indefinitely
- Exit code handling — non-zero exits from Kani subprocess

## 5. Echelon 3 — Interfaces

### 5.1 `lsp.rs` (320 LOC)

**Correctness:**
- Document lifecycle — `did_open`/`did_change`/`did_close` state management
- Diagnostics accuracy — match `contracts.rs` findings
- Code actions — correct application

**Security:**
- Race conditions — concurrent HashMap access in async context (needs `Arc<Mutex<>>` or `DashMap`?)
- URI parsing — untrusted input from editor
- Invalid positions/ranges — out-of-bounds handling

### 5.2 `mcp.rs` (291 LOC)

- JSON-RPC request validation — malformed requests, unknown methods
- Result mapping — `contracts.rs` output → MCP format correctness
- Error responses — proper JSON-RPC error codes

### 5.3 `cli.rs` (141 LOC)

- All `Commands` variants handled exhaustively
- Exit codes — correct for success/failure/error scenarios
- Argument validation — conflicting flags, missing required args

### 5.4 `lib.rs` (164 LOC)

- Public API completeness — all necessary types re-exported
- docs.rs compatibility — `rustdoc` warnings
- API surface — no accidental exposure of internal types

## 6. Echelon 4 — Infrastructure

### 6.1 `watch.rs` (89 LOC)
- Debounce logic correctness
- Resource leaks — file watchers not cleaned up
- Graceful shutdown on Ctrl+C

### 6.2 `trend.rs` (152 LOC)
- JSONL parsing — corrupted/truncated entries
- Timestamp handling — timezone consistency
- Empty history — correct behavior

### 6.3 `badge.rs` (113 LOC)
- SVG generation — user data escaping (XSS if badge is embedded in HTML)
- Score boundary rendering (0, 100, negative?)

### 6.4 `init.rs` (148 LOC)
- Existing file overwrite — prompt or skip?
- Template selection — all templates valid and complete
- Race conditions — concurrent `init` runs

### 6.5 `skills.rs` (93 LOC)
- YAML frontmatter parsing — malformed YAML handling
- File creation — path validation

### 6.6 `workspace.rs` (64 LOC)
- `cargo metadata` failure — error propagation
- Workspace vs single crate — correct detection

### 6.7 `util.rs` (31 LOC)
- Path traversal protection — completeness (`..\`, symlinks, null bytes, URL-encoded paths)
- Platform differences — Windows vs Unix path separators

## 7. Tests & CI/CD

### 7.1 Test Coverage Gaps
- Identify modules with zero test coverage
- Missing edge case tests for `contracts.rs` (the most critical module)
- Integration test completeness — all CLI commands covered?

### 7.2 CI/CD Security
- `dogfood.yml` — actions pinned to SHA? secrets exposure?
- `release.yml` — supply chain: build matrix, publish safety
- `deny.toml` — license coverage, vulnerability scanning

## 8. Output Format

Per-module findings table:

| Severity | Category | Description | File:Line | Recommendation |
|----------|----------|-------------|-----------|----------------|
| Critical | Correctness | ... | `contracts.rs:123` | ... |
| Major | Security | ... | `lsp.rs:45` | ... |

Severity levels:
- **Critical** — logic bugs causing incorrect results, security vulnerabilities
- **Major** — significant quality/reliability issues
- **Minor** — code quality, minor edge cases
- **Info** — suggestions, improvements

Final deliverable: prioritized action list grouped by severity.

## 9. Success Criteria

- Every `.rs` file in `src/` reviewed across all 5 dimensions
- All findings documented with file:line references
- Zero false claims — each finding verified against actual code
- Prioritized action list ready for implementation
