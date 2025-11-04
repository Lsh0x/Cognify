use crate::embeddings::EmbeddingProvider;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Text Embeddings Inference (TEI) provider from Hugging Face
/// Supports high-dimensional embeddings (e.g., 4096 dims from Qwen3-Embedding-8B)
pub struct TeiEmbeddingProvider {
    base_url: String,
    dimension: AtomicUsize, // Use AtomicUsize to allow runtime dimension updates (thread-safe)
}

impl TeiEmbeddingProvider {
    /// Create a new TEI embedding provider
    /// Default URL: http://127.0.0.1:8080
    pub fn new(base_url: Option<&str>, dimension: Option<usize>) -> Self {
        Self {
            base_url: base_url.unwrap_or("http://127.0.0.1:8080").to_string(),
            dimension: AtomicUsize::new(dimension.unwrap_or(4096)), // Default to 4096 for Qwen3-Embedding-8B
        }
    }
}

#[derive(Serialize)]
struct TeiEmbeddingRequest {
    inputs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    truncate: Option<bool>,
}

#[derive(Deserialize)]
struct TeiEmbeddingResponse {
    embeddings: Vec<Vec<f32>>,
}

#[async_trait::async_trait]
impl EmbeddingProvider for TeiEmbeddingProvider {
    async fn compute_embedding(&self, content: &str) -> Result<Vec<f32>> {
        // Ensure content is not empty
        let content = content.trim();
        if content.is_empty() {
            anyhow::bail!("Cannot generate embedding for empty content");
        }
        
        let url = format!("{}/embed", self.base_url);
        
        let request = TeiEmbeddingRequest {
            inputs: vec![content.to_string()],
            truncate: Some(true),
        };

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to connect to TEI server")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "TEI API returned error {}: {}",
                status,
                error_text
            );
        }

        let embedding_response: TeiEmbeddingResponse = response
            .json()
            .await
            .context("Failed to parse TEI embedding response")?;

        // TEI returns array of arrays (one embedding per input)
        if embedding_response.embeddings.is_empty() {
            anyhow::bail!("TEI returned empty embeddings array");
        }

        let embedding = embedding_response.embeddings[0].clone();

        // Handle empty or invalid embeddings
        if embedding.is_empty() {
            anyhow::bail!(
                "TEI returned empty embedding (dimension 0). This usually means the input content was too short or empty."
            );
        }

        // Update dimension if it doesn't match (model might have different dimension than expected)
        let actual_dimension = embedding.len();
        let expected_dimension = self.dimension.load(Ordering::Relaxed);
        if actual_dimension != expected_dimension {
            // Update the dimension to match what TEI actually returns
            eprintln!(
                "Info: TEI model returned embedding dimension {} (expected {}). Updating to match actual dimension.",
                actual_dimension, expected_dimension
            );
            self.dimension.store(actual_dimension, Ordering::Relaxed);
        }

        Ok(embedding)
    }

    fn dimension(&self) -> usize {
        self.dimension.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tei_embedding_provider_creation() {
        let provider = TeiEmbeddingProvider::new(None, None);
        assert_eq!(provider.base_url, "http://127.0.0.1:8080");
        assert_eq!(provider.dimension(), 4096);
    }
    
    #[test]
    fn test_tei_embedding_provider_with_explicit_dims() {
        let provider = TeiEmbeddingProvider::new(None, Some(2048));
        assert_eq!(provider.dimension(), 2048);
    }

    #[test]
    fn test_tei_embedding_provider_custom_url() {
        let provider = TeiEmbeddingProvider::new(Some("http://localhost:8081"), None);
        assert_eq!(provider.base_url, "http://localhost:8081");
    }

    #[tokio::test]
    #[ignore] // Requires TEI server running
    async fn test_tei_embedding_provider_compute() {
        let provider = TeiEmbeddingProvider::new(None, Some(4096));
        let embedding = provider.compute_embedding("test content").await.unwrap();
        assert_eq!(embedding.len(), 4096);
        assert!(!embedding.iter().all(|&x| x == 0.0)); // Not all zeros
    }
}

