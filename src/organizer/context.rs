use crate::constants::COMMON_DIRECTORY_NAMES;
use std::path::Path;

/// Extract meaningful tags from file path context (filename, parent directories)
pub fn extract_tags_from_path<P: AsRef<Path>>(path: P) -> Vec<String> {
    let path = path.as_ref();
    let mut tags = Vec::new();

    // Get current username to filter it out
    let current_user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default()
        .to_lowercase();

    // Extract from filename (remove extension)
    if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
        let filename_tags = extract_tags_from_string(file_stem);
        tags.extend(filename_tags);
    }

    // Extract from parent directories
    for ancestor in path.ancestors().skip(1) {
        if let Some(dir_name) = ancestor.file_name().and_then(|s| s.to_str()) {
            let dir_lower = dir_name.to_lowercase();
            
            // Skip common directory names, the current username, and system directories
            if !is_common_directory(&dir_lower) 
                && dir_lower != current_user 
                && dir_lower != "users" 
                && dir_lower != "home"
                && !is_username_directory(&dir_lower) {
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

/// Check if a directory name looks like a username directory
/// This helps filter out usernames from paths like /Users/username or /home/username
fn is_username_directory(dir: &str) -> bool {
    // First check if it's a common directory name - if so, it's not a username
    if is_common_directory(dir) {
        return false;
    }
    
    // Skip single-word directories that are likely usernames
    // These are typically short, lowercase, alphanumeric-only names
    // Common patterns: 2-8 characters, lowercase, alphanumeric or hyphen/underscore
    let is_short_alphanumeric = dir.len() >= 2 
        && dir.len() <= 8 
        && dir.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        && !dir.chars().any(|c| c.is_uppercase());
    
    // Also check common username patterns (short, lowercase, alphanumeric)
    is_short_alphanumeric
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
    fn test_extract_tags_from_path_filters_username() {
        // Test that username is filtered from paths like /Users/username/...
        let path = PathBuf::from("/Users/lsh/Documents/project-notes.txt");
        let tags = extract_tags_from_path(path);
        // Should not contain "lsh" (username)
        assert!(!tags.contains(&"lsh".to_string()));
        // Should still contain meaningful tags
        assert!(tags.contains(&"project".to_string()));
        assert!(tags.contains(&"notes".to_string()));
    }

    #[test]
    fn test_extract_tags_from_path_filters_home_dirs() {
        // Test that "Users", "home", etc. are filtered
        let path = PathBuf::from("/Users/user/Documents/file.txt");
        let tags = extract_tags_from_path(path);
        assert!(!tags.contains(&"users".to_string()));
        assert!(!tags.contains(&"home".to_string()));
    }

    #[test]
    fn test_is_username_directory() {
        // Test username detection
        assert!(is_username_directory("lsh"));      // Short lowercase
        assert!(is_username_directory("john"));    // Common username pattern
        assert!(is_username_directory("user123")); // Alphanumeric
        assert!(!is_username_directory("MyProject")); // Has uppercase
        assert!(!is_username_directory("verylongdirectoryname")); // Too long
        // Note: "project" matches the pattern (short, lowercase, alphanumeric)
        // but it's filtered elsewhere if it's in COMMON_DIRECTORY_NAMES
        // The real protection is the explicit username filtering in extract_tags_from_path
    }
    
    #[test]
    fn test_is_username_directory_common_words() {
        // Common directory names should not be considered usernames
        assert!(!is_username_directory("documents")); // In COMMON_DIRECTORY_NAMES
        assert!(!is_username_directory("downloads")); // In COMMON_DIRECTORY_NAMES
        assert!(!is_username_directory("home")); // In COMMON_DIRECTORY_NAMES
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

