use std::fs;
use std::path::Path;

const SKILL_TEMPLATE: &str = r#"---
name: {name}
version: 1.0.0
description: |
  {description}
---

# {title}

## When to Use

Describe the scenarios where this skill should be used.

## Steps

1. First, do this
2. Then, do that

## Examples

```rust
// Example code
```
"#;

/// { name matches ^[a-z0-9-]+$ }
/// pub fn cmd_skill_init(name: &str, description: Option<&str>) -> anyhow::Result<()>
/// { creates .kimi/skills/{name}/SKILL.md with YAML frontmatter }
pub fn cmd_skill_init(name: &str, description: Option<&str>) -> anyhow::Result<()> {
    if name.is_empty() {
        anyhow::bail!("Skill name cannot be empty");
    }
    if name.starts_with('.') || name.contains('/') || name.contains('\\') || name.contains("..") {
        anyhow::bail!(
            "Skill name cannot contain path separators, parent directory references, or leading dots"
        );
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        anyhow::bail!("Skill name must match ^[a-z0-9-]+$");
    }
    let skill_dir = format!(".kimi/skills/{}", name);
    fs::create_dir_all(&skill_dir)?;
    let skill_path = format!("{}/SKILL.md", skill_dir);
    if Path::new(&skill_path).exists() {
        anyhow::bail!("Skill '{}' already exists at {}", name, skill_path);
    }
    let desc = description.unwrap_or("TODO: describe what this skill does");
    let title = name
        .split('-')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    let content = SKILL_TEMPLATE
        .replace("{name}", name)
        .replace("{description}", desc)
        .replace("{title}", &title);
    fs::write(&skill_path, content)?;
    println!("✓ Created skill '{}' at {}", name, skill_path);
    println!("  Edit {} and run `cargo kimi check` to validate.", skill_path);
    Ok(())
}
