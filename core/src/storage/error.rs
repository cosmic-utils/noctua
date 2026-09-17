// SPDX-License-Identifier: GPL-3.0-or-later
// core/src/storage/error.rs
//
// Central error type for all storage-related operations (I/O, serialization, thumbnails).

#[derive(Debug, Clone, thiserror::Error)]
/// Central error type for all storage-related operations (I/O, serialization, thumbnails).
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(String),

    #[error("RON deserialization error: {0}")]
    Ron(String),

    #[error("RON serialization error: {0}")]
    RonSer(String),

    #[error("Thumbnail error: {0}")]
    Thumb(String),

    #[error("Document error: {0}")]
    Document(String),

    #[error("Data directory unavailable: {0}")]
    DataDir(String),
}

impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        StorageError::Io(err.to_string())
    }
}

impl From<ron::error::SpannedError> for StorageError {
    fn from(err: ron::error::SpannedError) -> Self {
        StorageError::Ron(err.to_string())
    }
}

impl From<ron::Error> for StorageError {
    fn from(err: ron::Error) -> Self {
        StorageError::RonSer(err.to_string())
    }
}
