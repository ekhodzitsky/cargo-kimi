// kimi:score-ignore=unsafe,unwrap
use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

static FN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(pub\s+)?(async\s+)?(unsafe\s+)?fn\s+").unwrap());
static UNWRAP_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\.unwrap\(\)").unwrap());
static EXPECT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\.expect\("([^"]*)"\)"#).unwrap());

#[derive(Debug, Clone)]
enum Fix {
    InsertBefore { line: usize, text: String },
    ReplaceLine { line: usize, new_text: String },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ReturnType {
    Result,
    Option,
}

    /// { strictness is a valid strictness level string }
    /// pub fn run_fix(dry_run: bool, strictness: &str) -> anyhow::Result<()>
    /// { if dry_run, prints diffs; otherwise applies fixes to source files }
pub fn run_fix(dry_run: bool, strictness: &str) -> anyhow::Result<()> {
    let config = crate::contracts::CheckConfig::from_strictness(strictness)?;
    let paths = crate::workspace::find_workspace_crates()?;
    let reports = crate::contracts::check_files(&paths, &config)?;

    let mut total_fixed = 0usize;
    let mut files_fixed = 0usize;

    for report in reports {
        if report.issues.is_empty() {
            continue;
        }

        let bytes = match std::fs::read(&report.file) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Warning: could not read {}: {}", report.file.display(), e);
                continue;
            }
        };
        let content = match String::from_utf8(bytes) {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Warning: skipping non-UTF8 file: {}", report.file.display());
                continue;
            }
        };

        let fixes = compute_fixes(&report.file, &content, &report.issues)?;
        if fixes.is_empty() {
            continue;
        }

        let new_content = apply_fixes(&content, &fixes);

        if dry_run {
            println!("--- {}", report.file.display());
            println!("+++ {}", report.file.display());
            print_diff(&content, &new_content);
            println!();
        } else {
            let mut tmp_path = report.file.as_os_str().to_os_string();
            tmp_path.push(format!(".tmp{}", std::process::id()));
            let tmp_path = std::path::PathBuf::from(tmp_path);
            std::fs::write(&tmp_path, new_content)?;
            std::fs::rename(&tmp_path, &report.file)?;
        }

        total_fixed += fixes.len();
        files_fixed += 1;
    }

    if dry_run {
        println!("Would fix {} issues in {} files", total_fixed, files_fixed);
    } else {
        println!("Fixed {} issues in {} files", total_fixed, files_fixed);
    }

    Ok(())
}

fn compute_fixes(_path: &Path, content: &str, issues: &[crate::contracts::Issue]) -> anyhow::Result<Vec<Fix>> {
    let lines: Vec<&str> = content.lines().collect();
    let mut fixes = Vec::new();

    // Precompute whether each line is inside a #[cfg(test)] block.
    let mut in_test = vec![false; lines.len()];
    let mut current_test = false;
    let mut depth = 0i32;
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("#[cfg(test)]") {
            current_test = true;
            depth = 0;
        }
        if current_test {
            depth += trimmed.matches('{').count() as i32;
            depth -= trimmed.matches('}').count() as i32;
            if depth <= 0 && trimmed == "}" {
                current_test = false;
            }
        }
        in_test[i] = current_test;
    }

    for issue in issues {
        let line_idx = issue.line.saturating_sub(1);
        if line_idx >= lines.len() {
            continue;
        }

        match issue.category {
            crate::contracts::IssueCategory::MissingHoareTriple => {
                if let Some(fix) = fix_missing_hoare(&lines, line_idx) {
                    fixes.push(fix);
                }
            }
            crate::contracts::IssueCategory::UnsafeWithoutSafety => {
                if let Some(fix) = fix_missing_safety(&lines, line_idx) {
                    fixes.push(fix);
                }
            }
            crate::contracts::IssueCategory::UnwrapExpectPanic => {
                if in_test[line_idx] {
                    continue;
                }
                if let Some(fix) = fix_unwrap(&lines, line_idx) {
                    fixes.push(fix);
                }
            }
        }
    }

    Ok(fixes)
}

