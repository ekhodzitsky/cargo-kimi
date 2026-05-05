# Full Code Audit — cargo-kimi v1.6.7 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Audit all source code (~3800 LOC, 17 modules) for correctness, security, quality, performance, and test coverage bugs. Produce a findings report with file:line references and a prioritized fix list.

**Architecture:** Module-by-module audit from core to periphery (4 echelons). Each task audits one module or group, produces findings in a structured table, and commits results to `docs/superpowers/audit/`.

**Tech Stack:** Rust 1.80+, regex-based static analysis, tower-lsp, tokio, serde

---

### Task 1: Setup audit report structure

**Files:**
- Create: `docs/superpowers/audit/2026-05-05-findings.md`

- [ ] **Step 1: Create audit report scaffold**

```markdown
# Code Audit Findings — cargo-kimi v1.6.7

**Date:** 2026-05-05
**Auditor:** Claude
**Scope:** All src/*.rs, tests/, CI/CD

## Summary

| Severity | Count |
|----------|-------|
| Critical | TBD |
| Major    | TBD |
| Minor    | TBD |
| Info     | TBD |

## Findings

<!-- Findings will be appended per-module -->
```

- [ ] **Step 2: Commit scaffold**

```bash
mkdir -p docs/superpowers/audit
git add docs/superpowers/audit/2026-05-05-findings.md
git commit -m "docs: scaffold audit findings report"
```

---

### Task 2: Audit `contracts.rs` — Core contract checker (830 LOC)

**Files:**
- Read: `src/contracts.rs`
- Modify: `docs/superpowers/audit/2026-05-05-findings.md`

The following are known issues to verify. The auditor MUST also search for additional issues beyond this list.

- [ ] **Step 1: Verify test-block brace counting bug (Critical)**

At `contracts.rs:236-237`, `test_block_depth` counts braces using `trimmed.matches('{').count()` — this counts braces inside string literals and comments. However, `count_braces_outside_strings()` exists at line 307 and is NOT used here. This means code like:

```rust
#[cfg(test)]
mod tests {
    fn test_it() {
        let s = "this { has } braces";  // counted incorrectly
    }
}
```

could cause `test_block_depth` to miscalculate, potentially ending the test block tracking early and flagging test code as production code.

Verify by reading lines 226-241 and confirming `count_braces_outside_strings` is only used in `compute_score` (line 388) but not in `check_file_contents`.

- [ ] **Step 2: Verify `in_safety_comment` reset logic bug (Critical)**

At `contracts.rs:249-291`, the `in_safety_comment` flag is set when a `// SAFETY:` comment is found and reset when any line ends with `}`. This is fragile:

1. Single-line unsafe: `unsafe { ptr::read(p) }` — the `}` on the same line resets the flag immediately, but the SAFETY comment was on the previous line, so this works by accident.
2. Multi-block: if a SAFETY comment is followed by an `if` block with `}` before the actual unsafe block, the flag resets prematurely.
3. The flag never resets if the unsafe block doesn't end with `}` on its own line.

Verify by reading lines 248-291.

- [ ] **Step 3: Verify scoring heuristic looseness (Major)**

At `contracts.rs:349-366`:
- **Newtype detection** (line 349-353): `t.starts_with("pub struct ") && t.contains('(') && !t.contains('{')` — matches any tuple struct, not just newtypes. `pub struct Pair(A, B)` would also match.
- **PhantomData** (line 358): `content.contains("PhantomData")` — a comment mentioning PhantomData satisfies this check.
- **Typestate** (line 363): `content.contains("enum ") && content.contains("impl ") && content.contains("From<")` — nearly any file with an enum, impl block, and a From conversion matches.

Verify by reading lines 349-366 and confirming these produce false positives.

- [ ] **Step 4: Verify `_PUB_FN_WITH_DOC_RE` is dead code (Minor)**

At `contracts.rs:16`, `_PUB_FN_WITH_DOC_RE` is defined but prefixed with `_` — never used anywhere. This is intentional dead code suppression but the regex itself is also wrong (uses `^` with multiline content but no `(?m)` flag).

Verify with: `grep -rn "PUB_FN_WITH_DOC_RE" src/`

