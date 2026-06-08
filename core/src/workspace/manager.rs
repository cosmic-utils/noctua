// SPDX-License-Identifier: GPL-3.0-or-later
// core/src/workspace/manager.rs
//
// Pure workspace manager — handles collection and document membership state.
// No I/O, no rendering, no document-level commands.

use indexmap::IndexMap;
use std::path::Path;
use uuid::Uuid;

use super::command::{Command, CommandResult};
use super::model::Workspace;
use super::update;

pub struct WorkspaceManager {
    workspace: Workspace,
}

impl WorkspaceManager {
    /// Create a new workspace manager.
    pub fn new(name: String) -> Self {
        Self {
            workspace: Workspace {
                name,
                workspace_path: None,
                collections: IndexMap::new(),
                active_collection_id: None,
                selected_collection_id: None,
            },
        }
    }

    pub fn from_workspace(workspace: Workspace) -> Self {
        Self { workspace }
    }

    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    pub fn workspace_mut(&mut self) -> &mut Workspace {
        &mut self.workspace
    }

    pub fn path(&self) -> Option<&Path> {
        self.workspace.workspace_path.as_deref()
    }

    /// Execute a membership-level command. No document-level commands, no I/O.
    pub fn execute(&mut self, command: Command) -> CommandResult {
        match command {
            // ── Workspace ──
            Command::WorkspaceRename { name } => {
                self.workspace.name = name;
                CommandResult::Ok
            }

            // ── Collections ──
            Command::CollectionAdd { collection_type } => {
                update::collection::add(&mut self.workspace, collection_type)
                    .map(CommandResult::CollectionAdded)
                    .unwrap_or_else(CommandResult::Error)
            }
            Command::CollectionSelect { collection_id } => {
                update::collection::select(&mut self.workspace, collection_id).into()
            }
            Command::CollectionActivate { collection_id } => {
                update::collection::activate(&mut self.workspace, collection_id).into()
            }
            Command::CollectionRemove { collection_id } => {
                update::collection::remove(&mut self.workspace, collection_id).into()
            }
            Command::CollectionDuplicate {
                collection_id,
                target_type,
            } => update::collection::duplicate(&mut self.workspace, collection_id, target_type)
                .map(CommandResult::CollectionAdded)
                .unwrap_or_else(CommandResult::Error),
            Command::CollectionRename {
                collection_id,
                name,
            } => update::collection::rename(&mut self.workspace, collection_id, name).into(),
            Command::CollectionMerge {
                source_id,
                target_id,
            } => update::collection::merge(&mut self.workspace, source_id, target_id).into(),
            Command::CollectionMoveUp { collection_id } => {
                update::collection::move_up(&mut self.workspace, collection_id).into()
            }
            Command::CollectionMoveDown { collection_id } => {
                update::collection::move_down(&mut self.workspace, collection_id).into()
            }
            Command::CollectionNavigateFirst => {
                update::collection::navigate_first(&mut self.workspace).into()
            }
            Command::CollectionNavigatePrevious => {
                update::collection::navigate_previous(&mut self.workspace).into()
            }
            Command::CollectionNavigateNext => {
                update::collection::navigate_next(&mut self.workspace).into()
            }
            Command::CollectionNavigateLast => {
                update::collection::navigate_last(&mut self.workspace).into()
            }

            // ── Document Membership ──
            Command::DocumentAdd {
                collection_id,
                entry,
            } => update::document::add(&mut self.workspace, collection_id, *entry)
                .map(CommandResult::DocumentAdded)
                .unwrap_or_else(CommandResult::Error),
            Command::DocumentAddMultiple {
                collection_id,
                entries,
            } => update::document::add_multiple(&mut self.workspace, collection_id, entries).into(),
            Command::DocumentSelect { document_id } => {
                update::document::select(&mut self.workspace, document_id).into()
            }
            Command::DocumentActivate { document_id } => {
                update::document::activate(&mut self.workspace, document_id).into()
            }
            Command::DocumentRemove { document_id } => {
                update::document::remove(&mut self.workspace, document_id).into()
            }
            Command::DocumentDuplicate {
                document_id,
                collection_id,
            } => update::document::duplicate(&mut self.workspace, document_id, collection_id)
                .map(CommandResult::DocumentAdded)
                .unwrap_or_else(CommandResult::Error),
            Command::DocumentRename { document_id, name } => {
                update::document::rename(&mut self.workspace, document_id, name).into()
            }
            Command::DocumentMoveUp { document_id } => {
                update::document::move_up(&mut self.workspace, document_id).into()
            }
            Command::DocumentMoveDown { document_id } => {
                update::document::move_down(&mut self.workspace, document_id).into()
            }
            Command::DocumentMoveToIndex {
                document_id,
                collection_index,
            } => {
                update::document::move_to_index(&mut self.workspace, document_id, collection_index)
                    .into()
            }
            Command::DocumentMoveToCollection {
                document_id,
                collection_id,
            } => update::document::move_to_collection(
                &mut self.workspace,
                document_id,
                collection_id,
            )
            .into(),
            Command::DocumentNavigateFirst => {
                update::document::navigate_first(&mut self.workspace).into()
            }
            Command::DocumentNavigatePrevious => {
                update::document::navigate_previous(&mut self.workspace).into()
            }
            Command::DocumentNavigateNext => {
                update::document::navigate_next(&mut self.workspace).into()
            }
            Command::DocumentNavigateLast => {
                update::document::navigate_last(&mut self.workspace).into()
            }

            // These are handled by the orchestrator, not here.
            Command::DocumentCommand(_) => {
                CommandResult::Error(super::WorkspaceError::InvalidState(
                    "DocumentCommand must be handled by Manager, not WorkspaceManager".into(),
                ))
            }
        }
    }

