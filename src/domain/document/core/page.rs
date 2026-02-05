// SPDX-License-Identifier: GPL-3.0-or-later
// src/domain/document/core/page.rs
//
// Page abstraction for multi-page documents.

use cosmic::widget::image::Handle as ImageHandle;

/// Represents a single page in a multi-page document.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Page {
    /// Page index (0-based).
    pub index: usize,
    /// Page width in pixels.
    pub width: u32,
    /// Page height in pixels.
    pub height: u32,
    /// Optional thumbnail handle.
    pub thumbnail: Option<ImageHandle>,
}

impl Page {
    /// Create a new page.
    #[must_use]
    pub fn new(index: usize, width: u32, height: u32) -> Self {
        Self {
            index,
            width,
            height,
            thumbnail: None,
        }
    }

    /// Create a page with a thumbnail.
    #[must_use]
    pub fn with_thumbnail(index: usize, width: u32, height: u32, thumbnail: ImageHandle) -> Self {
        Self {
            index,
            width,
            height,
            thumbnail: Some(thumbnail),
        }
    }

    /// Set the thumbnail for this page.
    pub fn set_thumbnail(&mut self, thumbnail: ImageHandle) {
        self.thumbnail = Some(thumbnail);
    }

    /// Check if this page has a thumbnail.
    #[must_use]
    pub fn has_thumbnail(&self) -> bool {
        self.thumbnail.is_some()
    }

    /// Get the aspect ratio of the page.
    #[must_use]
    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            1.0
        } else {
            #[allow(clippy::cast_precision_loss)]
            {
                self.width as f32 / self.height as f32
            }
        }
    }

    /// Get page dimensions as a tuple.
    #[must_use]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
