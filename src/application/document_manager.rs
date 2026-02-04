// SPDX-License-Identifier: GPL-3.0-or-later
// src/application/document_manager.rs
//
// Document manager: orchestrates document lifecycle and navigation.

use std::path::{Path, PathBuf};

use crate::domain::document::collection::DocumentCollection;
use crate::domain::document::core::content::DocumentContent;
use crate::domain::document::core::document::DocResult;
use crate::domain::document::core::metadata::DocumentMeta;
use crate::infrastructure::filesystem::file_ops;
use crate::infrastructure::loaders::DocumentLoaderFactory;

/// Central document manager.
///
/// Orchestrates document loading, metadata extraction, and folder navigation.
/// Uses DocumentCollection (Domain Layer) for navigation logic.
pub struct DocumentManager {
    /// Document collection for navigation (Domain Layer abstraction).
    collection: DocumentCollection,
    /// Current document metadata.
    current_metadata: Option<DocumentMeta>,
    /// Document loader factory.
    loader: DocumentLoaderFactory,
}

impl DocumentManager {
    /// Create a new document manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            collection: DocumentCollection::new(),
            current_metadata: None,
            loader: DocumentLoaderFactory::new(),
        }
    }

    /// Open a document from a file path or directory.
    ///
    /// If a directory is provided, opens the first supported file found.
    /// Also scans the parent folder for navigation.
    pub fn open_document(&mut self, path: &Path) -> DocResult<()> {
        // Determine the actual file to open
        let file_path = if path.is_dir() {
            // Scan directory and find first supported file
            let paths = file_ops::collect_supported_files(path);
            self.collection = DocumentCollection::from_paths(paths);

            self.collection
                .current_path()
                .ok_or_else(|| anyhow::anyhow!("No supported files found in directory"))?
                .clone()
        } else {
            path.to_path_buf()
        };

        // Load the document
        let document = self.loader.load(&file_path)?;

        // Extract metadata
        let metadata = self.extract_metadata(&file_path, &document);

        // Scan folder for navigation if not already done
        if !path.is_dir() {
            if let Some(parent) = file_path.parent() {
                let paths = file_ops::collect_supported_files(parent);
                self.collection = DocumentCollection::from_paths(paths);
                // Find and set current document index
                if let Some(idx) = self.collection.paths().iter().position(|p| p == &file_path) {
                    self.collection.goto(idx);
                }
            }
        }

        // Generate thumbnails for multi-page documents (PDF)
        let mut document = document;
        if document.is_multi_page() {
            log::info!("Generating thumbnails for multi-page document...");
            if let Err(e) = document.generate_thumbnails() {
                log::warn!("Failed to generate thumbnails: {e}");
            }
        }

        // Store document in collection
        self.collection.set_current_document(document);
        self.current_metadata = Some(metadata);

        Ok(())
    }

    /// Get the current document.
    #[must_use]
    pub fn current_document(&self) -> Option<&DocumentContent> {
        self.collection.current_document()
    }

    /// Get a mutable reference to the current document.
    #[must_use]
    pub fn current_document_mut(&mut self) -> Option<&mut DocumentContent> {
        self.collection.current_document_mut()
    }

    /// Get thumbnail handle for a specific page (read-only access).
    /// Returns None if the thumbnail hasn't been generated yet.
    #[must_use]
    pub fn get_thumbnail_handle(&self, page: usize) -> Option<cosmic::widget::image::Handle> {
        self.collection
            .current_document()?
            .get_thumbnail_handle(page)
    }

    /// Get the current document path.
    #[must_use]
    pub fn current_path(&self) -> Option<&Path> {
        self.collection.current_path().map(|p| p.as_path())
    }

    /// Get the current document metadata.
    #[must_use]
    pub fn current_metadata(&self) -> Option<&DocumentMeta> {
        self.current_metadata.as_ref()
    }

    /// Get all folder entries for navigation.
    #[must_use]
    pub fn folder_entries(&self) -> &[PathBuf] {
        self.collection.paths()
    }

    /// Get current index in folder.
    #[must_use]
    pub fn current_index(&self) -> Option<usize> {
        self.collection.current_index()
    }

    /// Navigate to the next document in the folder.
    ///
    /// Wraps around to the first document when at the end.
    pub fn next_document(&mut self) -> Option<PathBuf> {
        // Use DocumentCollection navigation
        if self.collection.has_next() {
            self.collection.next();
        } else if !self.collection.is_empty() {
            // Wrap around to first
            self.collection.goto(0);
        } else {
            return None;
        }

        let next_path = self.collection.current_path()?.clone();
        if self.open_document(&next_path).is_ok() {
            Some(next_path)
        } else {
            None
        }
    }

    /// Navigate to the previous document in the folder.
    ///
    /// Wraps around to the last document when at the beginning.
    pub fn previous_document(&mut self) -> Option<PathBuf> {
        // Use DocumentCollection navigation
        if self.collection.has_previous() {
            self.collection.previous();
        } else if !self.collection.is_empty() {
            // Wrap around to last
            let last_idx = self.collection.len() - 1;
            self.collection.goto(last_idx);
        } else {
            return None;
        }

        let prev_path = self.collection.current_path()?.clone();
        if self.open_document(&prev_path).is_ok() {
            Some(prev_path)
        } else {
            None
        }
    }

    /// Close the current document.
    #[allow(dead_code)]
    pub fn close_document(&mut self) {
        self.collection.clear_current_document();
        self.current_metadata = None;
    }

    /// Extract metadata from a document.
    fn extract_metadata(&self, path: &Path, document: &DocumentContent) -> DocumentMeta {
        // Use the document's own extract_meta() method
        // This properly delegates to the type-specific implementation
        // (RasterDocument, VectorDocument, or PortableDocument)
        document.extract_meta(path)
    }

    /// Check if there is a next document available.
    #[must_use]
    #[allow(dead_code)]
    pub fn has_next(&self) -> bool {
        self.collection.has_next()
    }

    /// Check if there is a previous document available.
    #[must_use]
    #[allow(dead_code)]
    pub fn has_previous(&self) -> bool {
        self.collection.has_previous()
    }
}

impl Default for DocumentManager {
    fn default() -> Self {
        Self::new()
    }
}
