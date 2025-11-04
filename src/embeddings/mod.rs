pub mod r#trait;
pub mod local;
pub mod tei;

pub use local::LocalEmbeddingProvider;
pub use tei::TeiEmbeddingProvider;
pub use r#trait::EmbeddingProvider;

