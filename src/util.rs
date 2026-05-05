use std::path::{Path, PathBuf};

/// { path does not contain .. components }
/// pub fn validate_project_path(path: &Path) -> `anyhow::Result<PathBuf>`
/// { returns the canonicalized absolute path, or an error if it escapes cwd }
pub fn validate_project_path(path: &Path) -> anyhow::Result<PathBuf> {
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        anyhow::bail!("Path cannot contain parent directory references (..)");
    }
    let cwd = std::env::current_dir()?.canonicalize()?;
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    let mut normalized = PathBuf::new();
    for comp in abs.components() {
        if !matches!(comp, std::path::Component::CurDir) {
            normalized.push(comp);
        }
    }
    if !normalized.starts_with(&cwd) {
        anyhow::bail!("Path must be inside the project directory");
    }
    Ok(normalized)
}
