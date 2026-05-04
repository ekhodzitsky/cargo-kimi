# cargo-kimi

Cargo subcommand for [kimi-dotfiles](https://github.com/ekhodzitsky/kimi-dotfiles) — structured contracts for Rust.

## Installation

```bash
cargo install --git https://github.com/ekhodzitsky/kimi-dotfiles cargo-kimi
```

## Usage

```bash
# Initialize guidelines in the current project
cargo kimi init

# Run mechanized checks (contracts, clippy, tests)
cargo kimi check

# Run formal verification with Kani (requires kani-verifier)
cargo kimi verify

# Show upgrade instructions
cargo kimi upgrade
```

## Templates

- `minimal` — Core rules only (AGENTS.md)
- `rust-only` — Core rules + Rust-specific lints (default)
- `full` — Core rules + Rust + CI + benchmarks

## Strictness

- `relaxed` — Only critical violations fail
- `standard` — Critical + major (default)
- `strict` — All violations including minor and info

## Options

| Flag | Description |
|------|-------------|
| `--template <NAME>` | Template to install |
| `--strictness <LEVEL>` | Strictness level |
| `--yes` | Skip confirmation prompts |

## Example

```bash
cargo kimi init --template rust-only --strictness strict --yes
cargo kimi check --strictness standard
```
