// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/document/file.rs
//
// Opening files, folder scanning, and navigation helpers.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::anyhow;

use super::portable::PortableDocument;
use super::raster::RasterDocument;
use super::vector::VectorDocument;
use super::{DocumentContent, DocumentKind};

use crate::app::model::{AppModel, ViewMode};

/// Open a document from a file path and dispatch to the correct type.
///
/// Raster formats are delegated to the `image` crate, which decides
/// based on enabled codecs (e.g. default-formats).
pub fn open_document(path: PathBuf) -> anyhow::Result<DocumentContent> {
    let kind = DocumentKind::from_path(&path)
        .ok_or_else(|| anyhow!("Unsupported document type: {:?}", path))?;

    let content = match kind {
        DocumentKind::Raster => {
            let raster = RasterDocument::open(path)?;
            DocumentContent::Raster(raster)
        }
        DocumentKind::Vector => {
            let vector = VectorDocument::open(path)?;
            DocumentContent::Vector(vector)
        }
        DocumentKind::Portable => {
            let portable = PortableDocument::open(path)?;
            DocumentContent::Portable(portable)
        }
    };

    Ok(content)
}

/// Open the initial path passed on the command line.
///
/// If `path` is a directory, this will collect supported documents inside it,
/// open the first one, and initialize navigation state. If it is a file, the
/// file is opened directly and the surrounding folder is scanned.
pub fn open_initial_path(model: &mut AppModel, path: PathBuf) {
    if path.is_dir() {
        open_from_directory(model, &path);
    } else {
        open_single_file(model, &path);
    }
}

/// Open the first supported document from the given directory and
/// populate folder navigation state.
pub fn open_from_directory(model: &mut AppModel, dir: &Path) {
    let entries = collect_supported_files(dir);

    if entries.is_empty() {
        model.set_error(format!(
            "No supported documents found in directory: {}",
            dir.display()
        ));
        return;
    }

    let first = entries[0].clone();
    model.folder_entries = entries;
    model.current_index = Some(0);

    load_document_into_model(model, &first);
}

/// Open a single file, update current path and refresh folder entries.
pub fn open_single_file(model: &mut AppModel, path: &Path) {
    load_document_into_model(model, path);

    // Refresh folder listing based on parent directory.
    if model.document.is_some() {
        if let Some(parent) = path.parent() {
            refresh_folder_entries(model, parent, path);
        }
    }
}

/// Load a document into the model, resetting view state.
fn load_document_into_model(model: &mut AppModel, path: &Path) {
    match open_document(path.to_path_buf()) {
        Ok(doc) => {
            model.document = Some(doc);
            model.current_path = Some(path.to_path_buf());
            model.clear_error();

            // Reset view state for new document.
            model.reset_pan();
            model.view_mode = ViewMode::Fit;
        }
        Err(err) => {
            model.document = None;
            model.current_path = None;
            model.set_error(err.to_string());
        }
    }
}

/// Refresh the `folder_entries` list and current index based on the
/// given folder and currently active file.
pub fn refresh_folder_entries(model: &mut AppModel, folder: &Path, current: &Path) {
    let entries = collect_supported_files(folder);

    // Determine current index.
    let current_index = entries.iter().position(|p| p == current);

    model.folder_entries = entries;
    model.current_index = current_index;
}

/// Collect all supported document files from a directory, sorted alphabetically.
fn collect_supported_files(dir: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = Vec::new();

    if let Ok(read_dir) = fs::read_dir(dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();

            // Only keep regular files that are recognized as supported documents.
            if path.is_file() && DocumentKind::from_path(&path).is_some() {
                entries.push(path);
            }
        }
    }

    entries.sort();
    entries
}

/// Navigate to the next document in the folder.
pub fn navigate_next(model: &mut AppModel) {
    if model.folder_entries.is_empty() {
        return;
    }

    let new_index = match model.current_index {
        Some(idx) => {
            if idx + 1 < model.folder_entries.len() {
                idx + 1
            } else {
                0 // Wrap around to first.
            }
        }
        None => 0,
    };

    if let Some(path) = model.folder_entries.get(new_index).cloned() {
        model.current_index = Some(new_index);
        load_document_into_model(model, &path);
    }
}

/// Navigate to the previous document in the folder.
pub fn navigate_prev(model: &mut AppModel) {
    if model.folder_entries.is_empty() {
        return;
    }

    let new_index = match model.current_index {
        Some(idx) => {
            if idx > 0 {
                idx - 1
            } else {
                model.folder_entries.len() - 1 // Wrap around to last.
            }
        }
        None => model.folder_entries.len().saturating_sub(1),
    };

    if let Some(path) = model.folder_entries.get(new_index).cloned() {
        model.current_index = Some(new_index);
        load_document_into_model(model, &path);
    }
}
