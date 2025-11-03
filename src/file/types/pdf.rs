use crate::file::SemanticSource;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

/// PDF file handler
pub struct PdfFile {
    path: std::path::PathBuf,
    extension: Option<String>,
}

impl PdfFile {
    pub fn new(path: std::path::PathBuf, extension: Option<String>) -> Self {
        Self { path, extension }
    }
}

#[async_trait]
impl SemanticSource for PdfFile {
    async fn to_text_impl(&self) -> Result<String> {
        let path = self.path.clone();
        let text = tokio::task::spawn_blocking(move || -> Result<String> {
            use lopdf::Document;
            
            // Suppress stderr warnings from pdf-extract by using lopdf directly
            let doc = Document::load(&path)
                .with_context(|| format!("Failed to load PDF: {}", path.display()))?;

            let mut text_content = String::new();
            
            // Extract text from all pages
            for page_num in doc.get_pages().keys() {
                if let Ok(page_text) = doc.extract_text(&[*page_num]) {
                    text_content.push_str(&page_text);
                    text_content.push('\n');
                }
            }

            if text_content.trim().is_empty() {
                // Fallback: try pdf-extract if lopdf doesn't extract text
                match pdf_extract::extract_text(&path) {
                    Ok(text) => Ok(text.trim().to_string()),
                    Err(_) => Ok(String::new()), // Return empty if both methods fail
                }
            } else {
                Ok(text_content.trim().to_string())
            }
        })
        .await?
        .map_err(anyhow::Error::from)?;

        Ok(text)
    }

    async fn to_metadata(&self) -> Result<Option<Value>> {
        let path = self.path.clone();
        let metadata_result = tokio::task::spawn_blocking(move || {
            use lopdf::Document;
            
            // Try to open PDF document to extract metadata
            match Document::load(&path) {
                Ok(doc) => {
                    let mut meta_map = serde_json::Map::new();
                    
                    // Get file size from filesystem
                    if let Ok(fs_metadata) = std::fs::metadata(&path) {
                        meta_map.insert("size_bytes".to_string(), Value::Number(fs_metadata.len().into()));
                    }
                    
                    // Extract PDF metadata if available
                    if let Ok(info_dict_obj) = doc.trailer.get(b"Info") {
                        if let Ok(info_dict) = info_dict_obj.as_dict() {
                            // Extract common PDF metadata fields
                            let extract_string = |key: &str| -> Option<String> {
                                if let Ok(lopdf::Object::String(ref bytes, _)) = info_dict.get(key.as_bytes()) {
                                    // Convert bytes to String, handling UTF-8
                                    String::from_utf8(bytes.clone()).ok()
                                } else {
                                    None
                                }
                            };
                            
                            if let Some(title) = extract_string("Title") {
                                meta_map.insert("title".to_string(), Value::String(title));
                            }
                            if let Some(author) = extract_string("Author") {
                                meta_map.insert("author".to_string(), Value::String(author));
                            }
                            if let Some(subject) = extract_string("Subject") {
                                meta_map.insert("subject".to_string(), Value::String(subject));
                            }
                            if let Some(keywords) = extract_string("Keywords") {
                                meta_map.insert("keywords".to_string(), Value::String(keywords));
                            }
                            if let Some(creator) = extract_string("Creator") {
                                meta_map.insert("creator".to_string(), Value::String(creator));
                            }
                            if let Some(producer) = extract_string("Producer") {
                                meta_map.insert("producer".to_string(), Value::String(producer));
                            }
                        }
                    }
                    
                    // Get page count
                    meta_map.insert("page_count".to_string(), Value::Number(doc.get_pages().len().into()));
                    
                    if !meta_map.is_empty() {
                        Some(Value::Object(meta_map))
                    } else {
                        None
                    }
                }
                Err(_) => {
                    // If we can't read PDF metadata, at least return file size
                    if let Ok(fs_metadata) = std::fs::metadata(&path) {
                        let mut meta_map = serde_json::Map::new();
                        meta_map.insert("size_bytes".to_string(), Value::Number(fs_metadata.len().into()));
                        Some(Value::Object(meta_map))
                    } else {
                        None
                    }
                }
            }
        })
        .await?;

        Ok(metadata_result)
    }

    async fn generate_tags(&self, content: &str) -> Result<Vec<String>> {
        // Start with default implementation
        use crate::file::r#trait::generate_tags_default;
        let mut tags = generate_tags_default(self.path(), self.extension(), content);

        // PDF-specific keywords
        let pdf_keywords: std::collections::HashMap<&str, &str> = [
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
        
        // Always add "document" tag for PDFs
        tags.push("document".to_string());

        // Add PDF-specific tags
        for (keyword, tag) in pdf_keywords.iter() {
            if content_lower.contains(keyword) && !tags.contains(&tag.to_string()) {
                tags.push(tag.to_string());
            }
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

