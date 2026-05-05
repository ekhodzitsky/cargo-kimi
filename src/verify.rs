// kimi:score-ignore=unsafe,unwrap
//! Formal verification backend for `cargo kimi verify`.
//!
//! Scans the project for Hoare-tripled `pub fn` and checks whether
//! each one has a corresponding `#[kani::proof]` harness.

use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static HOARE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*///\s*\{").unwrap());
static PUB_FN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*(pub\s+)?(async\s+)?(unsafe\s+)?fn\s+").unwrap());
static KANI_PROOF_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"#\[kani::proof\]").unwrap());
static FN_CALL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b([a-z_][a-z0-9_]*)\s*\(").unwrap());

/// Coverage report for a single crate.
#[derive(Debug, Clone, Default)]
pub struct CoverageReport {
    /// Functions that have a Hoare triple but no Kani proof harness.
    pub missing: Vec<MissingProof>,
    /// Functions that have both a Hoare triple and a Kani proof harness.
    pub covered: Vec<String>,
    /// Total `pub fn` with Hoare triples found.
    pub total: usize,
}

#[derive(Debug, Clone)]
pub struct MissingProof {
    pub file: PathBuf,
    pub line: usize,
    pub function: String,
}

/// { crate_root is a valid directory containing src/ }
/// pub fn check_coverage(crate_root: &Path) -> `anyhow::Result<CoverageReport>`
/// { returns coverage report with missing and covered Hoare-tripled functions }
pub fn check_coverage(crate_root: &Path) -> anyhow::Result<CoverageReport> {
    let src_dir = crate_root.join("src");
    if !src_dir.exists() {
        anyhow::bail!("No src/ directory found in {}", crate_root.display());
    }

    // 1. Collect all pub fn with Hoare triples
    let mut hoare_fns: HashMap<String, (PathBuf, usize)> = HashMap::new();
    for entry in walkdir::WalkDir::new(&src_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let content = std::fs::read_to_string(path)?;
        let lines: Vec<&str> = content.lines().collect();

        for (idx, line) in lines.iter().enumerate() {
            if !PUB_FN_RE.is_match(line) || !line.trim().starts_with("pub") {
                continue;
            }
            // Look backwards for Hoare triple
            let mut has_hoare = false;
            let mut i = idx;
            while i > 0 {
                i -= 1;
                let l = lines[i].trim();
                if l.starts_with("///") {
                    if HOARE_RE.is_match(l) {
                        has_hoare = true;
                        break;
                    }
                } else if l.is_empty() || l.starts_with("#") {
                    continue;
                } else {
                    break;
                }
            }
            if has_hoare {
                let name = extract_fn_name(line);
                hoare_fns.insert(name, (path.to_path_buf(), idx + 1));
            }
        }
    }

    // 2. Collect all functions called inside #[kani::proof] harnesses
    let mut proven_fns: HashSet<String> = HashSet::new();
    let kani_proofs = src_dir.join("kani_proofs.rs");
    if kani_proofs.exists() {
        let content = std::fs::read_to_string(&kani_proofs)?;
        let lines: Vec<&str> = content.lines().collect();
        let mut in_proof = false;
        let mut proof_depth = 0i32;
        for line in &lines {
            if KANI_PROOF_RE.is_match(line) {
                in_proof = true;
                proof_depth = 0;
                continue;
            }
            if in_proof {
                let trimmed = line.trim();
                proof_depth += trimmed.matches('{').count() as i32;
                proof_depth -= trimmed.matches('}').count() as i32;

                // Extract actual function calls: identifiers followed by `(`
                for cap in FN_CALL_RE.captures_iter(line) {
                    let name = &cap[1];
                    // Skip Rust keywords and kani helpers
                    if !matches!(name, "if" | "for" | "while" | "match" | "let" | "fn" | "return" | "kani") {
                        proven_fns.insert(name.to_string());
                    }
                }

                if proof_depth <= 0 && trimmed == "}" {
                    in_proof = false;
                }
            }
        }
    }

    // 3. Build report
    let mut report = CoverageReport {
        total: hoare_fns.len(),
        ..Default::default()
    };

    for (name, (file, line)) in hoare_fns {
        if proven_fns.contains(&name) {
            report.covered.push(name);
        } else {
            report.missing.push(MissingProof { file, line, function: name });
        }
    }

    Ok(report)
}

fn extract_fn_name(line: &str) -> String {
    let parts: Vec<&str> = line.split_whitespace().collect();
    for (i, &part) in parts.iter().enumerate() {
        if part == "fn" && i + 1 < parts.len() {
            let name = parts[i + 1].trim_start_matches("fn ");
            if let Some(pos) = name.find(['(', '<']) {
                return name[..pos].to_string();
            }
            return name.to_string();
        }
    }
    "unknown".to_string()
}

/// { report is a valid CoverageReport }
/// pub fn print_coverage(report: &CoverageReport)
/// { prints formatted coverage summary to stdout }
pub fn print_coverage(report: &CoverageReport) {
    use colored::*;

    println!(
        "\n{}  {}",
        "🔍 Formal Verification Coverage".bold(),
        format!("({}/{}", report.covered.len(), report.total).dimmed()
    );
    println!("{}", "───────────────────────────────────────────".dimmed());

    if report.missing.is_empty() {
        println!("  {} {}", "✅".green(), "All Hoare-tripled functions have Kani proofs.".green().bold());
    } else {
        println!(
            "  {} {} {}",
            "⚠️".yellow(),
            format!("{}", report.missing.len()).yellow().bold(),
            "functions missing formal proof harnesses:".yellow()
        );
        for m in &report.missing {
            println!(
                "     {} {} {}",
                "•".dimmed(),
                m.function.bold(),
                format!("({}:L{})", m.file.file_name().unwrap_or_default().to_string_lossy(), m.line).dimmed()
            );
        }
    }

    println!("{}", "───────────────────────────────────────────".dimmed());
}
