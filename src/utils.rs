use crate::constants::{
    BUNDLE_EXTENSIONS, PROTECTED_DIR_PATTERNS, PROTECTED_PATTERNS,
};
use anyhow::Result;
use blake3;
use std::fs::File;
use std::io::Read;

/// Compute Blake3 hash of file contents
pub fn compute_file_hash(file_path: &std::path::Path) -> Result<String> {
    let mut file = File::open(file_path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

/// Get file extension from path (without the dot)
pub fn get_extension(path: &std::path::Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
}

/// Check if a file is a macOS metadata file (starts with "._")
/// These files are created by macOS to store extended attributes and are often binary
pub fn is_macos_metadata_file(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|name| name.starts_with("._"))
        .unwrap_or(false)
}

/// Check if a directory path matches any protected pattern
pub fn matches_protected_pattern(path: &std::path::Path) -> bool {
    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        if PROTECTED_PATTERNS.iter().any(|pattern| {
            // Exact match for most patterns
            if file_name == *pattern {
                return true;
            }
            // Special case: bundle extensions that end with the pattern
            // (e.g., "MyApp.app", "MyFramework.framework", "MyPlugin.plugin")
            if BUNDLE_EXTENSIONS.contains(pattern) && file_name.ends_with(pattern) {
                return true;
            }
            false
        }) {
            return true;
        }
    }
    false
}

/// Check if a path is inside a protected directory structure
/// This includes version control systems and project directories
/// 
/// `base_dir` is the root directory being organized - we won't check beyond this
pub fn is_inside_protected_structure<P: AsRef<std::path::Path>>(path: P) -> bool {
    is_inside_protected_structure_with_base(path, None::<&std::path::Path>)
}

