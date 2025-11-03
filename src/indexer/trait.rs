use crate::models::FileMeta;
use anyhow::Result;

/// Trait for indexers that can store and search file metadata
#[async_trait::async_trait]
pub trait Indexer: Send + Sync {
    /// Index a file with its metadata and tags
    async fn index_file(&self, file: &FileMeta, tags: &[String]) -> Result<()>;

    /// Index a file with metadata, tags, and optional embedding vector
    async fn index_file_with_embedding(
        &self,
        file: &FileMeta,
        tags: &[String],
        embedding: Option<&[f32]>,
    ) -> Result<()>;

    /// Search for files matching the query
    async fn search(&self, query: &str) -> Result<Vec<FileMeta>>;
}

