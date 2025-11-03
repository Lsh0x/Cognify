use crate::models::FileMeta;
use crate::tagger::Taggable;
use anyhow::{Context, Result};
use std::fs;

/// Handler for plain text files (.txt)
pub struct TextHandler;

impl TextHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Taggable for TextHandler {
    async fn extract_text(&self, file: &FileMeta) -> Result<String> {
        tokio::task::spawn_blocking({
            let path = file.path.clone();
            move || fs::read_to_string(&path)
        })
        .await?
        .with_context(|| format!("Failed to read text file: {}", file.path.display()))
    }

    async fn generate_tags(&self, content: &str) -> Result<Vec<String>> {
        // Simple dictionary-based tagging
        // Keywords for common topics
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

        // If no tags found, add a generic "text" tag
        if tags.is_empty() {
            tags.push("text".to_string());
        }

        Ok(tags)
    }

    async fn compute_embedding(&self, _content: &str) -> Result<Vec<f32>> {
        // Placeholder: embeddings will be implemented in PR 5
        Ok(Vec::new())
    }

    fn supports_extension(&self, ext: &str) -> bool {
        ext == "txt"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_text_handler_supports_txt() {
        let handler = TextHandler::new();
        assert!(handler.supports_extension("txt"));
        assert!(!handler.supports_extension("md"));
    }

    #[tokio::test]
    async fn test_text_handler_generate_tags() {
        let handler = TextHandler::new();
        let content = "This is a TODO list for the meeting";
        let tags = handler.generate_tags(content).await.unwrap();
        
        assert!(tags.contains(&"task".to_string()));
        assert!(tags.contains(&"calendar".to_string()));
    }
}

