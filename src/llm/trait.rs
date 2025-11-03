use anyhow::Result;
use std::path::Path;

/// Trait for LLM providers that can generate tags from content
#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    /// Generate semantic tags from text content and file path using LLM
    /// The file_path provides context (filename, directory structure) that helps
    /// generate more accurate and relevant tags
    async fn generate_tags(&self, content: &str, file_path: &Path) -> Result<Vec<String>>;
}

