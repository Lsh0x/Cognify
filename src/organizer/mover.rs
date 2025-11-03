use crate::organizer::preview::PreviewTree;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Handles safe file reorganization with dry-run support
pub struct FileMover {
    base_path: std::path::PathBuf,
}

impl FileMover {
    /// Create a new FileMover for the given base directory
    pub fn new<P: AsRef<Path>>(base_path: P) -> Result<Self> {
        let base_path = base_path.as_ref().canonicalize()
            .context("Failed to canonicalize base path")?;

        if !base_path.is_dir() {
            anyhow::bail!("Base path is not a directory: {}", base_path.display());
        }

        Ok(Self { base_path })
    }

    /// Plan file moves based on a preview tree (dry-run)
    pub fn plan_moves(&self, preview: &PreviewTree) -> Result<PreviewTree> {
        // Validate that all operations are within the base path
        for dir_op in &preview.directories_to_create {
            if !dir_op.path.starts_with(&self.base_path) {
                anyhow::bail!(
                    "Directory creation outside base path: {}",
                    dir_op.path.display()
                );
            }
        }

        for move_op in &preview.files_to_move {
            if !move_op.source.starts_with(&self.base_path) {
                anyhow::bail!(
                    "Source file outside base path: {}",
                    move_op.source.display()
                );
            }
            if !move_op.destination.starts_with(&self.base_path) {
                anyhow::bail!(
                    "Destination outside base path: {}",
                    move_op.destination.display()
                );
            }
        }

        Ok(preview.clone())
    }

    /// Execute file moves after confirmation
    pub async fn execute(&self, preview: &PreviewTree, dry_run: bool) -> Result<()> {
        if dry_run {
            println!("DRY RUN - No files will be moved");
            return Ok(());
        }

        // Create directories first
        for dir_op in &preview.directories_to_create {
            fs::create_dir_all(&dir_op.path)
                .with_context(|| format!("Failed to create directory: {}", dir_op.path.display()))?;
        }

        // Move files
        for move_op in &preview.files_to_move {
            // Ensure destination directory exists
            if let Some(parent) = move_op.destination.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create destination directory: {}", parent.display()))?;
            }

            // Move the file
            fs::rename(&move_op.source, &move_op.destination)
                .with_context(|| {
                    format!(
                        "Failed to move {} to {}",
                        move_op.source.display(),
                        move_op.destination.display()
                    )
                })?;
        }

        Ok(())
    }

    /// Get the base path
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_mover_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mover = FileMover::new(temp_dir.path()).unwrap();
        assert_eq!(mover.base_path(), temp_dir.path().canonicalize().unwrap());
    }

    #[tokio::test]
    async fn test_mover_plan_moves() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();
        let mover = FileMover::new(&temp_path).unwrap();

        let mut preview = PreviewTree::new();
        preview.add_directory(temp_path.join("folder1"));
        preview.add_move(
            temp_path.join("file1.txt"),
            temp_path.join("folder1/file1.txt"),
        );

        let planned = mover.plan_moves(&preview).unwrap();
        assert_eq!(planned.directories_to_create.len(), 1);
        assert_eq!(planned.files_to_move.len(), 1);
    }

    #[tokio::test]
    async fn test_mover_execute_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let mover = FileMover::new(temp_dir.path()).unwrap();

        let mut preview = PreviewTree::new();
        preview.add_directory(temp_dir.path().join("folder1"));
        preview.add_move(
            temp_dir.path().join("file1.txt"),
            temp_dir.path().join("folder1/file1.txt"),
        );

        // Dry run should not create anything
        mover.execute(&preview, true).await.unwrap();
        assert!(!temp_dir.path().join("folder1").exists());
    }

    #[tokio::test]
    async fn test_mover_execute_real() {
        let temp_dir = TempDir::new().unwrap();
        let mover = FileMover::new(temp_dir.path()).unwrap();

        // Create a test file
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "content").unwrap();

        let mut preview = PreviewTree::new();
        preview.add_directory(temp_dir.path().join("moved"));
        preview.add_move(
            test_file.clone(),
            temp_dir.path().join("moved/test.txt"),
        );

        mover.execute(&preview, false).await.unwrap();

        // Verify file was moved
        assert!(!test_file.exists());
        assert!(temp_dir.path().join("moved/test.txt").exists());
    }

    #[tokio::test]
    async fn test_mover_rejects_outside_base() {
        let temp_dir = TempDir::new().unwrap();
        let mover = FileMover::new(temp_dir.path()).unwrap();

        let mut preview = PreviewTree::new();
        preview.add_move(
            PathBuf::from("/outside/file.txt"),
            temp_dir.path().join("file.txt"),
        );

        assert!(mover.plan_moves(&preview).is_err());
    }
}

