// SPDX-License-Identifier: GPL-3.0-or-later
// core/src/pdfium_ops/error.rs
//
// Errors for PDF operations.

#[derive(Debug, Clone, thiserror::Error)]
pub enum PdfOpsError {
    #[error("no document is open")]
    NoDocumentOpen,

    #[error("page {0} out of range")]
    PageOutOfRange(u32),

    #[error("pdfium error: {0}")]
    Pdfium(String),

    #[error("unsupported source format for binding: {0}")]
    UnsupportedSource(String),
}
