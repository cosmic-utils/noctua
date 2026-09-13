// SPDX-License-Identifier: GPL-3.0-or-later
// core/src/document.rs
//
// Pure document data types. No logic, no UI, no filesystem.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Specific properties for raster graphics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Raster {
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub dpi: Option<(u32, u32)>,
    pub color_space: String,
}

/// Specific properties for vector graphics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vector {
    pub format: String,
    pub viewbox_width: f32,
    pub viewbox_height: f32,
}

/// Specific properties for portable documents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portable {
    pub format: String,
    pub version: String,
    pub is_encrypted: bool,
    pub has_text_layer: bool,
    /// Detected by structural scan; never executed by Noctua.
    pub has_javascript: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Metadata {
    pub author: Option<String>,
    pub creator: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub title: Option<String>,
    pub description: Option<String>,
    /// Generic key-value store for Exif, XMP, or custom backend metadata.
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Kind {
    Raster(Raster),
    Vector(Vector),
    Portable(Portable),
    Unknown,
}

/// Metadata about a single document, extracted from the file itself.
///
/// The UI currently shows only the file size; the richer fields are the
/// tested basis for the planned status bar metadata display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentInfo {
    pub file_size_bytes: u64,
    pub number_of_pages: u32, // A document has a minimum of 1 page
    pub kind: Kind,
    pub metadata: Metadata,
}

/// Detailed layout for a single page within a paged document.
#[derive(Debug, Clone)]
pub struct PageInfo {
    /// Page width in document points.
    pub width_pt: f32,
    /// Page height in document points.
    pub height_pt: f32,
}
