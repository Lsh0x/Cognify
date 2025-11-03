use crate::models::FileMeta;
use serde_json::Value;

/// Semantic file structure containing all extracted information
#[derive(Debug, Clone)]
pub struct SemanticFile {
    pub path: std::path::PathBuf,
    pub extension: Option<String>,
    pub tags: Vec<String>,
    pub text: Option<String>,
    pub metadata: Option<Value>,
    pub size: u64,
    pub hash: String,
    pub created_at: std::time::SystemTime,
}

impl SemanticFile {
    pub fn new(
        path: std::path::PathBuf,
        extension: Option<String>,
        size: u64,
        hash: String,
        created_at: std::time::SystemTime,
    ) -> Self {
        Self {
            path,
            extension,
            tags: Vec::new(),
            text: None,
            metadata: None,
            size,
            hash,
            created_at,
        }
    }

    /// Create from FileMeta
    pub fn from_file_meta(meta: &FileMeta) -> Self {
        Self {
            path: meta.path.clone(),
            extension: meta.extension.clone(),
            tags: Vec::new(),
            text: None,
            metadata: None,
            size: meta.size,
            hash: meta.hash.clone(),
            created_at: meta.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::FileMeta;
    use std::path::PathBuf;
    use std::time::SystemTime;

    #[test]
    fn test_semantic_file_from_file_meta() {
        let now = SystemTime::now();
        let meta = FileMeta::new(
            PathBuf::from("/test/file.txt"),
            100,
            Some("txt".to_string()),
            now,
            now,
            "hash123".to_string(),
        );

        let semantic_file = SemanticFile::from_file_meta(&meta);
        assert_eq!(semantic_file.path, meta.path);
        assert_eq!(semantic_file.extension, meta.extension);
        assert_eq!(semantic_file.size, meta.size);
        assert_eq!(semantic_file.hash, meta.hash);
        assert!(semantic_file.tags.is_empty());
        assert!(semantic_file.text.is_none());
        assert!(semantic_file.metadata.is_none());
    }

    #[test]
    fn test_semantic_file_new() {
        let path = PathBuf::from("/test/file.txt");
        let semantic_file = SemanticFile::new(
            path.clone(),
            Some("txt".to_string()),
            100,
            "hash123".to_string(),
            SystemTime::now(),
        );

        assert_eq!(semantic_file.path, path);
        assert_eq!(semantic_file.extension, Some("txt".to_string()));
        assert_eq!(semantic_file.size, 100);
        assert_eq!(semantic_file.hash, "hash123");
        assert!(semantic_file.tags.is_empty());
        assert!(semantic_file.text.is_none());
        assert!(semantic_file.metadata.is_none());
    }
}

