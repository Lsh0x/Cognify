use crate::models::FileMeta;
use anyhow::Result;

/// Trait for text extractors that can extract text content from various file formats
#[async_trait::async_trait]
pub trait TextExtractor: Send + Sync {
    /// Extract text content from a file
    async fn extract(&self, file: &FileMeta) -> Result<String>;
    
    /// Check if this extractor supports the given file extension
    fn supports_extension(&self, ext: &str) -> bool;
}

