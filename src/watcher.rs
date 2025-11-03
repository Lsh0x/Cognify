use crate::models::FileMeta;
use crate::utils;
use anyhow::{Context, Result};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::SystemTime;
use tokio::sync::broadcast;

/// Event emitted by the filesystem watcher
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// File was created
    Created(FileMeta),
    /// File was modified
    Modified(FileMeta),
    /// File was deleted
    Deleted(PathBuf),
}

/// Filesystem watcher that monitors a specific directory
pub struct FileWatcher {
    watch_dir: PathBuf,
    event_sender: broadcast::Sender<WatchEvent>,
}

impl FileWatcher {
    /// Create a new FileWatcher for the given directory
    pub fn new<P: AsRef<Path>>(watch_dir: P) -> Result<Self> {
        let watch_dir = watch_dir.as_ref().canonicalize()
            .context("Failed to canonicalize watch directory path")?;

        if !watch_dir.is_dir() {
            anyhow::bail!("Path is not a directory: {}", watch_dir.display());
        }

        let (tx, _) = broadcast::channel(100);
        Ok(Self {
            watch_dir,
            event_sender: tx,
        })
    }

    /// Get a receiver for watch events
    pub fn subscribe(&self) -> broadcast::Receiver<WatchEvent> {
        self.event_sender.subscribe()
    }

    /// Start watching the directory
    pub async fn watch(&self) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = RecommendedWatcher::new(
            move |result: notify::Result<Event>| {
                if let Ok(event) = result {
                    let _ = tx.send(event);
                }
            },
            Config::default(),
        )
        .context("Failed to create filesystem watcher")?;

        // Watch the directory recursively
        watcher
            .watch(&self.watch_dir, RecursiveMode::Recursive)
            .context("Failed to start watching directory")?;

        // Spawn task to process events
        let watch_dir = self.watch_dir.clone();
        let event_sender = self.event_sender.clone();

        tokio::spawn(async move {
            while let Ok(event) = rx.recv() {
                Self::process_event(&watch_dir, event, &event_sender).await;
            }
        });

        // Keep the watcher alive
        tokio::task::yield_now().await;
        Ok(())
    }

    async fn process_event(
        watch_dir: &Path,
        event: Event,
        sender: &broadcast::Sender<WatchEvent>,
    ) {
        for path in event.paths {
            // Safety: only process paths within the watched directory
            if !path.starts_with(watch_dir) {
                continue;
            }

            // Skip if not a file
            if !path.is_file() {
                continue;
            }

            let watch_event = match event.kind {
                EventKind::Create(_) => {
                    match Self::create_file_meta(&path).await {
                        Ok(meta) => Some(WatchEvent::Created(meta)),
                        Err(e) => {
                            eprintln!("Error creating file meta for {:?}: {}", path, e);
                            None
                        }
                    }
                }
                EventKind::Modify(_) => {
                    match Self::create_file_meta(&path).await {
                        Ok(meta) => Some(WatchEvent::Modified(meta)),
                        Err(e) => {
                            eprintln!("Error creating file meta for {:?}: {}", path, e);
                            None
                        }
                    }
                }
                EventKind::Remove(_) => Some(WatchEvent::Deleted(path)),
                _ => None,
            };

            if let Some(evt) = watch_event {
                let _ = sender.send(evt);
            }
        }
    }

    async fn create_file_meta(path: &Path) -> Result<FileMeta> {
        let metadata = std::fs::metadata(path)
            .with_context(|| format!("Failed to read metadata for {}", path.display()))?;

        let size = metadata.len();
        let extension = utils::get_extension(path);
        let created_at = metadata
            .created()
            .or_else(|_| metadata.modified())
            .unwrap_or_else(|_| SystemTime::now());
        let updated_at = metadata
            .modified()
            .or_else(|_| metadata.created())
            .unwrap_or_else(|_| SystemTime::now());

        let hash = tokio::task::spawn_blocking({
            let path = path.to_path_buf();
            move || utils::compute_file_hash(&path)
        })
        .await??;

        Ok(FileMeta::new(path.to_path_buf(), size, extension, created_at, updated_at, hash))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_watcher_creation() {
        let temp_dir = TempDir::new().unwrap();
        let watcher = FileWatcher::new(temp_dir.path()).unwrap();
        assert_eq!(watcher.watch_dir, temp_dir.path().canonicalize().unwrap());
    }

    #[tokio::test]
    async fn test_watcher_rejects_non_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, b"test").unwrap();

        assert!(FileWatcher::new(&file_path).is_err());
    }
}

