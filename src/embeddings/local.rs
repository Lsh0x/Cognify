use crate::embeddings::EmbeddingProvider;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Local embedding provider using Ollama API
pub struct LocalEmbeddingProvider {
    base_url: String,
    model: String,
    dimension: AtomicUsize, // Use AtomicUsize to allow runtime dimension updates (thread-safe)
}

impl LocalEmbeddingProvider {
    /// Determine embedding dimension from model name
    pub(crate) fn get_dimension_for_model(model: &str) -> usize {
        // Known model dimensions
        match model {
            m if m.contains("mxbai-embed-large") => 1024,
            m if m.contains("mxbai-embed") => 1024, // mxbai-embed variants are typically 1024
            m if m.contains("nomic-embed") => 768,
            m if m.contains("e5") => 768, // Common E5 models
            _ => {
                // Default to 768 for unknown models, but we'll validate at runtime
                // This allows for flexibility while still having a default
                768
            }
        }
    }

    /// Create a new local embedding provider using Ollama
    /// Uses provided dimension, or auto-detects from model name if not provided
    pub fn new(base_url: Option<&str>, model: Option<&str>, dimension: Option<usize>) -> Self {
        let model_name = model.unwrap_or("nomic-embed-text").to_string();
        let dimension = dimension.unwrap_or_else(|| Self::get_dimension_for_model(&model_name));
        
        Self {
            base_url: base_url.unwrap_or("http://127.0.0.1:11434").to_string(),
            model: model_name,
            dimension: AtomicUsize::new(dimension),
        }
    }

    /// Create with mxbai-embed-large model (1024 dimensions)
    pub fn with_large_model(base_url: Option<&str>) -> Self {
        Self {
            base_url: base_url.unwrap_or("http://127.0.0.1:11434").to_string(),
            model: "mxbai-embed-large".to_string(),
            dimension: AtomicUsize::new(1024), // mxbai-embed-large dimension
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
        // Ensure content is not empty and has minimum length
        let content = content.trim();
        if content.is_empty() {
            anyhow::bail!("Cannot generate embedding for empty content");
        }
        
        // Some embedding models require minimum content length
        if content.len() < 3 {
            anyhow::bail!("Content too short ({} chars) to generate embedding. Minimum: 3 characters", content.len());
        }
        
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

        // Handle empty or invalid embeddings
        if embedding_response.embedding.is_empty() {
            anyhow::bail!(
                "Ollama returned empty embedding (dimension 0). This usually means the input content was too short or empty."
            );
        }

        // If the dimension doesn't match, update it (model might have different dimension than expected)
        // This allows for flexibility with different model variants
        let actual_dimension = embedding_response.embedding.len();
        let expected_dimension = self.dimension.load(Ordering::Relaxed);
        if actual_dimension != expected_dimension {
            // Update the dimension to match what Ollama actually returns
            eprintln!(
                "Info: Model '{}' returned embedding dimension {} (expected {}). Updating to match actual dimension.",
                self.model, actual_dimension, expected_dimension
            );
            self.dimension.store(actual_dimension, Ordering::Relaxed);
        }

        Ok(embedding_response.embedding)
    }

    fn dimension(&self) -> usize {
        self.dimension.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_embedding_provider_creation() {
        let provider = LocalEmbeddingProvider::new(None, None, None);
        assert_eq!(provider.base_url, "http://127.0.0.1:11434");
        assert_eq!(provider.model, "nomic-embed-text");
        assert_eq!(provider.dimension(), 768);
    }
    
    #[test]
    fn test_local_embedding_provider_with_explicit_dims() {
        let provider = LocalEmbeddingProvider::new(None, Some("custom-model"), Some(1024));
        assert_eq!(provider.model, "custom-model");
        assert_eq!(provider.dimension(), 1024);
    }
    
    #[test]
    fn test_get_dimension_for_model() {
        assert_eq!(LocalEmbeddingProvider::get_dimension_for_model("mxbai-embed-large"), 1024);
        assert_eq!(LocalEmbeddingProvider::get_dimension_for_model("nomic-embed-text"), 768);
        assert_eq!(LocalEmbeddingProvider::get_dimension_for_model("mxbai-embed-v1"), 1024);
        assert_eq!(LocalEmbeddingProvider::get_dimension_for_model("unknown-model"), 768); // default
    }

    #[test]
    fn test_local_embedding_provider_custom_url() {
        let provider = LocalEmbeddingProvider::new(Some("http://localhost:8080"), None, None);
        assert_eq!(provider.base_url, "http://localhost:8080");
    }

    #[test]
    fn test_local_embedding_provider_custom_model() {
        let provider = LocalEmbeddingProvider::new(None, Some("custom-model"), None);
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
        let provider = LocalEmbeddingProvider::new(None, None, None);
        let embedding = provider.compute_embedding("test content").await.unwrap();
        assert_eq!(embedding.len(), 768);
        assert!(!embedding.iter().all(|&x| x == 0.0)); // Not all zeros
    }
}

