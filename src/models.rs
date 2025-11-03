use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

/// Metadata about a file in the filesystem
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileMeta {
    /// Full path to the file
    pub path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// File extension (without the dot)
    pub extension: Option<String>,
    /// Creation time
    pub created_at: SystemTime,
    /// Last modification time
    pub updated_at: SystemTime,
    /// Blake3 hash of file contents
    pub hash: String,
}

impl FileMeta {
    /// Create a new FileMeta instance
    pub fn new(
        path: PathBuf,
        size: u64,
        extension: Option<String>,
        created_at: SystemTime,
        updated_at: SystemTime,
        hash: String,
    ) -> Self {
        Self {
            path,
            size,
            extension,
            created_at,
            updated_at,
            hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_meta_creation() {
        let path = PathBuf::from("/test/file.txt");
        let now = SystemTime::now();
        let hash = "abc123".to_string();

        let meta = FileMeta::new(path.clone(), 100, Some("txt".to_string()), now, now, hash.clone());

        assert_eq!(meta.path, path);
        assert_eq!(meta.size, 100);
        assert_eq!(meta.extension, Some("txt".to_string()));
        assert_eq!(meta.hash, hash);
        assert_eq!(meta.created_at, now);
        assert_eq!(meta.updated_at, now);
    }

    #[test]
    fn test_file_meta_without_extension() {
        let path = PathBuf::from("/test/file");
        let now = SystemTime::now();
        let hash = "def456".to_string();

        let meta = FileMeta::new(path, 200, None, now, now, hash);

        assert_eq!(meta.extension, None);
        assert_eq!(meta.size, 200);
    }

    #[test]
    fn test_file_meta_serialization() {
        let path = PathBuf::from("/test/file.json");
        let now = SystemTime::now();
        let hash = "hash123".to_string();

        let meta = FileMeta::new(path, 300, Some("json".to_string()), now, now, hash);

        let serialized = serde_json::to_string(&meta).unwrap();
        let deserialized: FileMeta = serde_json::from_str(&serialized).unwrap();

        assert_eq!(meta.path, deserialized.path);
        assert_eq!(meta.size, deserialized.size);
        assert_eq!(meta.extension, deserialized.extension);
        assert_eq!(meta.hash, deserialized.hash);
        assert_eq!(meta.created_at, deserialized.created_at);
        assert_eq!(meta.updated_at, deserialized.updated_at);
    }
}

