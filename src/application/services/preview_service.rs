// SPDX-License-Identifier: GPL-3.0-or-later
// src/application/services/preview_service.rs
//
// Preview service: generates thumbnails and previews for documents.
// Reserved for future async thumbnail generation implementation.

#![allow(dead_code)]

use cosmic::widget::image::Handle as ImageHandle;

use crate::domain::document::core::content::DocumentContent;
use crate::domain::document::core::document::DocResult;

/// Preview service for generating document thumbnails and previews.
///
/// Provides high-level preview generation operations for the application layer.
pub struct PreviewService {
    /// Target thumbnail size (width in pixels).
    thumbnail_size: u32,
}

impl PreviewService {
    /// Create a new preview service with default thumbnail size.
    #[must_use]
    pub fn new() -> Self {
        Self {
            thumbnail_size: 256,
        }
    }

    /// Create a preview service with a specific thumbnail size.
    #[must_use]
    pub fn with_thumbnail_size(size: u32) -> Self {
        Self {
            thumbnail_size: size,
        }
    }

    /// Set the thumbnail size.
    pub fn set_thumbnail_size(&mut self, size: u32) {
        self.thumbnail_size = size;
    }

    /// Get the current thumbnail size.
    #[must_use]
    pub fn thumbnail_size(&self) -> u32 {
        self.thumbnail_size
    }

    /// Generate a thumbnail for a document page.
    ///
    /// For single-page documents, the page parameter is ignored.
    pub fn generate_thumbnail(
        &self,
        document: &mut DocumentContent,
        page: usize,
    ) -> DocResult<Option<ImageHandle>> {
        if document.is_multi_page() {
            document.get_thumbnail(page)
        } else {
            // For single-page documents, return the current handle
            Ok(document.handle())
        }
    }

    /// Generate all thumbnails for a multi-page document.
    ///
    /// Returns the number of thumbnails generated.
    pub fn generate_all_thumbnails(&self, document: &mut DocumentContent) -> DocResult<usize> {
        if !document.is_multi_page() {
            return Ok(0);
        }

        document.generate_thumbnails()?;
        Ok(document.thumbnails_loaded())
    }

    /// Check if all thumbnails are ready for a document.
    #[must_use]
    pub fn thumbnails_ready(&self, document: &DocumentContent) -> bool {
        document.thumbnails_ready()
    }

    /// Get the number of thumbnails loaded for a document.
    #[must_use]
    pub fn thumbnails_loaded(&self, document: &DocumentContent) -> usize {
        document.thumbnails_loaded()
    }
}

impl Default for PreviewService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preview_service_creation() {
        let service = PreviewService::new();
        assert_eq!(service.thumbnail_size(), 256);
    }

    #[test]
    fn test_preview_service_with_size() {
        let service = PreviewService::with_thumbnail_size(512);
        assert_eq!(service.thumbnail_size(), 512);
    }

    #[test]
    fn test_set_thumbnail_size() {
        let mut service = PreviewService::new();
        service.set_thumbnail_size(128);
        assert_eq!(service.thumbnail_size(), 128);
    }
}