- [ ] **Step 5: Verify unwrap exemption not applied during detection (Major)**

At `contracts.rs:255`, the unwrap detection check is:
```rust
if UNWRAP_RE.is_match(line) && !FALSE_POSITIVE_RE.is_match(line) && !in_test_block && !in_safety_comment {
```

But `!exemptions.contains("unwrap")` is NOT checked here — exemptions are only checked in `is_exempt()` (used in display) and `compute_score()`. The issue is still created and appears in `report.issues` even when exempted. This means `has_critical_unexempted` works correctly (it calls `is_exempt`), but the issue count in JSON/SARIF output includes exempted issues.

Verify by reading lines 255-266 and cross-referencing with `is_exempt` at line 431.

- [ ] **Step 6: Verify attribute handling in Hoare triple search (Minor)**

At `contracts.rs:199`, the search for Hoare triples above `pub fn` skips lines where `line.trim().starts_with("#")`. This correctly skips `#[derive(...)]` and `#[cfg(...)]`, but also skips `#![...]` inner attributes. Inner attributes shouldn't appear between doc comments and `fn`, so this is fine in practice.

Verify by reading lines 191-204.

- [ ] **Step 7: Check for false positive on `unwrap_or` (Info)**

At `contracts.rs:12`, `FALSE_POSITIVE_RE` checks for `unwrap_or(`, `unwrap_or_else(`, `unwrap_or_default(`. But what about `unwrap_unchecked(`? This is an unsafe unwrap that should be flagged but isn't caught by `UNWRAP_RE` either (it only matches `unwrap()`).

Verify by checking both regex patterns.

- [ ] **Step 8: Document all findings for contracts.rs**

Add findings table to the audit report with severity, line numbers, and recommendations.

- [ ] **Step 9: Commit**

```bash
git add docs/superpowers/audit/2026-05-05-findings.md
git commit -m "audit: contracts.rs findings"
```

---

### Task 3: Audit `config.rs` — Configuration (91 LOC)

**Files:**
- Read: `src/config.rs`
- Modify: `docs/superpowers/audit/2026-05-05-findings.md`

- [ ] **Step 1: Verify excessive `#[allow(dead_code)]` (Minor)**

At `config.rs:24,27,34,37,42,44,50,55,70,78,90` — nearly every field and method has `#[allow(dead_code)]`. This suggests the public API was designed but not fully consumed. The `Strictness` newtype at line 91 is completely unused.

Verify by running: `grep -rn "Strictness" src/` and checking which `KimiConfig` methods are actually called.

- [ ] **Step 2: Verify `should_ignore` uses substring matching (Minor)**

At `config.rs:83-86`, `path.contains(pat)` does substring matching, not glob or prefix matching. A pattern `"test"` would match `src/test_utils.rs`, `src/contest.rs`, and `tests/`. This may be surprising to users.

Verify by reading lines 82-87.

- [ ] **Step 3: Verify `score.ignore` is not used anywhere (Major)**

The `score.ignore` config field exists in `ScoreConfig` at line 43-46 and `should_ignore` method at line 82, but neither `check_files` nor `check_file_contents` in `contracts.rs` calls `should_ignore`. The config is parsed but never applied.

Verify with: `grep -rn "should_ignore\|score.*ignore" src/`

- [ ] **Step 4: Document findings and commit**

```bash
git add docs/superpowers/audit/2026-05-05-findings.md
git commit -m "audit: config.rs findings"
```

---

### Task 4: Audit `fix.rs` — Auto-fixes (597 LOC)

**Files:**
- Read: `src/fix.rs`
- Modify: `docs/superpowers/audit/2026-05-05-findings.md`

- [ ] **Step 1: Verify chained unwrap false negative (Major)**

At `fix.rs:256-259`, `is_chained_unwrap` treats ANY `foo().unwrap()` as chained and skips it. This means:
```rust
let x = some_option_variable.unwrap(); // NOT chained, should be fixed
```
vs
```rust
let x = get_value().unwrap(); // Chained, skip
```

The heuristic `before.trim_end().ends_with(')')` is too aggressive — it skips legitimate unwrap fixes whenever the variable name ends in `)`.

