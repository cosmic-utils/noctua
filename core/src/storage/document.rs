// SPDX-License-Identifier: GPL-3.0-or-later
// core/src/storage/document.rs
//
// Manages physical document files on the filesystem.

use super::StorageError;
use crate::document::{DocumentInfo, Kind, Metadata};
use std::fs;
use std::path::Path;
use std::time::SystemTime;

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

/// Human-readable file size (B, KB, MB).
pub fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    if bytes as f64 >= GB {
        format!("{:.1} GB", bytes as f64 / GB)
    } else if bytes as f64 >= MB {
        format!("{:.1} MB", bytes as f64 / MB)
    } else if bytes as f64 >= KB {
        format!("{:.1} KB", bytes as f64 / KB)
    } else {
        format!("{bytes} B")
    }
}

/// Detected file format based on extension and magic bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
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

/// Detect the document format, reading only the magic bytes.
///
/// Uses magic-byte detection with an extension fallback, so this is more
/// reliable than checking the extension alone.
pub fn format(path: &Path) -> Result<Format, StorageError> {
    let first_bytes = read_first_bytes(path)?;
    Ok(detect_format(path, &first_bytes))
}

/// Whether the file is a supported document format.
pub fn supported(path: &Path) -> Result<bool, StorageError> {
    Ok(format(path)? != Format::Unknown)
}

/// Whether the file is a PDF document.
///
/// The UI uses this to decide which render path applies: PDFs must go
/// through the pdfium worker, everything else can be rendered in place.
pub fn is_pdf(path: &Path) -> Result<bool, StorageError> {
    Ok(format(path)? == Format::Pdf)
}

/// Load document metadata from a file path.
///
/// Detects the document format, extracts format-specific metadata, and
/// returns a complete `DocumentInfo`. The file content itself is not kept
/// in memory.
///
/// # Errors
/// Returns `StorageError` if the file cannot be read, is corrupted, or the
/// format is unsupported.
pub fn load(path: &Path) -> Result<DocumentInfo, StorageError> {
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

    // 5. Extract basic metadata
    let mut doc_metadata = Metadata::default();

    // Add file modification time to metadata
    if let Some(modified) = file_meta.modified {
        doc_metadata.modified_at = Some(chrono::DateTime::from(modified));
    }

    // 6. Construct DocumentInfo
    Ok(DocumentInfo {
        file_size_bytes: file_meta.size_bytes,
        number_of_pages,
        kind,
        metadata: doc_metadata,
    })
}
