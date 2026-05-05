use std::path::{Path, PathBuf};

/// { path does not contain .. components }
/// pub fn validate_project_path(path: &Path) -> `anyhow::Result<PathBuf>`
/// { returns the canonicalized absolute path, or an error if it escapes cwd }
pub fn validate_project_path(path: &Path) -> anyhow::Result<PathBuf> {
    if path.as_os_str().as_encoded_bytes().contains(&b'\0') {
        anyhow::bail!("Path must not contain null bytes");
    }
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
    let resolved = canonicalize_existing_prefix(&normalized);
    if !resolved.starts_with(&cwd) {
        anyhow::bail!("Path must be inside the project directory");
    }
    Ok(resolved)
}

fn canonicalize_existing_prefix(path: &Path) -> PathBuf {
    let mut existing = path.to_path_buf();
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    loop {
        if let Ok(canon) = existing.canonicalize() {
            let mut result = canon;
            for segment in tail.iter().rev() {
                result.push(segment);
            }
            return result;
        }
        match existing.file_name() {
            Some(name) => {
                tail.push(name.to_owned());
                existing.pop();
            }
            None => break,
        }
    }
    path.to_path_buf()
}
#[allow(dead_code)]
pub struct ProjectPath(pub(crate) std::path::PathBuf);