But also: `let x = (some_expr).unwrap()` would be treated as chained incorrectly.

Verify by reading lines 256-259 and tests at lines 528-548.

- [ ] **Step 2: Verify `apply_unwrap_fix` mutation during iteration (Major)**

At `fix.rs:261-313`, `apply_unwrap_fix` first runs `EXPECT_RE.replace_all`, then `UNWRAP_RE.replace_all` on the already-modified string. The `is_chained_unwrap` closure captures `&result` by reference. When called inside `UNWRAP_RE.replace_all` (line 280), `result` has already been modified by `EXPECT_RE.replace_all` (line 266-278).

If the EXPECT_RE replacement changes the string length, the match positions from UNWRAP_RE may be wrong relative to the original string. However, `replace_all` operates on the current `result` so the positions should be correct for the current state.

The real issue: `is_chained_unwrap` receives `&result` which is the string BEFORE the current `replace_all` starts, but `full_match` is from the string being iterated. These are the same string since `replace_all` creates a new Cow. So this is actually correct but confusing.

Verify by reading lines 261-313 carefully.

- [ ] **Step 3: Verify duplicated test-block detection logic (Minor)**

At `fix.rs:93-110`, the test-block detection logic duplicates `contracts.rs:226-241` with the same brace-counting-inside-strings bug. Both count braces using `trimmed.matches('{').count()` instead of `count_braces_outside_strings()`.

Verify by comparing the two implementations.

- [ ] **Step 4: Verify fix insertion preserves doc comments correctly (Info)**

At `fix.rs:143-178`, `fix_missing_hoare` searches backwards from the function for existing doc comments. If found, it inserts AFTER the last doc line (line 168). This was fixed in commit `887ef83`. Verify the fix is correct by reading the test at line 579-593.

- [ ] **Step 5: Verify atomic file writes (Info)**

At `fix.rs:68-72`, files are written atomically using write-to-temp + rename pattern. This is correct for crash safety. Verify the temp file naming (`format!(".tmp{}", std::process::id())`) doesn't collide with existing files.

- [ ] **Step 6: Document findings and commit**

```bash
git add docs/superpowers/audit/2026-05-05-findings.md
git commit -m "audit: fix.rs findings"
```

---

### Task 5: Audit `testgen.rs` — Test generation (472 LOC)

**Files:**
- Read: `src/testgen.rs`
- Modify: `docs/superpowers/audit/2026-05-05-findings.md`

- [ ] **Step 1: Verify regex capture group fragility (Minor)**

At `testgen.rs:7-13`, the regex patterns for trait impls have inconsistent capture groups:
- `IMPL_ADD_RE`: group 3 is type name (`impl (std::ops::)?Add (for )?(\w+)`)
- `IMPL_ORD_RE`: group 2 is type name (`impl Ord (for )?(\w+)`)
- `IMPL_EQ_RE`: group 3 is type name (`impl (Partial)?Eq (for )?(\w+)`)
- `IMPL_CLONE_RE`: group 2 is type name (`impl Clone (for )?(\w+)`)

This works but is error-prone — any change to the regex could shift group indices.

Verify by reading lines 7-13 and their usage at lines 184-201.

- [ ] **Step 2: Verify potential code injection via type names (Major)**

At `testgen.rs:229-234`, type names are inserted directly into generated Rust code:
```rust
output.push_str(&format!("    use crate::{};\n", case.type_name));
```

The type name comes from regex capture at line 168: `cap[1].to_string()` where the regex is `(\w+)` — this limits to word characters only, so injection is prevented by the regex. However, if the regex changes or a different source provides the name, this becomes exploitable.

Verify by tracing `type_name` from regex capture through to code generation.

- [ ] **Step 3: Verify derive parsing is one-line only (Minor)**

At `testgen.rs:162-179`, `pending_derive` is set when a line starts with `#[derive(`. But multi-line derives:
```rust
#[derive(
    Clone, Debug, Eq
)]
pub struct Foo(u32);
```
would not be captured because only the first line is stored and the struct line doesn't immediately follow.

Verify by reading lines 160-182.

