// SPDX-License-Identifier: GPL-3.0-or-later
// src/storage/portable.rs
//
// Portable document (PDF) metadata extraction.

use std::path::Path;

use crate::document::Portable;
use crate::storage::StorageError;

/// Load PDF metadata without rendering pages.
///
/// Extracts page count, PDF version, encryption status, and other document properties.
/// Returns a `Portable` struct and the number of pages.
pub fn load_pdf_metadata(path: &Path) -> Result<(Portable, u32), StorageError> {
    #[cfg(feature = "pdfium-render")]
    {
        use pdfium_render::prelude::*;

        // The single shared pdfium instance; binding pdfium twice hangs.
        let pdfium = crate::pdfium_ops::pdfium();

        // Load the PDF document (reads only metadata, not full content)
        let document = pdfium
            .load_pdf_from_file(path, None)
            .map_err(|e| StorageError::Document(format!("Failed to load PDF: {}", e)))?;

        let page_count = document.pages().len() as u32;

        // PdfDocumentVersion is an enum; map it to a display string.
        let version: String = match document.version() {
            PdfDocumentVersion::Pdf1_0 => "1.0".into(),
            PdfDocumentVersion::Pdf1_1 => "1.1".into(),
            PdfDocumentVersion::Pdf1_2 => "1.2".into(),
            PdfDocumentVersion::Pdf1_3 => "1.3".into(),
            PdfDocumentVersion::Pdf1_4 => "1.4".into(),
            PdfDocumentVersion::Pdf1_5 => "1.5".into(),
            PdfDocumentVersion::Pdf1_6 => "1.6".into(),
            PdfDocumentVersion::Pdf1_7 => "1.7".into(),
            PdfDocumentVersion::Pdf2_0 => "2.0".into(),
            PdfDocumentVersion::Other(v) => format!("{}.{}", v / 10, v % 10),
            PdfDocumentVersion::Unset => "unknown".into(),
        };

        // PdfSecurityHandlerRevision is not publicly exported; use Debug formatting
        // to check if the document has any security ("Unprotected" means no encryption).
        let is_encrypted = document
            .permissions()
            .security_handler_revision()
            .map(|rev| !format!("{:?}", rev).contains("Unprotected"))
            .unwrap_or(false);

        // Check if any page has a text layer
        let has_text_layer = document.pages().iter().any(|page| page.text().is_ok());

        // Never executed by Noctua; detected only to warn the user.
        let has_javascript = contains_javascript(path)?;

        let portable = Portable {
            format: "PDF".to_string(),
            version,
            is_encrypted,
            has_text_layer,
            has_javascript,
        };

        Ok((portable, page_count))
    }

    #[cfg(not(feature = "pdfium-render"))]
    {
        load_pdf_metadata_basic(path)
    }
}

/// Detect JavaScript constructs in a PDF file.
///
/// PDFs carry JavaScript in action dictionaries (`/JS` entries), the
/// document name tree (`/JavaScript`) and additional actions (`/AA`).
/// Noctua never executes JavaScript; the flag only drives a warning in
/// the UI. The raw byte scan misses scripts inside compressed object
/// streams — a documented limitation, not a bug.
pub fn contains_javascript(path: &Path) -> Result<bool, StorageError> {
    use std::io::Read;

    // Longest needle bounds the overlap that may span chunk borders.
    const OVERLAP: usize = 12;
    const CHUNK: usize = 64 * 1024;
    const NEEDLES: [&[u8]; 2] = [b"/JavaScript", b"/JS"];

    let mut file = std::fs::File::open(path)?;
    let mut buffer = vec![0u8; CHUNK + OVERLAP];
    let mut filled = 0usize;

    loop {
        let read = file.read(&mut buffer[filled..])?;
        if read == 0 {
            break;
        }
        let total = filled + read;
        if NEEDLES
            .iter()
            .any(|needle| buffer[..total].windows(needle.len()).any(|w| w == *needle))
        {
            return Ok(true);
        }
        filled = total.min(OVERLAP);
        buffer.copy_within(total - filled..total, 0);
    }

    Ok(false)
}

/// Minimal PDF metadata extraction without external dependencies.
///
/// Reads the PDF header to extract version and page count via simple parsing.
/// This is a fallback when pdfium-render is not available.
pub fn load_pdf_metadata_basic(path: &Path) -> Result<(Portable, u32), StorageError> {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(path)?;
    let mut buffer = [0u8; 1024]; // Read first KB for header
    let bytes_read = file.read(&mut buffer)?;
    let content = &buffer[..bytes_read];

    // Look for PDF header
    let header = std::str::from_utf8(content).unwrap_or("");
    let version = if let Some(line) = header.lines().next() {
        if let Some(stripped) = line.strip_prefix("%PDF-") {
            stripped.trim().to_string()
        } else {
            "1.0".to_string()
        }
    } else {
        "1.0".to_string()
    };

    // Very basic page count estimation by counting "endobj" references
    // This is not accurate but gives a rough estimate
    let page_count_estimate = content
        .windows(7)
        .filter(|window| window == b"/Type /Page")
        .count() as u32;

    let page_count = if page_count_estimate > 0 {
        page_count_estimate
    } else {
        1 // Assume at least one page
    };

    let portable = Portable {
        format: "PDF".to_string(),
        version,
        is_encrypted: false,   // Cannot detect without proper parser
        has_text_layer: false, // Unknown
        has_javascript: contains_javascript(path)?,
    };

    Ok((portable, page_count))
}
