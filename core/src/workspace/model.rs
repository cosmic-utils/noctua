// SPDX-License-Identifier: GPL-3.0-or-later
// src/workspace/model.rs
//
// Data model for workspace.
pub use crate::document::model::DocumentEntry;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollectionType {
    /// Transient image viewer mode (e.g., browsing a local directory)
    Browser,
    /// Persistent document and annotation mode (user-created groups)
    Session,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: Uuid,
    pub collection_type: CollectionType,
    pub name: String,
    pub documents: IndexMap<Uuid, DocumentEntry>,
    #[serde(default)]
    pub active_document_id: Option<Uuid>,
    pub selected_document_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Workspace {
    pub name: String,
    pub workspace_path: Option<PathBuf>,
    pub collections: IndexMap<Uuid, Collection>,
    #[serde(default)]
    pub active_collection_id: Option<Uuid>,
    pub selected_collection_id: Option<Uuid>,
}

impl Collection {
    pub fn active_document(&self) -> Option<&DocumentEntry> {
        self.active_document_id.and_then(|id| self.documents.get(&id))
    }

    pub fn selected_document(&self) -> Option<&DocumentEntry> {
        self.selected_document_id.and_then(|id| self.documents.get(&id))
    }
}

impl Workspace {
    pub fn active_collection(&self) -> Option<&Collection> {
        self.active_collection_id.and_then(|id| self.collections.get(&id))
    }

    pub fn selected_collection(&self) -> Option<&Collection> {
        self.selected_collection_id.and_then(|id| self.collections.get(&id))
    }
}
