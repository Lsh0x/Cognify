use crate::models::FileMeta;
use crate::tagger::Taggable;
use anyhow::{Context, Result};
use std::fs;

/// Handler for Markdown files (.md)
pub struct MarkdownHandler;

impl MarkdownHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Taggable for MarkdownHandler {
    async fn extract_text(&self, file: &FileMeta) -> Result<String> {
        tokio::task::spawn_blocking({
            let path = file.path.clone();
            move || fs::read_to_string(&path)
        })
        .await?
        .with_context(|| format!("Failed to read markdown file: {}", file.path.display()))
    }

    async fn generate_tags(&self, content: &str) -> Result<Vec<String>> {
        // Simple dictionary-based tagging for markdown
        let keywords: std::collections::HashMap<&str, &str> = [
            ("#", "documentation"),
            ("```", "code"),
            ("todo", "task"),
            ("bug", "issue"),
            ("feature", "enhancement"),
            ("api", "integration"),
            ("readme", "documentation"),
            ("guide", "documentation"),
            ("tutorial", "documentation"),
        ]
        .iter()
        .cloned()
        .collect();

        let content_lower = content.to_lowercase();
        let mut tags = Vec::new();

        // Check for markdown-specific patterns
        if content_lower.contains("#") {
            tags.push("documentation".to_string());
        }

        if content_lower.contains("```") {
            tags.push("code".to_string());
        }

        for (keyword, tag) in keywords.iter() {
            if content_lower.contains(keyword) && !tags.contains(&tag.to_string()) {
                tags.push(tag.to_string());
            }
        }

        // If no tags found, add a generic "markdown" tag
        if tags.is_empty() {
            tags.push("markdown".to_string());
        }

        Ok(tags)
    }

    async fn compute_embedding(&self, _content: &str) -> Result<Vec<f32>> {
        // Placeholder: embeddings will be implemented in PR 5
        Ok(Vec::new())
    }

    fn supports_extension(&self, ext: &str) -> bool {
        ext == "md" || ext == "markdown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_markdown_handler_supports_md() {
        let handler = MarkdownHandler::new();
        assert!(handler.supports_extension("md"));
        assert!(handler.supports_extension("markdown"));
        assert!(!handler.supports_extension("txt"));
    }

    #[tokio::test]
    async fn test_markdown_handler_generate_tags() {
        let handler = MarkdownHandler::new();
        let content = "# Title\n\n```rust\ncode here\n```";
        let tags = handler.generate_tags(content).await.unwrap();
        
        assert!(tags.contains(&"documentation".to_string()));
        assert!(tags.contains(&"code".to_string()));
    }
}

