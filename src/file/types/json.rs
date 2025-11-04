use crate::file::SemanticSource;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

/// JSON file handler
pub struct JsonFile {
    path: std::path::PathBuf,
    extension: Option<String>,
}

impl JsonFile {
    pub fn new(path: std::path::PathBuf, extension: Option<String>) -> Self {
        Self { path, extension }
    }
}

#[async_trait]
impl SemanticSource for JsonFile {
    async fn to_text_impl(&self) -> Result<String> {
        // Read file content
        let content = tokio::fs::read_to_string(&self.path)
            .await
            .with_context(|| format!("Failed to read JSON file: {}", self.path.display()))?;

        // Parse JSON to validate and pretty-print
        let parsed: Value = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON file: {}", self.path.display()))?;

        // Pretty-print with 2-space indentation for better readability
        serde_json::to_string_pretty(&parsed)
            .with_context(|| format!("Failed to serialize JSON: {}", self.path.display()))
    }

    async fn to_metadata(&self) -> Result<Option<Value>> {
        // Read and parse JSON
        let content = tokio::fs::read_to_string(&self.path)
            .await
            .with_context(|| format!("Failed to read JSON file: {}", self.path.display()))?;

        let parsed: Value = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON file: {}", self.path.display()))?;

        // Return the parsed JSON as metadata
        // This allows searching/indexing of JSON structure
        Ok(Some(parsed))
    }

    async fn generate_tags(&self, content: &str) -> Result<Vec<String>> {
        // Start with default implementation
        use crate::file::r#trait::generate_tags_default;
        let mut tags = generate_tags_default(self.path(), self.extension(), content);

        // Add JSON-specific tags
        if !tags.contains(&"json".to_string()) {
            tags.push("json".to_string());
        }

        // Try to infer content type from JSON structure
        if let Ok(parsed) = serde_json::from_str::<Value>(content) {
            match parsed {
                Value::Object(ref obj) => {
                    // Check for common JSON document types
                    if obj.contains_key("package") || obj.contains_key("dependencies") {
                        if !tags.contains(&"package".to_string()) {
                            tags.push("package".to_string());
                        }
                    }
                    if obj.contains_key("version") && obj.contains_key("name") {
                        if !tags.contains(&"config".to_string()) {
                            tags.push("config".to_string());
                        }
                    }
                    if obj.contains_key("scripts") || obj.contains_key("devDependencies") {
                        if !tags.contains(&"nodejs".to_string()) {
                            tags.push("nodejs".to_string());
                        }
                    }
                    if obj.contains_key("tasks") || obj.contains_key("version") {
                        if !tags.contains(&"build".to_string()) {
                            tags.push("build".to_string());
                        }
                    }
                }
                Value::Array(_) => {
                    if !tags.contains(&"list".to_string()) {
                        tags.push("list".to_string());
                    }
                }
                _ => {}
            }
        }

        Ok(tags)
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
    async fn test_json_file_extraction() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        let json_content = r#"{"name":"test","value":42}"#;
        std::fs::write(&path, json_content).unwrap();

        let json_file = JsonFile::new(path.clone(), Some("json".to_string()));
        let text = json_file.to_text().await.unwrap();
        // Should be pretty-printed
        assert!(text.contains("\n"));
        assert!(text.contains("name"));
        assert!(text.contains("test"));
        
        assert_eq!(json_file.path(), path);
        assert_eq!(json_file.extension(), Some("json"));
    }

    #[tokio::test]
    async fn test_json_file_metadata() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        let json_content = r#"{"name":"test","value":42}"#;
        std::fs::write(&path, json_content).unwrap();

        let json_file = JsonFile::new(path, Some("json".to_string()));
        let metadata = json_file.to_metadata().await.unwrap();
        assert!(metadata.is_some());
        
        if let Some(Value::Object(obj)) = metadata {
            assert!(obj.contains_key("name"));
            assert!(obj.contains_key("value"));
        } else {
            panic!("Expected object metadata");
        }
    }

    #[tokio::test]
    async fn test_json_file_tags() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        let json_content = r#"{"name":"test","version":"1.0.0"}"#;
        std::fs::write(&path, json_content).unwrap();

        let json_file = JsonFile::new(path, Some("json".to_string()));
        let tags = json_file.generate_tags(json_content).await.unwrap();
        assert!(tags.contains(&"json".to_string()));
        assert!(tags.contains(&"config".to_string()));
    }
}

