// SPDX-License-Identifier: MPL-2.0 OR Apache-2.0
// src/app/document/portable.rs
//
// Portable documents (e.g. PDF) – basic model and rendering stub.

use std::path::PathBuf;

use cosmic::iced::widget::image as iced_image;
use image::{GenericImageView, DynamicImage};

/// Represents a portable document (PDF).
pub struct PortableDocument {
    pub path: PathBuf,
    pub page_count: u32,
    pub current_page: u32,
    pub rotation: i32, // 0, 90, 180, 270; kept for future backend integration
    pub rendered: DynamicImage,
    pub handle: iced_image::Handle,
    // TODO: internal PDF handle from chosen backend
}

impl PortableDocument {
    /// Open a portable document and render the first page.
    ///
    /// Currently this uses a dummy 1x1 transparent image as placeholder.
    pub fn open(path: PathBuf) -> anyhow::Result<Self> {
        // TODO: open PDF and render first page using a proper backend.
        let dummy = DynamicImage::new_rgba8(1, 1);
        let handle = Self::build_handle(&dummy);

        Ok(Self {
            path,
            page_count: 1, // TODO: query real page count from backend
            current_page: 0,
            rotation: 0,
            rendered: dummy,
            handle,
        })
    }

    /// Construct an iced image handle from a DynamicImage.
    fn build_handle(img: &DynamicImage) -> iced_image::Handle {
        let (w, h) = img.dimensions();
        let rgba = img.to_rgba8();
        let pixels = rgba.into_raw();
        iced_image::Handle::from_rgba(w, h, pixels)
    }

    /// Rebuild the handle after mutating `rendered`.
    pub fn refresh_handle(&mut self) {
        self.handle = Self::build_handle(&self.rendered);
    }

    /// Returns the dimensions of the currently rendered page.
    pub fn dimensions(&self) -> (u32, u32) {
        self.rendered.dimensions()
    }

    /// Re-render the current page with the current rotation.
    pub fn rerender_page(&mut self) {
        // TODO: use PDF backend and self.rotation / self.current_page
        // self.rendered = render_page_to_dynamic(...);
        // self.refresh_handle();
    }
}
