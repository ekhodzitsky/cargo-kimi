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

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SkillName(String);

impl SkillName {
#[allow(dead_code)]
    pub(crate) fn new(name: &str) -> anyhow::Result<Self> {
        if name.is_empty() {
            anyhow::bail!("Skill name cannot be empty");
        }
        if name.starts_with('.') || name.contains('/') || name.contains('\\') || name.contains("..")
        {
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
        if name.starts_with('-') || name.ends_with('-') {
            anyhow::bail!("Skill name must not start or end with a hyphen");
        }
        if name.contains("--") {
            anyhow::bail!("Skill name must not contain consecutive hyphens");
        }
        Ok(Self(name.to_string()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

fn to_title_case(s: &str) -> String {
    s.split('-')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// { name matches ^[a-z0-9-]+$ }
/// pub fn cmd_skill_init(name: &str, description: Option<&str>) -> anyhow::Result<()>
/// { creates .kimi/skills/{name}/SKILL.md with YAML frontmatter }
pub fn cmd_skill_init(name: &str, description: Option<&str>) -> anyhow::Result<()> {
    let skill_name = SkillName::new(name)?;
    let skill_dir = format!(".kimi/skills/{}", skill_name.as_str());
    fs::create_dir_all(&skill_dir)?;
    let skill_path = format!("{}/SKILL.md", skill_dir);
    if Path::new(&skill_path).exists() {
        anyhow::bail!("Skill '{}' already exists at {}", name, skill_path);
    }
    let desc = description.unwrap_or("TODO: describe what this skill does");
    let title = to_title_case(skill_name.as_str());
    let content = SKILL_TEMPLATE
        .replace("{name}", skill_name.as_str())
        .replace("{description}", desc)
        .replace("{title}", &title);
    fs::write(&skill_path, content)?;
    println!("✓ Created skill '{}' at {}", name, skill_path);
    println!("  Edit {} and run `cargo kimi check` to validate.", skill_path);
    Ok(())
}
