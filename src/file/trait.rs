use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use crate::organizer::context::extract_tags_from_path;

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

    /// Generate tags from file content and path
    /// Default implementation uses path-based tags and content keywords
    async fn generate_tags(&self, content: &str) -> Result<Vec<String>> {
        Ok(generate_tags_default(self.path(), self.extension(), content))
    }

    /// Get the file path
    fn path(&self) -> &std::path::Path;

    /// Get the file extension
    fn extension(&self) -> Option<&str>;
}

/// Default tag generation implementation
pub(crate) fn generate_tags_default(
    path: &std::path::Path,
    extension: Option<&str>,
    content: &str,
) -> Vec<String> {
    let mut tags = Vec::new();

    // Extract tags from path (filename, parent directories)
    let path_tags = extract_tags_from_path(path);
    tags.extend(path_tags);

    // Extract tags from content using keyword matching
    let content_tags = extract_tags_from_content(content);
    tags.extend(content_tags);

    // Add extension-based tag
    if let Some(ext) = extension {
        tags.push(ext.to_lowercase());
    }

    // Remove duplicates while preserving order
    let mut seen = std::collections::HashSet::new();
    let unique_tags: Vec<String> = tags.into_iter()
        .filter(|tag| seen.insert(tag.clone()))
        .collect();

    // If no tags found, add a generic tag based on extension
    if unique_tags.is_empty() {
        vec!["unknown".to_string()]
    } else {
        unique_tags
    }
}

/// Extract tags from content using keyword matching
pub(crate) fn extract_tags_from_content(content: &str) -> Vec<String> {
    let keywords: std::collections::HashMap<&str, &str> = [
        ("todo", "task"),
        ("meeting", "calendar"),
        ("notes", "documentation"),
        ("code", "programming"),
        ("bug", "issue"),
        ("feature", "enhancement"),
        ("test", "testing"),
        ("api", "integration"),
        ("config", "configuration"),
        ("readme", "documentation"),
        ("invoice", "finance"),
        ("receipt", "finance"),
        ("contract", "legal"),
        ("nda", "legal"),
        ("resume", "career"),
        ("cv", "career"),
    ]
    .iter()
    .cloned()
    .collect();

    let content_lower = content.to_lowercase();
    let mut tags = Vec::new();

    for (keyword, tag) in keywords.iter() {
        if content_lower.contains(keyword) {
            tags.push(tag.to_string());
        }
    }

    tags
}