/// Check if a path is inside a protected directory structure, with an optional base directory limit
/// If `base_dir` is provided, we won't check beyond this directory
pub fn is_inside_protected_structure_with_base<P: AsRef<std::path::Path>, B: AsRef<std::path::Path>>(
    path: P,
    base_dir: Option<B>,
) -> bool {
    use std::path::Path;
    
    let path = path.as_ref();
    let base_dir_path = base_dir.as_ref().map(|b| b.as_ref());
    let mut current = if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        path
    };

    // Normalize base_dir if provided
    let base_dir_canonical = if let Some(base) = base_dir_path {
        if let Ok(canon) = base.canonicalize() {
            Some(canon)
        } else {
            None
        }
    } else {
        None
    };

    // Walk up the directory tree looking for protected patterns
    // Stop if we've gone beyond the base directory
    let mut depth = 0;
    const MAX_DEPTH: usize = 20; // Safety limit to prevent infinite loops
    
    while depth < MAX_DEPTH {
        let parent = match current.parent() {
            Some(p) => p,
            None => break, // Reached filesystem root
        };

        // Stop if we've gone beyond the base directory
        // We check AFTER getting the parent, so we check the current directory
        // before moving up
        if let Some(ref base_canon) = base_dir_canonical {
            if let Ok(current_canon) = current.canonicalize() {
                // If we're at the base directory or above it, stop checking
                // (we only want to check within the base directory)
                if current_canon == *base_canon {
                    break; // Reached base directory, stop here
                }
                // If we've gone above the base directory, stop
                if !current_canon.starts_with(base_canon) {
                    break;
                }
            }
        }
        // Check if current directory matches a protected pattern (like node_modules, target, etc.)
        if matches_protected_pattern(current) {
            return true;
        }
        
        // Check for protected directories INSIDE current (like .git, .hg, .app, .framework, etc.)
        // These are special because they're subdirectories, not the directory itself
        for pattern in PROTECTED_DIR_PATTERNS.iter() {
            // Check exact match (e.g., current directory is ".git")
            let protected_dir = current.join(pattern);
            if protected_dir.exists() && (protected_dir.is_dir() || protected_dir.is_file()) {
                return true;
            }
            // Check if any child directory ends with bundle extension
            if pattern.starts_with('.') {
                if let Ok(entries) = std::fs::read_dir(current) {
                    for entry in entries.flatten() {
                        if let Ok(file_name) = entry.file_name().into_string() {
                            if file_name.ends_with(pattern) && entry.path().is_dir() {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        
        // Check for project configuration files in current directory
        // These indicate this is a project root that should be protected
        for pattern in PROTECTED_PATTERNS.iter() {
            // Skip directory patterns (already checked above)
            if pattern.starts_with('.') && 
               (*pattern == ".git" || *pattern == ".hg" || *pattern == ".svn" || 
                *pattern == ".bzr" || *pattern == ".fossil" || *pattern == "CVS") {
                continue; // Already checked above
            }
            if *pattern == "node_modules" || 
               *pattern == "target" ||
               *pattern == "dist" ||
               *pattern == "build" ||
               *pattern == "venv" ||
               *pattern == ".venv" ||
               *pattern == "env" ||
               *pattern == ".env" ||
               *pattern == "__pycache__" {
                continue; // Directory patterns, already checked above
            }
            
            // Check for configuration files (package.json, Cargo.toml, etc.)
            let config_file = current.join(pattern);
            if config_file.exists() && config_file.is_file() {
                return true;
            }
        }
        
        current = parent;
        depth += 1;
    }

    false
}

/// Check if a path is inside a Git repository (for backward compatibility)
pub fn is_inside_git_repo<P: AsRef<std::path::Path>>(path: P) -> bool {
    use std::path::Path;
    
    let path = path.as_ref();
    let mut current = if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        path
    };

    // Walk up the directory tree looking for .git
    while let Some(parent) = current.parent() {
        let git_dir = current.join(".git");
        if git_dir.exists() && (git_dir.is_dir() || git_dir.is_file()) {
            return true;
        }
        current = parent;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_compute_file_hash() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "test content").unwrap();
        file.flush().unwrap();

        let hash = compute_file_hash(file.path()).unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // Blake3 hex string length
    }

    #[test]
    fn test_compute_file_hash_consistent() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "same content").unwrap();
        file.flush().unwrap();

        let hash1 = compute_file_hash(file.path()).unwrap();
        let hash2 = compute_file_hash(file.path()).unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_file_hash_different_content() {
        let mut file1 = NamedTempFile::new().unwrap();
        write!(file1, "content one").unwrap();
        file1.flush().unwrap();

        let mut file2 = NamedTempFile::new().unwrap();
        write!(file2, "content two").unwrap();
        file2.flush().unwrap();

        let hash1 = compute_file_hash(file1.path()).unwrap();
        let hash2 = compute_file_hash(file2.path()).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_get_extension_with_txt() {
        let path = std::path::Path::new("/path/to/file.txt");
        let ext = get_extension(path);
        assert_eq!(ext, Some("txt".to_string()));
    }

    #[test]
    fn test_get_extension_with_md() {
        let path = std::path::Path::new("/path/to/README.md");
        let ext = get_extension(path);
        assert_eq!(ext, Some("md".to_string()));
    }

    #[test]
    fn test_get_extension_lowercase() {
        let path = std::path::Path::new("/path/to/file.TXT");
        let ext = get_extension(path);
        assert_eq!(ext, Some("txt".to_string()));
    }

    #[test]
    fn test_get_extension_no_extension() {
        let path = std::path::Path::new("/path/to/file");
        let ext = get_extension(path);
        assert_eq!(ext, None);
    }

    #[test]
    fn test_get_extension_multiple_dots() {
        let path = std::path::Path::new("/path/to/file.tar.gz");
        let ext = get_extension(path);
        assert_eq!(ext, Some("gz".to_string()));
    }

    #[test]
    fn test_is_inside_git_repo() {
        use tempfile::TempDir;
        use std::fs;

        let temp_dir = TempDir::new().unwrap();
        
        // Create a .git directory
        let git_dir = temp_dir.path().join(".git");
        fs::create_dir(&git_dir).unwrap();
        
        // File inside git repo
        let file_in_repo = temp_dir.path().join("file.txt");
        fs::write(&file_in_repo, "content").unwrap();
        assert!(is_inside_git_repo(&file_in_repo));
        assert!(is_inside_git_repo(temp_dir.path()));
        
        // Nested file should also be detected
        let nested_dir = temp_dir.path().join("subdir");
        fs::create_dir(&nested_dir).unwrap();
        let nested_file = nested_dir.join("nested.txt");
        fs::write(&nested_file, "content").unwrap();
        assert!(is_inside_git_repo(&nested_file));
    }

    #[test]
    fn test_is_inside_protected_structure() {
        use tempfile::TempDir;
        use std::fs;

        // Test Git repository
        let temp_dir = TempDir::new().unwrap();
        let git_dir = temp_dir.path().join(".git");
        fs::create_dir(&git_dir).unwrap();
        let file_in_repo = temp_dir.path().join("file.txt");
        fs::write(&file_in_repo, "content").unwrap();
        assert!(is_inside_protected_structure(&file_in_repo));
        
        // Test Node.js project
        let node_dir = TempDir::new().unwrap();
        let package_json = node_dir.path().join("package.json");
        fs::write(&package_json, "{}").unwrap();
        let node_file = node_dir.path().join("src").join("index.js");
        fs::create_dir_all(node_file.parent().unwrap()).unwrap();
        fs::write(&node_file, "console.log('test');").unwrap();
        assert!(is_inside_protected_structure(&node_file));
        
        // Test Rust project
        let rust_dir = TempDir::new().unwrap();
        let cargo_toml = rust_dir.path().join("Cargo.toml");
        fs::write(&cargo_toml, "[package]").unwrap();
        let rust_file = rust_dir.path().join("src").join("main.rs");
        fs::create_dir_all(rust_file.parent().unwrap()).unwrap();
        fs::write(&rust_file, "fn main() {}").unwrap();
        assert!(is_inside_protected_structure(&rust_file));
        
        // Test that non-project files are NOT protected
        let normal_dir = TempDir::new().unwrap();
        let normal_file = normal_dir.path().join("document.txt");
        fs::write(&normal_file, "content").unwrap();
        assert!(!is_inside_protected_structure(&normal_file));
    }

    #[test]
    fn test_matches_protected_pattern() {
        use std::path::Path;
        
        assert!(matches_protected_pattern(Path::new("/path/.git")));
        assert!(matches_protected_pattern(Path::new("/path/node_modules")));
        assert!(matches_protected_pattern(Path::new("/path/target")));
        assert!(matches_protected_pattern(Path::new("/path/venv")));
        assert!(matches_protected_pattern(Path::new("/path/App.app")));
        assert!(matches_protected_pattern(Path::new("/path/.app")));
        assert!(matches_protected_pattern(Path::new("/path/MyFramework.framework")));
        assert!(matches_protected_pattern(Path::new("/path/MyPlugin.plugin")));
        assert!(matches_protected_pattern(Path::new("/path/MyApp.xcodeproj")));
        assert!(!matches_protected_pattern(Path::new("/path/normal_dir")));
    }

    #[test]
    fn test_is_macos_metadata_file() {
        use std::path::Path;
        
        assert!(is_macos_metadata_file(Path::new("/path/._file.txt")));
        assert!(is_macos_metadata_file(Path::new("/path/._ROUTING_EXPLANATION.md")));
        assert!(is_macos_metadata_file(Path::new("/path/._.DS_Store")));
        assert!(!is_macos_metadata_file(Path::new("/path/file.txt")));
        assert!(!is_macos_metadata_file(Path::new("/path/normal_file.md")));
        assert!(!is_macos_metadata_file(Path::new("/path/.git")));
    }
}

