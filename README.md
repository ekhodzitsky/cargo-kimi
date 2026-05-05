# cargo-kimi

Cargo subcommand for [kimi-dotfiles](https://github.com/ekhodzitsky/kimi-dotfiles) — structured contracts for Rust.

## Installation

```bash
cargo install --git https://github.com/ekhodzitsky/kimi-dotfiles cargo-kimi
```

## Commands

### `cargo kimi init`

Initialize AI coding guidelines in the current project.

```bash
cargo kimi init --template rust-only --strictness strict --yes
```

**Templates:**
- `minimal` — Core rules only (`AGENTS.md`)
- `rust-only` — Core rules + Rust-specific lints (default)
- `full` — Core rules + Rust + CI + benchmarks
- `modular` — Split rules into `.kimi/parts/` directory for large projects

### `cargo kimi check`

Run mechanized checks and compute a 0–100 contract score.

```bash
cargo kimi check                   # Standard strictness + clippy + tests
cargo kimi check --strictness strict
cargo kimi check --format json     # Pure JSON output for CI (no clippy/test)
```

Scoring breakdown:
- Hoare triples on `pub fn` — 30 pts
- No `unwrap`/`expect`/`panic` — 20 pts
- Newtype wrappers — 10 pts
- `PhantomData` usage — 10 pts
- Typestate patterns — 10 pts
- Average function length ≤ 40 lines — 10 pts
- Proper `Result` handling — 10 pts

**Score exemptions:** Add `// kimi:score-ignore=unwrap,unsafe` in the first 10 lines of a file to waive specific penalties (issues are still reported with `[EXEMPT]`). Useful for FFI boundaries and generated code.

### `cargo kimi fix`

Auto-fix mechanical issues: insert Hoare triple stubs, replace `unwrap()` with `?`,
and add `// SAFETY:` comments before `unsafe` blocks.

```bash
cargo kimi fix --dry-run           # Preview changes
cargo kimi fix                     # Apply fixes
```

**What it does:**
- Adds `/// { precondition }` / `/// { postcondition }` doc comments above `pub fn`
- Replaces `.unwrap()` → `?` where the return type allows it
- Replaces `.expect("msg")` → `.map_err(|e| format!("msg: {e}"))?`
- Adds `// SAFETY: TODO: explain why this is safe` before `unsafe` blocks

### `cargo kimi trend`

Show score history as an ASCII bar chart.

```bash
cargo kimi trend --days 30
```

Scores are appended to `.kimi/score-history.jsonl` after every `cargo kimi check`.

### `cargo kimi verify`

Run formal verification with Kani (requires `kani-verifier`).

```bash
cargo kimi verify
```

### `cargo kimi init-skill`

Generate a `SKILL.md` with YAML frontmatter compatible with agentskills.io.

```bash
cargo kimi init-skill my-skill "Description of what this skill does"
```

### `cargo kimi mcp`

Start an MCP server over stdio for Claude Code, Codex, and other MCP clients.

```bash
cargo kimi mcp
```

Exposes the `check_contracts` tool natively — no shell execution required.

## Strictness Levels

- `relaxed` — Only critical violations fail
- `standard` — Critical + major (default)
- `strict` — All violations including minor and info

## Options

| Flag | Description |
|------|-------------|
| `--template <NAME>` | Template to install (`init` only) |
| `--strictness <LEVEL>` | Strictness level |
| `--yes` | Skip confirmation prompts |
| `--dry-run` | Preview changes without applying (`fix` only) |
| `--days <N>` | Number of days for trend chart (`trend` only) |

## Kimi Skills

Two skills are included for use inside Kimi Code CLI:

- **`kimi-check`** — `/kimi.check [--strictness strict]` runs contract checks and reports scores
- **`kimi-fix`** — `/kimi.fix [--dry-run]` applies mechanical fixes

Install them by copying `skills/kimi-check/` and `skills/kimi-fix/` to your `~/.claude/skills/` directory.

## GitHub Action

Add contract checking to your CI with automatic PR comments:

```yaml
# .github/workflows/kimi.yml
name: Kimi Contract Check

on:
  pull_request:
    paths:
      - '**.rs'
      - 'Cargo.toml'

jobs:
  contracts:
    runs-on: ubuntu-latest
    permissions:
      pull-requests: write
    steps:
      - uses: actions/checkout@v4
      - uses: ekhodzitsky/kimi-dotfiles/.github/actions/cargo-kimi@main
        with:
          strictness: standard
          fail-on-drop: 60
          post-comment: true
```

**Inputs:**

| Input | Default | Description |
|-------|---------|-------------|
| `strictness` | `standard` | Contract strictness level |
| `fail-on-drop` | `0` | Fail CI if score drops below threshold (0 = off) |
| `post-comment` | `true` | Post PR comment with results |

## Example Workflow

```bash
# Initialize a Rust project with strict rules
cargo kimi init --template rust-only --strictness strict --yes

# Check current score
cargo kimi check

# Preview auto-fixes
cargo kimi fix --dry-run

# Apply mechanical fixes
cargo kimi fix

# Re-check after fixes
cargo kimi check

# View score trend over time
cargo kimi trend --days 14
```
