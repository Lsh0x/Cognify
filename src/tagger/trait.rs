use crate::models::FileMeta;
use anyhow::Result;

/// Trait for file type handlers that can extract text, generate tags, and compute embeddings
#[async_trait::async_trait]
pub trait Taggable: Send + Sync {
    /// Extract text content from a file
    async fn extract_text(&self, file: &FileMeta) -> Result<String>;

    /// Generate tags from extracted text content
    async fn generate_tags(&self, content: &str) -> Result<Vec<String>>;

    /// Compute embedding vector from text content
    async fn compute_embedding(&self, content: &str) -> Result<Vec<f32>>;

    /// Check if this handler supports the given file extension
    fn supports_extension(&self, ext: &str) -> bool;
}

