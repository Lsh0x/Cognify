use std::collections::HashMap;
use std::path::PathBuf;

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

    /// Generate a tree visualization string
    pub fn to_string(&self) -> String {
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
        
        let preview = tree.to_string();
        assert!(preview.contains("folder1"));
        assert!(preview.contains("file1.txt"));
        assert!(preview.contains("file2.txt"));
        assert!(preview.contains("Create 1 directories"));
        assert!(preview.contains("move 2 files"));
    }
}

