use crate::embeddings::EmbeddingProvider;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Local embedding provider using Ollama API
pub struct LocalEmbeddingProvider {
    base_url: String,
    model: String,
    dimension: usize,
}

impl LocalEmbeddingProvider {
    /// Create a new local embedding provider using Ollama
    /// Default model: nomic-embed-text (768 dimensions)
    pub fn new(base_url: Option<&str>, model: Option<&str>) -> Self {
        Self {
            base_url: base_url.unwrap_or("http://127.0.0.1:11434").to_string(),
            model: model.unwrap_or("nomic-embed-text").to_string(),
            dimension: 768, // nomic-embed-text dimension
        }
    }

    /// Create with mxbai-embed-large model (1024 dimensions)
    pub fn with_large_model(base_url: Option<&str>) -> Self {
        Self {
            base_url: base_url.unwrap_or("http://127.0.0.1:11434").to_string(),
            model: "mxbai-embed-large".to_string(),
            dimension: 1024, // mxbai-embed-large dimension
        }
    }
}

#[derive(Serialize, Deserialize)]
struct OllamaEmbeddingRequest {
    model: String,
    prompt: String,
}

#[derive(Deserialize)]
struct OllamaEmbeddingResponse {
    embedding: Vec<f32>,
}

#[async_trait::async_trait]
impl EmbeddingProvider for LocalEmbeddingProvider {
    async fn compute_embedding(&self, content: &str) -> Result<Vec<f32>> {
        let url = format!("{}/api/embeddings", self.base_url);
        
        let request = OllamaEmbeddingRequest {
            model: self.model.clone(),
            prompt: content.to_string(),
        };

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to connect to Ollama")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Ollama API returned error: {}",
                response.status()
            );
        }

        let embedding_response: OllamaEmbeddingResponse = response
            .json()
            .await
            .context("Failed to parse Ollama embedding response")?;

        if embedding_response.embedding.len() != self.dimension {
            anyhow::bail!(
                "Expected embedding dimension {}, got {}",
                self.dimension,
                embedding_response.embedding.len()
            );
        }

        Ok(embedding_response.embedding)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_embedding_provider_creation() {
        let provider = LocalEmbeddingProvider::new(None, None);
        assert_eq!(provider.base_url, "http://127.0.0.1:11434");
        assert_eq!(provider.model, "nomic-embed-text");
        assert_eq!(provider.dimension(), 768);
    }

    #[test]
    fn test_local_embedding_provider_custom_url() {
        let provider = LocalEmbeddingProvider::new(Some("http://localhost:8080"), None);
        assert_eq!(provider.base_url, "http://localhost:8080");
    }

    #[test]
    fn test_local_embedding_provider_custom_model() {
        let provider = LocalEmbeddingProvider::new(None, Some("custom-model"));
        assert_eq!(provider.model, "custom-model");
    }

    #[test]
    fn test_local_embedding_provider_large_model() {
        let provider = LocalEmbeddingProvider::with_large_model(None);
        assert_eq!(provider.model, "mxbai-embed-large");
        assert_eq!(provider.dimension(), 1024);
    }

    #[tokio::test]
    #[ignore] // Requires Ollama server running
    async fn test_local_embedding_provider_compute() {
        let provider = LocalEmbeddingProvider::new(None, None);
        let embedding = provider.compute_embedding("test content").await.unwrap();
        assert_eq!(embedding.len(), 768);
        assert!(!embedding.iter().all(|&x| x == 0.0)); // Not all zeros
    }
}

