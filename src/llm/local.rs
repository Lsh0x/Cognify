use crate::constants::LLM_KEYWORD_MAPPINGS;
use crate::llm::LlmProvider;
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Local LLM provider using llama-cpp to run GGUF models
pub struct LocalLlmProvider {
    model_path: PathBuf,
}

impl LocalLlmProvider {
    /// Create a new local LLM provider
    pub fn new<P: Into<PathBuf>>(model_path: P) -> Self {
        Self {
            model_path: model_path.into(),
        }
    }

    /// Construct from app Config (loads llm.model_path)
    pub fn from_config(config: &crate::config::Config) -> Self {
        let model_path = shellexpand::tilde(&config.llm.model_path).to_string();
        Self::new(model_path)
    }

    /// Check if the model file exists
    pub fn model_exists(&self) -> bool {
        // Expand ~ in path for the existence check as well
        let expanded_path = shellexpand::tilde(self.model_path.to_string_lossy().as_ref()).to_string();
        PathBuf::from(expanded_path).exists()
    }

    /// Call the LLM with a prompt using llama-cpp and parse the response
    async fn call_llm(&self, prompt: &str) -> Result<Vec<String>> {
        // Expand ~ in path
        let expanded_path = shellexpand::tilde(self.model_path.to_string_lossy().as_ref()).to_string();
        let model_path = PathBuf::from(expanded_path);

        // Heavy CPU-bound inference: run in a blocking thread
        let prompt_owned = prompt.to_string();
        let output = tokio::task::spawn_blocking(move || -> Result<String> {
            use llama_cpp::{standard_sampler::StandardSampler, LlamaModel, LlamaParams, SessionParams};

            // Load model
            let model = LlamaModel::load_from_file(
                model_path,
                LlamaParams::default(),
            )
            .context("Failed to load GGUF model with llama-cpp")?;

            // Create a session with desired context size
            let mut session = model
                .create_session(SessionParams { n_ctx: 4096, ..Default::default() })
                .context("Failed to create llama session")?;

            // Feed prompt into context
            session
                .advance_context(&prompt_owned)
                .context("Failed to advance context with prompt")?;

            // Generate up to 100 tokens with default sampler
            let mut output = String::new();
            let handle = session
                .start_completing_with(StandardSampler::default(), 100)
                .context("Failed to start completion")?;
            let mut completions = handle.into_strings();

            for piece in &mut completions {
                output.push_str(&piece);
            }

            Ok(output)
        })
        .await
        .context("Join error running llama inference")??;

        self.parse_llm_response(&output)
    }

