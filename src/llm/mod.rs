pub mod r#trait;

#[cfg(feature = "llm")]
pub mod local;

#[cfg(feature = "llm")]
pub use local::LocalLlmProvider;

#[cfg(not(feature = "llm"))]
pub struct LocalLlmProvider;

#[cfg(not(feature = "llm"))]
impl LocalLlmProvider {
    pub fn new<P: Into<std::path::PathBuf>>(_model_path: P) -> Self {
        Self
    }

    pub fn from_config(_config: &crate::config::Config) -> Self {
        Self
    }

    pub fn model_exists(&self) -> bool {
        false
    }
}

#[cfg(not(feature = "llm"))]
#[async_trait::async_trait]
impl LlmProvider for LocalLlmProvider {
    async fn generate_tags(&self, _content: &str, _file_path: &std::path::Path) -> anyhow::Result<Vec<String>> {
        anyhow::bail!("LLM feature is not enabled. Compile with --features llm to use LLM-based tagging.")
    }
}

pub use r#trait::LlmProvider;

