// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/document/vector.rs
//
// Vector documents (SVG, etc.).

use std::path::PathBuf;

use cosmic::iced::widget::image as iced_image;

/// Represents a vector document such as SVG.
/// For now this only stores the raw data and a rasterized handle.
pub struct VectorDocument {
    pub path: PathBuf,
    pub raw_data: String,
    pub handle: iced_image::Handle,
    /// Cached dimensions of the rasterized representation.
    pub width: u32,
    pub height: u32,
}

impl VectorDocument {
    pub fn open(path: PathBuf) -> anyhow::Result<Self> {
        let raw_data = std::fs::read_to_string(&path)?;

        // TODO: proper SVG parsing and rendering.
        // For now, use a placeholder size based on a typical default.
        let (width, height) = (800, 600);
        let handle = iced_image::Handle::from_rgba(1, 1, vec![0, 0, 0, 0]);

        Ok(Self {
            path,
            raw_data,
            handle,
            width,
            height,
        })
    }

    /// Returns the dimensions of the rasterized representation.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn refresh_handle(&mut self) {
        // TODO: re-render SVG to DynamicImage and rebuild handle.
        // Update self.width and self.height accordingly.
    }
}