fn fix_missing_hoare(lines: &[&str], fn_line_idx: usize) -> Option<Fix> {
    // Find insertion point: before the first doc comment in the contiguous block,
    // or before the fn line itself.
    let mut insert_at = fn_line_idx;
    for j in (0..fn_line_idx).rev() {
        let t = lines[j].trim();
        if t.starts_with("///") {
            insert_at = j;
        } else if t.is_empty() || t.starts_with("#[") {
            continue;
        } else {
            break;
        }
    }

    let sig = extract_fn_signature(lines, fn_line_idx);
    let doc = format!(
        "/// {{ TODO: precondition }}\n/// {}\n/// {{ TODO: postcondition }}",
        sig
    );

    Some(Fix::InsertBefore {
        line: insert_at + 1, // 1-based
        text: doc,
    })
}

fn fix_missing_safety(lines: &[&str], unsafe_line_idx: usize) -> Option<Fix> {
    let indent = leading_indent(lines[unsafe_line_idx]);
    let comment = format!("{}// SAFETY: TODO: explain why this is safe", indent);
    Some(Fix::InsertBefore {
        line: unsafe_line_idx + 1,
        text: comment,
    })
}

fn fix_unwrap(lines: &[&str], line_idx: usize) -> Option<Fix> {
    let return_type = find_enclosing_return_type(lines, line_idx)?;
    let old_line = lines[line_idx];
    let new_line = apply_unwrap_fix(old_line, return_type);
    if new_line == old_line {
        return None;
    }
    Some(Fix::ReplaceLine {
        line: line_idx + 1,
        new_text: new_line.to_string(),
    })
}

