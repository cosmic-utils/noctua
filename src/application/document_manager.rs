// SPDX-License-Identifier: GPL-3.0-or-later
// src/application/document_manager.rs
//
// Document manager: orchestrates document lifecycle and navigation.

use std::path::{Path, PathBuf};

use crate::domain::document::core::content::DocumentContent;
use crate::domain::document::core::document::{DocResult, Renderable};
use crate::domain::document::core::metadata::DocumentMeta;
use crate::infrastructure::filesystem::file_ops;
use crate::infrastructure::loaders::DocumentLoaderFactory;

/// Central document manager.
///
/// Orchestrates document loading, metadata extraction, and folder navigation.
pub struct DocumentManager {
    /// Current document (if any).
    current_document: Option<DocumentContent>,
    /// Current document path.
    current_path: Option<PathBuf>,
    /// Current document metadata.
    current_metadata: Option<DocumentMeta>,
    /// Folder entries for navigation.
    folder_entries: Vec<PathBuf>,
    /// Current index in folder entries.
    current_index: Option<usize>,
    /// Document loader factory.
    loader: DocumentLoaderFactory,
}

impl DocumentManager {
    /// Create a new document manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            current_document: None,
            current_path: None,
            current_metadata: None,
            folder_entries: Vec::new(),
            current_index: None,
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
            self.scan_folder(path);

            self.folder_entries
                .first()
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
                self.scan_folder(parent);
            }
        }

        // Find current document index
        self.current_index = self.folder_entries.iter().position(|p| p == &file_path);

        // Generate thumbnails for multi-page documents (PDF)
        let mut document = document;
        if document.is_multi_page() {
            log::info!("Generating thumbnails for multi-page document...");
            if let Err(e) = document.generate_thumbnails() {
                log::warn!("Failed to generate thumbnails: {e}");
            }
        }

        self.current_document = Some(document);
        self.current_path = Some(file_path);
        self.current_metadata = Some(metadata);

        Ok(())
    }

    /// Get the current document.
    #[must_use]
    pub fn current_document(&self) -> Option<&DocumentContent> {
        self.current_document.as_ref()
    }

    /// Get a mutable reference to the current document.
    #[must_use]
    pub fn current_document_mut(&mut self) -> Option<&mut DocumentContent> {
        self.current_document.as_mut()
    }

    /// Get thumbnail handle for a specific page (read-only access).
    /// Returns None if the thumbnail hasn't been generated yet.
    #[must_use]
    pub fn get_thumbnail_handle(&self, page: usize) -> Option<cosmic::widget::image::Handle> {
        self.current_document.as_ref()?.get_thumbnail_handle(page)
    }

    /// Get the current document path.
    #[must_use]
    pub fn current_path(&self) -> Option<&Path> {
        self.current_path.as_deref()
    }

    /// Get the current document metadata.
    #[must_use]
    pub fn current_metadata(&self) -> Option<&DocumentMeta> {
        self.current_metadata.as_ref()
    }

    /// Get folder entries for navigation.
    #[must_use]
    pub fn folder_entries(&self) -> &[PathBuf] {
        &self.folder_entries
    }

    /// Get current index in folder.
    #[must_use]
    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }

    /// Navigate to the next document in the folder.
    ///
    /// Wraps around to the first document when at the end.
    pub fn next_document(&mut self) -> Option<PathBuf> {
        if self.folder_entries.is_empty() {
            return None;
        }

        let new_index = match self.current_index {
            Some(idx) => {
                if idx + 1 < self.folder_entries.len() {
                    idx + 1
                } else {
                    0 // Wrap around to first
                }
            }
            None => 0,
        };

        let next_path = self.folder_entries.get(new_index)?.clone();
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
        if self.folder_entries.is_empty() {
            return None;
        }

        let new_index = match self.current_index {
            Some(idx) => {
                if idx > 0 {
                    idx - 1
                } else {
                    self.folder_entries.len() - 1 // Wrap around to last
                }
            }
            None => self.folder_entries.len().saturating_sub(1),
        };

        let prev_path = self.folder_entries.get(new_index)?.clone();
        if self.open_document(&prev_path).is_ok() {
            Some(prev_path)
        } else {
            None
        }
    }

    /// Close the current document.
    #[allow(dead_code)]
    pub fn close_document(&mut self) {
        self.current_document = None;
        self.current_path = None;
        self.current_metadata = None;
    }

    /// Scan a folder for supported documents.
    fn scan_folder(&mut self, folder: &Path) {
        self.folder_entries = file_ops::collect_supported_files(folder);
    }

    /// Extract metadata from a document.
    fn extract_metadata(&self, path: &Path, document: &DocumentContent) -> DocumentMeta {
        use crate::domain::document::core::metadata::{BasicMeta, DocumentMeta, ExifMeta};

        let info = document.info();
        let (width, height) = document.dimensions();

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let file_path = path.to_string_lossy().to_string();

        let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        let format = info.format;
        let color_type = format!("{}", document.kind());

        let basic = BasicMeta {
            file_name,
            file_path,
            format,
            width,
            height,
            file_size,
            color_type,
        };

        // Extract EXIF data for raster images (JPEG, TIFF)
        let exif =
            if document.kind() == crate::domain::document::core::content::DocumentKind::Raster {
                file_ops::read_file_bytes(path).and_then(|bytes| ExifMeta::from_bytes(&bytes))
            } else {
                None
            };

        DocumentMeta { basic, exif }
    }

    /// Check if there is a next document available.
    #[must_use]
    #[allow(dead_code)]
    pub fn has_next(&self) -> bool {
        if let Some(current) = self.current_index {
            current + 1 < self.folder_entries.len()
        } else {
            false
        }
    }

    /// Check if there is a previous document available.
    #[must_use]
    #[allow(dead_code)]
    pub fn has_previous(&self) -> bool {
        if let Some(current) = self.current_index {
            current > 0
        } else {
            false
        }
    }
}

impl Default for DocumentManager {
    fn default() -> Self {
        Self::new()
    }
}
