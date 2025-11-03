use anyhow::Result;

/// Trait for embedding providers that can compute semantic vectors from text
#[async_trait::async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Compute embedding vector from text content
    async fn compute_embedding(&self, content: &str) -> Result<Vec<f32>>;

    /// Get the dimension of embeddings produced by this provider
    fn dimension(&self) -> usize;
}

