// SPDX-License-Identifier: GPL-3.0-or-later
// src/domain/document/operations/transform.rs
//
// Document transformation operations.
//
// This module provides two levels of transformation operations:
//
// 1. **Low-level operations** (internal) for direct pixel manipulation on raster images:
//    - `apply_rotation()` - Rotate pixels by 90°, 180°, or 270° [pub(crate)]
//    - `apply_flip()` - Flip pixels horizontally or vertically [pub(crate)]
//    - `crop_image()` - Crop to a specific region [pub(crate)]
//    These are used internally by document type implementations only.
//
// 2. **High-level operations** that work on any document type (raster, vector, PDF):
//    - `rotate_document_cw()` - Rotate any document 90° clockwise
//    - `rotate_document_ccw()` - Rotate any document 90° counter-clockwise
//    - `flip_document_horizontal()` - Flip any document horizontally
//    - `flip_document_vertical()` - Flip any document vertically
//    - `rotate_document_to()` - Rotate to a specific angle
//    - `reset_document_transforms()` - Reset all transformations
//
// ## Usage Example
//
// ```rust
// use crate::domain::document::operations::transform;
//
// // High-level: Works with any DocumentContent (RECOMMENDED)
// let mut document = DocumentContent::Raster(raster_doc);
// transform::rotate_document_cw(&mut document)?;
// transform::flip_document_horizontal(&mut document)?;
// ```
//
// Note: Low-level operations (apply_rotation, apply_flip, crop_image) are
// internal helpers used by document type implementations and are not part
// of the public API.
//
// The high-level operations use the `Transformable` trait and work across all
// document types (Raster, Vector, Portable), while low-level operations work
// directly on pixel data.

use image::{DynamicImage, GenericImageView};

use crate::domain::document::core::content::DocumentContent;
use crate::domain::document::core::document::{
    DocResult, FlipDirection, Rotation, RotationMode, Transformable,
};

/// Apply a 90-degree rotation to a raster image.
///
/// This function performs the actual pixel manipulation for standard rotations.
/// Used internally by `RasterDocument` implementation.
#[must_use]
pub(crate) fn apply_rotation(img: DynamicImage, rotation: Rotation) -> DynamicImage {
    use image::imageops::{rotate180, rotate270, rotate90};

    match rotation {
        Rotation::None => img,
        Rotation::Cw90 => DynamicImage::ImageRgba8(rotate90(&img.to_rgba8())),
        Rotation::Cw180 => DynamicImage::ImageRgba8(rotate180(&img.to_rgba8())),
        Rotation::Cw270 => DynamicImage::ImageRgba8(rotate270(&img.to_rgba8())),
    }
}

/// Apply a flip transformation to a raster image.
///
/// This function performs the actual pixel manipulation for flip operations.
/// Used internally by `RasterDocument` and `PortableDocument` implementations.
#[must_use]
pub(crate) fn apply_flip(img: DynamicImage, direction: FlipDirection) -> DynamicImage {
    use image::imageops::{flip_horizontal, flip_vertical};

    match direction {
        FlipDirection::Horizontal => DynamicImage::ImageRgba8(flip_horizontal(&img.to_rgba8())),
        FlipDirection::Vertical => DynamicImage::ImageRgba8(flip_vertical(&img.to_rgba8())),
    }
}

