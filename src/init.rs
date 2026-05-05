use std::fs;
use std::io::{self, Write};
use std::path::Path;

// Embedded AGENTS.md templates
const AGENTS_MINIMAL: &str = include_str!("../assets/templates/minimal/AGENTS.md");
const AGENTS_RUST_ONLY: &str = include_str!("../assets/templates/rust-only/AGENTS.md");
const AGENTS_FULL: &str = include_str!("../assets/templates/full/AGENTS.md");
const AGENTS_MODULAR: &str = include_str!("../assets/templates/modular/AGENTS.md");

// Embedded clippy configs
const CLIPPY_RELAXED: &str = include_str!("../assets/strictness/relaxed.toml");
const CLIPPY_STANDARD: &str = include_str!("../assets/strictness/standard.toml");
const CLIPPY_STRICT: &str = include_str!("../assets/strictness/strict.toml");

/// { template is a known template name, location is auto|root|.kimi }
/// pub fn cmd_init(template: &str, strictness: &str, location: &str, yes: bool) -> anyhow::Result<()>
/// { creates AGENTS.md and .cargo/config.toml in the current project }
pub fn cmd_init(
    template: &str,
    strictness: &str,
    location: &str,
    yes: bool,
) -> anyhow::Result<()> {
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
        if !buf.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Write AGENTS.md
    fs::write(target_path, agents)?;
    println!("✓ Created {} (template: {})", target_path, template);

    // For modular template, also write parts/ for easier updates
    if template == "modular" {
        if let Some(parts_dir) = Path::new(target_path).parent() {
            let parts_dir = parts_dir.join("parts");
            fs::create_dir_all(&parts_dir)?;
            if let Ok(parts) = split_agents_into_parts(agents) {
                for (name, content) in parts {
                    let part_path = parts_dir.join(format!("{}.md", name));
                    fs::write(&part_path, content)?;
                    println!("  ✓ Created part {}", part_path.display());
                }
            }
        }
    }

    // Write .cargo/config.toml for Rust projects
    if Path::new("Cargo.toml").exists() {
        fs::create_dir_all(".cargo")?;
        fs::write(".cargo/config.toml", clippy)?;
        println!("✓ Created .cargo/config.toml (strictness: {})", strictness);
    } else {
        println!("⚠ No Cargo.toml found — skipping .cargo/config.toml");
    }

    if Path::new("Cargo.toml").exists() {
        println!(
            "\nNext: git add {} .cargo/config.toml && git commit -m 'Add kimi-dotfiles guidelines'",
            target_path
        );
    } else {
        println!(
            "\nNext: git add {} && git commit -m 'Add kimi-dotfiles guidelines'",
            target_path
        );
    }
    Ok(())
}

fn split_agents_into_parts(agents: &str) -> anyhow::Result<Vec<(String, String)>> {
    let mut parts = Vec::new();
    let mut current_name = String::from("00-preamble");
    let mut current_content = String::new();

    for line in agents.lines() {
        if let Some(name) = line.strip_prefix("<!-- PART: ") {
            if let Some(name) = name.strip_suffix(" -->") {
                if !current_content.trim().is_empty() {
                    parts.push((current_name.clone(), current_content.trim().to_string() + "\n"));
                }
                current_name = name.to_string();
                current_content.clear();
                continue;
            }
        }
        current_content.push_str(line);
        current_content.push('\n');
    }
    if !current_content.trim().is_empty() {
        parts.push((current_name, current_content.trim().to_string() + "\n"));
    }
    Ok(parts)
}

fn resolve_agents(name: &str) -> anyhow::Result<&'static str> {
    match name {
        "minimal" => Ok(AGENTS_MINIMAL),
        "rust-only" => Ok(AGENTS_RUST_ONLY),
        "full" => Ok(AGENTS_FULL),
        "modular" => Ok(AGENTS_MODULAR),
        _ => anyhow::bail!(
            "Unknown template: '{}'. Available: minimal, rust-only, full, modular",
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
