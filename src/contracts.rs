use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Critical,
    Major,
    Minor,
    Info,
}

impl Severity {
    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "CRITICAL" => Some(Severity::Critical),
            "MAJOR" => Some(Severity::Major),
            "MINOR" => Some(Severity::Minor),
            "INFO" => Some(Severity::Info),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Issue {
    pub file: PathBuf,
    pub line: usize,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Default)]
pub struct FileReport {
    pub file: PathBuf,
    pub issues: Vec<Issue>,
}

#[derive(Debug, Clone)]
pub struct CheckConfig {
    pub strictness: HashSet<Severity>,
}

impl CheckConfig {
    pub fn from_strictness(level: &str) -> anyhow::Result<Self> {
        let mut set = HashSet::new();
        match level {
            "relaxed" => {
                set.insert(Severity::Critical);
            }
            "standard" => {
                set.insert(Severity::Critical);
                set.insert(Severity::Major);
            }
            "strict" => {
                set.insert(Severity::Critical);
                set.insert(Severity::Major);
                set.insert(Severity::Minor);
                set.insert(Severity::Info);
            }
            _ => anyhow::bail!(
                "Unknown strictness: '{}'. Available: relaxed, standard, strict",
                level
            ),
        }
        Ok(CheckConfig { strictness: set })
    }
}

pub fn check_files(paths: &[PathBuf], config: &CheckConfig) -> anyhow::Result<Vec<FileReport>> {
    let mut reports = Vec::new();
    let mut files = Vec::new();

    for path in paths {
        if path.is_file() && path.extension().map(|e| e == "rs").unwrap_or(false) {
            files.push(path.to_path_buf());
        } else if path.is_dir() {
            for entry in walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .filter(|e| e.path().extension().map(|ext| ext == "rs").unwrap_or(false))
            {
                files.push(entry.path().to_path_buf());
            }
        }
    }

    files.sort();
    files.dedup();

    for file in &files {
        let report = check_file(file, config)?;
        if !report.issues.is_empty() {
            reports.push(report);
        }
    }

    Ok(reports)
}

