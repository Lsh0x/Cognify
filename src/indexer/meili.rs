use crate::indexer::Indexer;
use crate::models::FileMeta;
use anyhow::{Context, Result};
use meilisearch_sdk::{client::Client, indexes::Index, search::SearchResults};
use serde::{Deserialize, Serialize};

/// Document structure for Meilisearch
#[derive(Debug, Serialize, Deserialize)]
struct Document {
    path: String,
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

        // Create the index if it doesn't exist
        let _ = client
            .create_index(index_name, Some("path"))
            .await; // Ignore error if index already exists

        // Configure index settings for embeddings and search
        let index = client.index(index_name);
        
        // Configure searchable attributes (tags and path are searchable)
        // Note: Settings can also be configured via Meilisearch dashboard
        // For vector search with embeddings, Meilisearch v1.5+ supports vector fields
        
        Ok(Self { client, index })
    }

    /// Get a reference to the underlying index
    pub fn index(&self) -> &Index {
        &self.index
    }

    /// Index a semantic file with text and metadata
    pub async fn index_semantic_file(
        &self,
        file: &FileMeta,
        tags: &[String],
        text: Option<&str>,
        metadata: Option<&serde_json::Value>,
        embedding: Option<&[f32]>,
    ) -> Result<()> {
        let doc = Document {
            path: file.path.to_string_lossy().to_string(),
            size: file.size,
            extension: file.extension.clone(),
            tags: tags.to_vec(),
            text: text.map(|s| s.to_string()),
            metadata: metadata.cloned(),
            embedding: embedding.map(|e| e.to_vec()),
        };

        self.index
            .add_documents(&[doc], Some("path"))
            .await
            .context("Failed to add document to Meilisearch")?;

        Ok(())
    }
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

