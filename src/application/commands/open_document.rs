// SPDX-License-Identifier: GPL-3.0-or-later
// src/application/commands/open_document.rs
//
// Open document command: load a document from a file path.
// Reserved for future CQRS pattern - currently using direct DocumentManager methods.

#![allow(dead_code)]

use std::path::Path;

use crate::application::document_manager::DocumentManager;
use crate::domain::document::core::document::DocResult;

/// Open document command.
pub struct OpenDocumentCommand;

impl OpenDocumentCommand {
    /// Create a new open document command.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Execute the open document command.
    pub fn execute(&self, manager: &mut DocumentManager, path: &Path) -> DocResult<()> {
        manager.open_document(path)
    }
}

impl Default for OpenDocumentCommand {
    fn default() -> Self {
        Self::new()
    }
}
