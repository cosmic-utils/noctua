// SPDX-License-Identifier: GPL-3.0-or-later
// core/src/pdfium_ops/model.rs
//
// Data types for PDF operations. Pure data, no logic.

use std::path::PathBuf;

/// A source document to bind or insert. Pages are referenced relative
/// to the source: `pages` None means all pages, `Some(range)` selects
/// a 1-based inclusive page range string like "1,3-5".
#[derive(Debug, Clone)]
pub struct BindSource {
    pub path: PathBuf,
    /// Optional page selection, 1-based inclusive range string ("1,3-5").
    pub pages: Option<String>,
}

/// Reference to a single page of a source document. 1-based page number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRef {
    pub page: u32,
}

/// RGBA color for annotations and markup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnnotationColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl AnnotationColor {
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 255,
        }
    }
}

/// PDF properties read from a file via pdfium.
#[derive(Debug, Clone)]
pub struct PdfMetadata {
    /// PDF specification version as a display string ("1.7", "2.0", …).
    pub version: String,
    /// Whether the document uses a security handler (i.e. is encrypted).
    pub is_encrypted: bool,
    /// Whether at least one page exposes a text layer.
    pub has_text_layer: bool,
    /// Number of pages in the document.
    pub page_count: u32,
}

/// Common annotation colors used by the annotation mode.
pub mod palette {
    use super::AnnotationColor;

    pub const YELLOW: AnnotationColor = AnnotationColor::rgb(255, 235, 59);
    pub const GREEN: AnnotationColor = AnnotationColor::rgb(76, 175, 80);
    pub const RED: AnnotationColor = AnnotationColor::rgb(244, 67, 54);
    pub const BLUE: AnnotationColor = AnnotationColor::rgb(33, 150, 243);
    pub const ORANGE: AnnotationColor = AnnotationColor::rgb(255, 152, 0);
}
