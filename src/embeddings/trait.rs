use anyhow::{Context, Result};
use crate::utils;

/// Trait for embedding providers that can compute semantic vectors from text
#[async_trait::async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Compute embedding vector from text content
    async fn compute_embedding(&self, content: &str) -> Result<Vec<f32>>;

    /// Get the dimension of embeddings produced by this provider
    fn dimension(&self) -> usize;

    /// Compute embedding for potentially long content by chunking and combining embeddings
    /// 
    /// This method chunks the content if it exceeds max_tokens, computes embeddings
    /// for each chunk, and combines them using mean pooling (average of all chunk embeddings).
    /// 
    /// # Arguments
    /// * `content` - The text content to embed
    /// * `max_tokens` - Maximum tokens per chunk (content exceeding this will be chunked)
    /// 
    /// # Returns
    /// Combined embedding vector of the same dimension as individual chunks
    async fn compute_chunked_embedding(&self, content: &str, max_tokens: usize) -> Result<Vec<f32>> {
        // Estimate if content needs chunking (~4 chars per token)
        let estimated_tokens = content.len() / 4;
        
        if estimated_tokens <= max_tokens {
            // Content fits in one chunk, use regular embedding
            return self.compute_embedding(content).await;
        }

        // Chunk the content with overlap (10% of max_tokens for overlap)
        let overlap_tokens = (max_tokens / 10).max(20).min(50); // 10% overlap, min 20, max 50 tokens
        let chunks = utils::chunk_text_for_embedding(content, max_tokens, overlap_tokens);

        if chunks.is_empty() {
            anyhow::bail!("No chunks generated from content");
        }

        if chunks.len() == 1 {
            // Only one chunk, use regular embedding
            return self.compute_embedding(&chunks[0]).await;
        }

        // Compute embeddings for all chunks
        let mut chunk_embeddings = Vec::new();
        for (i, chunk) in chunks.iter().enumerate() {
            let embedding = self.compute_embedding(chunk)
                .await
                .with_context(|| format!("Failed to compute embedding for chunk {}/{}", i + 1, chunks.len()))?;
            
            if embedding.is_empty() {
                anyhow::bail!("Empty embedding returned for chunk {}/{}", i + 1, chunks.len());
            }

            // Validate dimension matches expected
            let expected_dim = self.dimension();
            if embedding.len() != expected_dim {
                anyhow::bail!(
                    "Chunk {}/{} embedding dimension mismatch: got {}, expected {}",
                    i + 1,
                    chunks.len(),
                    embedding.len(),
                    expected_dim
                );
            }

            chunk_embeddings.push(embedding);
        }

        // Combine embeddings using mean pooling (average)
        let dimension = self.dimension();
        let mut combined = vec![0.0f32; dimension];
        
        for embedding in &chunk_embeddings {
            for (i, &value) in embedding.iter().enumerate() {
                combined[i] += value;
            }
        }

        // Average by number of chunks
        let num_chunks = chunk_embeddings.len() as f32;
        for value in &mut combined {
            *value /= num_chunks;
        }

        Ok(combined)
    }
}

