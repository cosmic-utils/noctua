// SPDX-License-Identifier: GPL-3.0-or-later
// src/infrastructure/loaders/raster_loader.rs
//
// Loader for raster image documents (PNG, JPEG, WebP, etc.).

use std::path::Path;

use crate::domain::document::core::content::DocumentContent;
use crate::domain::document::core::document::DocResult;
use crate::domain::document::types::raster::RasterDocument;
use crate::infrastructure::loaders::document_loader::DocumentLoader;

/// Loader for raster image documents.
pub struct RasterLoader;

impl DocumentLoader for RasterLoader {
    fn load(&self, path: &Path) -> DocResult<DocumentContent> {
        let document = RasterDocument::open(path)
            .map_err(|e| anyhow::anyhow!("Failed to load raster document: {e}"))?;

        Ok(DocumentContent::Raster(document))
    }

    fn supports(&self, path: &Path) -> bool {
        use cosmic::iced_renderer::graphics::image::image_rs::ImageFormat;

        ImageFormat::from_path(path).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supports() {
        let loader = RasterLoader;

        assert!(loader.supports(Path::new("test.png")));
        assert!(loader.supports(Path::new("test.jpg")));
        assert!(loader.supports(Path::new("test.jpeg")));
        assert!(loader.supports(Path::new("test.webp")));
        assert!(!loader.supports(Path::new("test.pdf")));
        assert!(!loader.supports(Path::new("test.svg")));
    }
}
