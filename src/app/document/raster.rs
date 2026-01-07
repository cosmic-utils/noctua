// SPDX-License-Identifier: MPL-2.0
// src/app/document/raster.rs

use std::path::PathBuf;

use cosmic::iced::widget::image as iced_image;
use image::{GenericImageView, DynamicImage, ImageReader};

/// Represents a raster image document (PNG, JPEG, WebP, ...).
pub struct RasterDocument {
    pub path: Option<PathBuf>,
    pub image: DynamicImage,
    pub handle: iced_image::Handle,
}

impl RasterDocument {
    /// Load a raster document from disk.
    pub fn open(path: PathBuf) -> image::ImageResult<Self> {
        let img = ImageReader::open(&path)?.decode()?;
        let handle = Self::build_handle(&img);

        Ok(Self {
            path: Some(path),
            image: img,
            handle,
        })
    }

    /// Construct a handle from a DynamicImage.
    fn build_handle(img: &DynamicImage) -> iced_image::Handle {
        // Get image dimensions.
        let (w, h) = img.dimensions();

        // Convert to RGBA8 buffer and extract raw bytes.
        let rgba = img.to_rgba8();
        let pixels = rgba.into_raw(); // Vec<u8>

        // Build an iced image handle from raw RGBA pixels.
        iced_image::Handle::from_rgba(w, h, pixels)
    }

    /// Rebuild the handle after mutating `image`.
    pub fn refresh_handle(&mut self) {
        self.handle = Self::build_handle(&self.image);
    }

    /// Returns the native pixel dimensions (width, height).
    pub fn dimensions(&self) -> (u32, u32) {
        self.image.dimensions()
    }

    /// Save the current image back to disk (overwrite).
    pub fn save(&self) -> image::ImageResult<()> {
        if let Some(path) = &self.path {
            self.image.save(path)
        } else {
            // Cant imagine that it happen but caller should handle missing path case.
            Err(image::ImageError::Parameter(
                image::error::ParameterError::from_kind(image::error::ParameterErrorKind::Generic(
                    "RasterDocument does not have a path".into(),
                )),
            ))
        }
    }
}
