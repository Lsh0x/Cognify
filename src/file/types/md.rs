use crate::file::SemanticSource;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

/// Markdown file handler
pub struct MdFile {
    path: std::path::PathBuf,
    extension: Option<String>,
}

impl MdFile {
    pub fn new(path: std::path::PathBuf, extension: Option<String>) -> Self {
        Self { path, extension }
    }
}

#[async_trait]
impl SemanticSource for MdFile {
    async fn to_text_impl(&self) -> Result<String> {
        // Try to read as UTF-8 string
        match tokio::fs::read_to_string(&self.path).await {
            Ok(content) => Ok(content),
            Err(e) => {
                // If UTF-8 read fails, try to read as bytes and check if it's mostly binary
                match tokio::fs::read(&self.path).await {
                    Ok(bytes) => {
                        // Check if file is mostly text (contains mostly printable ASCII/UTF-8)
                        let printable_count = bytes.iter()
                            .filter(|&&b| (32..=126).contains(&b) || b == 9 || b == 10 || b == 13)
                            .count();
                        
                        if bytes.is_empty() {
                            Ok(String::new())
                        } else if printable_count * 100 / bytes.len() > 80 {
                            // Mostly printable, try UTF-8 conversion
                            String::from_utf8(bytes)
                                .map_err(|_| anyhow::anyhow!("File contains non-UTF-8 content"))
                                .with_context(|| format!("Failed to read markdown file: {}", self.path.display()))
                        } else {
                            // Mostly binary, return empty
                            Ok(String::new())
                        }
                    }
                    Err(_) => {
                        Err(e).with_context(|| format!("Failed to read markdown file: {}", self.path.display()))
                    }
                }
            }
        }
    }

    async fn to_metadata(&self) -> Result<Option<Value>> {
        Ok(None)
    }

    async fn generate_tags(&self, content: &str) -> Result<Vec<String>> {
        // Start with default implementation
        use crate::file::r#trait::generate_tags_default;
        let mut tags = generate_tags_default(self.path(), self.extension(), content);

        // Check for markdown-specific patterns
        let content_lower = content.to_lowercase();
        if content_lower.contains("#") {
            tags.push("documentation".to_string());
        }
        if content_lower.contains("```") {
            tags.push("code".to_string());
        }

        // Remove duplicates
        let mut seen = std::collections::HashSet::new();
        Ok(tags.into_iter()
            .filter(|tag| seen.insert(tag.clone()))
            .collect())
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn extension(&self) -> Option<&str> {
        self.extension.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_md_file_extraction() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        std::fs::write(&path, "# Hello\n\nWorld!").unwrap();

        let md_file = MdFile::new(path.clone(), Some("md".to_string()));
        let text = md_file.to_text().await.unwrap();
        assert_eq!(text, "# Hello\n\nWorld!");
        
        assert_eq!(md_file.path(), path);
        assert_eq!(md_file.extension(), Some("md"));
    }
}

