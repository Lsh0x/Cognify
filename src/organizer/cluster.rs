use std::collections::HashMap;

/// Represents a cluster of similar files based on embeddings
#[derive(Debug, Clone)]
pub struct FileCluster {
    pub files: Vec<(usize, Vec<f32>)>, // (file_index, embedding)
    pub centroid: Vec<f32>,            // Average embedding (centroid)
    pub dominant_tags: Vec<String>,    // Most common tags in this cluster
}

/// Simple clustering based on cosine similarity
pub struct EmbeddingClusterer {
    similarity_threshold: f32,
}

impl EmbeddingClusterer {
    /// Create a new clusterer with a similarity threshold
    /// Default threshold: 0.7 (files with >70% similarity are grouped together)
    pub fn new(threshold: f32) -> Self {
        Self {
            similarity_threshold: threshold.max(0.0).min(1.0),
        }
    }

    /// Cluster files based on their embeddings
    /// Returns a map from cluster_id to FileCluster
    pub fn cluster_files(
        &self,
        embeddings: &[(usize, Vec<f32>)],
        tags_map: &HashMap<usize, Vec<String>>,
    ) -> HashMap<usize, FileCluster> {
        if embeddings.is_empty() {
            return HashMap::new();
        }

        let mut clusters: HashMap<usize, FileCluster> = HashMap::new();
        let mut cluster_id = 0;
        let mut assigned: Vec<Option<usize>> = vec![None; embeddings.len()];

        // Simple clustering: assign each file to the most similar existing cluster
        // or create a new cluster if no similar one exists
        for (idx, (file_idx, embedding)) in embeddings.iter().enumerate() {
            let mut best_cluster: Option<usize> = None;
            let mut best_similarity = self.similarity_threshold;

            // Find the most similar existing cluster
            for (cluster_idx, cluster) in clusters.iter() {
                let similarity = cosine_similarity(embedding, &cluster.centroid);
                if similarity > best_similarity {
                    best_similarity = similarity;
                    best_cluster = Some(*cluster_idx);
                }
            }

            // Assign to best cluster or create new one
            if let Some(cluster_idx) = best_cluster {
                // Add to existing cluster
                let cluster = clusters.get_mut(&cluster_idx).unwrap();
                cluster.files.push((*file_idx, embedding.clone()));
                // Update centroid
                cluster.centroid = self.compute_centroid(&cluster.files);
                assigned[idx] = Some(cluster_idx);
            } else {
                // Create new cluster
                let tags = tags_map.get(file_idx).cloned().unwrap_or_default();
                let cluster = FileCluster {
                    files: vec![(*file_idx, embedding.clone())],
                    centroid: embedding.clone(),
                    dominant_tags: self.extract_dominant_tags(&tags),
                };
                clusters.insert(cluster_id, cluster);
                assigned[idx] = Some(cluster_id);
                cluster_id += 1;
            }
        }

        // Update dominant tags for each cluster based on all files in it
        for cluster in clusters.values_mut() {
            let mut tag_counts: HashMap<String, usize> = HashMap::new();
            for (file_idx, _) in &cluster.files {
                if let Some(tags) = tags_map.get(file_idx) {
                    for tag in tags {
                        *tag_counts.entry(tag.clone()).or_insert(0) += 1;
                    }
                }
            }
            // Get top 3 most common tags
            let mut sorted_tags: Vec<(String, usize)> = tag_counts.into_iter().collect();
            sorted_tags.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
            cluster.dominant_tags = sorted_tags
                .iter()
                .take(3)
                .map(|(tag, _)| tag.clone())
                .collect();
        }

        clusters
    }

    /// Compute centroid (average embedding) for a group of files
    fn compute_centroid(&self, files: &[(usize, Vec<f32>)]) -> Vec<f32> {
        if files.is_empty() {
            return Vec::new();
        }

        let dimension = files[0].1.len();
        let mut centroid = vec![0.0; dimension];

        for (_, embedding) in files {
            for (i, &value) in embedding.iter().enumerate() {
                if i < dimension {
                    centroid[i] += value;
                }
            }
        }

        let count = files.len() as f32;
        for val in &mut centroid {
            *val /= count;
        }

        centroid
    }

    /// Extract dominant tags from a list of tags
    fn extract_dominant_tags(&self, tags: &[String]) -> Vec<String> {
        // Return top tags, prioritizing important categories
        let mut sorted_tags = tags.to_vec();
        sorted_tags.sort();
        sorted_tags.dedup();
        sorted_tags.into_iter().take(3).collect()
    }
}

/// Compute cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_clusterer_empty() {
        let clusterer = EmbeddingClusterer::new(0.7);
        let embeddings = vec![];
        let tags_map = HashMap::new();
        let clusters = clusterer.cluster_files(&embeddings, &tags_map);
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_clusterer_single_file() {
        let clusterer = EmbeddingClusterer::new(0.7);
        let embeddings = vec![(0, vec![0.5, 0.5, 0.5])];
        let mut tags_map = HashMap::new();
        tags_map.insert(0, vec!["document".to_string()]);
        let clusters = clusterer.cluster_files(&embeddings, &tags_map);
        assert_eq!(clusters.len(), 1);
    }
}

