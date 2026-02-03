// SPDX-License-Identifier: GPL-3.0-or-later
// src/domain/document/operations/render.rs
//
// Rendering operations for documents.

use cosmic::widget::image::Handle as ImageHandle;
use image::{DynamicImage, GenericImageView};

/// Create an image handle from RGBA pixel data.
///
/// This is the primary way to create image handles for display in the UI.
#[must_use]
pub fn create_image_handle(pixels: Vec<u8>, width: u32, height: u32) -> ImageHandle {
    ImageHandle::from_rgba(width, height, pixels)
}

/// Create an image handle from a `DynamicImage`.
///
/// Converts the image to RGBA8 format and creates a handle.
#[must_use]
pub fn create_image_handle_from_image(img: &DynamicImage) -> ImageHandle {
    let (width, height) = img.dimensions();
    let pixels = img.to_rgba8().into_raw();
    create_image_handle(pixels, width, height)
}

/// Refresh image handle from a `DynamicImage`.
///
/// Alias for `create_image_handle_from_image` for compatibility.
#[must_use]
pub fn refresh_handle_from_image(img: &DynamicImage) -> ImageHandle {
    create_image_handle_from_image(img)
}

/// Calculate scaled dimensions maintaining aspect ratio.
///
/// Returns (width, height) scaled by the given factor.
#[must_use]
pub fn scale_dimensions(width: u32, height: u32, scale: f64) -> (u32, u32) {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let scaled_width = (f64::from(width) * scale).round() as u32;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let scaled_height = (f64::from(height) * scale).round() as u32;

    (scaled_width.max(1), scaled_height.max(1))
}

/// Calculate scale factor to fit dimensions into a target size.
///
/// Returns a scale factor that will make the image fit within the target
/// dimensions while maintaining aspect ratio.
#[must_use]
pub fn calculate_fit_scale(width: u32, height: u32, target_width: u32, target_height: u32) -> f64 {
    if width == 0 || height == 0 {
        return 1.0;
    }

    let width_scale = f64::from(target_width) / f64::from(width);
    let height_scale = f64::from(target_height) / f64::from(height);

    width_scale.min(height_scale)
}

/// Calculate scale factor to fill dimensions.
///
/// Returns a scale factor that will make the image fill the target dimensions
/// while maintaining aspect ratio (may crop).
#[must_use]
pub fn calculate_fill_scale(width: u32, height: u32, target_width: u32, target_height: u32) -> f64 {
    if width == 0 || height == 0 {
        return 1.0;
    }

    let width_scale = f64::from(target_width) / f64::from(width);
    let height_scale = f64::from(target_height) / f64::from(height);

    width_scale.max(height_scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_dimensions() {
        assert_eq!(scale_dimensions(100, 200, 2.0), (200, 400));
        assert_eq!(scale_dimensions(100, 200, 0.5), (50, 100));
        assert_eq!(scale_dimensions(100, 200, 0.0), (1, 1)); // Minimum 1x1
    }

    #[test]
    fn test_calculate_fit_scale() {
        // Landscape image fitting into square
        assert_eq!(calculate_fit_scale(200, 100, 100, 100), 0.5);
        // Portrait image fitting into square
        assert_eq!(calculate_fit_scale(100, 200, 100, 100), 0.5);
        // Square into square
        assert_eq!(calculate_fit_scale(100, 100, 100, 100), 1.0);
    }

    #[test]
    fn test_calculate_fill_scale() {
        // Landscape image filling square
        assert_eq!(calculate_fill_scale(200, 100, 100, 100), 1.0);
        // Portrait image filling square
        assert_eq!(calculate_fill_scale(100, 200, 100, 100), 1.0);
    }
}