- [ ] **Step 4: Verify generated code uses `wrapping_*` for integral types (Info)**

At `testgen.rs:245-284`, integral types get wrapping arithmetic tests (`wrapping_add` etc.) to avoid overflow panics. Non-integral types (e.g., `f64`) use direct operators. This is correct. Verify the separation logic at `is_integral_type` (line 105-107).

- [ ] **Step 5: Document findings and commit**

```bash
git add docs/superpowers/audit/2026-05-05-findings.md
git commit -m "audit: testgen.rs findings"
```

---

### Task 6: Audit `verify.rs` — Formal verification (185 LOC)

**Files:**
- Read: `src/verify.rs`
- Modify: `docs/superpowers/audit/2026-05-05-findings.md`

- [ ] **Step 1: Verify overly loose Kani proof function detection (Critical)**

At `verify.rs:106-118`, inside `#[kani::proof]` harnesses, the code splits each line on non-alphanumeric characters and treats EVERY word as a "proven function":
```rust
for word in line.split(|c: char| !c.is_alphanumeric() && c != '_') {
    if !word.is_empty() && word != "kani" {
        proven_fns.insert(word.to_string());
    }
}
```

This means variable names (`let`, `x`, `result`), keywords (`if`, `for`, `return`), type names, and any identifier inside the proof body are added to `proven_fns`. A proof like:
```rust
#[kani::proof]
fn check_foo() {
    let result = foo(42);
    assert!(result > 0);
}
```
would mark `check_foo`, `result`, `foo`, `assert`, and `0` all as "proven functions". This makes the coverage metric nearly useless — almost any function name would match by coincidence.

Verify by reading lines 106-118.

- [ ] **Step 2: Verify hardcoded `kani_proofs.rs` path (Major)**

At `verify.rs:90`, the Kani proofs are only searched in `src/kani_proofs.rs`. Proofs in other files (e.g., inline `#[kani::proof]` in the same file as the function, or in `tests/`) are ignored.

Verify by reading lines 89-91.

- [ ] **Step 3: Verify duplicated `extract_fn_name` (Minor)**

At `verify.rs:138-150`, `extract_fn_name` is an exact copy of `contracts.rs:414-426`. This violates DRY — any fix to one must be replicated in the other.

Verify with: `diff <(sed -n '414,426p' src/contracts.rs) <(sed -n '138,150p' src/verify.rs)`

- [ ] **Step 4: Verify no timeout for Kani subprocess (Minor)**

In `lib.rs:132`, `Command::new("cargo").args(["kani"]).status()` runs Kani with no timeout. Kani can run indefinitely on complex code. The user's only recourse is Ctrl-C.

- [ ] **Step 5: Document findings and commit**

```bash
git add docs/superpowers/audit/2026-05-05-findings.md
git commit -m "audit: verify.rs findings"
```

---

### Task 7: Audit `lsp.rs` — LSP server (320 LOC)

**Files:**
- Read: `src/lsp.rs`
- Modify: `docs/superpowers/audit/2026-05-05-findings.md`

- [ ] **Step 1: Verify no result caching (Major)**

In `lsp.rs`, every call to `hover()` (line 114), `code_action()` (line 88), and `check_and_publish()` (line 155) runs `contracts::check_file_contents()` independently. For a file with 100 functions, a single hover triggers a full re-check. There is no caching of results between requests for the same document version.

Verify by reading the three methods and confirming no caching mechanism exists.

- [ ] **Step 2: Verify `to_file_path().unwrap_or_default()` safety (Minor)**

At `lsp.rs:98,124,156`, `uri.to_file_path().unwrap_or_default()` returns an empty `PathBuf` if the URI is not a file URI (e.g., `untitled:` or `vscode-notebook:`). An empty path would cause `check_file_contents` to generate findings with an empty file path. This is harmless but produces confusing output.

Verify by reading these three lines.

- [ ] **Step 3: Verify `did_change` handles full sync only (Info)**

At `lsp.rs:77-81`, `did_change` takes only the first content change:
```rust
if let Some(change) = params.content_changes.into_iter().next() {
```
This is correct because the server declares `TextDocumentSyncKind::FULL` (line 47), so the client sends the entire document content as a single change. Not a bug, but the code should be documented.

