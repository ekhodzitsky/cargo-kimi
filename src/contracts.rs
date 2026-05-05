// kimi:score-ignore=unsafe,unwrap
use regex::Regex;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static HOARE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*///\s*\{").unwrap());
static UNWRAP_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(unwrap\(\)|expect\s*\(|panic!\s*\()").unwrap());
static FALSE_POSITIVE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(unwrap_or\(|unwrap_or_else\(|unwrap_or_default\()").unwrap());
static UNSAFE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bunsafe\b").unwrap());
static SAFETY_COMMENT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"//\s*SAFETY:").unwrap());
static PUB_FN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*(pub\s+)?(async\s+)?(unsafe\s+)?fn\s+").unwrap());
static _PUB_FN_WITH_DOC_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*///\s*\{.*\n^\s*(pub\s+)?(async\s+)?(unsafe\s+)?fn\s+").unwrap());

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Severity {
    Critical,
    Major,
    Minor,
    Info,
}

impl Severity {
    /// { s is a non-empty severity string }
    /// pub fn from_str(s: &str) -> Option<Self>
    /// { result == Some(sev) iff s matches a known severity }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum IssueCategory {
    MissingHoareTriple,
    UnwrapExpectPanic,
    UnsafeWithoutSafety,
}

#[derive(Debug, Clone, Serialize)]
pub struct Issue {
    pub file: PathBuf,
    pub line: usize,
    pub message: String,
    pub severity: Severity,
    pub category: IssueCategory,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct FileReport {
    pub file: PathBuf,
    pub issues: Vec<Issue>,
    pub score: u32,
    #[serde(skip)]
    pub exemptions: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct CheckConfig {
    pub strictness: HashSet<Severity>,
}

impl CheckConfig {
    /// { level is one of "relaxed", "standard", "strict" }
    /// pub fn from_strictness(level: &str) -> anyhow::Result<Self>
    /// { result contains the severities matching the strictness level }
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

    /// { paths contains valid file or directory paths }
    /// pub fn check_files(paths: &[PathBuf], config: &CheckConfig) -> anyhow::Result<Vec<FileReport>>
    /// { result contains FileReport for every .rs file found, filtered by config.strictness }
pub fn check_files(paths: &[PathBuf], config: &CheckConfig) -> anyhow::Result<Vec<FileReport>> {
    let mut reports = Vec::new();
    let mut files = Vec::new();

    for path in paths {
        if path.is_file() && path.extension().map(|e| e == "rs").unwrap_or(false) {
            files.push(path.to_path_buf());
        } else if path.is_dir() {
            for entry_result in walkdir::WalkDir::new(path).follow_links(false) {
                let entry = match entry_result {
                    Ok(e) => e,
                    Err(err) => {
                        eprintln!("Warning: {}", err);
                        continue;
                    }
                };
                if entry.file_type().is_file()
                    && entry.path().extension().map(|ext| ext == "rs").unwrap_or(false)
                {
                    files.push(entry.path().to_path_buf());
                }
            }
        }
    }

    files.sort();
    files.dedup();

    for file in &files {
        let bytes = match std::fs::read(file) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Warning: could not read {}: {}", file.display(), e);
                continue;
            }
        };
        let content = match String::from_utf8(bytes) {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Warning: skipping non-UTF8 file: {}", file.display());
                continue;
            }
        };
        let report = check_file_contents(file, &content, config)?;
        reports.push(report);
    }

    Ok(reports)
}

fn parse_exemptions(content: &str) -> HashSet<String> {
    let mut exempt = HashSet::new();
    for line in content.lines().take(10) {
        let trimmed = line.trim();
        if let Some(pos) = trimmed.find("// kimi:score-ignore=") {
            let cats = trimmed[pos + 21..].split(',');
            for cat in cats {
                exempt.insert(cat.trim().to_lowercase());
            }
        }
    }
    exempt
}

fn check_file_contents(path: &Path, content: &str, config: &CheckConfig) -> anyhow::Result<FileReport> {
    let lines: Vec<&str> = content.lines().collect();
    let exemptions = parse_exemptions(content);

    let mut issues = Vec::new();
    let mut in_test_block = false;
    let mut test_block_depth = 0;
    let mut in_safety_comment = false;

    // Two-pass: first find all pub fn and check Hoare triples
    let pub_fn_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| PUB_FN_RE.is_match(line))
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
                if HOARE_RE.is_match(line) {
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
                category: IssueCategory::MissingHoareTriple,
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
        if SAFETY_COMMENT_RE.is_match(line) {
            in_safety_comment = true;
            continue;
        }

        // Check unwrap/expect/panic
        if UNWRAP_RE.is_match(line) && !FALSE_POSITIVE_RE.is_match(line) && !in_test_block && !in_safety_comment {
            issues.push(Issue {
                file: path.to_path_buf(),
                line: idx + 1,
                message: format!(
                    "unwrap()/expect()/panic!() found outside tests or SAFETY block: {}",
                    trimmed.chars().take(60).collect::<String>()
                ),
                severity: Severity::Critical,
                category: IssueCategory::UnwrapExpectPanic,
            });
        }

        // Check unsafe without SAFETY
        if !exemptions.contains("unsafe")
            && UNSAFE_RE.is_match(line)
            && !line.contains("// SAFETY:")
        {
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
                    category: IssueCategory::UnsafeWithoutSafety,
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

    let score = compute_score(content, &issues, &exemptions);

    Ok(FileReport {
        file: path.to_path_buf(),
        issues,
        score,
        exemptions,
    })
}

fn compute_score(content: &str, issues: &[Issue], exemptions: &HashSet<String>) -> u32 {
    let mut score = 100u32;

    // Hoare triples: 30 pts (binary — all pub fn must have triples)
    let has_missing_hoare = issues.iter().any(|i| {
        i.category == IssueCategory::MissingHoareTriple && !exemptions.contains("hoare")
    });
    if has_missing_hoare {
        score = score.saturating_sub(30);
    }

    // unwrap/expect/panic: 20 pts
    let has_unwrap = issues.iter().any(|i| {
        i.category == IssueCategory::UnwrapExpectPanic && !exemptions.contains("unwrap")
    });
    if has_unwrap {
        score = score.saturating_sub(20);
    }

    // Newtype: 10 pts (tuple struct with single field, e.g. `pub struct Foo(Bar)`)
    let has_newtype = content.lines().any(|l| {
        let t = l.trim();
        t.starts_with("pub struct ") && t.contains('(') && !t.contains("{")
    });
    if !has_newtype {
        score = score.saturating_sub(10);
    }

    // PhantomData: 10 pts
    if !content.contains("PhantomData") {
        score = score.saturating_sub(10);
    }

    // Typestate: 10 pts (heuristic: enum + impl block + From conversion)
    let has_typestate = content.contains("enum ") && content.contains("impl ") && content.contains("From<");
    if !has_typestate {
        score = score.saturating_sub(10);
    }

    // Avg function length ≤40 lines: 10 pts
    let lines: Vec<&str> = content.lines().collect();
    let mut fn_lengths = Vec::new();
    let mut in_fn = false;
    let mut brace_depth = 0i32;
    let mut fn_start = 0usize;
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if !in_fn
            && (trimmed.starts_with("fn ")
                || trimmed.starts_with("pub fn ")
                || trimmed.starts_with("async fn ")
                || trimmed.starts_with("pub async fn ")
                || trimmed.starts_with("unsafe fn ")
                || trimmed.starts_with("pub unsafe fn "))
        {
            in_fn = true;
            fn_start = idx;
            brace_depth = 0;
        }
        if in_fn {
            brace_depth += trimmed.matches('{').count() as i32;
            brace_depth -= trimmed.matches('}').count() as i32;
            if brace_depth == 0 && idx > fn_start {
                fn_lengths.push((idx - fn_start + 1) as u32);
                in_fn = false;
            }
        }
    }
    let avg_len = if !fn_lengths.is_empty() {
        fn_lengths.iter().sum::<u32>() / fn_lengths.len() as u32
    } else {
        0
    };
    if avg_len > 40 {
        score = score.saturating_sub(10);
    }

    // Result handling: 10 pts
    if !content.contains("Result<") {
        score = score.saturating_sub(10);
    }

    score
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

    /// { issue and exemptions are valid }
    /// pub fn is_exempt(issue: &Issue, exemptions: &HashSet<String>) -> bool
    /// { result == true iff issue.category is waived by exemptions }
pub fn is_exempt(issue: &Issue, exemptions: &HashSet<String>) -> bool {
    if exemptions.contains("hoare") && issue.category == IssueCategory::MissingHoareTriple {
        return true;
    }
    if exemptions.contains("unwrap") && issue.category == IssueCategory::UnwrapExpectPanic {
        return true;
    }
    if exemptions.contains("unsafe") && issue.category == IssueCategory::UnsafeWithoutSafety {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn check_config_from_strictness_relaxed() {
        let cfg = CheckConfig::from_strictness("relaxed").unwrap();
        assert!(cfg.strictness.contains(&Severity::Critical));
        assert_eq!(cfg.strictness.len(), 1);
    }

    #[test]
    fn check_config_from_strictness_standard() {
        let cfg = CheckConfig::from_strictness("standard").unwrap();
        assert!(cfg.strictness.contains(&Severity::Critical));
        assert!(cfg.strictness.contains(&Severity::Major));
        assert_eq!(cfg.strictness.len(), 2);
    }

    #[test]
    fn check_config_from_strictness_strict() {
        let cfg = CheckConfig::from_strictness("strict").unwrap();
        assert!(cfg.strictness.contains(&Severity::Critical));
        assert!(cfg.strictness.contains(&Severity::Major));
        assert!(cfg.strictness.contains(&Severity::Minor));
        assert!(cfg.strictness.contains(&Severity::Info));
        assert_eq!(cfg.strictness.len(), 4);
    }

    #[test]
    fn check_config_from_strictness_invalid() {
        assert!(CheckConfig::from_strictness("invalid").is_err());
    }

    #[test]
    fn parse_exemptions_valid_comment() {
        let content = "// kimi:score-ignore=hoare\nfn foo() {}";
        let ex = parse_exemptions(content);
        assert!(ex.contains("hoare"));
    }

    #[test]
    fn parse_exemptions_multiple_categories() {
        let content = "// kimi:score-ignore=hoare, unwrap ,UNSAFE\nfn foo() {}";
        let ex = parse_exemptions(content);
        assert!(ex.contains("hoare"));
        assert!(ex.contains("unwrap"));
        assert!(ex.contains("unsafe"));
    }

    #[test]
    fn parse_exemptions_after_line_10_ignored() {
        let mut lines: Vec<String> = (0..10).map(|i| format!("line {}", i)).collect();
        lines.push("// kimi:score-ignore=hoare".to_string());
        let content = lines.join("\n");
        let ex = parse_exemptions(&content);
        assert!(!ex.contains("hoare"));
    }

    #[test]
    fn parse_exemptions_no_comment() {
        let content = "fn foo() {}";
        let ex = parse_exemptions(content);
        assert!(ex.is_empty());
    }

    fn dummy_issue(message: &str, category: IssueCategory) -> Issue {
        Issue {
            file: PathBuf::new(),
            line: 1,
            message: message.to_string(),
            severity: Severity::Major,
            category,
        }
    }

    #[test]
    fn compute_score_perfect_file() {
        let content = r#"pub struct Foo(Bar);
use std::marker::PhantomData;
enum State {}
impl State {}
impl From<i32> for State {}
fn f() -> Result<(), ()> {}
"#;
        let issues: Vec<Issue> = vec![];
        let exemptions = HashSet::new();
        assert_eq!(compute_score(content, &issues, &exemptions), 100);
    }

    #[test]
    fn compute_score_missing_hoare() {
        let content = r#"pub struct Foo(Bar);
use std::marker::PhantomData;
enum State {}
impl State {}
impl From<i32> for State {}
fn f() -> Result<(), ()> {}
"#;
        let issues = vec![dummy_issue("pub fn 'foo' missing Hoare triple doc comment", IssueCategory::MissingHoareTriple)];
        let exemptions = HashSet::new();
        assert_eq!(compute_score(content, &issues, &exemptions), 70);
    }

    #[test]
    fn compute_score_missing_hoare_exempt() {
        let content = r#"pub struct Foo(Bar);
use std::marker::PhantomData;
enum State {}
impl State {}
impl From<i32> for State {}
fn f() -> Result<(), ()> {}
"#;
        let issues = vec![dummy_issue("pub fn 'foo' missing Hoare triple doc comment", IssueCategory::MissingHoareTriple)];
        let mut exemptions = HashSet::new();
        exemptions.insert("hoare".to_string());
        assert_eq!(compute_score(content, &issues, &exemptions), 100);
    }

    #[test]
    fn compute_score_all_deductions() {
        let mut lines = vec!["fn long_fn() {"];
        for i in 1..=50 {
            lines.push(&*Box::leak(format!("{}", i).into_boxed_str()));
        }
        lines.push("}");
        let content = lines.join("\n");
        let issues = vec![
            dummy_issue("pub fn 'foo' missing Hoare triple doc comment", IssueCategory::MissingHoareTriple),
            dummy_issue("unwrap()/expect()/panic!() found outside tests", IssueCategory::UnwrapExpectPanic),
        ];
        let exemptions = HashSet::new();
        assert_eq!(compute_score(&content, &issues, &exemptions), 0);
    }

    #[test]
    fn extract_fn_name_various_signatures() {
        assert_eq!(extract_fn_name("pub fn foo("), "foo");
        assert_eq!(extract_fn_name("async fn bar<("), "bar");
        assert_eq!(extract_fn_name("fn baz("), "baz");
        assert_eq!(extract_fn_name("pub unsafe fn qux("), "qux");
    }

    #[test]
    fn is_exempt_all_categories() {
        let hoare_issue = dummy_issue("missing Hoare triple", IssueCategory::MissingHoareTriple);
        let unwrap_issue = dummy_issue("unwrap()/expect()/panic!()", IssueCategory::UnwrapExpectPanic);
        let unsafe_issue = dummy_issue("unsafe block without", IssueCategory::UnsafeWithoutSafety);
        let mismatch_issue = dummy_issue("something else", IssueCategory::UnwrapExpectPanic);

        let mut ex = HashSet::new();
        ex.insert("hoare".to_string());
        assert!(is_exempt(&hoare_issue, &ex));
        assert!(!is_exempt(&unwrap_issue, &ex));

        ex.clear();
        ex.insert("unwrap".to_string());
        assert!(is_exempt(&unwrap_issue, &ex));

        ex.clear();
        ex.insert("unsafe".to_string());
        assert!(is_exempt(&unsafe_issue, &ex));

        ex.clear();
        ex.insert("hoare".to_string());
        assert!(!is_exempt(&mismatch_issue, &ex));
    }
}

    /// { reports are valid check results }
    /// pub fn print_reports(reports: &[FileReport])
    /// { prints per-file issues, scores, and project average to stdout }
pub fn print_reports(reports: &[FileReport]) {
    let mut total_issues = 0;
    let mut critical = 0;
    let mut major = 0;
    let mut minor = 0;
    let mut info = 0;
    let mut total_score = 0u32;

    for report in reports {
        println!("\n{} (score: {})", report.file.display(), report.score);
        for issue in &report.issues {
            let sev_str = match issue.severity {
                Severity::Critical => "CRITICAL",
                Severity::Major => "MAJOR",
                Severity::Minor => "MINOR",
                Severity::Info => "INFO",
            };
            let exempt_tag = if is_exempt(issue, &report.exemptions) { " [EXEMPT]" } else { "" };
            println!("  [{}] L{}: {}{}", sev_str, issue.line, issue.message, exempt_tag);
            total_issues += 1;
            match issue.severity {
                Severity::Critical => critical += 1,
                Severity::Major => major += 1,
                Severity::Minor => minor += 1,
                Severity::Info => info += 1,
            }
        }
        total_score += report.score;
    }

    if total_issues == 0 {
        println!("✅ All contracts satisfied.");
    } else {
        println!(
            "\nFound {} issues (CRITICAL: {}, MAJOR: {}, MINOR: {}, INFO: {})",
            total_issues, critical, major, minor, info
        );
    }

    if !reports.is_empty() {
        let avg = total_score / reports.len() as u32;
        println!("Average score: {}/100", avg);
    }
}