    /// Parse LLM response to extract tags
    /// Expected format: comma-separated list of tags (lowercase, no spaces)
    fn parse_llm_response(&self, response: &str) -> Result<Vec<String>> {
        // Clean up the response - remove extra whitespace, newlines, etc.
        let cleaned = response.trim();

        // Remove any markdown code blocks or other formatting
        let cleaned = cleaned
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        // Try to find the tag list - it should be comma-separated
        // The LLM might add some explanation, so we look for a line that looks like tags
        let mut tags = Vec::new();

        // Split by comma and process each tag
        for part in cleaned.split(',') {
            let tag = part.trim().to_lowercase();

            // Remove common prefixes/suffixes that LLM might add
            let tag = tag
                .trim_start_matches("tags:")
                .trim_start_matches("tag:")
                .trim_start_matches("output:")
                .trim_start_matches("result:")
                .trim_start_matches("the tags are:")
                .trim_start_matches("tags are:")
                .trim_matches(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '[' || c == ']' || c == '.' || c == ':' || c == '-');

            // Skip empty tags and very long ones (likely errors)
            if !tag.is_empty() && tag.len() < 50 {
                // Remove spaces and underscores, convert to lowercase
                let tag = tag.replace(' ', "").replace('_', "").to_lowercase();
                if !tag.is_empty() && !tags.contains(&tag) {
                    tags.push(tag);
                }
            }
        }

        // If we got some tags, return them
        if !tags.is_empty() {
            Ok(tags)
        } else {
            // Try to extract tags from lines that look like tag lists
            // Look for lines containing multiple lowercase words separated by commas
            for line in cleaned.lines() {
                let line = line.trim();
                // If line contains commas and looks like a tag list, try parsing it
                if line.contains(',') && line.len() < 500 {
                    let line_tags: Vec<String> = line
                        .split(',')
                        .map(|s| s.trim().to_lowercase().replace(' ', "").replace('_', ""))
                        .filter(|s| !s.is_empty() && s.len() < 30)
                        .collect();

                    if !line_tags.is_empty() {
                        return Ok(line_tags);
                    }
                }
            }

            // Last resort: extract any word-like tokens that could be tags
            let words: Vec<String> = cleaned
                .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| s.len() >= 2 && s.len() <= 30 && s.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-'))
                .collect();

            if !words.is_empty() {
                Ok(words)
            } else {
                anyhow::bail!("Could not parse tags from LLM response: {}", cleaned)
            }
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for LocalLlmProvider {
    async fn generate_tags(&self, content: &str, file_path: &std::path::Path) -> Result<Vec<String>> {
        if !self.model_exists() {
            anyhow::bail!("Model file not found: {}", self.model_path.display());
        }

        // Extract context from file path
        let filename = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");

        let parent_dirs: Vec<String> = file_path
            .ancestors()
            .skip(1)
            .take(3) // Take up to 3 parent directories for context
            .filter_map(|p| p.file_name())
            .filter_map(|n| n.to_str())
            .filter(|d| !is_common_directory(d))
            .map(|s| s.to_string())
            .collect();

        // Build context from parent directories
        let context = if !parent_dirs.is_empty() {
            parent_dirs.join("/")
        } else {
            "root".to_string()
        };

        // Limit content preview to reasonable length
        let content_preview = if content.len() > 2000 {
            format!("{}...", &content.chars().take(2000).collect::<String>())
        } else {
            content.to_string()
        };

        // Build the prompt with the new format
        let prompt = format!(
            r#"Analyze the following file and extract metadata.

Context:
{context}

File path:
{path}

Filename:
{filename}

Content preview:
{content_preview}

Using all available information — especially the content, but also the file name and its location — infer the document's main purpose, domain, and topics.

Generate a comma-separated list of relevant tags (all lowercase, no spaces).  

Tags should represent the document's subject, type, or intent — not superficial words or file extensions.

Focus on meaning and category, not syntax or formatting.

---

Rules:

1. Prefer semantic and domain-relevant tags (e.g. "finance", "health", "marketing", "api", "backend").

2. If it's code, include the language and purpose (e.g. "rust", "javascript", "config", "cli").

3. If it's documentation, focus on the role (e.g. "readme", "tutorial", "architecture", "report").

4. If it's a data/config file, tag by format and tool (e.g. "json", "yaml", "terraform", "docker").

5. If it's personal or generic content, infer intent (e.g. "recipe", "travel", "invoice", "note", "project").

6. If the content is unavailable or minimal, infer from file path and filename structure only.

7. Never include formatting-related words (like "txt", "md", "pdf") unless they convey meaning (e.g. "markdown_doc" is **not** allowed).

8. Return **only** the comma-separated list of tags as output, without explanation or extra text."#,
            context = context,
            path = file_path.to_string_lossy(),
            filename = filename,
            content_preview = content_preview
        );

        // Try to call the LLM (llama-cpp) to generate tags
        match self.call_llm(&prompt).await {
            Ok(tags) => {
                if !tags.is_empty() {
                    return Ok(tags);
                }
                // Empty response, fall back to dictionary
            }
            Err(e) => {
                // Log error but continue with fallback
                eprintln!("Warning: LLM call failed: {}. Falling back to dictionary-based tagging.", e);
            }
        }

        // Fallback to enhanced dictionary-based tagging if LLM fails or returns empty
        let mut tags = Vec::new();

        // Extract tags from path context
        use crate::organizer::context::extract_tags_from_path;
        let path_tags = extract_tags_from_path(file_path);
        tags.extend(path_tags);

        // Content-based keyword extraction using constants
        let keywords: std::collections::HashMap<&str, &str> = LLM_KEYWORD_MAPPINGS.iter().cloned().collect();

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
        assert!(provider.model_exists());
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

