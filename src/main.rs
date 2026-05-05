mod cli;
mod contracts;
mod fix;
mod init;
mod mcp;
mod skills;
mod testgen;
mod trend;
mod watch;
mod workspace;

use std::path::Path;
use std::process::Command;

fn main() -> anyhow::Result<()> {
    cli::run()
}

/// { strictness is a valid strictness level, format is "text" or "json" }
/// fn cmd_check(strictness: &str, format: &str) -> anyhow::Result<()>
/// { runs contract checker, prints reports, then clippy + tests }
fn cmd_check(strictness: &str, format: &str) -> anyhow::Result<()> {
    let config = contracts::CheckConfig::from_strictness(strictness)?;
    let paths = workspace::find_workspace_crates()?;
    let reports = contracts::check_files(&paths, &config)?;

    if format == "json" {
        let json = serde_json::to_string_pretty(&reports)?;
        println!("{}", json);
        // Skip clippy/test and history when emitting JSON — output must be pure JSON
        let has_critical = reports.iter().any(|r| {
            r.issues.iter().any(|i| {
                i.severity == contracts::Severity::Critical
                    && !contracts::is_exempt(i, &r.exemptions)
            })
        });
        if has_critical {
            anyhow::bail!("Contract check failed: critical issues found");
        }
        return Ok(());
    }

    if format == "sarif" {
        contracts::print_sarif(&reports)?;
        // Skip clippy/test when emitting SARIF — output must be pure SARIF
        let has_critical = reports.iter().any(|r| {
            r.issues.iter().any(|i| {
                i.severity == contracts::Severity::Critical
                    && !contracts::is_exempt(i, &r.exemptions)
            })
        });
        if has_critical {
            anyhow::bail!("Contract check failed: critical issues found");
        }
        return Ok(());
    }

    println!("=== Running contract checker (strictness: {}) ===", strictness);
    contracts::print_reports(&reports);

    // Append scores to history for trend tracking
    if let Err(e) = trend::append_history(&reports) {
        eprintln!("⚠ Failed to append score history: {}", e);
    }

    let has_critical = reports.iter().any(|r| {
        r.issues.iter().any(|i| {
            i.severity == contracts::Severity::Critical
                && !contracts::is_exempt(i, &r.exemptions)
        })
    });
    if has_critical {
        anyhow::bail!("❌ Contract check failed: critical issues found");
    }

    println!("\n=== Running cargo clippy ===");
    let status = Command::new("cargo")
        .args(["clippy", "--workspace", "--", "-D", "warnings"])
        .status()?;
    if !status.success() {
        anyhow::bail!("❌ Clippy failed");
    }

    println!("\n=== Running cargo test ===");
    let status = Command::new("cargo").args(["test", "--workspace"]).status()?;
    if !status.success() {
        anyhow::bail!("❌ Tests failed");
    }

    println!("\n✅ All checks passed");
    Ok(())
}

/// { Kani verifier is installed }
/// fn cmd_verify() -> anyhow::Result<()>
/// { runs cargo kani on the current workspace }
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

/// { output path is inside project directory }
/// fn cmd_generate_tests(output: Option<&str>) -> anyhow::Result<()>
/// { scans src/ for newtypes and generates proptest property tests }
fn cmd_generate_tests(output: Option<&str>) -> anyhow::Result<()> {
    let src = Path::new("src");
    let out = output.map(Path::new);
    if let Some(p) = out {
        if p.components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            anyhow::bail!("Output path cannot contain parent directory references (..)");
        }
        let cwd = std::env::current_dir()?.canonicalize()?;
        let abs_path = if p.is_absolute() {
            p.to_path_buf()
        } else {
            cwd.join(p)
        };
        let mut normalized = std::path::PathBuf::new();
        for comp in abs_path.components() {
            match comp {
                std::path::Component::CurDir => {}
                _ => normalized.push(comp),
            }
        }
        if !normalized.starts_with(&cwd) {
            anyhow::bail!("Output path must be inside the project directory");
        }
    }
    testgen::write_tests(src, out)
}

/// { true }
/// fn cmd_upgrade() -> anyhow::Result<()>
/// { prints upgrade instructions to stdout }
fn cmd_upgrade() -> anyhow::Result<()> {
    println!("To upgrade cargo-kimi, run:");
    println!(
        "  cargo install --force --git https://github.com/ekhodzitsky/kimi-dotfiles cargo-kimi"
    );
    println!("\nTo update project guidelines, re-run:");
    println!("  cargo kimi init --template rust-only --yes");
    Ok(())
}
