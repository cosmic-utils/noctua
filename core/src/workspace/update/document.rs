// SPDX-License-Identifier: GPL-3.0-or-later
// src/workspace/update/document.rs
//
// Document-level state update within collections.

use crate::workspace::WorkspaceError;
use crate::workspace::model::{Collection, DocumentEntry, Workspace};
use uuid::Uuid;

fn find_collection_with_document_mut(
    workspace: &mut Workspace,
    document_id: Uuid,
) -> Option<&mut Collection> {
    workspace
        .collections
        .values_mut()
        .find(|c| c.documents.contains_key(&document_id))
}

fn active_collection_mut(workspace: &mut Workspace) -> Result<&mut Collection, WorkspaceError> {
    let collection_id = workspace
        .active_collection_id
        .ok_or_else(|| WorkspaceError::InvalidState("no active collection".to_string()))?;
    workspace
        .collections
        .get_mut(&collection_id)
        .ok_or(WorkspaceError::CollectionNotFound(collection_id))
}

pub fn add(
    workspace: &mut Workspace,
    collection_id: Uuid,
    entry: DocumentEntry,
) -> Result<Uuid, WorkspaceError> {
    let collection = workspace
        .collections
        .get_mut(&collection_id)
        .ok_or(WorkspaceError::CollectionNotFound(collection_id))?;

    let id = entry.id;
    collection.documents.insert(id, entry);

    if collection.active_document_id.is_none() {
        collection.active_document_id = Some(id);
    }
    if collection.selected_document_id.is_none() {
        collection.selected_document_id = Some(id);
    }

    Ok(id)
}

pub fn add_multiple(
    workspace: &mut Workspace,
    collection_id: Uuid,
    entries: Vec<DocumentEntry>,
) -> Result<(), WorkspaceError> {
    let collection = workspace
        .collections
        .get_mut(&collection_id)
        .ok_or(WorkspaceError::CollectionNotFound(collection_id))?;

    for entry in entries {
        let id = entry.id;
        collection.documents.insert(id, entry);

        if collection.active_document_id.is_none() {
            collection.active_document_id = Some(id);
        }
    }

    Ok(())
}

pub fn select(workspace: &mut Workspace, document_id: Uuid) -> Result<(), WorkspaceError> {
    let collection_id = {
        let collection = find_collection_with_document_mut(workspace, document_id)
            .ok_or(WorkspaceError::DocumentNotFound(document_id))?;
        collection.selected_document_id = Some(document_id);
        collection.id
    };
    workspace.selected_collection_id = Some(collection_id);
    Ok(())
}

pub fn activate(workspace: &mut Workspace, document_id: Uuid) -> Result<(), WorkspaceError> {
    let collection_id = {
        let collection = find_collection_with_document_mut(workspace, document_id)
            .ok_or(WorkspaceError::DocumentNotFound(document_id))?;
        collection.active_document_id = Some(document_id);
        collection.id
    };
    workspace.active_collection_id = Some(collection_id);
    Ok(())
}

pub fn remove(workspace: &mut Workspace, document_id: Uuid) -> Result<(), WorkspaceError> {
    let collection = find_collection_with_document_mut(workspace, document_id)
        .ok_or(WorkspaceError::DocumentNotFound(document_id))?;

    collection.documents.shift_remove(&document_id);
    if collection.active_document_id == Some(document_id) {
        collection.active_document_id = collection.documents.first().map(|(id, _)| *id);
    }
    if collection.selected_document_id == Some(document_id) {
        collection.selected_document_id = collection.active_document_id;
    }
    Ok(())
}

pub fn duplicate(
    workspace: &mut Workspace,
    document_id: Uuid,
    collection_id: Uuid,
) -> Result<Uuid, WorkspaceError> {
    // Check target collection first
    if !workspace.collections.contains_key(&collection_id) {
        return Err(WorkspaceError::CollectionNotFound(collection_id));
    }

    // Find source document
    let source_entry = workspace
        .collections
        .values()
        .find_map(|c| c.documents.get(&document_id).cloned());

    let mut new_entry = source_entry.ok_or(WorkspaceError::DocumentNotFound(document_id))?;
    let new_id = Uuid::new_v4();
    new_entry.id = new_id;

    let target_collection = workspace.collections.get_mut(&collection_id).unwrap();

    // If duplicating in the same collection, insert after source
    if let Some(index) = target_collection.documents.get_index_of(&document_id) {
        target_collection
            .documents
            .shift_insert(index + 1, new_id, new_entry);
    } else {
        target_collection.documents.insert(new_id, new_entry);
    }

    Ok(new_id)
}

