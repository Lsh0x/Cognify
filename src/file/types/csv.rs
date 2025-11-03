use crate::file::SemanticSource;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

/// CSV file handler
pub struct CsvFile {
    path: std::path::PathBuf,
    extension: Option<String>,
}

impl CsvFile {
    pub fn new(path: std::path::PathBuf, extension: Option<String>) -> Self {
        Self { path, extension }
    }
}

#[async_trait]
impl SemanticSource for CsvFile {
    async fn to_text_impl(&self) -> Result<String> {
        tokio::fs::read_to_string(&self.path)
            .await
            .with_context(|| format!("Failed to read CSV file: {}", self.path.display()))
    }

    async fn to_metadata(&self) -> Result<Option<Value>> {
        let path = self.path.clone();
        let metadata = tokio::task::spawn_blocking(move || -> Result<Option<Value>> {
            use std::fs::File;
            use std::io::BufReader;
            
            let file = File::open(&path)
                .with_context(|| format!("Failed to open CSV file: {}", path.display()))?;
            
            let reader = BufReader::new(file);
            let mut rdr = csv::Reader::from_reader(reader);
            
            let mut meta_map = serde_json::Map::new();
            
            // Get headers if available
            if let Ok(headers) = rdr.headers() {
                let header_list: Vec<String> = headers.iter()
                    .map(|h| h.to_string())
                    .collect();
                
                let column_count = header_list.len();
                
                if !header_list.is_empty() {
                    meta_map.insert(
                        "headers".to_string(),
                        Value::Array(header_list.into_iter().map(Value::String).collect())
                    );
                    meta_map.insert(
                        "column_count".to_string(),
                        Value::Number(column_count.into())
                    );
                }
            }
            
            // Count rows (excluding header)
            let mut row_count = 0u64;
            for result in rdr.records() {
                match result {
                    Ok(_) => row_count += 1,
                    Err(_) => break, // Stop on error
                }
            }
            
            meta_map.insert("row_count".to_string(), Value::Number(row_count.into()));
            
            // Get file size from filesystem
            if let Ok(fs_metadata) = std::fs::metadata(&path) {
                meta_map.insert("size_bytes".to_string(), Value::Number(fs_metadata.len().into()));
            }
            
            if !meta_map.is_empty() {
                Ok(Some(Value::Object(meta_map)))
            } else {
                Ok(None)
            }
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
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_csv_file_extraction() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        let csv_content = "name,age,city\nJohn,30,Paris\nJane,25,London";
        std::fs::write(&path, csv_content).unwrap();

        let csv_file = CsvFile::new(path.clone(), Some("csv".to_string()));
        let text = csv_file.to_text().await.unwrap();
        assert_eq!(text, csv_content);
        
        assert_eq!(csv_file.path(), path);
        assert_eq!(csv_file.extension(), Some("csv"));
    }

    #[tokio::test]
    async fn test_csv_file_metadata() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        let csv_content = "name,age,city\nJohn,30,Paris\nJane,25,London";
        std::fs::write(&path, csv_content).unwrap();

        let csv_file = CsvFile::new(path, Some("csv".to_string()));
        let metadata = csv_file.to_metadata().await.unwrap();
        
        assert!(metadata.is_some());
        let meta = metadata.unwrap();
        assert!(meta.get("headers").is_some());
        assert!(meta.get("column_count").is_some());
        assert!(meta.get("row_count").is_some());
        
        // Verify column count
        if let Some(Value::Number(col_count)) = meta.get("column_count") {
            assert_eq!(col_count.as_u64(), Some(3));
        }
        
        // Verify row count (excluding header)
        if let Some(Value::Number(row_count)) = meta.get("row_count") {
            assert_eq!(row_count.as_u64(), Some(2));
        }
    }
}