- [ ] **Step 4: Verify race condition potential in document store (Minor)**

At `lsp.rs:36-37`, documents are stored in `Arc<RwLock<HashMap<...>>>`. The pattern at line 72-73:
```rust
self.check_and_publish(uri.clone(), &text).await;
self.documents.write().await.insert(uri, text);
```
publishes diagnostics BEFORE inserting the document. If `code_action` or `hover` is called between these two lines, the document won't be found in the store. This is a minor race condition.

Verify by reading lines 69-73 and 76-81.

- [ ] **Step 5: Document findings and commit**

```bash
git add docs/superpowers/audit/2026-05-05-findings.md
git commit -m "audit: lsp.rs findings"
```

---

### Task 8: Audit `mcp.rs` — MCP server (291 LOC)

**Files:**
- Read: `src/mcp.rs`
- Modify: `docs/superpowers/audit/2026-05-05-findings.md`

- [ ] **Step 1: Verify path validation in tool call (Info)**

At `mcp.rs:209`, `validate_project_path` is correctly called on the user-provided path. This prevents path traversal attacks. Verify this is the only entry point for user-controlled paths.

- [ ] **Step 2: Verify no tests exist for MCP module (Major)**

The MCP module has zero unit or integration tests. The `run_server` function at line 247 reads from stdin and writes to stdout, making it testable via mocking.

Verify with: `grep -n "#\[test\]" src/mcp.rs` and `grep -n "mcp" tests/integration_cli.rs`

- [ ] **Step 3: Verify notification handling (Info)**

At `mcp.rs:269-271`, `notifications/initialized` is handled by `continue` (no response sent). This is correct per the MCP spec — notifications don't get responses.

- [ ] **Step 4: Document findings and commit**

```bash
git add docs/superpowers/audit/2026-05-05-findings.md
git commit -m "audit: mcp.rs findings"
```

---

### Task 9: Audit `cli.rs` + `lib.rs` — CLI and public API (305 LOC)

**Files:**
- Read: `src/cli.rs`, `src/lib.rs`
- Modify: `docs/superpowers/audit/2026-05-05-findings.md`

- [ ] **Step 1: Verify division by zero in `cmd_check` (Critical)**

At `lib.rs:84`:
```rust
let avg = reports.iter().map(|r| r.score).sum::<u32>() / reports.len() as u32;
```
This line is inside the `if format == "text"` branch. If no `.rs` files are found, `reports` is empty and this panics with division by zero. The JSON and SARIF branches have their own checks, but the text branch does not guard against empty reports here.

Verify by reading `lib.rs:83-88` and confirming no `reports.is_empty()` guard exists.

- [ ] **Step 2: Verify same bug in `cmd_badge` (Critical)**

At `lib.rs:100-103`, `cmd_badge` handles empty reports correctly:
```rust
let avg = if reports.is_empty() { 0 } else { ... };
```
This is correct. But verify `cmd_check` at line 84 does NOT have this guard.

- [ ] **Step 3: Verify `Commands` enum exhaustive handling (Info)**

At `cli.rs:114-138`, all variants of `Commands` are handled in the match. Verify no variant is missing by comparing the `Commands` enum definition (lines 14-97) with the match arms.

- [ ] **Step 4: Verify `cargo kimi` argument stripping (Info)**

At `cli.rs:106-109`, when invoked as `cargo kimi`, the code strips the `kimi` argument. This is standard for cargo subcommands. Verify it handles edge cases: `cargo-kimi` directly (no stripping needed), `cargo kimi check` (strips `kimi`).

- [ ] **Step 5: Document findings and commit**

```bash
git add docs/superpowers/audit/2026-05-05-findings.md
git commit -m "audit: cli.rs + lib.rs findings"
```

---

### Task 10: Audit `watch.rs` — File watcher (89 LOC)

**Files:**
- Read: `src/watch.rs`
- Modify: `docs/superpowers/audit/2026-05-05-findings.md`

- [ ] **Step 1: Verify misleading poll interval usage (Minor)**

