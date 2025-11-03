use crate::models::FileMeta;
use crate::tagger::Taggable;
use anyhow::Result;

/// Generic fallback handler for unknown file types
pub struct GenericHandler;

impl GenericHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Taggable for GenericHandler {
    async fn extract_text(&self, _file: &FileMeta) -> Result<String> {
        // Generic handler cannot extract text from unknown formats
        Ok(String::new())
    }

    async fn generate_tags(&self, _content: &str) -> Result<Vec<String>> {
        // Return generic tags based on extension or metadata
        Ok(vec!["unknown".to_string()])
    }

    async fn compute_embedding(&self, _content: &str) -> Result<Vec<f32>> {
        Ok(Vec::new())
    }

    fn supports_extension(&self, _ext: &str) -> bool {
        // Generic handler supports all extensions as fallback
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::FileMeta;
    use std::path::PathBuf;
    use std::time::SystemTime;

    #[tokio::test]
    async fn test_generic_handler_supports_any_extension() {
        let handler = GenericHandler::new();
        assert!(handler.supports_extension("txt"));
        assert!(handler.supports_extension("md"));
        assert!(handler.supports_extension("pdf"));
        assert!(handler.supports_extension("unknown"));
    }

    #[tokio::test]
    async fn test_generic_handler_extract_text_returns_empty() {
        let handler = GenericHandler::new();
        let file = FileMeta::new(
            PathBuf::from("/test/file.xyz"),
            0,
            Some("xyz".to_string()),
            SystemTime::now(),
            "hash".to_string(),
        );

        let text = handler.extract_text(&file).await.unwrap();
        assert_eq!(text, "");
    }

    #[tokio::test]
    async fn test_generic_handler_generate_tags_returns_unknown() {
        let handler = GenericHandler::new();
        let tags = handler.generate_tags("any content").await.unwrap();
        assert_eq!(tags, vec!["unknown"]);
    }

    #[tokio::test]
    async fn test_generic_handler_compute_embedding_returns_empty() {
        let handler = GenericHandler::new();
        let embedding = handler.compute_embedding("any content").await.unwrap();
        assert_eq!(embedding, Vec::<f32>::new());
    }
}

