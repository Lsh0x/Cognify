use crate::indexer::Indexer;
use crate::models::FileMeta;
use anyhow::{Context, Result};
use meilisearch_sdk::{client::Client, indexes::Index, search::SearchResults};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Document structure for Meilisearch
#[derive(Debug, Serialize, Deserialize)]
struct Document {
    id: String, // Stable identifier: hash(path) only (no timestamp)
    path: String,
    file_hash: String, // Blake3 hash of file content for change detection
    size: u64,
    extension: Option<String>,
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    embedding: Option<Vec<f32>>,
}

/// Generate a stable document ID from file path (hash only, no timestamp)
/// This allows us to update existing documents instead of creating duplicates
fn generate_doc_id(path: &std::path::Path) -> String {
    let path_str = path.to_string_lossy();
    let hash = blake3::hash(path_str.as_bytes());
    // Use first 32 hex chars of hash for a stable ID
    // Same path = same ID, allowing updates instead of duplicates
    format!("doc_{}", &hash.to_hex()[..32])
}

/// Meilisearch implementation of the Indexer trait
pub struct MeilisearchIndexer {
    client: Client,
    index: Index,
}

impl MeilisearchIndexer {
    /// Create a new Meilisearch indexer
    pub async fn new(url: &str, api_key: Option<&str>, index_name: &str) -> Result<Self> {
        let client = if let Some(key) = api_key {
            Client::new(url, Some(key.to_string()))?
        } else {
            Client::new(url, None::<String>)?
        };

        // Check if index exists and verify its primary key
        let needs_recreation = Self::check_index_primary_key(url, api_key, index_name).await?;
        
        if needs_recreation {
            eprintln!("Index '{}' has incorrect primary key. Recreating with 'id' as primary key...", index_name);
            // Delete the existing index
            let _ = client.delete_index(index_name).await;
            // Create the index with "id" as primary key
            client
                .create_index(index_name, Some("id"))
                .await
                .context("Failed to create index with 'id' as primary key")?;
        } else {
            // Index doesn't exist or has correct primary key, try to create it
            let create_result = client
                .create_index(index_name, Some("id"))
                .await;
            
            // Ignore error if index already exists with correct primary key
            if let Err(e) = create_result {
                let error_msg = e.to_string();
                // Only propagate error if it's not about index already existing
                if !error_msg.contains("already exists") && !error_msg.contains("Index already exists") {
                    return Err(e).context("Failed to create index");
                }
            }
        }

        let index = client.index(index_name);
        
        // Configure searchable attributes (tags and path are searchable)
        // Note: Settings can also be configured via Meilisearch dashboard
        // For vector search with embeddings, Meilisearch v1.5+ supports vector fields
        
        Ok(Self { client, index })
    }

    /// Check if index exists and if its primary key is correct
    /// Returns true if index needs to be recreated (wrong primary key or doesn't exist)
    async fn check_index_primary_key(url: &str, api_key: Option<&str>, index_name: &str) -> Result<bool> {
        let client = reqwest::Client::new();
        let url = url.trim_end_matches('/');
        let endpoint = format!("{}/indexes/{}", url, index_name);
        
        let mut request = client.get(&endpoint);
        if let Some(key) = api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }
        
        let response = request.send().await;
        
        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    let index_info: Value = resp.json().await.context("Failed to parse index info")?;
                    let primary_key = index_info.get("primaryKey")
                        .and_then(|v| v.as_str())
                        .or_else(|| index_info.get("primary_key").and_then(|v| v.as_str()));
                    
