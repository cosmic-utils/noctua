// SPDX-License-Identifier: GPL-3.0-or-later
// src/document/command.rs
//
// Command structure for documents

use crate::document::error::DocumentError;

use serde::{Deserialize, Serialize};

/// Commands issued by the UI or other orchestrators to the DocumentManager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    // View state tracking
    SelectPage {
        page: u32,
    },
    NavigateNext,
    NavigatePrevious,
    NavigateFirst,
    NavigateLast,

    // Transformation modifications (Recipe)
    Rotate {
        degrees: u16,
    },
    Flip {
        horizontal: bool,
        vertical: bool,
    },

    // Physical/Dimensional (usually applied on export or for display calculation)
    Resize {
        width: u32,
        height: u32,
    },
}

/// The outcome of executing a Command.
#[derive(Debug, Clone)]
pub enum CommandResult {
    /// The command was executed successfully.
    Ok,
    /// The command failed with an error.
    Error(DocumentError),
    /// A new page was selected.
    PageSelected(u32),
    /// Properties (dimensions, orientation) have changed.
    PropertiesChanged,
}

impl From<Result<(), DocumentError>> for CommandResult {
    fn from(res: Result<(), DocumentError>) -> Self {
        match res {
            Ok(_) => CommandResult::PropertiesChanged,
            Err(e) => CommandResult::Error(e),
        }
    }
}

impl From<Result<u32, DocumentError>> for CommandResult {
    fn from(res: Result<u32, DocumentError>) -> Self {
        match res {
            Ok(p) => CommandResult::PageSelected(p),
            Err(e) => CommandResult::Error(e),
        }
    }
}
