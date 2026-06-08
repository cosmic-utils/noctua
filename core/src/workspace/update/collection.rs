// SPDX-License-Identifier: GPL-3.0-or-later
// src/workspace/update/collection.rs
//
// Collection-level state update.

use crate::workspace::WorkspaceError;
use crate::workspace::model::{Collection, CollectionType, Workspace};
use indexmap::IndexMap;
use uuid::Uuid;

pub fn add(
    workspace: &mut Workspace,
    collection_type: CollectionType,
) -> Result<Uuid, WorkspaceError> {
    let count = workspace.collections.len() + 1;
    let name = match collection_type {
        CollectionType::Browser => format!("Browser {}", count),
        CollectionType::Session => format!("Session {}", count),
    };

    let id = Uuid::new_v4();
    let new_collection = Collection {
        id,
        name,
        collection_type,
        documents: IndexMap::new(),
        active_document_id: None,
        selected_document_id: None,
    };

    workspace.collections.insert(id, new_collection);

    // Auto-activate if it's the first collection
    if workspace.active_collection_id.is_none() {
        workspace.active_collection_id = Some(id);
    }

    Ok(id)
}

pub fn select(workspace: &mut Workspace, collection_id: Uuid) -> Result<(), WorkspaceError> {
    if workspace.collections.contains_key(&collection_id) {
        workspace.selected_collection_id = Some(collection_id);
        Ok(())
    } else {
        Err(WorkspaceError::CollectionNotFound(collection_id))
    }
}

pub fn activate(workspace: &mut Workspace, collection_id: Uuid) -> Result<(), WorkspaceError> {
    if workspace.collections.contains_key(&collection_id) {
        workspace.active_collection_id = Some(collection_id);
        Ok(())
    } else {
        Err(WorkspaceError::CollectionNotFound(collection_id))
    }
}

pub fn remove(workspace: &mut Workspace, collection_id: Uuid) -> Result<(), WorkspaceError> {
    workspace
        .collections
        .shift_remove(&collection_id)
        .ok_or(WorkspaceError::CollectionNotFound(collection_id))?;

    if workspace.active_collection_id == Some(collection_id) {
        workspace.active_collection_id = workspace.collections.first().map(|(id, _)| *id);
    }
    if workspace.selected_collection_id == Some(collection_id) {
        workspace.selected_collection_id = workspace.active_collection_id;
    }
    Ok(())
}

pub fn duplicate(
    workspace: &mut Workspace,
    collection_id: Uuid,
    target_type: Option<CollectionType>,
) -> Result<Uuid, WorkspaceError> {
    let index = workspace
        .collections
        .get_index_of(&collection_id)
        .ok_or(WorkspaceError::CollectionNotFound(collection_id))?;

    let (_, source) = workspace.collections.get_index(index).unwrap();
    let mut new_collection = source.clone();

    let new_id = Uuid::new_v4();
    new_collection.id = new_id;
    new_collection.name = format!("{} (copy)", new_collection.name);
    if let Some(t) = target_type {
        new_collection.collection_type = t;
    }

    // New UUIDs for all documents in the duplicate
    let mut new_documents = IndexMap::new();
    for doc in new_collection.documents.values() {
        let mut new_doc = doc.clone();
        new_doc.id = Uuid::new_v4();
        new_documents.insert(new_doc.id, new_doc);
    }
    new_collection.documents = new_documents;
    new_collection.active_document_id = new_collection.documents.first().map(|(id, _)| *id);
    new_collection.selected_document_id = new_collection.active_document_id;

    // Insert at next position
    workspace
        .collections
        .shift_insert(index + 1, new_id, new_collection);

    Ok(new_id)
}

pub fn rename(
    workspace: &mut Workspace,
    collection_id: Uuid,
    name: String,
) -> Result<(), WorkspaceError> {
    let collection = workspace
        .collections
        .get_mut(&collection_id)
        .ok_or(WorkspaceError::CollectionNotFound(collection_id))?;
    collection.name = name;
    Ok(())
}

pub fn move_up(workspace: &mut Workspace, collection_id: Uuid) -> Result<(), WorkspaceError> {
    let index = workspace
        .collections
        .get_index_of(&collection_id)
        .ok_or(WorkspaceError::CollectionNotFound(collection_id))?;

    if index > 0 {
        workspace.collections.swap_indices(index, index - 1);
        Ok(())
    } else {
        Err(WorkspaceError::MoveOutOfBounds)
    }
}

pub fn move_down(workspace: &mut Workspace, collection_id: Uuid) -> Result<(), WorkspaceError> {
    let index = workspace
        .collections
        .get_index_of(&collection_id)
        .ok_or(WorkspaceError::CollectionNotFound(collection_id))?;

    if index + 1 < workspace.collections.len() {
        workspace.collections.swap_indices(index, index + 1);
        Ok(())
    } else {
        Err(WorkspaceError::MoveOutOfBounds)
    }
}

pub fn navigate_first(workspace: &mut Workspace) -> Result<(), WorkspaceError> {
    let (id, _) = workspace
        .collections
        .first()
        .ok_or_else(|| WorkspaceError::InvalidState("no collections available".to_string()))?;
    workspace.active_collection_id = Some(*id);
    Ok(())
}

pub fn navigate_previous(workspace: &mut Workspace) -> Result<(), WorkspaceError> {
    let current_id = workspace
        .active_collection_id
        .ok_or_else(|| WorkspaceError::InvalidState("no active collection".to_string()))?;
    let index = workspace
        .collections
        .get_index_of(&current_id)
        .ok_or(WorkspaceError::CollectionNotFound(current_id))?;

    if index > 0 {
        let (id, _) = workspace.collections.get_index(index - 1).unwrap();
        workspace.active_collection_id = Some(*id);
        Ok(())
    } else {
        Err(WorkspaceError::MoveOutOfBounds)
    }
}

pub fn navigate_next(workspace: &mut Workspace) -> Result<(), WorkspaceError> {
    let current_id = workspace
        .active_collection_id
        .ok_or_else(|| WorkspaceError::InvalidState("no active collection".to_string()))?;
    let index = workspace
        .collections
        .get_index_of(&current_id)
        .ok_or(WorkspaceError::CollectionNotFound(current_id))?;

    if index + 1 < workspace.collections.len() {
        let (id, _) = workspace.collections.get_index(index + 1).unwrap();
        workspace.active_collection_id = Some(*id);
        Ok(())
    } else {
        Err(WorkspaceError::MoveOutOfBounds)
    }
}

pub fn navigate_last(workspace: &mut Workspace) -> Result<(), WorkspaceError> {
    let (id, _) = workspace
        .collections
        .last()
        .ok_or_else(|| WorkspaceError::InvalidState("no collections available".to_string()))?;
    workspace.active_collection_id = Some(*id);
    Ok(())
}

pub fn merge(
    workspace: &mut Workspace,
    source_id: Uuid,
    target_id: Uuid,
) -> Result<(), WorkspaceError> {
    if source_id == target_id {
        return Err(WorkspaceError::MergeSelf);
    }

    let mut source = workspace
        .collections
        .shift_remove(&source_id)
        .ok_or(WorkspaceError::CollectionNotFound(source_id))?;

    let target = workspace
        .collections
        .get_mut(&target_id)
        .ok_or(WorkspaceError::CollectionNotFound(target_id))?;

    for (id, doc) in source.documents.drain(..) {
        target.documents.insert(id, doc);
    }

    if target.active_document_id.is_none() {
        target.active_document_id = target.documents.first().map(|(id, _)| *id);
    }

    Ok(())
}
