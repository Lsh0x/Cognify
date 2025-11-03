use crate::llm::LlmProvider;
use anyhow::Result;
use std::path::PathBuf;

/// Local LLM provider using guff (or similar local LLM binary)
pub struct LocalLlmProvider {
    model_path: PathBuf,
    executable: String,
}

impl LocalLlmProvider {
    /// Create a new local LLM provider
    pub fn new<P: Into<PathBuf>>(model_path: P) -> Self {
        Self {
            model_path: model_path.into(),
            executable: "guff".to_string(),
        }
    }

    /// Set the LLM executable name (default: "guff")
    pub fn with_executable(mut self, executable: String) -> Self {
        self.executable = executable;
        self
    }

    /// Check if the model file exists
    pub fn model_exists(&self) -> bool {
        self.model_path.exists()
    }
}

#[async_trait::async_trait]
impl LlmProvider for LocalLlmProvider {
    async fn generate_tags(&self, content: &str) -> Result<Vec<String>> {
        // For now, use a simple prompt-based approach
        // In a real implementation, this would call guff/llama.cpp with proper FFI
        // This is a placeholder that can be extended
        
        if !self.model_exists() {
            anyhow::bail!("Model file not found: {}", self.model_path.display());
        }

        // Simple keyword extraction as placeholder
        // TODO: Replace with actual LLM call once FFI bindings are available
        let _prompt = format!(
            "Extract key topics and tags from the following text. Return only comma-separated tags:\n\n{}",
            content.chars().take(500).collect::<String>() // Limit content length
        );

        // Placeholder: In production, this would:
        // 1. Call guff binary or use llama.cpp FFI
        // 2. Process the response
        // 3. Parse tags from LLM output
        
        // For now, return dictionary-based fallback
        let keywords: std::collections::HashMap<&str, &str> = [
            ("todo", "task"),
            ("meeting", "calendar"),
            ("code", "programming"),
            ("bug", "issue"),
            ("feature", "enhancement"),
            ("api", "integration"),
        ]
        .iter()
        .cloned()
        .collect();

        let content_lower = content.to_lowercase();
        let mut tags = Vec::new();

        for (keyword, tag) in keywords.iter() {
            if content_lower.contains(keyword) && !tags.contains(&tag.to_string()) {
                tags.push(tag.to_string());
            }
        }

        // If no tags found, add a generic tag
        if tags.is_empty() {
            tags.push("document".to_string());
        }

        Ok(tags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_local_llm_provider_creation() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model.bin");
        
        // Create a dummy model file
        fs::write(&model_path, b"dummy").unwrap();

        let provider = LocalLlmProvider::new(&model_path);
        assert_eq!(provider.model_path, model_path);
        assert!(provider.model_exists());
    }

    #[tokio::test]
    async fn test_local_llm_provider_with_executable() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model.bin");
        fs::write(&model_path, b"dummy").unwrap();

        let provider = LocalLlmProvider::new(&model_path).with_executable("llama".to_string());
        assert_eq!(provider.executable, "llama");
    }

    #[tokio::test]
    async fn test_local_llm_provider_generate_tags() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model.bin");
        fs::write(&model_path, b"dummy").unwrap();

        let provider = LocalLlmProvider::new(&model_path);
        let content = "This is a TODO list for the meeting. We need to fix a bug and add a new feature.";
        
        let tags = provider.generate_tags(content).await.unwrap();
        
        assert!(tags.contains(&"task".to_string()));
        assert!(tags.contains(&"calendar".to_string()));
        assert!(tags.contains(&"issue".to_string()));
        assert!(tags.contains(&"enhancement".to_string()));
    }

    #[tokio::test]
    async fn test_local_llm_provider_fallback_tag() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model.bin");
        fs::write(&model_path, b"dummy").unwrap();

        let provider = LocalLlmProvider::new(&model_path);
        let content = "Random content with no keywords";
        
        let tags = provider.generate_tags(content).await.unwrap();
        
        assert!(tags.contains(&"document".to_string()));
    }

    #[tokio::test]
    async fn test_local_llm_provider_model_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("nonexistent.bin");

        let provider = LocalLlmProvider::new(&model_path);
        assert!(!provider.model_exists());
        
        let result = provider.generate_tags("test").await;
        assert!(result.is_err());
    }
}

