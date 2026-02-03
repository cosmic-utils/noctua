// SPDX-License-Identifier: GPL-3.0-or-later
// src/domain/errors.rs
//
// Domain-specific error types.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// Domain-specific errors.
#[derive(Debug)]
pub enum DomainError {
    /// Document loading failed.
    DocumentLoad {
        path: PathBuf,
        reason: String,
    },
    /// Unsupported document format.
    UnsupportedFormat {
        path: PathBuf,
        extension: Option<String>,
    },
    /// Document rendering failed.
    RenderFailed {
        reason: String,
    },
    /// Page navigation error (invalid page index).
    InvalidPage {
        requested: usize,
        total: usize,
    },
    /// Transformation operation failed.
    TransformFailed {
        operation: String,
        reason: String,
    },
    /// Export operation failed.
    ExportFailed {
        path: PathBuf,
        reason: String,
    },
    /// I/O error.
    Io {
        path: Option<PathBuf>,
        error: io::Error,
    },
    /// Invalid dimensions.
    InvalidDimensions {
        width: u32,
        height: u32,
    },
    /// Viewport error.
    Viewport {
        reason: String,
    },
    /// Generic error with message.
    Other {
        message: String,
    },
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DocumentLoad { path, reason } => {
                write!(f, "Failed to load document '{}': {}", path.display(), reason)
            }
            Self::UnsupportedFormat { path, extension } => {
                if let Some(ext) = extension {
                    write!(
                        f,
                        "Unsupported format '.{}' for file '{}'",
                        ext,
                        path.display()
                    )
                } else {
                    write!(f, "Unsupported format for file '{}'", path.display())
                }
            }
            Self::RenderFailed { reason } => {
                write!(f, "Rendering failed: {reason}")
            }
            Self::InvalidPage { requested, total } => {
                write!(
                    f,
                    "Invalid page index {requested} (document has {total} pages)"
                )
            }
            Self::TransformFailed { operation, reason } => {
                write!(f, "Transformation '{operation}' failed: {reason}")
            }
            Self::ExportFailed { path, reason } => {
                write!(f, "Export to '{}' failed: {}", path.display(), reason)
            }
            Self::Io { path, error } => {
                if let Some(p) = path {
                    write!(f, "I/O error for '{}': {}", p.display(), error)
                } else {
                    write!(f, "I/O error: {error}")
                }
            }
            Self::InvalidDimensions { width, height } => {
                write!(f, "Invalid dimensions: {width}x{height}")
            }
            Self::Viewport { reason } => {
                write!(f, "Viewport error: {reason}")
            }
            Self::Other { message } => {
                write!(f, "{message}")
            }
        }
    }
}

impl std::error::Error for DomainError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { error, .. } => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for DomainError {
    fn from(error: io::Error) -> Self {
        Self::Io { path: None, error }
    }
}

impl From<String> for DomainError {
    fn from(message: String) -> Self {
        Self::Other { message }
    }
}

impl From<&str> for DomainError {
    fn from(message: &str) -> Self {
        Self::Other {
            message: message.to_string(),
        }
    }
}
