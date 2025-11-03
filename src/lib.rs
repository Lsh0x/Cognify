pub mod embeddings;
pub mod indexer;
pub mod llm;
pub mod models;
pub mod organizer;
pub mod tagger;
pub mod utils;
pub mod watcher;

pub use embeddings::EmbeddingProvider;
pub use indexer::Indexer;
pub use llm::LlmProvider;
pub use models::FileMeta;
pub use tagger::Taggable;
pub use watcher::{FileWatcher, WatchEvent};
