use crate::constants::LLM_KEYWORD_MAPPINGS;
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

    /// Construct from app Config (reads llm.model_path)
    pub fn from_config(config: &crate::config::Config) -> Self {
        let model_path = shellexpand::tilde(&config.llm.model_path).to_string();
        Self::new(model_path)
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
    async fn generate_tags(&self, content: &str, file_path: &std::path::Path) -> Result<Vec<String>> {
        // For now, use a simple prompt-based approach
        // In a real implementation, this would call guff/llama.cpp with proper FFI
        // This is a placeholder that can be extended
        
        if !self.model_exists() {
            anyhow::bail!("Model file not found: {}", self.model_path.display());
        }

        // Extract context from file path
        let filename = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        
        let parent_dirs: Vec<String> = file_path
            .ancestors()
            .skip(1)
            .take(3) // Take up to 3 parent directories for context
            .filter_map(|p| p.file_name())
            .filter_map(|n| n.to_str())
            .filter(|d| !is_common_directory(d))
            .map(|s| s.to_string())
            .collect();

        // Build a context-aware prompt
        let mut context_parts = Vec::new();
        
        if !parent_dirs.is_empty() {
            context_parts.push(format!("Location: {}", parent_dirs.join("/")));
        }
        
        context_parts.push(format!("Filename: {}", filename));
        
        let context = context_parts.join("\n");
        let content_preview = content.chars().take(1000).collect::<String>(); // Limit content length
        
        let _prompt = format!(
            "Analyze the following file and extract key topics and tags.\n\n\
Context:\n{}\n\n\
Content preview:\n{}\n\n\
Based on the file location, filename, and content, generate relevant tags (comma-separated, lowercase, no spaces). 
Focus on the main topics, purpose, or category:\n",
            context,
            content_preview
        );

        // Placeholder: In production, this would:
        // 1. Call guff binary or use llama.cpp FFI with the prompt
        // 2. Process the response
        // 3. Parse tags from LLM output
        
        // For now, return enhanced dictionary-based fallback that uses path context
        let mut tags = Vec::new();
        
        // Extract tags from path context
        use crate::organizer::context::extract_tags_from_path;
        let path_tags = extract_tags_from_path(file_path);
        tags.extend(path_tags);
        
        // Content-based keyword extraction using constants
        let keywords: std::collections::HashMap<&str, &str> = LLM_KEYWORD_MAPPINGS
            .iter()
            .cloned()
            .collect();

        let content_lower = content.to_lowercase();
        for (keyword, tag) in keywords.iter() {
            if content_lower.contains(keyword) && !tags.contains(&tag.to_string()) {
                tags.push(tag.to_string());
            }
        }

        // Extract from filename if not already tagged
        let filename_lower = filename.to_lowercase();
        for (keyword, tag) in keywords.iter() {
            if filename_lower.contains(keyword) && !tags.contains(&tag.to_string()) {
                tags.push(tag.to_string());
            }
        }

        // If no tags found, try to infer from extension
        if tags.is_empty() {
            if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                match ext.to_lowercase().as_str() {
                    "pdf" | "doc" | "docx" => tags.push("document".to_string()),
                    "jpg" | "jpeg" | "png" | "gif" => tags.push("image".to_string()),
                    "mp4" | "avi" => tags.push("video".to_string()),
                    _ => tags.push("document".to_string()),
                }
            } else {
                tags.push("document".to_string());
            }
        }

        Ok(tags)
    }
}

/// Check if a directory name is too common to be useful as context
fn is_common_directory(dir: &str) -> bool {
    use crate::constants::COMMON_DIRECTORY_NAMES;
    COMMON_DIRECTORY_NAMES.contains(&dir.to_lowercase().as_str())
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
        let file_path = std::path::Path::new("/home/user/meeting-notes.txt");
        
        let tags = provider.generate_tags(content, file_path).await.unwrap();
        
        assert!(tags.contains(&"task".to_string()));
        assert!(tags.contains(&"calendar".to_string()));
        assert!(tags.contains(&"issue".to_string()));
        assert!(tags.contains(&"enhancement".to_string()));
    }

    #[tokio::test]
    async fn test_local_llm_provider_generate_tags_with_path_context() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model.bin");
        fs::write(&model_path, b"dummy").unwrap();

        let provider = LocalLlmProvider::new(&model_path);
        let content = "Some content";
        // Path with meaningful context
        let file_path = std::path::Path::new("/home/user/Projects/Invoice-2024.pdf");
        
        let tags = provider.generate_tags(content, file_path).await.unwrap();
        
        // Should extract tags from path (invoice, projects, 2024)
        assert!(!tags.is_empty());
        // Should contain path-based tags
        assert!(tags.iter().any(|t| t.contains("invoice") || t.contains("project")));
    }

    #[tokio::test]
    async fn test_local_llm_provider_fallback_tag() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model.bin");
        fs::write(&model_path, b"dummy").unwrap();

        let provider = LocalLlmProvider::new(&model_path);
        let content = "Random content with no keywords";
        // Use a path that doesn't generate meaningful tags
        let file_path = std::path::Path::new("/tmp/abc123.xyz");
        
        let tags = provider.generate_tags(content, file_path).await.unwrap();
        
        // Should always return at least one tag
        assert!(!tags.is_empty());
        // Should either have path-based tags or fallback to "document"
        let has_document = tags.contains(&"document".to_string());
        let has_path_tags = tags.iter().any(|t| t != "document");
        assert!(has_document || has_path_tags, "Should have either document tag or path-based tags");
    }

    #[tokio::test]
    async fn test_local_llm_provider_model_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("nonexistent.bin");

        let provider = LocalLlmProvider::new(&model_path);
        assert!(!provider.model_exists());
        
        let file_path = std::path::Path::new("/tmp/test.txt");
        let result = provider.generate_tags("test", file_path).await;
        assert!(result.is_err());
    }
}