/// Crop a raster image to the specified region.
///
/// Coordinates are in pixels relative to the top-left corner.
/// Returns None if the crop region is invalid.
/// Used internally for crop operations.
#[must_use]
pub(crate) fn crop_image(
    img: &DynamicImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Option<DynamicImage> {
    let (img_width, img_height) = img.dimensions();

    // Validate crop region
    if x >= img_width || y >= img_height {
        return None;
    }

    // Clamp dimensions to image bounds
    let crop_width = width.min(img_width - x);
    let crop_height = height.min(img_height - y);

    if crop_width == 0 || crop_height == 0 {
        return None;
    }

    Some(img.crop_imm(x, y, crop_width, crop_height))
}

/// Calculate dimensions after rotation.
///
/// For 90° and 270° rotations, width and height are swapped.
#[must_use]
pub fn dimensions_after_rotation(width: u32, height: u32, rotation: Rotation) -> (u32, u32) {
    match rotation {
        Rotation::None | Rotation::Cw180 => (width, height),
        Rotation::Cw90 | Rotation::Cw270 => (height, width),
    }
}

// ============================================================================
// High-Level Document Operations (Type-agnostic)
// ============================================================================
//
// These operations work on ANY document type (Raster, Vector, Portable) through
// the DocumentContent abstraction. They should be preferred over direct trait
// calls when implementing UI commands or application logic.
//
// Benefits:
// - Single API for all document types
// - Handles rotation mode conversions (Standard ↔ Fine)
// - Returns Result for error handling
// - Future-proof for new document types

/// Rotate a document 90 degrees clockwise.
///
/// This operation works on any document type (Raster, Vector, Portable) by
/// delegating to the underlying document's `Transformable` implementation.
///
/// # Examples
///
/// ```no_run
/// use crate::domain::document::operations::transform::rotate_document_cw;
///
/// // Works with any document type
/// rotate_document_cw(&mut document)?;
/// ```
///
/// # Implementation Details
///
/// - Raster: Actual pixel rotation via image operations
/// - Vector: Viewport matrix transformation (lossless)
/// - Portable: View rotation, rendered by backend
pub fn rotate_document_cw(document: &mut DocumentContent) -> DocResult<()> {
    let new_rotation_mode = document.transform_state().rotation.rotate_cw();

    match new_rotation_mode {
        RotationMode::Standard(rot) => {
            document.rotate(rot);
        }
        RotationMode::Fine(deg) => {
            // Convert to nearest 90° rotation
            let normalized = ((deg / 90.0).round() as i16 * 90) % 360;
            let rot = match normalized {
                0 => Rotation::None,
                90 => Rotation::Cw90,
                180 => Rotation::Cw180,
                270 => Rotation::Cw270,
                _ => Rotation::None,
            };
            document.rotate(rot);
        }
    }

    Ok(())
}

/// Rotate a document 90 degrees counter-clockwise.
///
/// This operation works on any document type (Raster, Vector, Portable) by
/// delegating to the underlying document's `Transformable` implementation.
///
/// # Examples
///
/// ```no_run
/// use crate::domain::document::operations::transform::rotate_document_ccw;
///
/// rotate_document_ccw(&mut document)?;
/// ```
pub fn rotate_document_ccw(document: &mut DocumentContent) -> DocResult<()> {
    let new_rotation_mode = document.transform_state().rotation.rotate_ccw();

    match new_rotation_mode {
        RotationMode::Standard(rot) => {
            document.rotate(rot);
        }
        RotationMode::Fine(deg) => {
            // Convert to nearest 90° rotation
            let normalized = ((deg / 90.0).round() as i16 * 90 + 360) % 360;
            let rot = match normalized {
                0 => Rotation::None,
                90 => Rotation::Cw90,
                180 => Rotation::Cw180,
                270 => Rotation::Cw270,
                _ => Rotation::None,
            };
            document.rotate(rot);
        }
    }

    Ok(())
}

/// Flip a document horizontally (mirror left-right).
///
/// This operation works on any document type by delegating to the underlying
/// document's `Transformable` implementation.
///
/// # Examples
///
/// ```no_run
/// use crate::domain::document::operations::transform::flip_document_horizontal;
///
/// flip_document_horizontal(&mut document)?;
/// ```
pub fn flip_document_horizontal(document: &mut DocumentContent) -> DocResult<()> {
    document.flip(FlipDirection::Horizontal);
    Ok(())
}

/// Flip a document vertically (mirror top-bottom).
///
/// This operation works on any document type by delegating to the underlying
/// document's `Transformable` implementation.
///
/// # Examples
///
/// ```no_run
/// use crate::domain::document::operations::transform::flip_document_vertical;
///
/// flip_document_vertical(&mut document)?;
/// ```
pub fn flip_document_vertical(document: &mut DocumentContent) -> DocResult<()> {
    document.flip(FlipDirection::Vertical);
    Ok(())
}

/// Rotate a document to a specific angle (0°, 90°, 180°, or 270°).
///
/// This operation works on any document type by delegating to the underlying
/// document's `Transformable` implementation.
///
/// # Arguments
///
/// * `document` - The document to rotate
/// * `rotation` - Target rotation angle
///
/// # Examples
///
/// ```no_run
/// use crate::domain::document::core::document::Rotation;
/// use crate::domain::document::operations::transform::rotate_document_to;
///
/// // Rotate to 180 degrees
/// rotate_document_to(&mut document, Rotation::Cw180)?;
/// ```
pub fn rotate_document_to(document: &mut DocumentContent, rotation: Rotation) -> DocResult<()> {
    document.rotate(rotation);
    Ok(())
}

/// Reset all transformations on a document.
///
/// This resets the document to its original state (no rotation, no flips).
/// Useful for implementing "Reset View" functionality.
///
/// # Examples
///
/// ```no_run
/// use crate::domain::document::operations::transform::reset_document_transforms;
///
/// // Undo all rotations and flips
/// reset_document_transforms(&mut document)?;
/// ```
pub fn reset_document_transforms(document: &mut DocumentContent) -> DocResult<()> {
    // Reset to no rotation
    document.rotate(Rotation::None);

    // Reset flips by checking current state and flipping back if needed
    let state = document.transform_state();
    if state.flip_h {
        document.flip(FlipDirection::Horizontal);
    }
    if state.flip_v {
        document.flip(FlipDirection::Vertical);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimensions_after_rotation() {
        assert_eq!(
            dimensions_after_rotation(100, 200, Rotation::None),
            (100, 200)
        );
        assert_eq!(
            dimensions_after_rotation(100, 200, Rotation::Cw90),
            (200, 100)
        );
        assert_eq!(
            dimensions_after_rotation(100, 200, Rotation::Cw180),
            (100, 200)
        );
        assert_eq!(
            dimensions_after_rotation(100, 200, Rotation::Cw270),
            (200, 100)
        );
    }
}