    // ── Read-Only for UI ──

    pub fn name(&self) -> &str {
        &self.workspace.name
    }

    pub fn active_collection_id(&self) -> Option<Uuid> {
        self.workspace.active_collection_id
    }

    pub fn selected_collection_id(&self) -> Option<Uuid> {
        self.workspace.selected_collection_id
    }

    pub fn active_collection_name(&self) -> Option<&str> {
        self.workspace.active_collection().map(|c| c.name.as_str())
    }

    pub fn active_document_id(&self) -> Option<Uuid> {
        self.workspace.active_collection()?.active_document_id
    }

    pub fn active_document_path(&self) -> Option<&Path> {
        self.workspace
            .active_collection()?
            .active_document()
            .map(|d| d.path.as_path())
    }

    pub fn active_document(&self) -> Option<&super::DocumentEntry> {
        self.workspace.active_collection()?.active_document()
    }

    pub fn selected_document_id(&self) -> Option<Uuid> {
        self.workspace.selected_collection()?.selected_document_id
    }

    pub fn selected_document_path(&self) -> Option<&Path> {
        self.workspace
            .selected_collection()?
            .selected_document()
            .map(|d| d.path.as_path())
    }

    /// Provides the UI with a list of all collections: (ID, Name, Is_Active)
    pub fn collection_list_view(&self) -> impl Iterator<Item = (Uuid, &str, bool)> + '_ {
        let active_id = self.workspace.active_collection_id;
        self.workspace
            .collections
            .values()
            .map(move |c| (c.id, c.name.as_str(), Some(c.id) == active_id))
    }

    /// Provides the UI with a list of documents for a given collection: (ID, Name, Path, Is_Active)
    pub fn document_list_view(
        &self,
        collection_id: Uuid,
    ) -> impl Iterator<Item = (Uuid, std::borrow::Cow<'_, str>, &Path, bool)> + '_ {
        let collection = self.workspace.collections.get(&collection_id);
        let active_doc_id = collection.and_then(|c| c.active_document_id);

        collection
            .into_iter()
            .flat_map(|c| c.documents.values())
            .map(move |d| {
                (
                    d.id,
                    d.display_name
                        .as_deref()
                        .map(std::borrow::Cow::Borrowed)
                        .unwrap_or_else(|| {
                            d.path
                                .file_name()
                                .map(|s: &std::ffi::OsStr| s.to_string_lossy())
                                .unwrap_or(std::borrow::Cow::Borrowed("Unknown"))
                        }),
                    d.path.as_path(),
                    Some(d.id) == active_doc_id,
                )
            })
    }
}
