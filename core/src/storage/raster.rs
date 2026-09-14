// SPDX-License-Identifier: GPL-3.0-or-later
// src/storage/raster.rs
//
// Raster image metadata extraction (PNG, JPEG, GIF, BMP, TIFF, WebP).

use std::path::Path;

use crate::document::Raster;
use crate::storage::StorageError;

/// Load raster image metadata without decoding pixel data.
///
/// Extracts image dimensions, format, and color space.
/// Returns a `Raster` struct and the number of pages (always 1 for raster images).
pub fn load_raster_metadata(path: &Path) -> Result<(Raster, u32), StorageError> {
    // Open the image file with the `image` crate (reads only the header)
    let reader = image::ImageReader::open(path)
        .map_err(|e| StorageError::Document(format!("Failed to open image: {}", e)))?;

    let (width, height) = reader
        .into_dimensions()
        .map_err(|e| StorageError::Document(format!("Failed to read image dimensions: {}", e)))?;

    // Determine format from file extension
    let format = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown")
        .to_string()
        .to_lowercase();

    // Default color space assumption (most common for consumer images)
    let color_space = "sRGB".to_string();

    let raster = Raster {
        format,
        width,
        height,
        dpi: None,
        color_space,
    };

    Ok((raster, 1)) // Raster images always have 1 page
}
