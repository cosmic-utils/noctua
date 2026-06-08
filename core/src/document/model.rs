// SPDX-License-Identifier: GPL-3.0-or-later
// core/src/document/model.rs
//
// Core domain models representing different document types and their metadata.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(not(any(feature = "resvg", feature = "pdfium-render")))]
use std::marker::PhantomData;
use std::path::PathBuf;

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

/// The core document representation containing purely state and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentInfo {
    pub file_size_bytes: u64,
    pub number_of_pages: u32, // A document has a minimum of 1 page
    pub kind: Kind,
    pub metadata: Metadata,
}

/// The core document representation containing purely state and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentEntry {
    pub id: uuid::Uuid,
    pub path: PathBuf,

    /// User-defined display name overriding the filename.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    // User State / Non-destructive edits
    #[serde(default)]
    pub current_page: u32,
    #[serde(default)]
    pub rotation_degrees: u16,
    #[serde(default)]
    pub flip_horizontal: bool,
    #[serde(default)]
    pub flip_vertical: bool,

    #[serde(skip)]
    pub info: Option<DocumentInfo>,
}

impl DocumentEntry {
    /// Returns the total number of pages if the document info is loaded.
    pub fn total_pages(&self) -> u32 {
        self.info.as_ref().map(|i| i.number_of_pages).unwrap_or(1)
    }
}

// ── Render Engine Types ──

/// Content loaded from disk, kept in memory for the active document.
///
/// The render engine consumes this to produce RGBA pixels.
/// It is intentionally not serializable — it lives only in RAM.
#[derive(Debug)]
pub enum LoadedContent<'a> {
    /// Single raster image (PNG, JPEG, WebP, etc.).
    Raster {
        data: Vec<u8>,
        width: u32,
        height: u32,
    },
    /// SVG document parsed into a resvg tree.
    #[cfg(feature = "resvg")]
    Svg {
        tree: resvg::usvg::Tree,
        width: f32,
        height: f32,
    },
    /// PDF document loaded via pdfium.
    #[cfg(feature = "pdfium-render")]
    Pdf {
        document: pdfium_render::prelude::PdfDocument<'a>,
    },
    /// Placeholder variant that carries the lifetime when no render features are active.
    #[cfg(not(any(feature = "resvg", feature = "pdfium-render")))]
    _Phantom { _marker: PhantomData<&'a ()> },
}

/// Detailed layout for a single page within a paged document.
#[derive(Debug, Clone)]
pub struct PageInfo {
    /// Page width in document points.
    pub width_pt: f32,
    /// Page height in document points.
    pub height_pt: f32,
}
