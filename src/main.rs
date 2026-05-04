use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

/// Cargo subcommand for kimi-dotfiles — structured contracts for Rust
#[derive(Parser)]
#[command(name = "cargo-kimi")]
#[command(about = "Initialize, check, and verify Rust projects with kimi-dotfiles guidelines")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize kimi-dotfiles in the current project
    Init {
        /// Template to use
        #[arg(short, long, value_name = "NAME", default_value = "rust-only")]
        template: String,
        /// Strictness level for clippy config
        #[arg(short, long, value_name = "LEVEL", default_value = "standard")]
        strictness: String,
        /// Where to place AGENTS.md (root or .kimi)
        #[arg(short, long, value_name = "PATH", default_value = "auto")]
        location: String,
        /// Skip confirmation prompts
        #[arg(short, long)]
        yes: bool,
    },
    /// Run mechanized checks: contracts, clippy, tests
    Check {
        /// Strictness level for contract checker
        #[arg(short, long, value_name = "LEVEL", default_value = "standard")]
        strictness: String,
    },
    /// Run formal verification with Kani (if installed)
    Verify,
    /// Show upgrade instructions
    Upgrade,
}

// Embedded AGENTS.md templates
const AGENTS_MINIMAL: &str = include_str!("../../templates/minimal/AGENTS.md");
const AGENTS_RUST_ONLY: &str = include_str!("../../templates/rust-only/AGENTS.md");
const AGENTS_FULL: &str = include_str!("../../templates/full/AGENTS.md");

// Embedded clippy configs
const CLIPPY_RELAXED: &str = include_str!("../../strictness/relaxed.toml");
const CLIPPY_STANDARD: &str = include_str!("../../strictness/standard.toml");
const CLIPPY_STRICT: &str = include_str!("../../strictness/strict.toml");

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    // When invoked as `cargo kimi`, cargo passes `kimi` as the first argument.
    // We need to strip it so clap sees only our subcommands.
    let args = if args.get(1).map(|s| s.as_str()) == Some("kimi") {
        let mut filtered = vec![args[0].clone()];
        filtered.extend(args.into_iter().skip(2));
        filtered
    } else {
        args
    };
    let cli = Cli::parse_from(args);
    match cli.command {
        Commands::Init {
            template,
            strictness,
            location,
            yes,
        } => cmd_init(&template, &strictness, &location, yes),
        Commands::Check { strictness } => cmd_check(&strictness),
        Commands::Verify => cmd_verify(),
        Commands::Upgrade => cmd_upgrade(),
    }
}

fn cmd_init(template: &str, strictness: &str, location: &str, yes: bool) -> anyhow::Result<()> {
    let agents = resolve_agents(template)?;
    let clippy = resolve_clippy(strictness)?;

    let target_path = match location {
        "auto" => {
            if Path::new(".kimi/AGENTS.md").exists() {
                ".kimi/AGENTS.md"
            } else {
                "AGENTS.md"
            }
        }
        ".kimi" => {
            fs::create_dir_all(".kimi")?;
            ".kimi/AGENTS.md"
        }
        "root" => "AGENTS.md",
        _ => anyhow::bail!("Unknown location: '{}'. Available: auto, root, .kimi", location),
    };

    // Confirm overwrite
    if Path::new(target_path).exists() && !yes {
        print!("{} already exists. Overwrite? [y/N] ", target_path);
        io::stdout().flush()?;
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        if buf.trim().to_ascii_lowercase() != "y" {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Write AGENTS.md
    fs::write(target_path, agents)?;
    println!("✓ Created {} (template: {})", target_path, template);

    // Write .cargo/config.toml for Rust projects
    if Path::new("Cargo.toml").exists() {
        fs::create_dir_all(".cargo")?;
        fs::write(".cargo/config.toml", clippy)?;
        println!("✓ Created .cargo/config.toml (strictness: {})", strictness);
    } else {
        println!("⚠ No Cargo.toml found — skipping .cargo/config.toml");
    }

    if Path::new("Cargo.toml").exists() {
        println!("\nNext: git add {} .cargo/config.toml && git commit -m 'Add kimi-dotfiles guidelines'", target_path);
    } else {
        println!("\nNext: git add {} && git commit -m 'Add kimi-dotfiles guidelines'", target_path);
    }
    Ok(())
}

fn cmd_check(strictness: &str) -> anyhow::Result<()> {
    println!("=== Running contract checker (strictness: {}) ===", strictness);
    if Path::new("scripts/check-contracts.py").exists() {
        let status = Command::new("python3")
            .args(["scripts/check-contracts.py", "src/", "--strictness", strictness])
            .status()?;
        if !status.success() {
            anyhow::bail!("❌ Contract check failed");
        }
    } else {
        println!("⚠ scripts/check-contracts.py not found — skipping contract check");
    }

    println!("\n=== Running cargo clippy ===");
    let status = Command::new("cargo")
        .args(["clippy", "--", "-D", "warnings"])
        .status()?;
    if !status.success() {
        anyhow::bail!("❌ Clippy failed");
    }

    println!("\n=== Running cargo test ===");
    let status = Command::new("cargo").args(["test"]).status()?;
    if !status.success() {
        anyhow::bail!("❌ Tests failed");
    }

    println!("\n✅ All checks passed");
    Ok(())
}

fn cmd_verify() -> anyhow::Result<()> {
    println!("=== Checking Kani installation ===");
    let status = Command::new("cargo")
        .args(["kani", "--version"])
        .status();
    match status {
        Ok(s) if s.success() => {}
        _ => {
            anyhow::bail!(
                "❌ Kani not installed.\n   Install: cargo install --locked kani-verifier && cargo kani setup"
            );
        }
    }

    println!("\n=== Running cargo kani ===");
    let status = Command::new("cargo").args(["kani"]).status()?;
    if !status.success() {
        anyhow::bail!("❌ Kani verification failed");
    }

    println!("\n✅ Kani verification passed");
    Ok(())
}

fn cmd_upgrade() -> anyhow::Result<()> {
    println!("To upgrade cargo-kimi, run:");
    println!("  cargo install --force --git https://github.com/ekhodzitsky/kimi-dotfiles cargo-kimi");
    println!("\nTo update project guidelines, re-run:");
    println!("  cargo kimi init --template rust-only --yes");
    Ok(())
}

fn resolve_agents(name: &str) -> anyhow::Result<&'static str> {
    match name {
        "minimal" => Ok(AGENTS_MINIMAL),
        "rust-only" => Ok(AGENTS_RUST_ONLY),
        "full" => Ok(AGENTS_FULL),
        _ => anyhow::bail!(
            "Unknown template: '{}'. Available: minimal, rust-only, full",
            name
        ),
    }
}

fn resolve_clippy(level: &str) -> anyhow::Result<&'static str> {
    match level {
        "relaxed" => Ok(CLIPPY_RELAXED),
        "standard" => Ok(CLIPPY_STANDARD),
        "strict" => Ok(CLIPPY_STRICT),
        _ => anyhow::bail!(
            "Unknown strictness: '{}'. Available: relaxed, standard, strict",
            level
        ),
    }
}
