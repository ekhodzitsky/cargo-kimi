// kimi:score-ignore=unwrap
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct Metadata {
    workspace_members: Vec<String>,
    packages: Vec<Package>,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    #[allow(dead_code)]
    manifest_path: String,
    targets: Vec<Target>,
}

#[derive(Debug, Deserialize)]
struct Target {
    kind: Vec<String>,
    src_path: String,
}

/// Extracts the package name from a workspace member string.
/// Handles modern URI format `path+file:///foo/bar#name@version` by extracting
/// the name between `#` and `@`. Falls back to the legacy `name version (path)` format.
fn parse_member_name(member: &str) -> String {
    // Modern format: path+file:///foo/bar#my-crate@1.0.0
    if let Some(hash_pos) = member.rfind('#') {
        let after_hash = &member[hash_pos + 1..];
        if let Some(at_pos) = after_hash.find('@') {
            return after_hash[..at_pos].to_string();
        }
        return after_hash.to_string();
    }
    // Legacy format: name 0.1.0 (path+file:///...)
    member.split(' ').next().unwrap_or("").to_string()
}

    /// { current directory is inside a Cargo workspace or package }
    /// pub fn find_workspace_crates() -> `anyhow::Result<Vec<PathBuf>>`
    /// { result contains src/ directories for all workspace members }
pub fn find_workspace_crates() -> anyhow::Result<Vec<PathBuf>> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to run cargo metadata");
    }

    let metadata: Metadata = serde_json::from_slice(&output.stdout)?;
    let member_names: std::collections::HashSet<_> = metadata
        .workspace_members
        .iter()
        .map(|m| parse_member_name(m))
        .collect();

    let mut src_dirs = Vec::new();
    for pkg in metadata.packages {
        if member_names.contains(&pkg.name) {
            // Find the lib or bin target
            for target in &pkg.targets {
                if target.kind.iter().any(|k| k == "lib" || k == "bin") {
                    let src_path = PathBuf::from(&target.src_path);
                    if let Some(parent) = src_path.parent() {
                        src_dirs.push(parent.to_path_buf());
                        break;
                    }
                }
            }
        }
    }

    if src_dirs.is_empty() {
        src_dirs.push(PathBuf::from("src"));
    }

    Ok(src_dirs)
}
#[allow(dead_code)]
pub struct CratePath(pub(crate) std::path::PathBuf);
