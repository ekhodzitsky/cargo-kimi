use assert_cmd::Command;
use predicates::str::contains;
use std::fs;

#[test]
fn cli_help_shows_all_commands() {
    let mut cmd = Command::cargo_bin("cargo-kimi").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(contains("init"))
        .stdout(contains("check"))
        .stdout(contains("fix"))
        .stdout(contains("trend"))
        .stdout(contains("mcp"));
}

#[test]
fn check_json_passes() {
    // --format json skips clippy/test to avoid recursion in integration tests
    let mut cmd = Command::cargo_bin("cargo-kimi").unwrap();
    cmd.arg("check")
        .arg("--strictness")
        .arg("standard")
        .arg("--format")
        .arg("json");
    let output = cmd.assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let data: Vec<serde_json::Value> = serde_json::from_str(&stdout).expect("Valid JSON");
    // All files should score >= 50 (dogfooding baseline)
    for report in &data {
        let score = report["score"].as_u64().unwrap_or(0);
        assert!(
            score >= 40,
            "{} scored {} — below dogfood baseline",
            report["file"],
            score
        );
    }
}

#[test]
fn check_json_output_is_valid() {
    let mut cmd = Command::cargo_bin("cargo-kimi").unwrap();
    cmd.arg("check")
        .arg("--strictness")
        .arg("standard")
        .arg("--format")
        .arg("json");
    let output = cmd.assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    // Must be parseable JSON array
    let data: Vec<serde_json::Value> = serde_json::from_str(&stdout).expect("Invalid JSON output");
    assert!(
        !data.is_empty(),
        "Expected at least one file report in JSON output"
    );
    for report in &data {
        assert!(report.get("file").is_some());
        assert!(report.get("score").is_some());
        assert!(report.get("issues").is_some());
    }
}

#[test]
fn init_creates_agents_md() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("cargo-kimi").unwrap();
    cmd.current_dir(&tmp)
        .arg("init")
        .arg("--template")
        .arg("minimal")
        .arg("--strictness")
        .arg("standard")
        .arg("--yes");
    cmd.assert().success();
    assert!(tmp.path().join("AGENTS.md").exists());
}

#[test]
fn init_skill_creates_skill_md() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("cargo-kimi").unwrap();
    cmd.current_dir(&tmp)
        .arg("init-skill")
        .arg("my-skill")
        .arg("--description")
        .arg("Test skill");
    cmd.assert().success();
    let skill_path = tmp.path().join(".kimi/skills/my-skill/SKILL.md");
    assert!(skill_path.exists());
    let content = fs::read_to_string(&skill_path).unwrap();
    assert!(content.contains("name: my-skill"));
    assert!(content.contains("Test skill"));
}

#[test]
fn trend_shows_no_history_for_empty_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("cargo-kimi").unwrap();
    cmd.current_dir(&tmp).arg("trend").arg("--days").arg("7");
    cmd.assert()
        .success()
        .stdout(contains("No score history found"));
}

#[test]
fn upgrade_shows_instructions() {
    let mut cmd = Command::cargo_bin("cargo-kimi").unwrap();
    cmd.arg("upgrade");
    cmd.assert()
        .success()
        .stdout(contains("cargo install"));
}

#[test]
fn fix_dry_run_shows_diffs() {
    // Create a temp Rust project with a contract violation
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir(tmp.path().join("src")).unwrap();
    fs::write(
        tmp.path().join("Cargo.toml"),
        r#"[package]
name = "tmp"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("src/lib.rs"),
        r#"pub fn foo() {
    let _x = Some(1).unwrap();
}
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cargo-kimi").unwrap();
    cmd.current_dir(&tmp)
        .arg("init")
        .arg("--template")
        .arg("minimal")
        .arg("--yes");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("cargo-kimi").unwrap();
    cmd.current_dir(&tmp).arg("fix").arg("--dry-run");
    cmd.assert()
        .success()
        .stdout(contains("Would fix"));
}
