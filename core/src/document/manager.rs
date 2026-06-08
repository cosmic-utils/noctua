// SPDX-License-Identifier: GPL-3.0-or-later
// src/document/manager.rs
//
// Document manager handling state mutations and providing read-only access.

use uuid::Uuid;

use super::command::{Command, CommandResult};
pub use super::error::DocumentError;
use super::model::DocumentEntry;
use super::update;

pub struct DocumentManager {
    // Wrapped in Option since the app starts without a loaded document
    document: Option<DocumentEntry>,
}

impl Default for DocumentManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentManager {
    /// Create a new document manager.
    pub fn new() -> Self {
        Self { document: None }
    }

    /// Load a document entry into the manager, replacing any previous state.
    pub fn load(&mut self, entry: DocumentEntry) {
        self.document = Some(entry);
    }

    /// Take the managed document out, leaving the manager empty.
    pub fn unload(&mut self) -> Option<DocumentEntry> {
        self.document.take()
    }

    /// Returns a reference to the current document if any.
    pub fn document(&self) -> Option<&DocumentEntry> {
        self.document.as_ref()
    }

    pub fn execute(&mut self, command: Command) -> CommandResult {
        let doc = match self.document.as_mut() {
            Some(d) => d,
            None => {
                return CommandResult::Error(DocumentError::InvalidOperation(
                    "No document loaded".to_string(),
                ));
            }
        };

        match command {
            Command::SelectPage { page } => update::page::select(doc, page).into(),
            Command::NavigateNext => update::page::next(doc).into(),
            Command::NavigatePrevious => update::page::previous(doc).into(),
            Command::NavigateFirst => update::page::first(doc).into(),
            Command::NavigateLast => update::page::last(doc).into(),
            Command::Rotate { degrees } => update::transform::rotate(doc, degrees).into(),
            Command::Flip {
                horizontal,
                vertical,
            } => update::transform::flip(doc, horizontal, vertical).into(),
            Command::Resize { width, height } => {
                update::transform::resize(doc, width, height).into()
            }
        }
    }

    // Read-Only state for UI

    pub fn number_of_pages(&self) -> u32 {
        self.document
            .as_ref()
            .and_then(|doc| doc.info.as_ref())
            .map(|info| info.number_of_pages)
            .unwrap_or(0)
    }

    pub fn file_size(&self) -> u64 {
        self.document
            .as_ref()
            .and_then(|doc| doc.info.as_ref())
            .map(|info| info.file_size_bytes)
            .unwrap_or(0)
    }

    pub fn id(&self) -> Uuid {
        self.document
            .as_ref()
            .map(|doc| doc.id)
            .unwrap_or_else(Uuid::nil)
    }

    pub fn current_page(&self) -> u32 {
        self.document
            .as_ref()
            .map(|doc| doc.current_page)
            .unwrap_or(0)
    }

    pub fn rotation(&self) -> u16 {
        self.document
            .as_ref()
            .map(|doc| doc.rotation_degrees)
            .unwrap_or(0)
    }

    pub fn flip_horizontal(&self) -> bool {
        self.document
            .as_ref()
            .map(|doc| doc.flip_horizontal)
            .unwrap_or(false)
    }

    pub fn flip_vertical(&self) -> bool {
        self.document
            .as_ref()
            .map(|doc| doc.flip_vertical)
            .unwrap_or(false)
    }
}
