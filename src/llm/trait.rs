use anyhow::Result;

/// Trait for LLM providers that can generate tags from content
#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    /// Generate semantic tags from text content using LLM
    async fn generate_tags(&self, content: &str) -> Result<Vec<String>>;
}

