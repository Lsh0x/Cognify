use std::collections::HashMap;

/// Generates folder names from tags or cluster summaries
pub struct FolderGenerator;

impl FolderGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Generate folder name from dominant tags
    pub fn from_tags(&self, tags: &[String]) -> String {
        if tags.is_empty() {
            return "uncategorized".to_string();
        }

        // Count tag frequency
        let mut tag_counts: HashMap<&String, usize> = HashMap::new();
        for tag in tags {
            *tag_counts.entry(tag).or_insert(0) += 1;
        }

        // Get the most frequent tag, or first tag if tie
        let dominant_tag = tag_counts
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(tag, _)| *tag)
            .unwrap_or(&tags[0]);

        // Sanitize folder name (replace spaces with hyphens, lowercase)
        dominant_tag
            .to_lowercase()
            .replace(' ', "-")
            .replace('_', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect::<String>()
    }

    /// Generate folder name from multiple tags (combines top 2-3)
    pub fn from_multiple_tags(&self, tags: &[String], max_tags: usize) -> String {
        if tags.is_empty() {
            return "uncategorized".to_string();
        }

        let mut tag_counts: HashMap<&String, usize> = HashMap::new();
        for tag in tags {
            *tag_counts.entry(tag).or_insert(0) += 1;
        }

        // Get top N tags
        let mut sorted_tags: Vec<_> = tag_counts.iter().collect();
        sorted_tags.sort_by_key(|(_, &count)| std::cmp::Reverse(count));

        let selected_tags: Vec<String> = sorted_tags
            .iter()
            .take(max_tags.min(tags.len()))
            .map(|(tag, _)| {
                tag.to_lowercase()
                    .replace(' ', "-")
                    .replace('_', "-")
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '-')
                    .collect()
            })
            .collect();

        selected_tags.join("_")
    }
}

impl Default for FolderGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator_from_tags_single() {
        let generator = FolderGenerator::new();
        let tags = vec!["programming".to_string()];
        let folder = generator.from_tags(&tags);
        assert_eq!(folder, "programming");
    }

    #[test]
    fn test_generator_from_tags_multiple() {
        let generator = FolderGenerator::new();
        let tags = vec!["programming".to_string(), "rust".to_string(), "programming".to_string()];
        let folder = generator.from_tags(&tags);
        assert_eq!(folder, "programming"); // Most frequent
    }

    #[test]
    fn test_generator_from_tags_empty() {
        let generator = FolderGenerator::new();
        let tags = vec![];
        let folder = generator.from_tags(&tags);
        assert_eq!(folder, "uncategorized");
    }

    #[test]
    fn test_generator_sanitizes_name() {
        let generator = FolderGenerator::new();
        let tags = vec!["AI Research".to_string()];
        let folder = generator.from_tags(&tags);
        assert_eq!(folder, "ai-research");
    }

    #[test]
    fn test_generator_from_multiple_tags() {
        let generator = FolderGenerator::new();
        let tags = vec!["programming".to_string(), "rust".to_string(), "tutorial".to_string()];
        let folder = generator.from_multiple_tags(&tags, 2);
        assert!(folder.contains("programming") || folder.contains("rust") || folder.contains("tutorial"));
        assert!(folder.contains("_"));
    }
}