At `watch.rs:16`:
```rust
Config::default().with_poll_interval(Duration::from_millis(debounce_ms))
```
`with_poll_interval` sets the polling interval for the filesystem watcher backend, NOT the debounce interval. The actual debounce is the `thread::sleep` at line 45. The CLI parameter `--debounce-ms` (default 500) is used for both polling and debounce, which is misleading.

- [ ] **Step 2: Verify crude debounce loses events (Minor)**

At `watch.rs:45-46`:
```rust
std::thread::sleep(Duration::from_millis(debounce_ms));
while rx.try_recv().is_ok() {}
```
This drains all events accumulated during sleep. If multiple files change simultaneously, only one re-check happens (good), but the specific files that changed are lost (not used anyway, so this is fine for current functionality).

- [ ] **Step 3: Verify no graceful shutdown (Info)**

There is no signal handler for Ctrl-C. The `rx.recv()` at line 34 will return `Err` when the sender is dropped (watcher goes out of scope), which breaks the loop. But on Ctrl-C, the process terminates immediately without cleanup. For a file watcher, this is acceptable.

- [ ] **Step 4: Document findings and commit**

```bash
git add docs/superpowers/audit/2026-05-05-findings.md
git commit -m "audit: watch.rs findings"
```

---

### Task 11: Audit `trend.rs`, `badge.rs`, `init.rs`, `skills.rs` — Infrastructure (446 LOC)

**Files:**
- Read: `src/trend.rs`, `src/badge.rs`, `src/init.rs`, `src/skills.rs`
- Modify: `docs/superpowers/audit/2026-05-05-findings.md`

- [ ] **Step 1: Verify JSONL parsing aborts on corrupted entry (Major)**

At `trend.rs:85`:
```rust
let entry: HistoryEntry = serde_json::from_str(&line)?;
```
The `?` operator returns an error if ANY line in the JSONL file is corrupted. This aborts `show_trend` entirely instead of skipping the bad entry. A single corrupted write (e.g., from a crash) makes all history inaccessible.

Verify by reading lines 80-93.

- [ ] **Step 2: Verify `init.rs` overwrites .cargo/config.toml without prompt (Minor)**

At `init.rs:76-78`:
```rust
if Path::new("Cargo.toml").exists() {
    fs::create_dir_all(".cargo")?;
    fs::write(".cargo/config.toml", clippy)?;
```
The overwrite confirmation (lines 45-54) only applies to AGENTS.md. `.cargo/config.toml` is always overwritten without asking, even when `--yes` is not passed.

- [ ] **Step 3: Verify badge SVG has no XSS risk (Info)**

At `badge.rs:19-72`, the label `"kimi"` is hardcoded and the score is `u32`. Both are safe for XML content. No user-controlled strings are inserted into the SVG.

Confirm by reading lines 19-72.

- [ ] **Step 4: Verify `SkillName` validation is correct (Info)**

At `skills.rs:36-51`, `SkillName::new` validates: non-empty, no path separators, no `..`, no leading `.`, matches `[a-z0-9-]+`. This is thorough.

- [ ] **Step 5: Document findings and commit**

```bash
git add docs/superpowers/audit/2026-05-05-findings.md
git commit -m "audit: trend.rs, badge.rs, init.rs, skills.rs findings"
```

---

### Task 12: Audit `workspace.rs` + `util.rs` — Workspace discovery and security (95 LOC)

**Files:**
- Read: `src/workspace.rs`, `src/util.rs`
- Modify: `docs/superpowers/audit/2026-05-05-findings.md`

- [ ] **Step 1: Verify fragile workspace member parsing (Minor)**

At `workspace.rs:39`:
```rust
m.split(' ').next().unwrap_or("").to_string()
```
Workspace member IDs have the format `"name version (path)"`. The code extracts the name by splitting on space. If a package name ever contains a space (invalid in Cargo, but possible in edge cases), this would break.

Verify by reading line 39 and confirming Cargo guarantees no spaces in package names.

- [ ] **Step 2: Verify path traversal protection completeness (Major)**

At `util.rs:6-29`, `validate_project_path`:
1. Checks for `..` components — correct
2. Canonicalizes `cwd` but NOT the target path — symlinks in the target path could escape the project directory
3. No null byte check — on some systems, null bytes in paths could bypass checks

