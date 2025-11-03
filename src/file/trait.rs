use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

/// Trait for extracting semantic content from files
#[async_trait]
pub trait SemanticSource: Send + Sync {
    /// Extract text content from the file (internal implementation)
    async fn to_text_impl(&self) -> Result<String>;

    /// Extract text content from the file (public API with size check)
    async fn to_text(&self) -> Result<String> {
        // Check file size first - if empty, return empty string
        match tokio::fs::metadata(self.path()).await {
            Ok(metadata) => {
                if metadata.len() == 0 {
                    return Ok(String::new());
                }
            }
            Err(_) => {
                // Can't get metadata, try to read anyway
            }
        }
        
        // Call the implementation
        self.to_text_impl().await
    }

    /// Extract metadata specific to the file type (EXIF, PDF metadata, etc.)
    async fn to_metadata(&self) -> Result<Option<Value>> {
        Ok(None)
    }

    /// Get the file path
    fn path(&self) -> &std::path::Path;

    /// Get the file extension
    fn extension(&self) -> Option<&str>;
}

