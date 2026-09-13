// SPDX-License-Identifier: GPL-3.0-or-later
// core/src/pdfium_ops/command.rs
//
// The language of the annotation mode. All operations on an open PDF
// document are expressed as commands.

use crate::pdfium_ops::model::{AnnotationColor, BindSource};
use std::path::PathBuf;

/// Commands accepted by the PDF operations manager.
#[derive(Debug, Clone)]
pub enum Command {
    /// Open an existing PDF for editing. Replaces any currently open document.
    Open { path: PathBuf },

    /// Create a new empty PDF and open it.
    New,

    /// Close the current document without saving.
    Close,

    /// Bind source documents into a new PDF and open it.
    /// PDF sources are imported page by page; raster and SVG sources are
    /// embedded as full-page images.
    Bind {
        sources: Vec<BindSource>,
        target: PathBuf,
    },

    /// Insert pages of a source document at the given 1-based position.
    InsertPages { source: BindSource, at: u32 },

    /// Delete the given 1-based pages.
    DeletePages { pages: Vec<u32> },

    /// Move a 1-based page to a new 1-based position.
    MovePage { from: u32, to: u32 },

    /// Rotate a 1-based page by 0, 90, 180 or 270 degrees (absolute).
    RotatePage { page: u32, degrees: u16 },

    /// Add a text (pop-up comment) annotation at the given page and position.
    AddTextAnnotation {
        page: u32,
        text: String,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },

    /// Add an ink (freehand drawing) annotation. The points are given in
    /// page coordinates (points, origin bottom-left).
    AddInkAnnotation {
        page: u32,
        color: AnnotationColor,
        points: Vec<(f32, f32)>,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },

    /// Add a highlight annotation over the given rectangle.
    AddHighlightAnnotation {
        page: u32,
        color: AnnotationColor,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },

    /// Save the current document to its open path.
    Save,

    /// Save the current document to a new path and continue editing there.
    SaveAs { path: PathBuf },

    /// Number of pages in the open document.
    PageCount,

    /// Width and height of every page in points, in page order.
    PageSizes,

    /// Render a 1-based page at the given zoom factor, returning RGBA pixels.
    RenderPage { page: u32, zoom: f32 },
}

/// Result of executing a command.
#[derive(Debug)]
pub enum CommandResult {
    /// Command executed successfully.
    Ok,
    /// Number of pages in the open document.
    PageCount(u32),
    /// Width and height of every page in points, in page order.
    PageSizes(Vec<(f32, f32)>),
    /// A rendered page as RGBA pixels.
    Rendered {
        width: u32,
        height: u32,
        rgba_data: Vec<u8>,
    },
    /// Command failed.
    Error(crate::pdfium_ops::PdfOpsError),
}

impl From<Result<(), crate::pdfium_ops::PdfOpsError>> for CommandResult {
    fn from(res: Result<(), crate::pdfium_ops::PdfOpsError>) -> Self {
        match res {
            Ok(()) => CommandResult::Ok,
            Err(e) => CommandResult::Error(e),
        }
    }
}
