use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Application configuration loaded from settings.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub meilisearch: MeilisearchConfig,
    pub ollama: OllamaConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub organizer: OrganizerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeilisearchConfig {
    pub url: String,
    #[serde(default)]
    pub api_key: Option<String>,
    pub index_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub url: String,
    pub model: String,
    #[serde(default = "default_embedding_dims")]
    pub dims: usize,
}

fn default_embedding_dims() -> usize {
    768 // Default to nomic-embed-text dimension
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmConfig {
    pub provider: String,
    pub model_path: String,
    pub executable: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrganizerConfig {
    #[serde(default)]
    pub skip_confirmation: bool,
    #[serde(default)]
    pub dry_run_default: bool,
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;
        
        let config: Config = toml::from_str(&content)
            .context("Failed to parse config file")?;

        Ok(config)
    }

    /// Load configuration from default location or return defaults
    pub fn load() -> Result<Self> {
        // Try default config locations
        let default_paths = [
            PathBuf::from("config/settings.toml"),
            PathBuf::from("./config/settings.toml"),
            PathBuf::from("~/.config/cognifs/settings.toml"),
        ];

        for path in &default_paths {
            if path.exists() {
                return Self::from_file(path);
            }
        }

        // Return defaults if no config found
        Ok(Self::default())
    }

    /// Get Meilisearch API key from config or environment variable
    pub fn meilisearch_api_key(&self) -> Option<String> {
        self.meilisearch.api_key
            .clone()
            .or_else(|| std::env::var("MEILI_MASTER_KEY").ok())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            meilisearch: MeilisearchConfig {
                url: "http://127.0.0.1:7700".to_string(),
                api_key: None,
                index_name: "cognifs".to_string(),
            },
            ollama: OllamaConfig {
                url: "http://127.0.0.1:11434".to_string(),
                model: "nomic-embed-text".to_string(),
                dims: 768,
            },
            llm: LlmConfig {
                provider: "local".to_string(),
                model_path: "~/.local/share/models/guff/model.bin".to_string(),
                executable: "guff".to_string(),
            },
            organizer: OrganizerConfig {
                skip_confirmation: false,
                dry_run_default: false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.meilisearch.url, "http://127.0.0.1:7700");
        assert_eq!(config.ollama.url, "http://127.0.0.1:11434");
        assert_eq!(config.ollama.model, "nomic-embed-text");
    }

    #[test]
    fn test_config_from_file() {
        let temp_file = std::env::temp_dir().join("test_config.toml");
        std::fs::write(
            &temp_file,
            r#"
[meilisearch]
url = "http://localhost:7700"
index_name = "test"

[ollama]
url = "http://localhost:11434"
model = "mxbai-embed-large"
dims = 1024
"#,
        )
        .unwrap();

        let config = Config::from_file(&temp_file).unwrap();
        assert_eq!(config.meilisearch.url, "http://localhost:7700");
        assert_eq!(config.ollama.model, "mxbai-embed-large");
    }
}

