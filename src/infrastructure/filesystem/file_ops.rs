// SPDX-License-Identifier: GPL-3.0-or-later
// src/infrastructure/filesystem/file_ops.rs
//
// File system operations for document handling.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::anyhow;

use crate::domain::document::core::content::{DocumentContent, DocumentKind};

use crate::domain::document::types::raster::RasterDocument;
#[cfg(feature = "vector")]
use crate::domain::document::types::vector::VectorDocument;
#[cfg(feature = "portable")]
use crate::domain::document::types::portable::PortableDocument;

/// Open a document from a file path and dispatch to the correct type.
///
/// Raster formats are delegated to the `image` crate, which decides
/// based on enabled codecs (e.g. default-formats).
pub fn open_document(path: &Path) -> anyhow::Result<DocumentContent> {
    let kind = DocumentKind::from_path(path)
        .ok_or_else(|| anyhow!("Unsupported document type: {}", path.display()))?;

    let content = match kind {
        DocumentKind::Raster => {
            let raster = RasterDocument::open(path)?;
            DocumentContent::Raster(raster)
        }
        #[cfg(feature = "vector")]
        DocumentKind::Vector => {
            let vector = VectorDocument::open(path)?;
            DocumentContent::Vector(vector)
        }
        #[cfg(feature = "portable")]
        DocumentKind::Portable => {
            let portable = PortableDocument::open(path)?;
            DocumentContent::Portable(portable)
        }
        #[cfg(not(any(feature = "vector", feature = "portable")))]
        _ => return Err(anyhow!("No document features enabled")),
    };

    Ok(content)
}

/// Collect all supported document files from a directory, sorted alphabetically.
///
/// This scans the directory and returns a list of files that are recognized as
/// supported document types (images, PDFs, SVGs, etc.).
pub fn collect_supported_files(dir: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = Vec::new();

    if let Ok(read_dir) = fs::read_dir(dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();

            // Only keep regular files that are recognized as supported documents.
            if path.is_file() && DocumentKind::from_path(&path).is_some() {
                entries.push(path);
            }
        }
    }

    entries.sort();
    entries
}

// ---------------------------------------------------------------------------
// File metadata helpers
// ---------------------------------------------------------------------------

/// Retrieve the file size in bytes. Returns 0 if the file cannot be accessed.
pub fn file_size(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// Read raw bytes from a file for metadata extraction (e.g., EXIF).
/// Returns None if the file cannot be read.
pub fn read_file_bytes(path: &Path) -> Option<Vec<u8>> {
    fs::read(path).ok()
}

// ---------------------------------------------------------------------------
// DEPRECATED FUNCTIONS
// ---------------------------------------------------------------------------
// The following functions have been replaced by DocumentManager and are
// commented out to avoid AppModel dependencies.
//
// Instead of using these functions directly, use:
// - DocumentManager::open_document() for opening files
// - DocumentManager::next_document() / previous_document() for navigation
// - Application commands for operations like crop, save, etc.
// ---------------------------------------------------------------------------

/*
/// Open the initial path passed on the command line.
///
/// DEPRECATED: Use DocumentManager::open_document() instead.
pub fn open_initial_path(model: &mut AppModel, path: &PathBuf) {
    if path.is_dir() {
        open_from_directory(model, path);
    } else {
        open_single_file(model, path);
    }
}

/// Open the first supported document from the given directory.
///
/// DEPRECATED: Use DocumentManager::open_document() instead.
pub fn open_from_directory(model: &mut AppModel, dir: &Path) {
    let entries = collect_supported_files(dir);

    if entries.is_empty() {
        model.set_error(format!(
            "No supported documents found in directory: {}",
            dir.display()
        ));
        return;
    }

    let first = entries[0].clone();
    model.folder_entries = entries;
    model.current_index = Some(0);

    load_document_into_model(model, &first);
}

/// Open a single file.
///
/// DEPRECATED: Use DocumentManager::open_document() instead.
pub fn open_single_file(model: &mut AppModel, path: &Path) {
    load_document_into_model(model, path);

    if model.document.is_some()
        && let Some(parent) = path.parent()
    {
        refresh_folder_entries(model, parent, path);
    }
}

/// Load a document into the model.
///
/// DEPRECATED: Use DocumentManager methods instead.
fn load_document_into_model(model: &mut AppModel, path: &Path) {
    // Implementation omitted - use DocumentManager instead
}

/// Refresh folder entries.
///
/// DEPRECATED: DocumentManager handles this automatically.
pub fn refresh_folder_entries(model: &mut AppModel, folder: &Path, current: &Path) {
    // Implementation omitted - use DocumentManager instead
}

/// Navigate to the next document.
///
/// DEPRECATED: Use DocumentManager::next_document() instead.
pub fn navigate_next(model: &mut AppModel) {
    // Implementation omitted - use DocumentManager instead
}

/// Navigate to the previous document.
///
/// DEPRECATED: Use DocumentManager::previous_document() instead.
pub fn navigate_prev(model: &mut AppModel) {
    // Implementation omitted - use DocumentManager instead
}

/// Apply crop operation.
///
/// DEPRECATED: Use CropDocumentCommand instead.
pub fn apply_crop(
    crop_selection: &CropSelection,
    doc: &DocumentContent,
    current_path: &Path,
    canvas_size: cosmic::iced::Size,
    image_size: cosmic::iced::Size,
    scale: f32,
    pan_x: f32,
    pan_y: f32,
    view_mode: &ViewMode,
) -> Result<PathBuf, String> {
    // Implementation omitted - use CropDocumentCommand instead
    Err("Deprecated function - use CropDocumentCommand".to_string())
}
*/