Verify by reading lines 6-29 and considering: what happens if `path` is `/absolute/path/outside/project`? The code handles this (line 25: `!normalized.starts_with(&cwd)`). But a symlink `src/link -> /etc/` would pass because `normalized` is `{cwd}/src/link` which starts with `cwd`.

- [ ] **Step 3: Document findings and commit**

```bash
git add docs/superpowers/audit/2026-05-05-findings.md
git commit -m "audit: workspace.rs + util.rs findings"
```

---

### Task 13: Audit tests and CI/CD

**Files:**
- Read: `tests/integration_cli.rs`, `.github/workflows/dogfood.yml`, `.github/workflows/release.yml`
- Modify: `docs/superpowers/audit/2026-05-05-findings.md`

- [ ] **Step 1: Identify test coverage gaps (Major)**

Modules with NO unit tests:
- `mcp.rs` — zero tests
- `watch.rs` — zero tests
- `workspace.rs` — zero tests
- `init.rs` — zero unit tests (only integration test)
- `trend.rs` — zero unit tests (only integration test)
- `verify.rs` — zero tests
- `util.rs` — zero tests

Missing test scenarios for `contracts.rs`:
- Code inside string literals matched by regex (e.g., `let s = "fn unwrap()";`)
- Multi-line function signatures
- Nested `mod tests` blocks
- Files with only `//` comments, no `///` doc comments

Verify by running: `grep -c "#\[test\]" src/*.rs`

- [ ] **Step 2: Check CI action pinning**

Read `.github/workflows/dogfood.yml` and `.github/workflows/release.yml`. Check if GitHub Actions are pinned to SHA (e.g., `uses: actions/checkout@v4` vs `uses: actions/checkout@abc123`). Unpinned actions are a supply chain risk.

- [ ] **Step 3: Check for secrets exposure in CI**

Verify `CARGO_REGISTRY_TOKEN` in `release.yml` is only used in the publish step and not echoed or logged.

- [ ] **Step 4: Document findings and commit**

```bash
git add docs/superpowers/audit/2026-05-05-findings.md
git commit -m "audit: tests and CI/CD findings"
```

---

### Task 14: Compile final audit report

**Files:**
- Modify: `docs/superpowers/audit/2026-05-05-findings.md`

- [ ] **Step 1: Count findings by severity**

Go through all documented findings and update the Summary table with actual counts.

- [ ] **Step 2: Create prioritized action list**

Add a "Recommended Fix Order" section:

1. **Critical fixes first** — division by zero in `lib.rs:84`, Kani coverage detection in `verify.rs`
2. **Major correctness fixes** — brace counting bug in `contracts.rs`, `score.ignore` not applied
3. **Major reliability** — JSONL parsing abort in `trend.rs`, test coverage gaps
4. **Minor improvements** — all Minor findings
5. **Info items** — documentation, suggestions

- [ ] **Step 3: Final commit**

```bash
git add docs/superpowers/audit/2026-05-05-findings.md
git commit -m "audit: final report with prioritized action list"
```

---

## Cross-Cutting Concerns

These patterns appear across multiple modules:

1. **Duplicated code:** `extract_fn_name` in `contracts.rs` and `verify.rs`. Test-block detection in `contracts.rs` and `fix.rs`.
2. **Brace counting inconsistency:** `count_braces_outside_strings` exists in `contracts.rs` but is only used in `compute_score`, not in `check_file_contents` or `fix.rs`.
3. **Regex-based parsing limitations:** No module uses a proper Rust AST parser. All parsing is line-by-line regex which fails on multi-line constructs, macros, and code inside string literals.
4. **`#[allow(dead_code)]` proliferation:** Unused newtypes at the bottom of almost every file (`Score`, `FixIndex`, `TypeName`, `LspUri`, `McpMethod`, `CommandName`, `DebounceMs`, `CratePath`, `ProjectPath`, `TemplateName`, `Strictness`). These appear to be added for the newtype scoring criterion.
5. **No `score.ignore` integration:** The config option exists but is never applied during checking.
