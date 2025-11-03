use crate::file::SemanticSource;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

/// Generic file handler for unsupported types
pub struct GenericFile {
    path: std::path::PathBuf,
    extension: Option<String>,
}

impl GenericFile {
    pub fn new(path: std::path::PathBuf, extension: Option<String>) -> Self {
        Self { path, extension }
    }
}

#[async_trait]
impl SemanticSource for GenericFile {
    async fn to_text_impl(&self) -> Result<String> {
        Ok(String::new())
    }

    async fn to_metadata(&self) -> Result<Option<Value>> {
        Ok(None)
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
    async fn test_generic_file_extraction() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        std::fs::write(&path, "binary data").unwrap();

        let generic_file = GenericFile::new(path.clone(), Some("bin".to_string()));
        let text = generic_file.to_text().await.unwrap();
        assert_eq!(text, ""); // Generic files return empty text
        
        assert_eq!(generic_file.path(), path);
        assert_eq!(generic_file.extension(), Some("bin"));
    }
}

