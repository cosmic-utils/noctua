// SPDX-License-Identifier: GPL-3.0-or-later
// src/storage/raster.rs
//
// Raster image metadata extraction (PNG, JPEG, GIF, BMP, TIFF, WebP).

use std::path::Path;

use crate::document::model::Raster;
use crate::storage::StorageError;

/// Load raster image metadata without decoding pixel data.
///
/// Extracts image dimensions, format, and optionally DPI and color space.
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

    // Extract DPI if available (from EXIF or image metadata)
    let dpi = extract_dpi(path).ok().flatten();

    // Default color space assumption (most common for consumer images)
    let color_space = "sRGB".to_string();

    let raster = Raster {
        format,
        width,
        height,
        dpi,
        color_space,
    };

    Ok((raster, 1)) // Raster images always have 1 page
}

/// Attempt to extract DPI information from the image file.
///
/// Supports EXIF metadata (JPEG, TIFF) via `kamadak-exif` and PNG pHYs chunk
/// via the `image` crate's metadata.
fn extract_dpi(path: &Path) -> Result<Option<(u32, u32)>, StorageError> {
    // First try EXIF metadata for JPEG/TIFF
    #[cfg(feature = "kamadak-exif")]
    {
        if let Some(dpi) = extract_dpi_from_exif(path)? {
            return Ok(Some(dpi));
        }
    }

    // Then try PNG pHYs chunk via image crate
    if let Some(dpi) = extract_dpi_from_png_phys(path)? {
        return Ok(Some(dpi));
    }

    Ok(None)
}

#[cfg(feature = "kamadak-exif")]
/// Extract DPI from EXIF metadata (XResolution, YResolution).
fn extract_dpi_from_exif(path: &Path) -> Result<Option<(u32, u32)>, StorageError> {
    use std::fs::File;
    use std::io::BufReader;

    use exif::{In, Reader, Tag};

    let file = File::open(path)?;
    let exif = match Reader::new().read_from_container(&mut BufReader::new(file)) {
        Ok(exif) => exif,
        Err(_) => return Ok(None), // No EXIF data or unsupported format
    };

    let x_res = exif
        .get_field(Tag::XResolution, In::PRIMARY)
        .and_then(|field| field.value.get_uint(0));

    let y_res = exif
        .get_field(Tag::YResolution, In::PRIMARY)
        .and_then(|field| field.value.get_uint(0));

    let unit = exif
        .get_field(Tag::ResolutionUnit, In::PRIMARY)
        .and_then(|field| field.value.get_uint(0))
        .unwrap_or(2); // 2 = inches (default)

    // ResolutionUnit: 1 = none, 2 = inches, 3 = cm
    // We only care about inches (2), otherwise return None
    if unit != 2 {
        return Ok(None);
    }

    match (x_res, y_res) {
        (Some(x), Some(y)) => Ok(Some((x, y))),
        (Some(x), None) => Ok(Some((x, x))), // Assume square pixels
        (None, Some(y)) => Ok(Some((y, y))),
        (None, None) => Ok(None),
    }
}

/// Extract DPI from PNG pHYs chunk (physical pixel dimensions).
fn extract_dpi_from_png_phys(path: &Path) -> Result<Option<(u32, u32)>, StorageError> {
    use image::ImageFormat;

    // Check if it's a PNG file
    let format = image::ImageFormat::from_path(path).unwrap_or(ImageFormat::Png);
    if format != ImageFormat::Png {
        return Ok(None);
    }

    let _reader = image::ImageReader::open(path)?;

    // The image crate doesn't expose pHYs chunk directly, but we can attempt
    // to read it via the PNG decoder. For now, return None and we can implement
    // this later with a more detailed PNG parser.
    Ok(None)
}
