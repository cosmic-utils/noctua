// SPDX-License-Identifier: GPL-3.0-or-later
// src/storage/document.rs
//
// Manages physical document files on the filesystem.

use super::StorageError;
use crate::document::model::{DocumentEntry, DocumentInfo, Kind, Metadata};
use chrono;
use std::fs;
use std::path::Path;
use std::time::SystemTime;
use uuid::Uuid;

/// Metadata about a physical file, obtained via `std::fs::metadata`.
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// Size of the file in bytes.
    pub size_bytes: u64,
    /// Last modification time (if available).
    pub modified: Option<SystemTime>,
}

/// Reads the entire file into memory.
///
/// Returns the raw bytes of the file. For large files this may be inefficient;
/// later switch to memory‑mapped I/O or streaming.
pub fn open(path: &Path) -> Result<Vec<u8>, StorageError> {
    let bytes = fs::read(path)?;
    Ok(bytes)
}

/// Retrieves basic file metadata (size, modification time).
pub fn metadata(path: &Path) -> Result<FileMetadata, StorageError> {
    let meta = fs::metadata(path)?;
    let size_bytes = meta.len();
    let modified = meta.modified().ok(); // SystemTime may fail on some filesystems
    Ok(FileMetadata {
        size_bytes,
        modified,
    })
}

/// Deletes a file from the filesystem.
///
/// **Warning:** This is a permanent operation. The UI must ask for confirmation
/// before issuing a `Command::Delete`.
pub fn delete(path: &Path) -> Result<(), StorageError> {
    fs::remove_file(path)?;
    Ok(())
}

/// Copies a file from `source` to `destination` (save as).
pub fn copy(source: &Path, destination: &Path) -> Result<u64, StorageError> {
    let bytes_copied = fs::copy(source, destination)?;
    Ok(bytes_copied)
}

/// Detected file format based on extension and magic bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Raster,
    Pdf,
    Svg,
    Unknown,
}

/// Maximum bytes to read for magic number detection.
const MAGIC_BYTES_LEN: usize = 16;

/// Detect document format from path and file content.
fn detect_format(path: &Path, first_bytes: &[u8]) -> Format {
    // Check extension first
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());

    // Check magic bytes for common formats
    if first_bytes.len() >= 8 {
        // PNG
        if first_bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
            return Format::Raster;
        }
        // JPEG
        if first_bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Format::Raster;
        }
        // GIF
        if first_bytes.starts_with(b"GIF87a") || first_bytes.starts_with(b"GIF89a") {
            return Format::Raster;
        }
        // BMP
        if first_bytes.starts_with(b"BM") {
            return Format::Raster;
        }
        // PDF
        if first_bytes.starts_with(b"%PDF-") {
            return Format::Pdf;
        }
        // SVG (check for XML declaration or SVG root)
        if first_bytes.starts_with(b"<?xml") || first_bytes.starts_with(b"<svg") {
            return Format::Svg;
        }
        // TIFF (big/little endian)
        if first_bytes.starts_with(&[0x49, 0x49, 0x2A, 0x00]) || // II*
           first_bytes.starts_with(&[0x4D, 0x4D, 0x00, 0x2A])
        {
            // MM*
            return Format::Raster;
        }
        // WebP
        if first_bytes.len() >= 12
            && first_bytes[0..4] == [0x52, 0x49, 0x46, 0x46]
            && first_bytes[8..12] == [0x57, 0x45, 0x42, 0x50]
        {
            return Format::Raster;
        }
    }

    // Fall back to extension if magic bytes inconclusive
    match ext.as_deref() {
        Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("bmp") | Some("tiff")
        | Some("tif") | Some("webp") => Format::Raster,
        Some("pdf") => Format::Pdf,
        Some("svg") => Format::Svg,
        _ => Format::Unknown,
    }
}

/// Read the first few bytes of a file for magic number detection.
fn read_first_bytes(path: &Path) -> Result<Vec<u8>, StorageError> {
    use std::io::Read;

    let mut file = fs::File::open(path)?;
    let mut buffer = vec![0u8; MAGIC_BYTES_LEN];
    let bytes_read = file.read(&mut buffer)?;
    buffer.truncate(bytes_read);
    Ok(buffer)
}

/// Load a document from a file path, extracting all metadata without loading content.
///
/// This is the central document loading function that follows the same pattern as
/// `storage::workspace::load()`. It detects the document format, extracts format-specific
/// metadata, and returns a complete `DocumentEntry` ready for the `DocumentManager`.
///
/// # Errors
/// Returns `StorageError` if the file cannot be read, is corrupted, or the format
/// is unsupported.
pub fn load(path: &Path) -> Result<DocumentEntry, StorageError> {
    // 1. Basic file metadata (size, modification time)
    let file_meta = metadata(path)?;

    // 2. Read first bytes for magic number detection
    let first_bytes = read_first_bytes(path)?;

    // 3. Detect format
    let format = detect_format(path, &first_bytes);

    // 4. Extract format-specific metadata
    let (kind, number_of_pages) = match format {
        Format::Raster => {
            let (raster, pages) = super::raster::load_raster_metadata(path)?;
            (Kind::Raster(raster), pages)
        }
        Format::Pdf => {
            let (portable, pages) = super::portable::load_pdf_metadata(path)?;
            (Kind::Portable(portable), pages)
        }
        Format::Svg => {
            let (vector, pages) = super::vector::load_svg_metadata(path)?;
            (Kind::Vector(vector), pages)
        }
        Format::Unknown => (Kind::Unknown, 1),
    };

    // 5. Extract EXIF metadata (primarily for raster images)
    let mut doc_metadata = Metadata::default();

    // Add file modification time to metadata
    if let Some(modified) = file_meta.modified {
        doc_metadata.modified_at = Some(chrono::DateTime::from(modified));
    }

    // TODO: Extract additional metadata (EXIF, XMP, etc.)

    // 6. Construct DocumentEntry
    Ok(DocumentEntry {
        id: Uuid::new_v4(),
        path: path.to_path_buf(),
        display_name: None,
        current_page: 1,
        rotation_degrees: 0,
        flip_horizontal: false,
        flip_vertical: false,
        info: Some(DocumentInfo {
            file_size_bytes: file_meta.size_bytes,
            number_of_pages,
            kind,
            metadata: doc_metadata,
        }),
    })
}
