use anyhow::Result;
use blake3;
use std::fs::File;
use std::io::Read;

/// Compute Blake3 hash of file contents
pub fn compute_file_hash(file_path: &std::path::Path) -> Result<String> {
    let mut file = File::open(file_path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

/// Get file extension from path (without the dot)
pub fn get_extension(path: &std::path::Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_compute_file_hash() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "test content").unwrap();
        file.flush().unwrap();

        let hash = compute_file_hash(file.path()).unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // Blake3 hex string length
    }

    #[test]
    fn test_compute_file_hash_consistent() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "same content").unwrap();
        file.flush().unwrap();

        let hash1 = compute_file_hash(file.path()).unwrap();
        let hash2 = compute_file_hash(file.path()).unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_file_hash_different_content() {
        let mut file1 = NamedTempFile::new().unwrap();
        write!(file1, "content one").unwrap();
        file1.flush().unwrap();

        let mut file2 = NamedTempFile::new().unwrap();
        write!(file2, "content two").unwrap();
        file2.flush().unwrap();

        let hash1 = compute_file_hash(file1.path()).unwrap();
        let hash2 = compute_file_hash(file2.path()).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_get_extension_with_txt() {
        let path = std::path::Path::new("/path/to/file.txt");
        let ext = get_extension(path);
        assert_eq!(ext, Some("txt".to_string()));
    }

    #[test]
    fn test_get_extension_with_md() {
        let path = std::path::Path::new("/path/to/README.md");
        let ext = get_extension(path);
        assert_eq!(ext, Some("md".to_string()));
    }

    #[test]
    fn test_get_extension_lowercase() {
        let path = std::path::Path::new("/path/to/file.TXT");
        let ext = get_extension(path);
        assert_eq!(ext, Some("txt".to_string()));
    }

    #[test]
    fn test_get_extension_no_extension() {
        let path = std::path::Path::new("/path/to/file");
        let ext = get_extension(path);
        assert_eq!(ext, None);
    }

    #[test]
    fn test_get_extension_multiple_dots() {
        let path = std::path::Path::new("/path/to/file.tar.gz");
        let ext = get_extension(path);
        assert_eq!(ext, Some("gz".to_string()));
    }
}

