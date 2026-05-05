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
#[allow(dead_code)]
pub struct KimiConfig {
    pub contracts: Option<ContractsConfig>,
    #[allow(dead_code)]
    pub score: Option<ScoreConfig>,
    pub output: Option<OutputConfig>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct ContractsConfig {
    pub strictness: Option<String>,
    #[serde(rename = "fail-on-drop")]
    #[allow(dead_code)]
    pub fail_on_drop: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct ScoreConfig {
    #[allow(dead_code)]
    pub ignore: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct OutputConfig {
    pub format: Option<String>,
}

impl KimiConfig {
#[allow(dead_code)]
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

    #[allow(dead_code)]
    /// { self.contracts.fail_on_drop is Some or None }
    /// pub fn fail_on_drop(&self) -> `Option<u32>`
    /// { returns the configured fail-on-drop threshold }
    pub fn fail_on_drop(&self) -> Option<u32> {
        self.contracts.as_ref()?.fail_on_drop
    }

    #[allow(dead_code)]
    /// { path is any string }
    /// pub fn should_ignore(&self, path: &str) -> bool
    /// { true if path matches any ignore pattern }
    pub fn should_ignore(&self, path: &str) -> bool {
        self.score
            .as_ref()
            .map(|s| s.ignore.iter().any(|pat| path.contains(pat)))
            .unwrap_or(false)
    }
}

#[allow(dead_code)]
pub struct Strictness(pub(crate) String);
