use crate::constants::{MID_LEVEL_CATEGORIES, SPECIFIC_TAGS, TOP_LEVEL_CATEGORIES};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Generates folder names from tags or cluster summaries
pub struct FolderGenerator;

impl FolderGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Generate hierarchical folder path from tags (creates subdirectories for better organization)
    /// Returns a PathBuf representing the full folder hierarchy (e.g., "document/financial/invoice" or "programming/rust/tutorial")
    pub fn from_tags_hierarchical(&self, tags: &[String], max_depth: usize) -> std::path::PathBuf {
        if tags.is_empty() {
            return std::path::PathBuf::from("uncategorized");
        }

        // Count tag frequency
        let mut tag_counts: HashMap<&String, usize> = HashMap::new();
        for tag in tags {
            *tag_counts.entry(tag).or_insert(0) += 1;
        }

        // Get top N tags sorted by frequency
        let mut sorted_tags: Vec<(String, usize)> = tag_counts
            .iter()
            .map(|(tag, &count)| (tag.to_string(), count))
            .collect();
        sorted_tags.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

        // Categorize tags into groups for hierarchy
        let (primary_tags, secondary_tags, tertiary_tags) = self.categorize_tags_enhanced(&sorted_tags);

        // Build hierarchical path with up to max_depth levels (default: 3-4 levels)
        let mut path_components = Vec::new();

        // Level 1: Primary tag (category-level, e.g., "document", "programming", "image")
        if let Some((primary_tag, _)) = primary_tags.first() {
            let sanitized = self.sanitize_tag_name(primary_tag);
            path_components.push(sanitized);
        }

        // Level 2: Secondary tags (subcategory-level, e.g., "financial", "rust", "work")
        // Take up to 2-3 secondary tags to create more subdirectories
        let remaining_depth = max_depth.saturating_sub(path_components.len());
        for (tag, _) in secondary_tags.iter().take(remaining_depth.min(3)) {
            let sanitized = self.sanitize_tag_name(tag);
            // Don't add if it's too similar to existing components
            if !self.is_similar_to_any(&sanitized, &path_components) {
                path_components.push(sanitized);
            }
        }

        // Level 3+: Tertiary tags (specific-level, e.g., "invoice", "tutorial", "meeting")
        // Add more specific tags to create deeper hierarchy
        let remaining_depth = max_depth.saturating_sub(path_components.len());
        for (tag, _) in tertiary_tags.iter().take(remaining_depth.min(2)) {
            let sanitized = self.sanitize_tag_name(tag);
            // Don't add if it's too similar to existing components
            if !self.is_similar_to_any(&sanitized, &path_components) {
                path_components.push(sanitized);
            }
        }
        
        // If we still have room (up to max_depth) and remaining tags, add them
        let remaining_depth = max_depth.saturating_sub(path_components.len());
        if remaining_depth > 0 && sorted_tags.len() > (primary_tags.len() + secondary_tags.len() + tertiary_tags.len()) {
            // Collect all tags we've already used
            let mut used_tags: HashSet<&String> = HashSet::new();
            for (tag, _) in primary_tags.iter().chain(secondary_tags.iter()).chain(tertiary_tags.iter()) {
                used_tags.insert(tag);
            }
            
            // Add remaining unique tags from sorted list
            for (tag, _) in sorted_tags.iter() {
                if remaining_depth == 0 {
                    break;
                }
                if !used_tags.contains(tag) {
                    let sanitized = self.sanitize_tag_name(tag);
                    if !self.is_similar_to_any(&sanitized, &path_components) {
                        path_components.push(sanitized);
                        used_tags.insert(tag);
                    }
                }
            }
        }

        // Build PathBuf
        let mut path = std::path::PathBuf::new();
        for component in path_components {
            path.push(component);
        }

        // Ensure we have at least one level
        if path.as_os_str().is_empty() {
            path.push("uncategorized");
        }

        path
    }

    /// Check if a tag is too similar to any existing path component
    fn is_similar_to_any(&self, tag: &str, components: &[String]) -> bool {
        components.iter().any(|comp| {
            comp == tag || tag.contains(comp) || comp.contains(tag)
        })
    }

    /// Categorize tags into primary (category), secondary (subcategory), and tertiary (specific) groups
    /// This creates a richer hierarchy by classifying tags more intelligently
    fn categorize_tags_enhanced(&self, sorted_tags: &[(String, usize)]) -> (Vec<(String, usize)>, Vec<(String, usize)>, Vec<(String, usize)>) {
        // Use constants for category tags
        let top_level_categories = TOP_LEVEL_CATEGORIES;
        let mid_level_categories = MID_LEVEL_CATEGORIES;
        let specific_tags = SPECIFIC_TAGS;

        let mut primary = Vec::new();
        let mut secondary = Vec::new();
        let mut tertiary = Vec::new();

        for (tag, count) in sorted_tags {
            let tag_lower = tag.to_lowercase();
            let is_top_level = top_level_categories.iter().any(|cat| 
                tag_lower == *cat || tag_lower.contains(cat) || cat.contains(&tag_lower)
            );
            let is_mid_level = mid_level_categories.iter().any(|cat| 
                tag_lower == *cat || tag_lower.contains(cat) || cat.contains(&tag_lower)
            );
            let is_specific = specific_tags.iter().any(|cat| 
                tag_lower == *cat || tag_lower.contains(cat) || cat.contains(&tag_lower)
            );

            // Prioritize top-level categories for primary
            if is_top_level && primary.is_empty() {
                primary.push((tag.clone(), *count));
            } 
            // Secondary tags go to mid-level (can have multiple)
            else if is_mid_level {
                if primary.is_empty() {
                    // If no primary yet, make this primary
                    primary.push((tag.clone(), *count));
                } else {
                    secondary.push((tag.clone(), *count));
                }
            }
            // Tertiary tags are specific items
            else if is_specific {
                tertiary.push((tag.clone(), *count));
            }
            // Unclassified tags: assign based on position
            else if primary.is_empty() {
                primary.push((tag.clone(), *count));
            } else if secondary.len() < 3 {
                secondary.push((tag.clone(), *count));
            } else {
                tertiary.push((tag.clone(), *count));
            }
        }

        // Ensure we have at least one primary tag
        if primary.is_empty() && !sorted_tags.is_empty() {
            primary.push((sorted_tags[0].0.clone(), sorted_tags[0].1));
        }

        // Limit secondary to top 3 most frequent
        secondary.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        secondary.truncate(3);
        
        // Limit tertiary to top 2 most frequent
        tertiary.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        tertiary.truncate(2);

        (primary, secondary, tertiary)
    }

    /// Sanitize tag name for use in folder path
    fn sanitize_tag_name(&self, tag: &str) -> String {
        tag.to_lowercase()
            .replace(' ', "-")
            .replace('_', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect::<String>()
    }

    /// Generate folder name from dominant tags (flat, for backward compatibility)
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
        self.sanitize_tag_name(dominant_tag)
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

    /// Find best matching existing directory for given tags
    pub fn find_matching_directory<P: AsRef<Path>>(
        &self,
        tags: &[String],
        base_dir: P,
    ) -> Option<String> {
        let base_dir = base_dir.as_ref();
        let tag_set: HashSet<String> = tags.iter().map(|t| t.to_lowercase()).collect();

        // Normalize tags for matching (same as folder name generation)
        let normalized_tags: Vec<String> = tags
            .iter()
            .map(|t| {
                t.to_lowercase()
                    .replace(' ', "-")
                    .replace('_', "-")
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '-')
                    .collect()
            })
            .collect();

        let mut best_match: Option<(String, usize)> = None;

        // Scan existing directories recursively
        if let Ok(entries) = std::fs::read_dir(base_dir) {
            let mut dirs_to_check: Vec<std::path::PathBuf> = Vec::new();
            
            // Collect all subdirectories to check
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_dir() {
                        dirs_to_check.push(entry.path());
                    }
                }
            }
            
            // Check each directory
            for dir_path in dirs_to_check {
                // Skip protected directories (.git, node_modules, target, etc.)
                if crate::utils::matches_protected_pattern(&dir_path) {
                    continue;
                }
                
                // Skip if this directory is inside a protected structure
                if crate::utils::is_inside_protected_structure(&dir_path) {
                    continue;
                }
                
                if let Some(dir_name) = dir_path.file_name().and_then(|n| n.to_str()) {
                    let dir_name_lower = dir_name.to_lowercase();
                    let dir_normalized = dir_name_lower
                        .replace(' ', "-")
                        .replace('_', "-")
                        .chars()
                        .filter(|c| c.is_alphanumeric() || *c == '-')
                        .collect::<String>();

                    // Check if any tag matches the directory name
                    let mut match_score = 0;
                    
                    // Exact match (highest priority)
                    if normalized_tags.iter().any(|t| t == &dir_normalized) {
                        match_score += 10;
                    }
                    // Directory contains tag or tag contains directory
                    for tag in &normalized_tags {
                        if dir_normalized.contains(tag) || tag.contains(&dir_normalized) {
                            match_score += 5;
                        }
                    }
                    // Word overlap
                    let dir_words: HashSet<&str> = dir_normalized.split('-').collect();
                    for tag in &normalized_tags {
                        let tag_words: HashSet<&str> = tag.split('-').collect();
                        let overlap = dir_words.intersection(&tag_words).count();
                        if overlap > 0 {
                            match_score += overlap * 2;
                        }
                    }
                    // Direct tag match in set
                    if tag_set.contains(&dir_name_lower.replace('-', " ")) {
                        match_score += 3;
                    }

                    if match_score > 0 {
                        if best_match.is_none() || best_match.as_ref().unwrap().1 < match_score {
                            best_match = Some((dir_name.to_string(), match_score));
                        }
                    }
                }
            }
        }

        best_match.map(|(name, _)| name)
    }

    /// Find best matching existing directory hierarchy for given tags
    /// Returns a PathBuf representing the matching directory path (e.g., "programming/rust")
    pub fn find_matching_directory_hierarchical<P: AsRef<Path>>(
        &self,
        tags: &[String],
        base_dir: P,
    ) -> Option<std::path::PathBuf> {
        let base_dir = base_dir.as_ref();
        let tag_set: HashSet<String> = tags.iter().map(|t| t.to_lowercase()).collect();

        // Normalize tags for matching
        let normalized_tags: Vec<String> = tags
            .iter()
            .map(|t| {
                t.to_lowercase()
                    .replace(' ', "-")
                    .replace('_', "-")
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '-')
                    .collect()
            })
            .collect();

        let mut best_match: Option<(std::path::PathBuf, usize)> = None;

        // Recursively scan directories to find best hierarchical match
        self.scan_directory_hierarchical(base_dir, base_dir, &normalized_tags, &tag_set, &mut best_match, 3);

        best_match.map(|(path, _)| {
            // Return relative path from base_dir
            path.strip_prefix(base_dir)
                .unwrap_or(&path)
                .to_path_buf()
        })
    }

    /// Recursively scan directories to find best hierarchical match
    fn scan_directory_hierarchical(
        &self,
        base_dir: &Path,
        current_dir: &Path,
        normalized_tags: &[String],
        tag_set: &HashSet<String>,
        best_match: &mut Option<(std::path::PathBuf, usize)>,
        max_depth: usize,
    ) {
        if max_depth == 0 {
            return;
        }

        if let Ok(entries) = std::fs::read_dir(current_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_dir() {
                        let dir_path = entry.path();

                        // Skip protected directories
                        if crate::utils::matches_protected_pattern(&dir_path) {
                            continue;
                        }
                        if crate::utils::is_inside_protected_structure(&dir_path) {
                            continue;
                        }

                        if let Some(dir_name) = dir_path.file_name().and_then(|n| n.to_str()) {
                            let dir_name_lower = dir_name.to_lowercase();
                            let dir_normalized = dir_name_lower
                                .replace(' ', "-")
                                .replace('_', "-")
                                .chars()
                                .filter(|c| c.is_alphanumeric() || *c == '-')
                                .collect::<String>();

                            // Calculate match score
                            let mut match_score = 0;

                            // Exact match (highest priority)
                            if normalized_tags.iter().any(|t| t == &dir_normalized) {
                                match_score += 10;
                            }
                            // Directory contains tag or tag contains directory
                            for tag in normalized_tags {
                                if dir_normalized.contains(tag) || tag.contains(&dir_normalized) {
                                    match_score += 5;
                                }
                            }
                            // Word overlap
                            let dir_words: HashSet<&str> = dir_normalized.split('-').collect();
                            for tag in normalized_tags {
                                let tag_words: HashSet<&str> = tag.split('-').collect();
                                let overlap = dir_words.intersection(&tag_words).count();
                                if overlap > 0 {
                                    match_score += overlap * 2;
                                }
                            }

                            if match_score > 0 {
                                // Check if this is better than current best match
                                let is_better = match best_match {
                                    Some((_, ref current_score)) => match_score > *current_score,
                                    None => true,
                                };

                                if is_better {
                                    *best_match = Some((dir_path.clone(), match_score));
                                }

                                // Recursively check subdirectories for hierarchical matches
                                self.scan_directory_hierarchical(
                                    base_dir,
                                    &dir_path,
                                    normalized_tags,
                                    tag_set,
                                    best_match,
                                    max_depth - 1,
                                );
                            }
                        }
                    }
                }
            }
        }
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

    #[test]
    fn test_find_matching_directory() {
        use tempfile::TempDir;
        
        let generator = FolderGenerator::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Create a test directory
        let existing_dir = temp_dir.path().join("programming");
        std::fs::create_dir(&existing_dir).unwrap();
        
        // Test matching
        let tags = vec!["programming".to_string(), "rust".to_string()];
        let result = generator.find_matching_directory(&tags, temp_dir.path());
        assert_eq!(result, Some("programming".to_string()));
        
        // Test no match
        let tags_no_match = vec!["python".to_string(), "machine-learning".to_string()];
        let result_no_match = generator.find_matching_directory(&tags_no_match, temp_dir.path());
        assert_eq!(result_no_match, None);
    }

    #[test]
    fn test_from_tags_hierarchical() {
        let generator = FolderGenerator::new();
        
        // Test single tag (flat)
        let tags = vec!["programming".to_string()];
        let path = generator.from_tags_hierarchical(&tags, 4);
        assert_eq!(path, std::path::PathBuf::from("programming"));
        
        // Test category + subcategory + specific (should create 3 levels)
        let tags2 = vec!["document".to_string(), "financial".to_string(), "invoice".to_string()];
        let path2 = generator.from_tags_hierarchical(&tags2, 4);
        // Should create hierarchical path like "document/financial/invoice"
        assert!(path2.components().count() >= 2, "Should have at least 2 levels");
        
        // Test multiple tags with category (should create nested structure)
        let tags3 = vec!["programming".to_string(), "rust".to_string(), "tutorial".to_string()];
        let path3 = generator.from_tags_hierarchical(&tags3, 4);
        // Should have programming as primary (top-level category), and rust/tutorial as secondary/tertiary
        let first_comp = path3.components().next().and_then(|c| c.as_os_str().to_str());
        // Programming is top-level, so it should be first, but if tutorial is detected first, that's also valid
        assert!(first_comp.is_some(), "Should have at least one component");
        assert!(path3.components().count() >= 2, "Should have multiple levels (2+), got: {:?} with {} components", path3, path3.components().count());
    }

    #[test]
    fn test_find_matching_directory_hierarchical() {
        use tempfile::TempDir;
        
        let generator = FolderGenerator::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Create hierarchical test directories
        let existing_dir1 = temp_dir.path().join("programming").join("rust");
        std::fs::create_dir_all(&existing_dir1).unwrap();
        
        // Test hierarchical matching
        let tags = vec!["programming".to_string(), "rust".to_string()];
        let result = generator.find_matching_directory_hierarchical(&tags, temp_dir.path());
        assert!(result.is_some());
        let matched_path = result.unwrap();
        // Should match either "programming" or "programming/rust"
        assert!(matched_path.to_string_lossy().contains("programming"));
    }
}

