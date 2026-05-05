use serde::Deserialize;
use std::path::Path;

/// { path points to a valid TOML file or directory containing .kimi.toml }
/// pub fn load_config(path: Option<&Path>) -> anyhow::Result<`KimiConfig`>
/// { returns parsed config or default if no file exists }
pub fn load_config(path: Option<&Path>) -> anyhow::Result<KimiConfig> {
    let config_path = if let Some(p) = path {
        p.to_path_buf()
    } else if Path::new(".kimi.toml").exists() {
        Path::new(".kimi.toml").to_path_buf()
    } else if Path::new("kimi.toml").exists() {
        Path::new("kimi.toml").to_path_buf()
    } else {
        return Ok(KimiConfig::default());
    };

    let content = std::fs::read_to_string(&config_path)?;
    let config: KimiConfig = toml::from_str(&content)?;
    Ok(config)
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct KimiConfig {
    pub contracts: Option<ContractsConfig>,
    pub score: Option<ScoreConfig>,
    pub output: Option<OutputConfig>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ContractsConfig {
    pub strictness: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ScoreConfig {
    #[serde(default)]
    pub ignore: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OutputConfig {
    pub format: Option<String>,
}

impl KimiConfig {
    /// { self.contracts.strictness is Some or None }
    /// pub fn strictness(&self) -> Option<&str>
    /// { returns the configured strictness level }
    pub fn strictness(&self) -> Option<&str> {
        self.contracts.as_ref()?.strictness.as_deref()
    }

    /// { self.output.format is Some or None }
    /// pub fn output_format(&self) -> Option<&str>
    /// { returns the configured output format }
    pub fn output_format(&self) -> Option<&str> {
        self.output.as_ref()?.format.as_deref()
    }

    /// { path is any file path string }
    /// pub fn should_ignore(&self, path: &str) -> bool
    /// { true if any path component equals a pattern or path starts with a pattern }
    pub fn should_ignore(&self, path: &str) -> bool {
        self.score
            .as_ref()
            .map(|s| {
                s.ignore.iter().any(|pat| {
                    // Check if the path starts with the pattern (prefix match)
                    path.starts_with(pat)
                        // Or if any path component exactly equals the pattern
                        || Path::new(path)
                            .components()
                            .any(|c| c.as_os_str() == pat.as_str())
                })
            })
            .unwrap_or(false)
    }

    /// { self.score.ignore contains zero or more patterns }
    /// pub fn ignore_patterns(&self) -> &[String]
    /// { returns the configured ignore patterns slice }
    pub fn ignore_patterns(&self) -> &[String] {
        self.score
            .as_ref()
            .map(|s| s.ignore.as_slice())
            .unwrap_or(&[])
    }
}
