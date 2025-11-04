use crate::embeddings::EmbeddingProvider;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Multi-Ollama embedding provider with load balancing and automatic failover
/// Supports multiple Ollama servers with round-robin distribution and automatic fallback
pub struct MultiOllamaEmbeddingProvider {
    urls: Vec<String>,
    model: String,
    dimension: AtomicUsize,
    current_index: AtomicUsize, // For round-robin
}

impl MultiOllamaEmbeddingProvider {
    /// Create a new multi-Ollama provider with a list of server URLs
    pub fn new(urls: Vec<String>, model: Option<&str>, dimension: Option<usize>) -> Self {
        let model_name = model.unwrap_or("nomic-embed-text").to_string();
        let dimension = dimension.unwrap_or_else(|| {
            LocalEmbeddingProvider::get_dimension_for_model(&model_name)
        });

        Self {
            urls,
            model: model_name,
            dimension: AtomicUsize::new(dimension),
            current_index: AtomicUsize::new(0),
        }
    }

    /// Get the next URL in round-robin fashion
    fn get_next_url(&self) -> String {
        let index = self.current_index.fetch_add(1, Ordering::Relaxed);
        let url = &self.urls[index % self.urls.len()];
        url.clone()
    }

    /// Try to compute embedding using a specific URL
    async fn try_compute_with_url(&self, url: &str, content: &str) -> Result<Vec<f32>> {
        let full_url = format!("{}/api/embeddings", url);
        
        let request = OllamaEmbeddingRequest {
            model: self.model.clone(),
            prompt: content.to_string(),
        };

        let client = reqwest::Client::new();
        let response = client
            .post(&full_url)
            .json(&request)
            .timeout(std::time::Duration::from_secs(60)) // 60 second timeout
            .send()
            .await
            .with_context(|| format!("Failed to connect to Ollama at {}", url))?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Ollama API at {} returned error: {}",
                url,
                response.status()
            );
        }

        let embedding_response: OllamaEmbeddingResponse = response
            .json()
            .await
            .with_context(|| format!("Failed to parse Ollama response from {}", url))?;

        if embedding_response.embedding.is_empty() {
            anyhow::bail!(
                "Ollama at {} returned empty embedding",
                url
            );
        }

        // Update dimension if needed
        let actual_dimension = embedding_response.embedding.len();
        let expected_dimension = self.dimension.load(Ordering::Relaxed);
        if actual_dimension != expected_dimension {
            eprintln!(
                "Info: Model '{}' at {} returned embedding dimension {} (expected {}). Updating to match actual dimension.",
                self.model, url, actual_dimension, expected_dimension
            );
            self.dimension.store(actual_dimension, Ordering::Relaxed);
        }

        Ok(embedding_response.embedding)
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

// Import LocalEmbeddingProvider for dimension detection
use crate::embeddings::local::LocalEmbeddingProvider;

#[async_trait::async_trait]
impl EmbeddingProvider for MultiOllamaEmbeddingProvider {
    async fn compute_embedding(&self, content: &str) -> Result<Vec<f32>> {
        // Ensure content is valid
        let content = content.trim();
        if content.is_empty() {
            anyhow::bail!("Cannot generate embedding for empty content");
        }
        
        if content.len() < 3 {
            anyhow::bail!("Content too short ({} chars) to generate embedding. Minimum: 3 characters", content.len());
        }

        if self.urls.is_empty() {
            anyhow::bail!("No Ollama servers configured");
        }

        // Try round-robin first, then fallback to all servers if that fails
        let start_index = self.current_index.load(Ordering::Relaxed) % self.urls.len();
        let mut last_error = None;

        // Try starting from the round-robin URL, then try all others
        for offset in 0..self.urls.len() {
            let index = (start_index + offset) % self.urls.len();
            let url = &self.urls[index];

            match self.try_compute_with_url(url, content).await {
                Ok(embedding) => {
                    // Success! Update round-robin index for next time
                    self.current_index.store(index + 1, Ordering::Relaxed);
                    return Ok(embedding);
                }
                Err(e) => {
                    last_error = Some((url.clone(), e));
                    // Continue to next server
                }
            }
        }

        // All servers failed
        if let Some((url, error)) = last_error {
            anyhow::bail!(
                "All Ollama servers failed. Last error from {}: {}",
                url,
                error
            );
        } else {
            anyhow::bail!("All Ollama servers failed");
        }
    }

    fn dimension(&self) -> usize {
        self.dimension.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_ollama_provider_creation() {
        let urls = vec![
            "http://127.0.0.1:11434".to_string(),
            "http://127.0.0.1:11435".to_string(),
        ];
        let provider = MultiOllamaEmbeddingProvider::new(urls.clone(), None, None);
        assert_eq!(provider.urls, urls);
        assert_eq!(provider.model, "nomic-embed-text");
        assert_eq!(provider.dimension(), 768);
    }

    #[test]
    fn test_multi_ollama_round_robin() {
        let urls = vec![
            "http://127.0.0.1:11434".to_string(),
            "http://127.0.0.1:11435".to_string(),
            "http://127.0.0.1:11436".to_string(),
        ];
        let provider = MultiOllamaEmbeddingProvider::new(urls, None, None);
        
        // First call should use index 0
        let url1 = provider.get_next_url();
        assert_eq!(url1, "http://127.0.0.1:11434");
        
        // Second call should use index 1
        let url2 = provider.get_next_url();
        assert_eq!(url2, "http://127.0.0.1:11435");
        
        // Third call should use index 2
        let url3 = provider.get_next_url();
        assert_eq!(url3, "http://127.0.0.1:11436");
        
        // Fourth call should wrap around to index 0
        let url4 = provider.get_next_url();
        assert_eq!(url4, "http://127.0.0.1:11434");
    }
}

