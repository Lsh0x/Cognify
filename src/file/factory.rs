use crate::file::SemanticSource;
use crate::file::types::{CsvFile, GenericFile, JsonFile, MdFile, PdfFile, TxtFile, ZipFile};
use crate::models::FileMeta;
use std::path::PathBuf;
use std::sync::Arc;

/// Factory for creating SemanticSource instances based on file extension
pub struct FileFactory;

impl FileFactory {
    /// Create a SemanticSource from FileMeta
    pub fn create_from_meta(meta: &FileMeta) -> Arc<dyn SemanticSource> {
        let extension = meta.extension.as_ref().map(|s| s.to_lowercase());
        let path = meta.path.clone();

        Self::create(path, extension)
    }

    /// Create a SemanticSource from path and extension
    pub fn create(path: PathBuf, extension: Option<String>) -> Arc<dyn SemanticSource> {
        let ext_lower = extension.as_ref().map(|s| s.to_lowercase());

        match ext_lower.as_deref() {
            Some("txt") | Some("text") => Arc::new(TxtFile::new(path, extension)),
            Some("md") | Some("markdown") => Arc::new(MdFile::new(path, extension)),
            Some("pdf") => Arc::new(PdfFile::new(path, extension)),
            Some("csv") => Arc::new(CsvFile::new(path, extension)),
            Some("json") => Arc::new(JsonFile::new(path, extension)),
            Some("zip") => Arc::new(ZipFile::new(path, extension)),
            _ => Arc::new(GenericFile::new(path, extension)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::FileMeta;
    use std::path::PathBuf;
    use std::time::SystemTime;

    #[test]
    fn test_factory_txt_file() {
        let path = PathBuf::from("/test/file.txt");
        let source = FileFactory::create(path.clone(), Some("txt".to_string()));
        assert_eq!(source.path(), path.as_path());
        assert_eq!(source.extension(), Some("txt"));
    }

    #[test]
    fn test_factory_md_file() {
        let path = PathBuf::from("/test/file.md");
        let source = FileFactory::create(path.clone(), Some("md".to_string()));
        assert_eq!(source.path(), path.as_path());
        assert_eq!(source.extension(), Some("md"));
    }

    #[test]
    fn test_factory_pdf_file() {
        let path = PathBuf::from("/test/file.pdf");
        let source = FileFactory::create(path.clone(), Some("pdf".to_string()));
        assert_eq!(source.path(), path.as_path());
        assert_eq!(source.extension(), Some("pdf"));
    }

    #[test]
    fn test_factory_csv_file() {
        let path = PathBuf::from("/test/file.csv");
        let source = FileFactory::create(path.clone(), Some("csv".to_string()));
        assert_eq!(source.path(), path.as_path());
        assert_eq!(source.extension(), Some("csv"));
    }

    #[test]
    fn test_factory_json_file() {
        let path = PathBuf::from("/test/file.json");
        let source = FileFactory::create(path.clone(), Some("json".to_string()));
        assert_eq!(source.path(), path.as_path());
        assert_eq!(source.extension(), Some("json"));
    }

    #[test]
    fn test_factory_zip_file() {
        let path = PathBuf::from("/test/file.zip");
        let source = FileFactory::create(path.clone(), Some("zip".to_string()));
        assert_eq!(source.path(), path.as_path());
        assert_eq!(source.extension(), Some("zip"));
    }

    #[test]
    fn test_factory_generic_file() {
        let path = PathBuf::from("/test/file.unknown");
        let source = FileFactory::create(path.clone(), Some("unknown".to_string()));
        assert_eq!(source.path(), path.as_path());
        assert_eq!(source.extension(), Some("unknown"));
    }

    #[test]
    fn test_factory_from_file_meta() {
        let now = SystemTime::now();
        let meta = FileMeta::new(
            PathBuf::from("/test/file.txt"),
            100,
            Some("txt".to_string()),
            now,
            now,
            "hash123".to_string(),
        );

        let source = FileFactory::create_from_meta(&meta);
        assert_eq!(source.path(), meta.path.as_path());
        assert_eq!(source.extension(), Some("txt"));
    }
}