fn extract_fn_signature(lines: &[&str], fn_line_idx: usize) -> String {
    let mut parts = Vec::new();
    for line in lines.iter().skip(fn_line_idx) {
        let line = line.trim();
        if let Some(pos) = line.find('{') {
            let before_brace = line[..pos].trim();
            if !before_brace.is_empty() {
                parts.push(before_brace);
            }
            break;
        } else {
            parts.push(line);
        }
    }
    parts
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn leading_indent(line: &str) -> String {
    line.chars()
        .take_while(|c| c.is_whitespace())
        .collect()
}

fn find_enclosing_return_type(lines: &[&str], line_idx: usize) -> Option<ReturnType> {
    for i in (0..=line_idx).rev() {
        let trimmed = lines[i].trim();
        if FN_RE.is_match(trimmed) {
            let mut sig = String::new();
            for line in lines.iter().skip(i) {
                sig.push_str(line);
                if line.contains('{') {
                    break;
                }
            }

            if let Some(arrow_pos) = sig.find("->") {
                let after = sig[arrow_pos + 2..].trim();
                if after.contains("Result<") {
                    return Some(ReturnType::Result);
                } else if after.contains("Option<") {
                    return Some(ReturnType::Option);
                }
            }
            return None;
        }
    }
    None
}

fn is_chained_unwrap(line: &str, m: &regex::Match) -> bool {
    let before = &line[..m.start()];
    before.trim_end().ends_with(')')
}

fn apply_unwrap_fix(line: &str, return_type: ReturnType) -> String {
    let mut result = line.to_string();

    match return_type {
        ReturnType::Result => {
            result = EXPECT_RE
                .replace_all(&result, |caps: &regex::Captures| {
                    let full_match = caps.get(0).expect("regex group 0 always exists");
                    if is_chained_unwrap(&result, &full_match) {
                        return caps[0].to_string();
                    }
                    let msg = &caps[1];
                    format!(
                        r#".map_err(|e| format!("{}: {{}}", e))?"#,
                        msg
                    )
                })
                .to_string();
            result = UNWRAP_RE
                .replace_all(&result, |caps: &regex::Captures| {
                    let full_match = caps.get(0).expect("regex group 0 always exists");
                    if is_chained_unwrap(&result, &full_match) {
                        return caps[0].to_string();
                    }
                    r#".ok_or("unwrap failed")?"#.to_string()
                })
                .to_string();
        }
        ReturnType::Option => {
            result = EXPECT_RE
                .replace_all(&result, |caps: &regex::Captures| {
                    let full_match = caps.get(0).expect("regex group 0 always exists");
                    if is_chained_unwrap(&result, &full_match) {
                        return caps[0].to_string();
                    }
                    let msg = &caps[1];
                    format!(r#".ok_or("{}")?"#, msg)
                })
                .to_string();
            result = UNWRAP_RE
                .replace_all(&result, |caps: &regex::Captures| {
                    let full_match = caps.get(0).expect("regex group 0 always exists");
                    if is_chained_unwrap(&result, &full_match) {
                        return caps[0].to_string();
                    }
                    "?".to_string()
                })
                .to_string();
        }
    }

    result
}

fn apply_fixes(content: &str, fixes: &[Fix]) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut new_lines: Vec<String> = Vec::new();

    let mut inserts: std::collections::BTreeMap<usize, Vec<String>> =
        std::collections::BTreeMap::new();
    let mut replaces: std::collections::BTreeMap<usize, String> =
        std::collections::BTreeMap::new();

    for fix in fixes {
        match fix {
            Fix::InsertBefore { line, text } => {
                inserts.entry(*line).or_default().push(text.clone());
            }
            Fix::ReplaceLine { line, new_text } => {
                replaces.insert(*line, new_text.clone());
            }
        }
    }

    for (i, line) in lines.iter().enumerate() {
        let line_num = i + 1;

        if let Some(texts) = inserts.get(&line_num) {
            for text in texts {
                for l in text.lines() {
                    new_lines.push(l.to_string());
                }
            }
        }

        if let Some(new_text) = replaces.get(&line_num) {
            new_lines.push(new_text.clone());
        } else {
            new_lines.push(line.to_string());
        }
    }

    let mut result = new_lines.join("\n");
    if content.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_fixes_insert_before() {
        let content = "a\nb\nc";
        let fixes = vec![Fix::InsertBefore {
            line: 2,
            text: "insert".to_string(),
        }];
        let result = apply_fixes(content, &fixes);
        assert_eq!(result, "a\ninsert\nb\nc");
    }

    #[test]
    fn apply_fixes_replace_line() {
        let content = "a\nb\nc";
        let fixes = vec![Fix::ReplaceLine {
            line: 2,
            new_text: "replaced".to_string(),
        }];
        let result = apply_fixes(content, &fixes);
        assert_eq!(result, "a\nreplaced\nc");
    }

    #[test]
    fn apply_fixes_multi_line_insert() {
        let content = "a\nb\nc";
        let fixes = vec![Fix::InsertBefore {
            line: 1,
            text: "x\ny".to_string(),
        }];
        let result = apply_fixes(content, &fixes);
        assert_eq!(result, "x\ny\na\nb\nc");
    }

    #[test]
    fn apply_fixes_preserve_trailing_newline() {
        let content = "a\nb\nc\n";
        let fixes = vec![];
        let result = apply_fixes(content, &fixes);
        assert!(result.ends_with('\n'));
    }

    #[test]
    fn leading_indent_spaces() {
        assert_eq!(leading_indent("    foo"), "    ");
    }

    #[test]
    fn leading_indent_tabs() {
        assert_eq!(leading_indent("\tfoo"), "\t");
    }

    #[test]
    fn leading_indent_none() {
        assert_eq!(leading_indent("foo"), "");
    }

    #[test]
    fn find_enclosing_return_type_result() {
        let lines = vec!["fn foo() -> Result<(), ()> {", "    let x = y.unwrap();"];
        assert_eq!(find_enclosing_return_type(&lines, 1), Some(ReturnType::Result));
    }

    #[test]
    fn find_enclosing_return_type_option() {
        let lines = vec!["fn foo() -> Option<i32> {", "    let x = y.unwrap();"];
        assert_eq!(find_enclosing_return_type(&lines, 1), Some(ReturnType::Option));
    }

    #[test]
    fn find_enclosing_return_type_none() {
        let lines = vec!["fn foo() {", "    let x = y.unwrap();"];
        assert_eq!(find_enclosing_return_type(&lines, 1), None);
    }

    #[test]
    fn find_enclosing_return_type_multiline_signature() {
        let lines = vec!["fn foo(", ") -> Result<(), ()> {", "    let x = y.unwrap();"];
        assert_eq!(find_enclosing_return_type(&lines, 2), Some(ReturnType::Result));
    }

    #[test]
    fn apply_unwrap_fix_result_unwrap() {
        let line = "let x = y.unwrap();";
        let fixed = apply_unwrap_fix(line, ReturnType::Result);
        assert_eq!(fixed, r#"let x = y.ok_or("unwrap failed")?;"#);
    }

    #[test]
    fn apply_unwrap_fix_result_expect() {
        let line = r#"let x = y.expect("msg");"#;
        let fixed = apply_unwrap_fix(line, ReturnType::Result);
        assert_eq!(fixed, r#"let x = y.map_err(|e| format!("msg: {}", e))?;"#);
    }

    #[test]
    fn apply_unwrap_fix_option_unwrap() {
        let line = "let x = y.unwrap();";
        let fixed = apply_unwrap_fix(line, ReturnType::Option);
        assert_eq!(fixed, "let x = y?;");
    }

    #[test]
    fn apply_unwrap_fix_option_expect() {
        let line = r#"let x = y.expect("msg");"#;
        let fixed = apply_unwrap_fix(line, ReturnType::Option);
        assert_eq!(fixed, r#"let x = y.ok_or("msg")?;"#);
    }

    #[test]
    fn apply_unwrap_fix_chained_unchanged() {
        let line = "let x = foo().unwrap();";
        let fixed_result = apply_unwrap_fix(line, ReturnType::Result);
        let fixed_option = apply_unwrap_fix(line, ReturnType::Option);
        assert_eq!(fixed_result, line);
        assert_eq!(fixed_option, line);
    }

    #[test]
    fn is_chained_unwrap_chained() {
        let line = "foo().unwrap();";
        let m = UNWRAP_RE.find(line).unwrap();
        assert!(is_chained_unwrap(line, &m));
    }

    #[test]
    fn is_chained_unwrap_non_chained() {
        let line = "foo.unwrap();";
        let m = UNWRAP_RE.find(line).unwrap();
        assert!(!is_chained_unwrap(line, &m));
    }
}

fn print_diff(old: &str, new: &str) {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let mut i = 0usize;
    let mut j = 0usize;

    while i < old_lines.len() || j < new_lines.len() {
        if i < old_lines.len()
            && j < new_lines.len()
            && old_lines[i] == new_lines[j]
        {
            println!(" {}", old_lines[i]);
            i += 1;
            j += 1;
        } else {
            let mut found = false;
            for offset in 1..=5 {
                if i + offset < old_lines.len()
                    && j < new_lines.len()
                    && old_lines[i + offset] == new_lines[j]
                {
                    for k in 0..offset {
                        println!("-{}", old_lines[i + k]);
                    }
                    i += offset;
                    found = true;
                    break;
                }
                if i < old_lines.len()
                    && j + offset < new_lines.len()
                    && old_lines[i] == new_lines[j + offset]
                {
                    for k in 0..offset {
                        println!("+{}", new_lines[j + k]);
                    }
                    j += offset;
                    found = true;
                    break;
                }
            }
            if !found {
                if i < old_lines.len() {
                    println!("-{}", old_lines[i]);
                    i += 1;
                }
                if j < new_lines.len() {
                    println!("+{}", new_lines[j]);
                    j += 1;
                }
            }
        }
    }
}
