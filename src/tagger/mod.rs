pub mod document;
pub mod generic;
pub mod registry;
pub mod r#trait;
pub mod text;

pub use document::PdfHandler;
pub use generic::GenericHandler;
pub use registry::TaggerRegistry;
pub use r#trait::Taggable;
pub use text::{MarkdownHandler, TextHandler};

