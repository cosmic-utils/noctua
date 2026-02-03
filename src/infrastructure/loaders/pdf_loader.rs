// SPDX-License-Identifier: GPL-3.0-or-later
// src/infrastructure/loaders/pdf_loader.rs
//
// Loader for PDF portable documents.

use std::path::Path;

use crate::domain::document::core::content::DocumentContent;
use crate::domain::document::core::document::DocResult;
use crate::domain::document::types::portable::PortableDocument;
use crate::infrastructure::loaders::document_loader::DocumentLoader;

/// Loader for PDF portable documents.
pub struct PdfLoader;

impl DocumentLoader for PdfLoader {
    fn load(&self, path: &Path) -> DocResult<DocumentContent> {
        let document = PortableDocument::open(path)
            .map_err(|e| anyhow::anyhow!("Failed to load PDF document: {e}"))?;

        Ok(DocumentContent::Portable(document))
    }

    fn supports(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            ext_str == "pdf"
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supports() {
        let loader = PdfLoader;

        assert!(loader.supports(Path::new("test.pdf")));
        assert!(loader.supports(Path::new("test.PDF")));
        assert!(loader.supports(Path::new("document.pdf")));
        assert!(!loader.supports(Path::new("test.png")));
        assert!(!loader.supports(Path::new("test.svg")));
        assert!(!loader.supports(Path::new("test.jpg")));
        assert!(!loader.supports(Path::new("test.txt")));
    }
}
