// SPDX-License-Identifier: GPL-3.0-or-later
// src/infrastructure/loaders/svg_loader.rs
//
// Loader for SVG vector documents.

use std::path::Path;

use crate::domain::document::core::content::DocumentContent;
use crate::domain::document::core::document::DocResult;
use crate::domain::document::types::vector::VectorDocument;
use crate::infrastructure::loaders::document_loader::DocumentLoader;

/// Loader for SVG vector documents.
pub struct SvgLoader;

impl DocumentLoader for SvgLoader {
    fn load(&self, path: &Path) -> DocResult<DocumentContent> {
        let document = VectorDocument::open(path)
            .map_err(|e| anyhow::anyhow!("Failed to load SVG document: {e}"))?;

        Ok(DocumentContent::Vector(document))
    }

    fn supports(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            ext_str == "svg" || ext_str == "svgz"
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
        let loader = SvgLoader;

        assert!(loader.supports(Path::new("test.svg")));
        assert!(loader.supports(Path::new("test.SVG")));
        assert!(loader.supports(Path::new("test.svgz")));
        assert!(!loader.supports(Path::new("test.png")));
        assert!(!loader.supports(Path::new("test.pdf")));
        assert!(!loader.supports(Path::new("test.jpg")));
    }
}