pub fn rename(
    workspace: &mut Workspace,
    document_id: Uuid,
    name: String,
) -> Result<(), WorkspaceError> {
    let collection = find_collection_with_document_mut(workspace, document_id)
        .ok_or(WorkspaceError::DocumentNotFound(document_id))?;

    let doc = collection
        .documents
        .get_mut(&document_id)
        .ok_or(WorkspaceError::DocumentNotFound(document_id))?;

    doc.display_name = if name.is_empty() { None } else { Some(name) };
    Ok(())
}

pub fn move_up(workspace: &mut Workspace, document_id: Uuid) -> Result<(), WorkspaceError> {
    let collection = find_collection_with_document_mut(workspace, document_id)
        .ok_or(WorkspaceError::DocumentNotFound(document_id))?;

    let index = collection.documents.get_index_of(&document_id).unwrap();
    if index > 0 {
        collection.documents.swap_indices(index, index - 1);
        Ok(())
    } else {
        Err(WorkspaceError::MoveOutOfBounds)
    }
}

pub fn move_down(workspace: &mut Workspace, document_id: Uuid) -> Result<(), WorkspaceError> {
    let collection = find_collection_with_document_mut(workspace, document_id)
        .ok_or(WorkspaceError::DocumentNotFound(document_id))?;

    let index = collection.documents.get_index_of(&document_id).unwrap();
    if index + 1 < collection.documents.len() {
        collection.documents.swap_indices(index, index + 1);
        Ok(())
    } else {
        Err(WorkspaceError::MoveOutOfBounds)
    }
}

pub fn move_to_index(
    workspace: &mut Workspace,
    document_id: Uuid,
    target_index: usize,
) -> Result<(), WorkspaceError> {
    let collection = find_collection_with_document_mut(workspace, document_id)
        .ok_or(WorkspaceError::DocumentNotFound(document_id))?;

    if target_index < collection.documents.len() {
        let entry = collection.documents.shift_remove(&document_id).unwrap();
        collection
            .documents
            .shift_insert(target_index, document_id, entry);
        Ok(())
    } else {
        Err(WorkspaceError::MoveOutOfBounds)
    }
}

pub fn move_to_collection(
    workspace: &mut Workspace,
    document_id: Uuid,
    target_collection_id: Uuid,
) -> Result<(), WorkspaceError> {
    // 1. Find and remove from source
    let entry = {
        let collection = find_collection_with_document_mut(workspace, document_id)
            .ok_or(WorkspaceError::DocumentNotFound(document_id))?;

        let e = collection.documents.shift_remove(&document_id).unwrap();
        if collection.active_document_id == Some(document_id) {
            collection.active_document_id = collection.documents.first().map(|(id, _)| *id);
        }
        e
    };

    // 2. Insert into target
    let target = workspace
        .collections
        .get_mut(&target_collection_id)
        .ok_or(WorkspaceError::CollectionNotFound(target_collection_id))?;

    target.documents.insert(document_id, entry);
    if target.active_document_id.is_none() {
        target.active_document_id = Some(document_id);
    }

    Ok(())
}

pub fn navigate_first(workspace: &mut Workspace) -> Result<(), WorkspaceError> {
    let collection = active_collection_mut(workspace)?;

    let (id, _) = collection
        .documents
        .first()
        .ok_or_else(|| WorkspaceError::InvalidState("no documents in collection".to_string()))?;
    collection.active_document_id = Some(*id);
    Ok(())
}

pub fn navigate_previous(workspace: &mut Workspace) -> Result<(), WorkspaceError> {
    let collection = active_collection_mut(workspace)?;

    let current_id = collection
        .active_document_id
        .ok_or_else(|| WorkspaceError::InvalidState("no active document".to_string()))?;
    let index = collection.documents.get_index_of(&current_id).unwrap();

    if index > 0 {
        let (id, _) = collection.documents.get_index(index - 1).unwrap();
        collection.active_document_id = Some(*id);
        Ok(())
    } else {
        Err(WorkspaceError::MoveOutOfBounds)
    }
}

pub fn navigate_next(workspace: &mut Workspace) -> Result<(), WorkspaceError> {
    let collection = active_collection_mut(workspace)?;

    let current_id = collection
        .active_document_id
        .ok_or_else(|| WorkspaceError::InvalidState("no active document".to_string()))?;
    let index = collection.documents.get_index_of(&current_id).unwrap();

    if index + 1 < collection.documents.len() {
        let (id, _) = collection.documents.get_index(index + 1).unwrap();
        collection.active_document_id = Some(*id);
        Ok(())
    } else {
        Err(WorkspaceError::MoveOutOfBounds)
    }
}

pub fn navigate_last(workspace: &mut Workspace) -> Result<(), WorkspaceError> {
    let collection = active_collection_mut(workspace)?;

    let (id, _) = collection
        .documents
        .last()
        .ok_or_else(|| WorkspaceError::InvalidState("no documents in collection".to_string()))?;
    collection.active_document_id = Some(*id);
    Ok(())
}
