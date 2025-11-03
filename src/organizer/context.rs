use crate::constants::COMMON_DIRECTORY_NAMES;
use std::path::Path;

/// Extract meaningful tags from file path context (filename, parent directories)
pub fn extract_tags_from_path<P: AsRef<Path>>(path: P) -> Vec<String> {
    let path = path.as_ref();
    let mut tags = Vec::new();

    // Extract from filename (remove extension)
    if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
        let filename_tags = extract_tags_from_string(file_stem);
        tags.extend(filename_tags);
    }

    // Extract from parent directories
    for ancestor in path.ancestors().skip(1) {
        if let Some(dir_name) = ancestor.file_name().and_then(|s| s.to_str()) {
            // Skip common directory names that aren't meaningful
            if !is_common_directory(dir_name) {
                let dir_tags = extract_tags_from_string(dir_name);
                tags.extend(dir_tags);
            }
        }
    }

    // Remove duplicates while preserving order
    let mut seen = std::collections::HashSet::new();
    tags.into_iter()
        .filter(|tag| seen.insert(tag.clone()))
        .collect()
}

/// Extract tags from a string by splitting on common delimiters
fn extract_tags_from_string(s: &str) -> Vec<String> {
    let mut tags = Vec::new();

    // Split on common delimiters: space, underscore, hyphen, camelCase, etc.
    let parts: Vec<&str> = s
        .split(|c: char| c.is_whitespace() || c == '_' || c == '-' || c == '.')
        .filter(|part| !part.is_empty() && part.len() > 1)
        .collect();

    for part in parts {
        let cleaned = clean_tag(part);
        if cleaned.len() >= 2 && cleaned.len() <= 30 {
            // Also split camelCase
            let camel_parts = split_camel_case(&cleaned);
            for camel_part in camel_parts {
                if camel_part.len() >= 2 {
                    tags.push(camel_part.to_lowercase());
                }
            }
        }
    }

    // If no meaningful parts found, try the whole string as one tag
    if tags.is_empty() {
        let cleaned = clean_tag(s);
        if cleaned.len() >= 2 && cleaned.len() <= 30 {
            tags.push(cleaned.to_lowercase());
        }
    }

    tags
}

/// Split camelCase or PascalCase string
fn split_camel_case(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();

    for ch in s.chars() {
        if ch.is_uppercase() && !current.is_empty() {
            parts.push(current.clone());
            current = ch.to_lowercase().collect::<String>();
        } else {
            current.extend(ch.to_lowercase());
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    if parts.is_empty() {
        vec![s.to_string()]
    } else {
        parts
    }
}

/// Clean a tag string (remove special chars, normalize)
fn clean_tag(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Check if a directory name is too common to be useful as a tag
fn is_common_directory(dir: &str) -> bool {
    COMMON_DIRECTORY_NAMES.contains(&dir.to_lowercase().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_tags_from_path_simple() {
        let path = PathBuf::from("/home/user/Documents/project-notes.txt");
        let tags = extract_tags_from_path(path);
        assert!(tags.contains(&"project".to_string()));
        assert!(tags.contains(&"notes".to_string()));
    }

    #[test]
    fn test_extract_tags_from_path_camelcase() {
        let path = PathBuf::from("/home/user/MyProjectFiles.pdf");
        let tags = extract_tags_from_path(path);
        assert!(tags.contains(&"my".to_string()));
        assert!(tags.contains(&"project".to_string()));
        assert!(tags.contains(&"files".to_string()));
    }

    #[test]
    fn test_extract_tags_from_path_underscores() {
        let path = PathBuf::from("/home/user/meeting_notes_2024.txt");
        let tags = extract_tags_from_path(path);
        assert!(tags.contains(&"meeting".to_string()));
        assert!(tags.contains(&"notes".to_string()));
        assert!(tags.contains(&"2024".to_string()));
    }

    #[test]
    fn test_split_camel_case() {
        assert_eq!(
            split_camel_case("MyProject"),
            vec!["my".to_string(), "project".to_string()]
        );
        assert_eq!(
            split_camel_case("camelCase"),
            vec!["camel".to_string(), "case".to_string()]
        );
    }
}

