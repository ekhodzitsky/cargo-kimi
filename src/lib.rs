#[allow(dead_code)]
pub struct Score(pub(crate) u32);

pub mod badge;
pub mod cli;
pub mod config;
pub mod contracts;
pub mod fix;
pub mod init;
pub mod lsp;
pub mod mcp;
pub mod skills;
pub mod testgen;
pub mod trend;
pub mod util;
pub mod verify;
pub mod watch;
pub mod workspace;

use std::path::Path;
use std::process::Command;

/// { strictness is a valid strictness level, format is "text" or "json" }
/// fn cmd_check(strictness: &str, format: &str) -> anyhow::Result<()>
/// { runs contract checker, prints reports, then clippy + tests }
pub fn cmd_check(strictness: &str, format: &str) -> anyhow::Result<()> {
    let cfg = config::load_config(None)?;
    let strictness = cfg.strictness().unwrap_or(strictness);
    let format = cfg.output_format().unwrap_or(format);

    let config = contracts::CheckConfig::from_strictness(strictness)?;
    let paths = workspace::find_workspace_crates()?;
    let reports = contracts::check_files(&paths, &config)?;

    if format == "json" {
        let json = serde_json::to_string_pretty(&reports)?;
        println!("{}", json);
        // Skip clippy/test and history when emitting JSON — output must be pure JSON
        if contracts::has_critical_unexempted(&reports) {
            anyhow::bail!("Contract check failed: critical issues found");
        }
        return Ok(());
    }

    if format == "sarif" {
        contracts::print_sarif(&reports)?;
        // Skip clippy/test when emitting SARIF — output must be pure SARIF
        if contracts::has_critical_unexempted(&reports) {
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

    if contracts::has_critical_unexempted(&reports) {
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

    // Auto-generate badge if configured or if --format is not json/sarif
    if format == "text" {
        let avg = reports.iter().map(|r| r.score).sum::<u32>() / reports.len() as u32;
        if let Err(e) = badge::write_badge(std::path::Path::new("kimi-score.svg"), badge::BadgeScore::new(avg)) {
            eprintln!("⚠ Failed to write badge: {}", e);
        }
    }

    Ok(())
}

/// { output is a valid path, strictness is valid }
/// fn cmd_badge(output: &str, strictness: &str) -> anyhow::Result<()>
/// { runs contract check and writes SVG badge to output path }
pub fn cmd_badge(output: &str, strictness: &str) -> anyhow::Result<()> {
    let config = contracts::CheckConfig::from_strictness(strictness)?;
    let paths = workspace::find_workspace_crates()?;
    let reports = contracts::check_files(&paths, &config)?;
    let avg = if reports.is_empty() {
        0
    } else {
        reports.iter().map(|r| r.score).sum::<u32>() / reports.len() as u32
    };
    badge::write_badge(std::path::Path::new(output), badge::BadgeScore::new(avg))?;
    println!("✅ Badge written to {} (score: {})", output, avg);
    Ok(())
}

/// { Kani verifier is installed }
/// fn cmd_verify() -> anyhow::Result<()>
/// { checks proof coverage, then runs cargo kani on the current workspace }
pub fn cmd_verify() -> anyhow::Result<()> {
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

    println!("\n=== Checking formal verification coverage ===");
    let coverage = verify::check_coverage(std::env::current_dir()?.as_path())?;
    verify::print_coverage(&coverage);

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
pub fn cmd_generate_tests(output: Option<&str>) -> anyhow::Result<()> {
    let src = Path::new("src");
    let out = output.map(Path::new);
    if let Some(p) = out {
        util::validate_project_path(p)?;
    }
    testgen::write_tests(src, out)
}

/// { true }
/// fn cmd_upgrade() -> anyhow::Result<()>
/// { prints upgrade instructions to stdout }
pub fn cmd_upgrade() -> anyhow::Result<()> {
    println!("To upgrade cargo-kimi, run:");
    println!(
        "  cargo install --force --git https://github.com/ekhodzitsky/cargo-kimi cargo-kimi"
    );
    println!("\nTo update project guidelines, re-run:");
    println!("  cargo kimi init --template rust-only --yes");
    Ok(())
}
