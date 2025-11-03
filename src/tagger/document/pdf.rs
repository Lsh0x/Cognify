use crate::extractor::pdf::PdfExtractor;
use crate::models::FileMeta;
use crate::tagger::Taggable;
use anyhow::Result;

/// Handler for PDF files
pub struct PdfHandler {
    extractor: PdfExtractor,
}

impl PdfHandler {
    pub fn new() -> Self {
        Self {
            extractor: PdfExtractor::new(),
        }
    }
}

#[async_trait::async_trait]
impl Taggable for PdfHandler {
    async fn extract_text(&self, file: &FileMeta) -> Result<String> {
        // Use the PDF extractor to get text content
        // The extractor implements TextExtractor trait
        use crate::extractor::r#trait::TextExtractor;
        self.extractor.extract(file).await
    }

    async fn generate_tags(&self, content: &str) -> Result<Vec<String>> {
        // Dictionary-based tagging for PDFs
        let keywords: std::collections::HashMap<&str, &str> = [
            ("invoice", "financial"),
            ("receipt", "financial"),
            ("statement", "financial"),
            ("bill", "financial"),
            ("payment", "financial"),
            ("tax", "financial"),
            ("report", "reporting"),
            ("meeting", "calendar"),
            ("minutes", "calendar"),
            ("agenda", "calendar"),
            ("contract", "legal"),
            ("agreement", "legal"),
            ("nda", "legal"),
            ("resume", "personal"),
            ("cv", "personal"),
            ("certificate", "personal"),
            ("diploma", "personal"),
        ]
        .iter()
        .cloned()
        .collect();

        let content_lower = content.to_lowercase();
        let mut tags = Vec::new();

        // Always add "document" tag for PDFs
        tags.push("document".to_string());

        for (keyword, tag) in keywords.iter() {
            if content_lower.contains(keyword) && !tags.contains(&tag.to_string()) {
                tags.push(tag.to_string());
            }
        }

        Ok(tags)
    }

    async fn compute_embedding(&self, _content: &str) -> Result<Vec<f32>> {
        // Embeddings are computed by the embedding provider, not handlers
        Ok(Vec::new())
    }

    fn supports_extension(&self, ext: &str) -> bool {
        ext.to_lowercase() == "pdf"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::FileMeta;
    use std::path::PathBuf;
    use std::time::SystemTime;

    #[tokio::test]
    async fn test_pdf_handler_supports_pdf() {
        let handler = PdfHandler::new();
        assert!(handler.supports_extension("pdf"));
        assert!(handler.supports_extension("PDF"));
        assert!(!handler.supports_extension("txt"));
    }

    #[tokio::test]
    async fn test_pdf_handler_generate_tags_includes_document() {
        let handler = PdfHandler::new();
        let tags = handler.generate_tags("Invoice for services").await.unwrap();
        assert!(tags.contains(&"document".to_string()));
        assert!(tags.contains(&"financial".to_string()));
    }
}

