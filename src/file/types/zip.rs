use crate::file::SemanticSource;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::io::Read;
use std::path::Path;

/// ZIP archive file handler
pub struct ZipFile {
    path: std::path::PathBuf,
    extension: Option<String>,
}

impl ZipFile {
    pub fn new(path: std::path::PathBuf, extension: Option<String>) -> Self {
        Self { path, extension }
    }

    /// Check if a file extension is text-based
    fn is_text_extension(ext: &str) -> bool {
        let text_extensions = [
            "txt", "md", "markdown", "json", "xml", "html", "htm", "css", "js", "ts",
            "rs", "py", "java", "c", "cpp", "h", "hpp", "go", "rb", "php", "sh", "bash",
            "yaml", "yml", "toml", "ini", "cfg", "conf", "log", "csv", "tsv",
        ];
        text_extensions.contains(&ext.to_lowercase().as_str())
    }
}

#[async_trait]
impl SemanticSource for ZipFile {
    async fn to_text_impl(&self) -> Result<String> {
        let path = self.path.clone();
        let text = tokio::task::spawn_blocking(move || -> Result<String> {
            use std::fs::File;
            use zip::ZipArchive;

            let file = File::open(&path)
                .with_context(|| format!("Failed to open ZIP file: {}", path.display()))?;

            let mut archive = ZipArchive::new(file)
                .with_context(|| format!("Failed to read ZIP archive: {}", path.display()))?;

            let mut all_text = String::new();

            // Extract text from all text-based files in the archive
            let archive_len = archive.len();
            for i in 0..archive_len {
                let mut file = archive.by_index(i)
                    .with_context(|| format!("Failed to read file {} in ZIP", i))?;

                // Skip directories
                if file.is_dir() {
                    continue;
                }

                let file_name = file.name().to_string();
                
                // Check if file has a text extension
                let is_text = if let Some(ext) = std::path::Path::new(&file_name)
                    .extension()
                    .and_then(|e| e.to_str())
                {
                    Self::is_text_extension(ext)
                } else {
                    false
                };

                if is_text {
                    let mut contents = String::new();
                    if file.read_to_string(&mut contents).is_ok() {
                        // Add file name and content
                        all_text.push_str(&format!("\n--- File: {} ---\n", file_name));
                        all_text.push_str(&contents);
                        all_text.push_str("\n");
                    }
                }
            }

            if all_text.is_empty() {
                Ok(format!("ZIP archive containing {} files (no text files extracted)", archive.len()))
            } else {
                Ok(all_text.trim().to_string())
            }
        })
        .await?
        .map_err(anyhow::Error::from)?;

        Ok(text)
    }

    async fn to_metadata(&self) -> Result<Option<Value>> {
        let path = self.path.clone();
        let metadata = tokio::task::spawn_blocking(move || -> Result<Option<Value>> {
            use std::fs::File;
            use zip::ZipArchive;

            let file = File::open(&path)
                .with_context(|| format!("Failed to open ZIP file: {}", path.display()))?;

            let mut archive = ZipArchive::new(file)
                .with_context(|| format!("Failed to read ZIP archive: {}", path.display()))?;

            let mut meta_map = serde_json::Map::new();

            // File count
            meta_map.insert("file_count".to_string(), Value::Number(archive.len().into()));

            // List of files in archive
            let mut file_list = Vec::new();
            let mut total_size = 0u64;
            let mut text_file_count = 0u64;

            for i in 0..archive.len() {
                if let Ok(file) = archive.by_index(i) {
                    let file_name = file.name().to_string();
                    let file_size = file.size();
                    total_size += file_size;

                    if !file.is_dir() {
                        file_list.push(Value::String(file_name.clone()));

                        // Count text files
                        if let Some(ext) = std::path::Path::new(&file_name)
                            .extension()
                            .and_then(|e| e.to_str())
                        {
                            if Self::is_text_extension(ext) {
                                text_file_count += 1;
                            }
                        }
                    }
                }
            }

            meta_map.insert("files".to_string(), Value::Array(file_list));
            meta_map.insert("total_size_bytes".to_string(), Value::Number(total_size.into()));
            meta_map.insert("text_file_count".to_string(), Value::Number(text_file_count.into()));

            // Get file size from filesystem
            if let Ok(fs_metadata) = std::fs::metadata(&path) {
                meta_map.insert("archive_size_bytes".to_string(), Value::Number(fs_metadata.len().into()));
            }

            Ok(Some(Value::Object(meta_map)))
        })
        .await?
        .map_err(anyhow::Error::from)?;

        Ok(metadata)
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
    use std::io::Write;
    use tempfile::NamedTempFile;
    use zip::write::FileOptions;
    use zip::ZipWriter;

    fn create_test_zip() -> (tempfile::TempPath, std::path::PathBuf) {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        
        let file = std::fs::File::create(&path).unwrap();
        let mut zip = ZipWriter::new(file);
        
        // Add a text file
        zip.start_file("readme.txt", FileOptions::default()).unwrap();
        zip.write_all(b"Hello from ZIP!").unwrap();
        
        // Add another text file
        zip.start_file("doc.md", FileOptions::default()).unwrap();
        zip.write_all(b"# Markdown Content\n\nThis is a test.").unwrap();
        
        // Add a binary file (should be skipped)
        zip.start_file("image.png", FileOptions::default()).unwrap();
        zip.write_all(&[0x89, 0x50, 0x4E, 0x47]).unwrap();
        
        zip.finish().unwrap();
        (temp_file.into_temp_path(), path)
    }

    #[tokio::test]
    async fn test_zip_file_extraction() {
        let (_temp_path, zip_path) = create_test_zip();
        let zip_file = ZipFile::new(zip_path.clone(), Some("zip".to_string()));
        let text = zip_file.to_text().await.unwrap();
        
        assert!(text.contains("Hello from ZIP!"));
        assert!(text.contains("Markdown Content"));
        assert!(text.contains("readme.txt"));
        assert!(text.contains("doc.md"));
        
        assert_eq!(zip_file.path(), zip_path);
        assert_eq!(zip_file.extension(), Some("zip"));
    }

    #[tokio::test]
    async fn test_zip_file_metadata() {
        let (_temp_path, zip_path) = create_test_zip();
        let zip_file = ZipFile::new(zip_path, Some("zip".to_string()));
        let metadata = zip_file.to_metadata().await.unwrap();
        
        assert!(metadata.is_some());
        let meta = metadata.unwrap();
        assert!(meta.get("file_count").is_some());
        assert!(meta.get("files").is_some());
        assert!(meta.get("text_file_count").is_some());
    }
}

