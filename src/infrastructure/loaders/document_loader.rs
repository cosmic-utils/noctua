// SPDX-License-Identifier: GPL-3.0-or-later
// src/infrastructure/loaders/document_loader.rs
//
// Document loader trait and factory for loading documents from files.

use std::path::Path;

use crate::domain::document::core::content::{DocumentContent, DocumentKind};
use crate::domain::document::core::document::DocResult;

use super::raster_loader::RasterLoader;
#[cfg(feature = "vector")]
use super::svg_loader::SvgLoader;
#[cfg(feature = "portable")]
use super::pdf_loader::PdfLoader;

/// Trait for loading documents from files.
///
/// Implementations handle specific document formats (raster, vector, portable).
pub trait DocumentLoader {
    /// Load a document from a file path.
    fn load(&self, path: &Path) -> DocResult<DocumentContent>;

    /// Check if this loader supports the given file.
    fn supports(&self, path: &Path) -> bool;
}

/// Document loader factory.
///
/// Detects the document format and delegates to the appropriate loader.
pub struct DocumentLoaderFactory;

impl DocumentLoaderFactory {
    /// Create a new document loader factory.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Load a document from a file, automatically detecting the format.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file format is not supported
    /// - The file cannot be read
    /// - The document is malformed
    pub fn load(&self, path: &Path) -> DocResult<DocumentContent> {
        let kind = DocumentKind::from_path(path).ok_or_else(|| {
            anyhow::anyhow!(
                "Unsupported file format: {}",
                path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown")
            )
        })?;

        match kind {
            DocumentKind::Raster => {
                let loader = RasterLoader;
                loader.load(path)
            }
            #[cfg(feature = "vector")]
            DocumentKind::Vector => {
                let loader = SvgLoader;
                loader.load(path)
            }
            #[cfg(feature = "portable")]
            DocumentKind::Portable => {
                let loader = PdfLoader;
                loader.load(path)
            }
            #[cfg(not(any(feature = "vector", feature = "portable")))]
            _ => Err(anyhow::anyhow!(
                "No document loaders available (check feature flags)"
            )),
        }
    }

    /// Detect the document kind from a file path.
    #[must_use]
    pub fn detect_kind(&self, path: &Path) -> Option<DocumentKind> {
        DocumentKind::from_path(path)
    }

    /// Check if a file is supported by any loader.
    #[must_use]
    pub fn is_supported(&self, path: &Path) -> bool {
        DocumentKind::from_path(path).is_some()
    }
}

impl Default for DocumentLoaderFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_creation() {
        let factory = DocumentLoaderFactory::new();
        assert!(std::ptr::eq(&factory, &factory)); // Just a dummy test
    }

    #[test]
    fn test_detect_kind() {
        let factory = DocumentLoaderFactory::new();

        assert_eq!(
            factory.detect_kind(Path::new("test.png")),
            Some(DocumentKind::Raster)
        );
        assert_eq!(
            factory.detect_kind(Path::new("test.jpg")),
            Some(DocumentKind::Raster)
        );

        #[cfg(feature = "vector")]
        {
            assert_eq!(
                factory.detect_kind(Path::new("test.svg")),
                Some(DocumentKind::Vector)
            );
        }

        #[cfg(feature = "portable")]
        {
            assert_eq!(
                factory.detect_kind(Path::new("test.pdf")),
                Some(DocumentKind::Portable)
            );
        }

        assert_eq!(factory.detect_kind(Path::new("test.txt")), None);
    }

    #[test]
    fn test_is_supported() {
        let factory = DocumentLoaderFactory::new();

        assert!(factory.is_supported(Path::new("test.png")));
        assert!(!factory.is_supported(Path::new("test.txt")));
    }
}
