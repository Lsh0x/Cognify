use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Represents a file move operation
#[derive(Debug, Clone)]
pub struct MoveOperation {
    pub source: PathBuf,
    pub destination: PathBuf,
}

/// Represents a directory creation operation
#[derive(Debug, Clone)]
pub struct CreateDirOperation {
    pub path: PathBuf,
}

/// Collection of operations for preview
#[derive(Debug, Clone, Default)]
pub struct PreviewTree {
    pub directories_to_create: Vec<CreateDirOperation>,
    pub files_to_move: Vec<MoveOperation>,
}

impl PreviewTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a directory to be created
    pub fn add_directory(&mut self, path: PathBuf) {
        self.directories_to_create.push(CreateDirOperation { path });
    }

    /// Add a file move operation
    pub fn add_move(&mut self, source: PathBuf, destination: PathBuf) {
        self.files_to_move.push(MoveOperation {
            source,
            destination,
        });
    }

    /// Generate a tree visualization string showing current and proposed structure
    pub fn to_string(&self, base_dir: &Path) -> String {
        let mut output = String::new();
        
        // Show current structure as a tree (like `tree` command)
        output.push_str("Current structure:\n");
        let mut current_files: Vec<_> = self.files_to_move.iter().collect();
        current_files.sort_by_key(|op| &op.source);
        
        if current_files.is_empty() {
            output.push_str("  (no files to organize)\n");
        } else {
            // Group current files by their source directories
            let mut current_dir_files: BTreeMap<PathBuf, Vec<&MoveOperation>> = BTreeMap::new();
            for op in &current_files {
                let dir = op.source.parent().unwrap_or(&op.source.clone()).to_path_buf();
                current_dir_files.entry(dir).or_insert_with(Vec::new).push(op);
            }

            // Get all source directories (including nested ones)
            let mut current_all_dirs: Vec<PathBuf> = current_dir_files.keys().cloned().collect();
            current_all_dirs.sort();

            // Build nested directory tree structure for current files
            let current_dir_tree = self.build_directory_tree(&current_all_dirs, base_dir);
            
            // Render current tree structure starting from base_dir (use source paths)
            let base_name = base_dir.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_else(|| base_dir.to_str().unwrap_or("."));
            output.push_str(&format!("{}\n", base_name));
            output.push_str(&self.render_tree(&current_dir_tree, &current_dir_files, base_dir, "", true));
        }

        output.push_str("\n\nProposed new structure:\n");
        
        // Group files by destination directory
        let mut dir_files: BTreeMap<PathBuf, Vec<&MoveOperation>> = BTreeMap::new();
        for op in &self.files_to_move {
            let dir = op.destination.parent().unwrap_or(&op.destination.clone()).to_path_buf();
            dir_files.entry(dir).or_insert_with(Vec::new).push(op);
        }

        // Get all directories (including nested ones)
        let mut all_dirs: Vec<PathBuf> = dir_files.keys().cloned().collect();
        all_dirs.sort();

        // Build nested directory tree structure
        let dir_tree = self.build_directory_tree(&all_dirs, base_dir);
        
        // Render tree structure starting from base_dir (use destination paths)
        let base_name = base_dir.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_else(|| base_dir.to_str().unwrap_or("."));
        output.push_str(&format!("{}\n", base_name));
        output.push_str(&self.render_tree(&dir_tree, &dir_files, base_dir, "", false));

        // Count new vs existing directories
        let new_dirs: HashSet<PathBuf> = self.directories_to_create
            .iter()
            .map(|d| d.path.strip_prefix(base_dir).unwrap_or(&d.path).to_path_buf())
            .collect();
        
        let mut existing_dirs_count = 0;
        let mut new_dirs_count = 0;
        
        for dir in &all_dirs {
            let dir_rel: PathBuf = dir.strip_prefix(base_dir).unwrap_or(dir).to_path_buf();
            if new_dirs.contains(&dir_rel) {
                new_dirs_count += 1;
            } else {
                existing_dirs_count += 1;
            }
        }

        output.push_str(&format!(
            "\n\nSummary:\n  • Create {} new directories\n  • Use {} existing directories\n  • Move {} files\n\nProceed? [y/N]:",
            new_dirs_count,
            existing_dirs_count,
            self.files_to_move.len()
        ));

        output
    }

    /// Build a hierarchical directory tree structure
    fn build_directory_tree(&self, dirs: &[PathBuf], base_dir: &Path) -> BTreeMap<PathBuf, Vec<PathBuf>> {
        let mut tree: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
        
        for dir in dirs {
            let rel_dir = dir.strip_prefix(base_dir).unwrap_or(dir);
            let mut current = PathBuf::new();
            
            for component in rel_dir.components() {
                let next = current.join(component);
                let parent_path = base_dir.join(&current);
                let child_path = base_dir.join(&next);
                
                if !tree.contains_key(&parent_path) {
                    tree.insert(parent_path.clone(), Vec::new());
                }
                
                if let Some(children) = tree.get_mut(&parent_path) {
                    if !children.contains(&child_path) {
                        children.push(child_path.clone());
                    }
                }
                
                current = next;
            }
        }
        
        // Sort children for each directory
        for children in tree.values_mut() {
            children.sort();
        }
        
        tree
    }

    /// Render the tree structure with proper tree characters
    /// `use_source` determines whether to use `source` (current structure) or `destination` (proposed structure)
    fn render_tree(
        &self,
        dir_tree: &BTreeMap<PathBuf, Vec<PathBuf>>,
        file_map: &BTreeMap<PathBuf, Vec<&MoveOperation>>,
        current_dir: &Path,
        prefix: &str,
        use_source: bool,
    ) -> String {
        let mut output = String::new();
        
        // Get immediate children directories
        let children = dir_tree.get(current_dir).cloned().unwrap_or_default();
        let has_files = file_map.contains_key(current_dir);
        
        // Combine files and directories into a single sorted list
        let mut items: Vec<(bool, PathBuf)> = Vec::new(); // (is_file, path)
        
        // Add files
        if has_files {
            for file_op in &file_map[current_dir] {
                let file_path = if use_source {
                    file_op.source.clone()
                } else {
                    file_op.destination.clone()
                };
                items.push((true, file_path));
            }
        }
        
        // Add directories
        for child_dir in &children {
            items.push((false, child_dir.clone()));
        }
        
        // Sort: directories first, then files (by name)
        items.sort_by(|a, b| {
            match (a.0, b.0) {
                (false, true) => std::cmp::Ordering::Less, // dirs before files
                (true, false) => std::cmp::Ordering::Greater,
                _ => {
                    let name_a = a.1.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let name_b = b.1.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    name_a.cmp(name_b)
                }
            }
        });
        
        // Render each item
        for (idx, (is_file, path)) in items.iter().enumerate() {
            let is_last_item = idx == items.len() - 1;
            let tree_char = if is_last_item {
                "└──"
            } else {
                "├──"
            };
            
            let next_prefix = if is_last_item {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };
            
            if *is_file {
                // It's a file
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                output.push_str(&format!("{}{}{}\n", prefix, tree_char, file_name));
            } else {
                // It's a directory
                let dir_name = path.strip_prefix(current_dir)
                    .unwrap_or(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                output.push_str(&format!("{}{} {}\n", prefix, tree_char, dir_name));
                
                // Render files in this directory
                if let Some(files) = file_map.get(path) {
                    let mut sorted_files: Vec<_> = files.iter().collect();
                    if use_source {
                        sorted_files.sort_by_key(|op| &op.source);
                    } else {
                        sorted_files.sort_by_key(|op| &op.destination);
                    }
                    
                    let file_count = sorted_files.len();
                    let has_subdirs = dir_tree.get(path).map(|d| !d.is_empty()).unwrap_or(false);
                    
                    for (file_idx, file_op) in sorted_files.iter().enumerate() {
                        let is_last_file = file_idx == file_count - 1 && !has_subdirs;
                        let file_path = if use_source {
                            &file_op.source
                        } else {
                            &file_op.destination
                        };
                        let file_name = file_path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown");
                        
                        let file_tree_char = if is_last_file {
                            "└──"
                        } else {
                            "├──"
                        };
                        
                        output.push_str(&format!("{}{}{}\n", next_prefix, file_tree_char, file_name));
                    }
                }
                
                // Recursively render subdirectories
                if let Some(subdirs) = dir_tree.get(path) {
                    if !subdirs.is_empty() {
                        output.push_str(&self.render_tree(
                            dir_tree,
                            file_map,
                            path,
                            &next_prefix,
                            use_source,
                        ));
                    }
                }
            }
        }
        
        output
    }

    /// Generate a simple preview string (backward compatibility)
    pub fn to_string_simple(&self) -> String {
        let mut output = String::new();
        output.push_str("Proposed changes:\n");

        // Group files by directory
        let mut dir_files: HashMap<PathBuf, Vec<&MoveOperation>> = HashMap::new();
        for op in &self.files_to_move {
            let dir = op.destination.parent().unwrap_or(&op.destination.clone()).to_path_buf();
            dir_files.entry(dir).or_insert_with(Vec::new).push(op);
        }

        // Display directory structure
        let mut dirs: Vec<_> = dir_files.keys().collect();
        dirs.sort();

        for dir in dirs {
            let display_dir = dir.display();
            output.push_str(&format!("📁 {}/\n", display_dir));

            if let Some(files) = dir_files.get(dir) {
                for file_op in files {
                    let file_name = file_op.destination.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");
                    let source_display = file_op.source.display();
                    output.push_str(&format!("  📄 {} (from: {})\n", file_name, source_display));
                }
            }
        }

        output.push_str(&format!(
            "\nCreate {} directories, move {} files? [y/N]:",
            self.directories_to_create.len(),
            self.files_to_move.len()
        ));

        output
    }

    /// Check if there are any operations
    pub fn is_empty(&self) -> bool {
        self.directories_to_create.is_empty() && self.files_to_move.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preview_tree_empty() {
        let tree = PreviewTree::new();
        assert!(tree.is_empty());
    }

    #[test]
    fn test_preview_tree_add_operations() {
        let mut tree = PreviewTree::new();
        tree.add_directory(PathBuf::from("/test/folder1"));
        tree.add_move(PathBuf::from("/test/file.txt"), PathBuf::from("/test/folder1/file.txt"));
        
        assert!(!tree.is_empty());
        assert_eq!(tree.directories_to_create.len(), 1);
        assert_eq!(tree.files_to_move.len(), 1);
    }

    #[test]
    fn test_preview_tree_to_string() {
        let mut tree = PreviewTree::new();
        tree.add_directory(PathBuf::from("/test/folder1"));
        tree.add_move(PathBuf::from("/test/file1.txt"), PathBuf::from("/test/folder1/file1.txt"));
        tree.add_move(PathBuf::from("/test/file2.txt"), PathBuf::from("/test/folder1/file2.txt"));
        
        let preview = tree.to_string_simple();
        assert!(preview.contains("folder1"));
        assert!(preview.contains("file1.txt"));
        assert!(preview.contains("file2.txt"));
        assert!(preview.contains("Create 1 directories"));
        assert!(preview.contains("move 2 files"));
    }
}

