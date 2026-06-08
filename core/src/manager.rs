// SPDX-License-Identifier: GPL-3.0-or-later
// core/src/manager.rs
//
// Central orchestrator — the single public entry point for all UIs.
// Owns workspace state, document state, and loaded content.
// May call storage for I/O and render for display output.

use std::path::Path;
use uuid::Uuid;

use crate::document::manager::DocumentManager;
use crate::workspace::Workspace;
use crate::workspace::WorkspaceError;
use crate::workspace::manager::WorkspaceManager;
use crate::workspace::{Command, CommandResult};

pub struct Manager {
    workspace_mgr: WorkspaceManager,
    document_mgr: DocumentManager,
}

impl Manager {
    /// Create a new manager with an empty workspace.
    pub fn new(name: String) -> Self {
        Self {
            workspace_mgr: WorkspaceManager::new(name),
            document_mgr: DocumentManager::new(),
        }
    }

    /// Create a manager from a previously stored workspace.
    pub fn from_workspace(workspace: Workspace) -> Self {
        Self {
            workspace_mgr: WorkspaceManager::from_workspace(workspace),
            document_mgr: DocumentManager::new(),
        }
    }

    /// Read-only access to the full workspace model.
    pub fn workspace(&self) -> &Workspace {
        self.workspace_mgr.workspace()
    }

    pub fn path(&self) -> Option<&Path> {
        self.workspace_mgr.path()
    }

    /// Central command execution. Routes to sub-managers, handles I/O.
    pub fn execute(&mut self, command: Command) -> CommandResult {
        //
        // Commands that need document content synchronization first.
        //
        let needs_sync = matches!(
            command,
            Command::DocumentCommand(_)
                | Command::CollectionActivate { .. }
                | Command::CollectionRemove { .. }
                | Command::DocumentActivate { .. }
                | Command::DocumentRemove { .. }
                | Command::DocumentMoveToCollection { .. }
                | Command::DocumentNavigateFirst
                | Command::DocumentNavigatePrevious
                | Command::DocumentNavigateNext
                | Command::DocumentNavigateLast
                | Command::CollectionNavigateFirst
                | Command::CollectionNavigatePrevious
                | Command::CollectionNavigateNext
                | Command::CollectionNavigateLast
        );

        if needs_sync {
            self.sync_document_out();
        }

        let result = match command {
            // ── Document-level — delegated to DocumentManager ──
            Command::DocumentCommand(doc_cmd) => {
                self.ensure_document_loaded();
                let doc_result = self.document_mgr.execute(doc_cmd);

                // Write state back
                self.sync_document_in();

                match doc_result {
                    crate::document::CommandResult::Ok
                    | crate::document::CommandResult::PageSelected(_)
                    | crate::document::CommandResult::PropertiesChanged => CommandResult::Ok,
                    crate::document::CommandResult::Error(e) => {
                        CommandResult::Error(WorkspaceError::Other(format!("{e:?}")))
                    }
                }
            }

            // ── Everything else — delegated to WorkspaceManager ──
            _ => self.workspace_mgr.execute(command),
        };

        // Reload document content after navigation / activation
        if needs_sync && !matches!(result, CommandResult::Error(_)) {
            self.sync_document_out();
        }

        result
    }

    /// Load document content on demand if the DocumentManager is empty.
    fn ensure_document_loaded(&mut self) {
        if self.document_mgr.document().is_none()
            && let Some(entry) = self
                .workspace_mgr
                .workspace()
                .active_collection()
                .and_then(|c| c.active_document().cloned())
        {
            self.document_mgr.load(entry);
        }
    }

    /// Write DocumentManager state back to the active DocumentEntry in the workspace.
    fn sync_document_in(&mut self) {
        let Some(doc) = self.document_mgr.unload() else {
            return;
        };
        let doc_id = doc.id;
        if let Some(collection) = self
            .workspace_mgr
            .workspace_mut()
            .active_collection_id
            .and_then(|id| self.workspace_mgr.workspace_mut().collections.get_mut(&id))
        {
            if collection.documents.contains_key(&doc_id) {
                collection.documents.insert(doc_id, doc);
            }
        } else {
            // If the collection is gone, put the document back.
            self.document_mgr.load(doc);
        }
    }

    /// Reload the active DocumentEntry into the DocumentManager.
    /// Called after navigation or activation commands.
    fn sync_document_out(&mut self) {
        // Discard current state
        self.document_mgr.unload();

        // Load fresh entry from workspace
        if let Some(entry) = self
            .workspace_mgr
            .workspace()
            .active_collection()
            .and_then(|c| c.active_document().cloned())
        {
            self.document_mgr.load(entry);
        }
    }

    // ── Read-Only for UI ──

    pub fn name(&self) -> &str {
        self.workspace_mgr.name()
    }

    pub fn active_collection_id(&self) -> Option<Uuid> {
        self.workspace_mgr.active_collection_id()
    }

    pub fn selected_collection_id(&self) -> Option<Uuid> {
        self.workspace_mgr.selected_collection_id()
    }

    pub fn active_collection_name(&self) -> Option<&str> {
        self.workspace_mgr.active_collection_name()
    }

    pub fn active_document_id(&self) -> Option<Uuid> {
        self.workspace_mgr.active_document_id()
    }

    pub fn active_document_path(&self) -> Option<&Path> {
        self.workspace_mgr.active_document_path()
    }

    pub fn selected_document_id(&self) -> Option<Uuid> {
        self.workspace_mgr.selected_document_id()
    }

    pub fn selected_document_path(&self) -> Option<&Path> {
        self.workspace_mgr.selected_document_path()
    }

    /// Document-level state accessors from the internal DocumentManager.
    pub fn current_page(&self) -> u32 {
        self.document_mgr.current_page()
    }

    pub fn number_of_pages(&self) -> u32 {
        self.document_mgr.number_of_pages()
    }

    pub fn rotation(&self) -> u16 {
        self.document_mgr.rotation()
    }

    pub fn flip_horizontal(&self) -> bool {
        self.document_mgr.flip_horizontal()
    }

    pub fn flip_vertical(&self) -> bool {
        self.document_mgr.flip_vertical()
    }

    pub fn collection_list_view(&self) -> impl Iterator<Item = (Uuid, &str, bool)> + '_ {
        self.workspace_mgr.collection_list_view()
    }

    pub fn document_list_view(
        &self,
        collection_id: Uuid,
    ) -> impl Iterator<Item = (Uuid, std::borrow::Cow<'_, str>, &Path, bool)> + '_ {
        self.workspace_mgr.document_list_view(collection_id)
    }
}