fn check_file(path: &Path, config: &CheckConfig) -> anyhow::Result<FileReport> {
    let content = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();

    let mut issues = Vec::new();
    let mut in_test_block = false;
    let mut test_block_depth = 0;
    let mut in_safety_comment = false;

    let hoare_re = Regex::new(r"^\s*///\s*\{").unwrap();
    let unwrap_re = Regex::new(r"\b(unwrap\(\)|expect\s*\(|panic!\s*\()").unwrap();
    let false_positive_re =
        Regex::new(r"\b(unwrap_or\(|unwrap_or_else\(|unwrap_or_default\()").unwrap();
    let unsafe_re = Regex::new(r"\bunsafe\b").unwrap();
    let safety_comment_re = Regex::new(r"//\s*SAFETY:").unwrap();
    let pub_fn_re = Regex::new(r"^\s*(pub\s+)?(async\s+)?(unsafe\s+)?fn\s+").unwrap();
    let _pub_fn_with_doc_re =
        Regex::new(r"^\s*///\s*\{.*\n^\s*(pub\s+)?(async\s+)?(unsafe\s+)?fn\s+")
            .unwrap();

    // Two-pass: first find all pub fn and check Hoare triples
    let pub_fn_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| pub_fn_re.is_match(line))
        .map(|(i, _)| i)
        .collect();

    for &fn_idx in &pub_fn_indices {
        // Check if the function has a Hoare triple in the preceding doc comments
        let mut has_hoare = false;
        let mut i = fn_idx;
        while i > 0 {
            i -= 1;
            let line = lines[i];
            if line.trim().starts_with("///") {
                if hoare_re.is_match(line) {
                    has_hoare = true;
                    break;
                }
            } else if line.trim().is_empty() || line.trim().starts_with("#") {
                continue;
            } else {
                break;
            }
        }

        // Check if the function is actually pub (not just `fn`)
        let fn_line = lines[fn_idx];
        let is_pub = fn_line.contains("pub ");

        if is_pub && !has_hoare {
            issues.push(Issue {
                file: path.to_path_buf(),
                line: fn_idx + 1,
                message: format!("pub fn '{}' missing Hoare triple doc comment (/// {{ ... }})", extract_fn_name(fn_line)),
                severity: Severity::Major,
            });
        }
    }

    // Second pass: check unwraps, unsafes
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Track test blocks
        if trimmed.starts_with("#[cfg(test)]") {
            in_test_block = true;
            test_block_depth = 0;
            continue;
        }

        if in_test_block {
            test_block_depth += trimmed.matches('{').count() as i32;
            test_block_depth -= trimmed.matches('}').count() as i32;
            if test_block_depth <= 0 && trimmed == "}" {
                in_test_block = false;
            }
        }

        // Skip doc comments for unwrap counting
        if trimmed.starts_with("///") {
            continue;
        }

        // Track SAFETY comments
        if safety_comment_re.is_match(line) {
            in_safety_comment = true;
            continue;
        }

        // Check unwrap/expect/panic
        if unwrap_re.is_match(line) && !false_positive_re.is_match(line) {
            if !in_test_block && !in_safety_comment {
                issues.push(Issue {
                    file: path.to_path_buf(),
                    line: idx + 1,
                    message: format!(
                        "unwrap()/expect()/panic!() found outside tests or SAFETY block: {}",
                        trimmed.chars().take(60).collect::<String>()
                    ),
                    severity: Severity::Critical,
                });
            }
        }

        // Check unsafe without SAFETY
        if unsafe_re.is_match(line) && !line.contains("// SAFETY:") {
            // Allow `unsafe fn` declarations (they need SAFETY on impl, not declaration)
            if !trimmed.starts_with("unsafe fn") && !trimmed.starts_with("pub unsafe fn") {
                issues.push(Issue {
                    file: path.to_path_buf(),
                    line: idx + 1,
                    message: format!(
                        "unsafe block without // SAFETY: comment: {}",
                        trimmed.chars().take(60).collect::<String>()
                    ),
                    severity: Severity::Critical,
                });
            }
        }

        // Reset safety comment flag at end of block or next statement
        if trimmed.ends_with("}") || trimmed.ends_with(";") {
            in_safety_comment = false;
        }
    }

    // Filter by strictness
    issues.retain(|issue| config.strictness.contains(&issue.severity));

    Ok(FileReport {
        file: path.to_path_buf(),
        issues,
    })
}

fn extract_fn_name(line: &str) -> String {
    let parts: Vec<&str> = line.split_whitespace().collect();
    for (i, &part) in parts.iter().enumerate() {
        if part == "fn" && i + 1 < parts.len() {
            return parts[i + 1]
                .trim_start_matches("fn ")
                .trim_end_matches("(")
                .trim_end_matches("<")
                .to_string();
        }
    }
    "unknown".to_string()
}

pub fn print_reports(reports: &[FileReport]) {
    let mut total_issues = 0;
    let mut critical = 0;
    let mut major = 0;
    let mut minor = 0;
    let mut info = 0;

    for report in reports {
        println!("\n{}", report.file.display());
        for issue in &report.issues {
            let sev_str = match issue.severity {
                Severity::Critical => "CRITICAL",
                Severity::Major => "MAJOR",
                Severity::Minor => "MINOR",
                Severity::Info => "INFO",
            };
            println!("  [{}] L{}: {}", sev_str, issue.line, issue.message);
            total_issues += 1;
            match issue.severity {
                Severity::Critical => critical += 1,
                Severity::Major => major += 1,
                Severity::Minor => minor += 1,
                Severity::Info => info += 1,
            }
        }
    }

    if total_issues == 0 {
        println!("✅ All contracts satisfied.");
    } else {
        println!(
            "\nFound {} issues (CRITICAL: {}, MAJOR: {}, MINOR: {}, INFO: {})",
            total_issues, critical, major, minor, info
        );
    }
}
