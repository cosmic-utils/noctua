// SPDX-License-Identifier: GPL-3.0-or-later
// src/workspace/command.rs
//
// Command structure for workspace.

use super::error::WorkspaceError;
use super::model::{CollectionType, DocumentEntry};
use crate::document;
use uuid::Uuid;

/// A command structure encompassing all Workspace operations.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Command {
    // Workspace Commands
    WorkspaceRename {
        name: String,
    },

    // Collection Commands
    /// adding a collection with a default name based on its type
    /// (e.g. "Session N" or "Browser N")
    CollectionAdd {
        collection_type: CollectionType,
    },
    CollectionSelect {
        collection_id: Uuid,
    },
    CollectionActivate {
        collection_id: Uuid,
    },
    CollectionRemove {
        collection_id: Uuid,
    },
    /// duplicating a collection within the same workspace and
    /// setting the new name to the original name + "(copy)",
    /// while allowing to change its type (e.g. Browser -> Session)
    CollectionDuplicate {
        collection_id: Uuid,
        target_type: Option<CollectionType>,
    },
    CollectionRename {
        collection_id: Uuid,
        name: String,
    },
    CollectionMerge {
        source_id: Uuid,
        target_id: Uuid,
    },
    CollectionMoveUp {
        collection_id: Uuid,
    },
    CollectionMoveDown {
        collection_id: Uuid,
    },

    CollectionNavigateFirst,
    CollectionNavigatePrevious,
    CollectionNavigateNext,
    CollectionNavigateLast,

    // Document Commands
    /// adding a pre-loaded Document to a collection
    DocumentAdd {
        collection_id: Uuid,
        entry: Box<DocumentEntry>,
    },
    DocumentAddMultiple {
        collection_id: Uuid,
        entries: Vec<DocumentEntry>,
    },
    DocumentSelect {
        document_id: Uuid,
    },
    DocumentActivate {
        document_id: Uuid,
    },
    DocumentRemove {
        document_id: Uuid,
    },
    DocumentDuplicate {
        document_id: Uuid,
        collection_id: Uuid,
    },
    DocumentRename {
        document_id: Uuid,
        name: String,
    },

    // Delegated commands — forwarded to DocumentManager
    DocumentCommand(document::Command),

    DocumentMoveUp {
        document_id: Uuid,
    },
    DocumentMoveDown {
        document_id: Uuid,
    },

    DocumentMoveToIndex {
        document_id: Uuid,
        collection_index: usize,
    },
    DocumentMoveToCollection {
        document_id: Uuid,
        collection_id: Uuid,
    },

    DocumentNavigateFirst,
    DocumentNavigatePrevious,
    DocumentNavigateNext,
    DocumentNavigateLast,
}

/// Result of a command execution.
#[derive(Debug, Clone)]
pub enum CommandResult {
    Ok,
    DocumentAdded(Uuid),
    CollectionAdded(Uuid),
    Error(WorkspaceError),
}

impl From<Result<(), WorkspaceError>> for CommandResult {
    fn from(res: Result<(), WorkspaceError>) -> Self {
        match res {
            Ok(_) => CommandResult::Ok,
            Err(e) => CommandResult::Error(e),
        }
    }
}