                    // If primary key is not "id", we need to recreate
                    Ok(primary_key != Some("id"))
                } else if resp.status() == 404 {
                    // Index doesn't exist, we'll create it
                    Ok(false)
                } else {
                    // Other error, assume we need to recreate
                    Ok(true)
                }
            }
            Err(_) => {
                // Network error, assume we need to recreate
                Ok(true)
            }
        }
    }

    /// Get a reference to the underlying index
    pub fn index(&self) -> &Index {
        &self.index
    }

    /// Index a semantic file with text and metadata
    /// This will update the document if it already exists (same path = same ID)
    pub async fn index_semantic_file(
        &self,
        file: &FileMeta,
        tags: &[String],
        text: Option<&str>,
        metadata: Option<&serde_json::Value>,
        embedding: Option<&[f32]>,
    ) -> Result<()> {
        let doc_id = generate_doc_id(&file.path);
        let doc = Document {
            id: doc_id,
            path: file.path.to_string_lossy().to_string(),
            file_hash: file.hash.clone(), // Store file hash for change detection
            size: file.size,
            extension: file.extension.clone(),
            tags: tags.to_vec(),
            text: text.map(|s| s.to_string()),
            metadata: metadata.cloned(),
            embedding: embedding.map(|e| e.to_vec()),
        };

        // add_documents with same ID will update existing document
        self.index
            .add_documents(&[doc], Some("id"))
            .await
            .context("Failed to add/update document in Meilisearch")?;

        Ok(())
    }

    /// Get all indexed file paths
    /// Useful for syncing: find files that no longer exist
    pub async fn get_all_indexed_paths(&self) -> Result<Vec<String>> {
        // Use search with empty query to get all documents
        // Limit to a large number to get all documents
        let search_results: SearchResults<Document> = self
            .index
            .search()
            .with_query("")
            .with_limit(10000) // Adjust limit as needed
            .execute()
            .await
            .context("Failed to search all documents")?;

        Ok(search_results.hits.iter()
            .map(|hit| hit.result.path.clone())
            .collect())
    }

    /// Delete documents for files that no longer exist
    pub async fn delete_missing_files(&self, existing_paths: &std::collections::HashSet<String>) -> Result<usize> {
        // Get all indexed paths
        let indexed_paths = self.get_all_indexed_paths().await?;
        
        // Find paths that are indexed but no longer exist on filesystem
        let mut to_delete = Vec::new();
        for indexed_path in indexed_paths {
            if !existing_paths.contains(&indexed_path) {
                // Generate ID for this path to delete it
                let doc_id = generate_doc_id(std::path::Path::new(&indexed_path));
                to_delete.push(doc_id);
            }
        }

        if to_delete.is_empty() {
            return Ok(0);
        }

        // Delete documents by ID
        self.index
            .delete_documents(&to_delete)
            .await
            .context("Failed to delete missing files from index")?;

        Ok(to_delete.len())
    }

    /// Synchronize index with filesystem
    /// - Updates existing documents that have changed (different file_hash)
    /// - Deletes documents for files that no longer exist
    /// - Returns statistics about the sync operation
    pub async fn sync_index(
        &self,
        current_files: &[&FileMeta],
    ) -> Result<SyncStats> {
        let mut stats = SyncStats {
            updated: 0,
            deleted: 0,
            unchanged: 0,
        };

        // Build set of current file paths
        let current_paths: std::collections::HashSet<String> = current_files
            .iter()
            .map(|f| f.path.to_string_lossy().to_string())
            .collect();

        // Delete documents for files that no longer exist
        stats.deleted = self.delete_missing_files(&current_paths).await?;

        // Get all indexed documents to check for changes
        let search_results: SearchResults<Document> = self
            .index
            .search()
            .with_query("")
            .with_limit(10000)
            .execute()
            .await
            .context("Failed to get indexed documents for sync")?;

        // Build map of indexed documents by path
        let mut indexed_by_path: std::collections::HashMap<String, Document> = search_results.hits
            .into_iter()
            .map(|hit| (hit.result.path.clone(), hit.result))
            .collect();

        // Check each current file: update if changed, mark unchanged if same
        for file in current_files {
            let path_str = file.path.to_string_lossy().to_string();
            if let Some(indexed_doc) = indexed_by_path.get(&path_str) {
                if indexed_doc.file_hash == file.hash {
                    // File hasn't changed
                    stats.unchanged += 1;
                } else {
                    // File has changed - will be updated by next index operation
                    // Just count it here
                    stats.updated += 1;
                }
            }
            // New files (not in indexed_by_path) will be added during normal indexing
        }

        Ok(stats)
    }
}

/// Statistics about a sync operation
#[derive(Debug, Clone)]
pub struct SyncStats {
    /// Number of files that were updated (changed content)
    pub updated: usize,
    /// Number of files that were deleted (no longer exist)
    pub deleted: usize,
    /// Number of files that were unchanged
    pub unchanged: usize,
}

#[async_trait::async_trait]
impl Indexer for MeilisearchIndexer {
    async fn index_file(&self, file: &FileMeta, tags: &[String]) -> Result<()> {
        self.index_file_with_embedding(file, tags, None).await
    }

    async fn index_file_with_embedding(
        &self,
        file: &FileMeta,
        tags: &[String],
        embedding: Option<&[f32]>,
    ) -> Result<()> {
        self.index_semantic_file(file, tags, None, None, embedding).await
    }

    async fn search(&self, query: &str) -> Result<Vec<FileMeta>> {
        let search_results: SearchResults<Document> = self
            .index
            .search()
            .with_query(query)
            .execute()
            .await
            .context("Failed to search Meilisearch index")?;

        let mut results = Vec::new();

        for hit in search_results.hits {
            let doc = hit.result;
            let path = std::path::PathBuf::from(&doc.path);

            // Try to get metadata from filesystem
            let metadata = std::fs::metadata(&path).ok();
            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(doc.size);
            let created_at = metadata
                .as_ref()
                .and_then(|m| m.created().ok())
                .or_else(|| metadata.as_ref().and_then(|m| m.modified().ok()))
                .unwrap_or_else(|| std::time::SystemTime::now());
            let updated_at = metadata
                .as_ref()
                .and_then(|m| m.modified().ok())
                .or_else(|| metadata.as_ref().and_then(|m| m.created().ok()))
                .unwrap_or_else(|| std::time::SystemTime::now());

            // For now, use a placeholder hash (in production, this should be retrieved from index)
            let hash = format!("meili-{}", doc.path);

            let file_meta = FileMeta::new(path, size, doc.extension, created_at, updated_at, hash);
            results.push(file_meta);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::FileMeta;
    use std::path::PathBuf;
    use std::time::SystemTime;

    // Note: These tests require a running Meilisearch instance
    // They are marked with #[ignore] by default

    #[tokio::test]
    #[ignore]
    async fn test_meilisearch_indexer_creation() {
        let indexer = MeilisearchIndexer::new("http://127.0.0.1:7700", None, "test_index")
            .await
            .unwrap();
        // Index UID is stored internally, this test just verifies creation succeeds
        assert!(indexer.index().uid == "test_index");
    }

    #[tokio::test]
    #[ignore]
    async fn test_meilisearch_index_file() {
        let indexer = MeilisearchIndexer::new("http://127.0.0.1:7700", None, "test_index")
            .await
            .unwrap();

        let now = SystemTime::now();
        let file = FileMeta::new(
            PathBuf::from("/test/file.txt"),
            100,
            Some("txt".to_string()),
            now,
            now,
            "hash123".to_string(),
        );

        let tags = vec!["test".to_string(), "documentation".to_string()];
        indexer.index_file(&file, &tags).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_meilisearch_search() {
        let indexer = MeilisearchIndexer::new("http://127.0.0.1:7700", None, "test_index")
            .await
            .unwrap();

        let results = indexer.search("test").await.unwrap();
        assert!(!results.is_empty());
    }
}

