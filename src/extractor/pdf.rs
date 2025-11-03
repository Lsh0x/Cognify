use crate::extractor::r#trait::TextExtractor;
use crate::models::FileMeta;
use anyhow::{Context, Result};
use std::process::Command;

/// PDF text extractor using external tools
/// Supports: pdftotext (Poppler), pdfgrep, or other PDF extraction tools
pub struct PdfExtractor {
    use_pdftotext: bool,
    use_pdfgrep: bool,
}

impl PdfExtractor {
    pub fn new() -> Self {
        // Check which tools are available
        let use_pdftotext = Self::check_command("pdftotext");
        let use_pdfgrep = Self::check_command("pdfgrep");
        
        Self {
            use_pdftotext,
            use_pdfgrep,
        }
    }
    
    /// Check if a command is available in PATH
    fn check_command(cmd: &str) -> bool {
        Command::new("which")
            .arg(cmd)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    /// Extract text using pdftotext (Poppler)
    fn extract_with_pdftotext(&self, path: &std::path::Path) -> Result<String> {
        let output = Command::new("pdftotext")
            .arg("-layout") // Preserve layout
            .arg("-") // Output to stdout
            .arg(path)
            .arg("-") // stdin (not used, but required)
            .output()
            .context("Failed to execute pdftotext. Install Poppler utils: brew install poppler (macOS) or apt-get install poppler-utils (Linux)")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("pdftotext failed: {}", stderr);
        }
        
        String::from_utf8(output.stdout)
            .context("pdftotext returned invalid UTF-8")
    }
    
    /// Extract text using pdfgrep (alternative method)
    fn extract_with_pdfgrep(&self, path: &std::path::Path) -> Result<String> {
        // pdfgrep is primarily a search tool, but we can use it to extract
        // Note: This is a fallback if pdftotext is not available
        let output = Command::new("pdfgrep")
            .arg(".")
            .arg(path)
            .output()
            .context("Failed to execute pdfgrep")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("pdfgrep failed: {}", stderr);
        }
        
        String::from_utf8(output.stdout)
            .context("pdfgrep returned invalid UTF-8")
    }
}

#[async_trait::async_trait]
impl TextExtractor for PdfExtractor {
    async fn extract(&self, file: &FileMeta) -> Result<String> {
        let path = file.path.clone();
        let use_pdftotext = self.use_pdftotext;
        let use_pdfgrep = self.use_pdfgrep;
        
        // Try pdftotext first (best quality)
        if use_pdftotext {
            match tokio::task::spawn_blocking({
                let path = path.clone();
                move || {
                    let extractor = PdfExtractor {
                        use_pdftotext: true,
                        use_pdfgrep: false,
                    };
                    extractor.extract_with_pdftotext(&path)
                }
            })
            .await?
            {
                Ok(text) => {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        return Ok(trimmed.to_string());
                    }
                }
                Err(e) => {
                    eprintln!("Warning: pdftotext failed for {}: {}", path.display(), e);
                }
            }
        }
        
        // Fallback to pdfgrep if available
        if use_pdfgrep {
            match tokio::task::spawn_blocking({
                let path = path.clone();
                move || {
                    let extractor = PdfExtractor {
                        use_pdftotext: false,
                        use_pdfgrep: true,
                    };
                    extractor.extract_with_pdfgrep(&path)
                }
            })
            .await?
            {
                Ok(text) => {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        return Ok(trimmed.to_string());
                    }
                }
                Err(e) => {
                    eprintln!("Warning: pdfgrep failed for {}: {}", path.display(), e);
                }
            }
        }
        
        // If no tools available or both failed, return empty
        if !use_pdftotext && !use_pdfgrep {
            anyhow::bail!(
                "No PDF extraction tools available. Install Poppler utils: brew install poppler (macOS) or apt-get install poppler-utils (Linux)"
            );
        }
        
        Ok(String::new())
    }
    
    fn supports_extension(&self, ext: &str) -> bool {
        matches!(ext.to_lowercase().as_str(), "pdf")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::FileMeta;
    use std::path::PathBuf;
    use std::time::SystemTime;

    #[tokio::test]
    async fn test_pdf_extractor_supports_pdf() {
        let extractor = PdfExtractor::new();
        assert!(extractor.supports_extension("pdf"));
        assert!(extractor.supports_extension("PDF"));
        assert!(!extractor.supports_extension("txt"));
    }
}

