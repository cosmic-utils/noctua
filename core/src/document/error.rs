// SPDX-License-Identifier: GPL-3.0-or-later
// src/document/error.rs
//
// Document-specific error types for document loading, processing, and manipulation.

use crate::storage::StorageError;

/// Errors that can occur during document operations (loading, saving, transformation).
///
/// This type encompasses both storage‑layer errors (I/O, serialization) and
/// document‑specific semantic errors (unsupported format, invalid page numbers).
#[derive(Debug, Clone, thiserror::Error)]
pub enum DocumentError {
    /// A storage‑layer error (file I/O, serialization, thumbnails).
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    /// The requested file could not be found at the given path.
    #[error("file not found: {0}")]
    NotFound(String),

    /// The document format is not supported by any available backend.
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    /// The file appears to be corrupted or malformed.
    #[error("corrupted document: {0}")]
    Corrupted(String),

    /// The requested page number is out of the valid range (1..=number_of_pages).
    #[error("page {0} out of range")]
    PageOutOfRange(u32),

    /// The requested operation is invalid for the current document state.
    #[error("invalid operation: {0}")]
    InvalidOperation(String),

    /// A generic error for unexpected conditions, with a descriptive message.
    #[error("document error: {0}")]
    Other(String),
}
