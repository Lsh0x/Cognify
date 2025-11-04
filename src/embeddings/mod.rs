pub mod r#trait;
pub mod local;
pub mod tei;
pub mod multi_ollama;

pub use local::LocalEmbeddingProvider;
pub use tei::TeiEmbeddingProvider;
pub use multi_ollama::MultiOllamaEmbeddingProvider;
pub use r#trait::EmbeddingProvider;

