// SPDX-License-Identifier: GPL-3.0-or-later
// src/workspace/error.rs
//
// Workspace-specific error types for business logic operations.

use uuid::Uuid;

/// Errors that can occur during workspace, collection, or entry operations.
///
/// This type encompasses both pure business‑logic errors (e.g., “collection not found”)
/// and underlying I/O errors that have been lifted via `#[from]`.
#[derive(Debug, Clone, thiserror::Error)]
pub enum WorkspaceError {
    /// The requested collection does not exist in the workspace.
    #[error("collection not found: {0}")]
    CollectionNotFound(Uuid),

    /// The requested document does not exist in any collection.
    #[error("document not found: {0}")]
    DocumentNotFound(Uuid),

    /// The workspace is in an invalid state for the requested operation.
    #[error("invalid workspace state: {0}")]
    InvalidState(String),

    /// A collection could not be merged because source and target are the same.
    #[error("cannot merge a collection with itself")]
    MergeSelf,

    /// An operation tried to move an element beyond the valid bounds.
    #[error("move out of bounds")]
    MoveOutOfBounds,

    /// A generic error for unexpected conditions, with a descriptive message.
    #[error("workspace error: {0}")]
    Other(String),
}
