// SPDX-License-Identifier: GPL-3.0-or-later
// src/document/update/transform.rs
//
// Document transformation logic (Rotation, Flip, Resize).

use crate::document::error::DocumentError;
use crate::document::model::DocumentEntry;

pub fn rotate(document: &mut DocumentEntry, degrees: u16) -> Result<(), DocumentError> {
    // Normalize to 0, 90, 180, 270
    document.rotation_degrees = (document.rotation_degrees + degrees) % 360;
    Ok(())
}

pub fn flip(
    document: &mut DocumentEntry,
    horizontal: bool,
    vertical: bool,
) -> Result<(), DocumentError> {
    if horizontal {
        document.flip_horizontal = !document.flip_horizontal;
    }
    if vertical {
        document.flip_vertical = !document.flip_vertical;
    }
    Ok(())
}

pub fn resize(
    _document: &mut DocumentEntry,
    _width: u32,
    _height: u32,
) -> Result<(), DocumentError> {
    // Currently dimensions are intrinsic to DocumentInfo.
    // If we want a persistent 'view resize', we should add it to Document.
    // For now, this is a placeholder for future 'scaling' logic.
    Ok(())
}
