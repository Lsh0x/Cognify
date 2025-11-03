pub mod factory;
pub mod r#trait;
pub mod types;

pub use factory::FileFactory;
pub use r#trait::SemanticSource;
pub use types::{GenericFile, MdFile, PdfFile, SemanticFile, TxtFile};

