// SPDX-License-Identifier: GPL-3.0-only
// core/src/storage/raster.rs
//
// Raster image metadata extraction (all formats the `image` crate decodes).

use std::path::Path;

use crate::document::Raster;
use crate::storage::StorageError;

/// Load raster image metadata without decoding pixel data.
///
/// Extracts image dimensions, format, and color space.
/// Returns a `Raster` struct and the number of pages (always 1 for raster images).
pub fn load_raster_metadata(path: &Path) -> Result<(Raster, u32), StorageError> {
    let (width, height) = raster_dimensions(path)?;

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

/// Read image dimensions, sniffing the format from magic bytes with an
/// extension fallback for formats without a signature (TGA).
fn raster_dimensions(path: &Path) -> Result<(u32, u32), StorageError> {
    if let Ok(reader) = image::ImageReader::open(path)
        && let Ok(dims) = reader.into_dimensions()
    {
        return Ok(dims);
    }

    let format = image::ImageFormat::from_path(path)
        .map_err(|e| StorageError::Document(format!("Unsupported image format: {e}")))?;
    let mut reader = image::ImageReader::open(path)
        .map_err(|e| StorageError::Document(format!("Failed to open image: {e}")))?;
    reader.set_format(format);
    reader
        .into_dimensions()
        .map_err(|e| StorageError::Document(format!("Failed to read image dimensions: {e}")))
}
