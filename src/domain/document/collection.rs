// SPDX-License-Identifier: GPL-3.0-or-later
// src/domain/document/collection.rs
//
// Document collection for managing multiple documents.

use std::path::PathBuf;

use crate::domain::document::core::content::DocumentContent;

/// A collection of documents with navigation support.
///
/// This abstraction is useful for:
/// - Browsing through folders of images
/// - Batch operations on multiple documents
/// - Comparison views (showing multiple documents side-by-side)
#[derive(Debug)]
pub struct DocumentCollection {
    /// List of document paths in the collection.
    paths: Vec<PathBuf>,
    /// Currently active document index.
    current_index: Option<usize>,
    /// Currently loaded document (lazy-loaded).
    current_document: Option<DocumentContent>,
}

impl DocumentCollection {
    /// Create an empty collection.
    #[must_use]
    pub fn new() -> Self {
        Self {
            paths: Vec::new(),
            current_index: None,
            current_document: None,
        }
    }

    /// Create a collection from a list of paths.
    #[must_use]
    pub fn from_paths(paths: Vec<PathBuf>) -> Self {
        let current_index = if paths.is_empty() { None } else { Some(0) };

        Self {
            paths,
            current_index,
            current_document: None,
        }
    }

    /// Get the number of documents in the collection.
    #[must_use]
    pub fn len(&self) -> usize {
        self.paths.len()
    }

    /// Check if the collection is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    /// Get the current document index (0-based).
    #[must_use]
    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }

    /// Get the current document path.
    #[must_use]
    pub fn current_path(&self) -> Option<&PathBuf> {
        self.current_index.and_then(|idx| self.paths.get(idx))
    }

    /// Get all paths in the collection.
    #[must_use]
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    /// Get a reference to the currently loaded document.
    #[must_use]
    pub fn current_document(&self) -> Option<&DocumentContent> {
        self.current_document.as_ref()
    }

    /// Get a mutable reference to the currently loaded document.
    #[must_use]
    pub fn current_document_mut(&mut self) -> Option<&mut DocumentContent> {
        self.current_document.as_mut()
    }

    /// Set the currently loaded document.
    pub fn set_current_document(&mut self, document: DocumentContent) {
        self.current_document = Some(document);
    }

    /// Clear the currently loaded document.
    pub fn clear_current_document(&mut self) {
        self.current_document = None;
    }

    /// Navigate to the next document in the collection.
    ///
    /// Returns the new index if successful, None if already at the end.
    pub fn next(&mut self) -> Option<usize> {
        if let Some(current) = self.current_index
            && current + 1 < self.paths.len() {
                self.current_index = Some(current + 1);
                self.current_document = None; // Clear document (needs reload)
                return self.current_index;
            }
        None
    }

    /// Navigate to the previous document in the collection.
    ///
    /// Returns the new index if successful, None if already at the start.
    pub fn previous(&mut self) -> Option<usize> {
        if let Some(current) = self.current_index
            && current > 0 {
                self.current_index = Some(current - 1);
                self.current_document = None; // Clear document (needs reload)
                return self.current_index;
            }
        None
    }

    /// Navigate to a specific index.
    ///
    /// Returns true if the index is valid and navigation succeeded.
    pub fn goto(&mut self, index: usize) -> bool {
        if index < self.paths.len() {
            self.current_index = Some(index);
            self.current_document = None; // Clear document (needs reload)
            true
        } else {
            false
        }
    }

    /// Add a document path to the collection.
    pub fn add_path(&mut self, path: PathBuf) {
        self.paths.push(path);
        if self.current_index.is_none() {
            self.current_index = Some(0);
        }
    }

    /// Remove a document path at the given index.
    ///
    /// Returns the removed path if successful.
    pub fn remove_at(&mut self, index: usize) -> Option<PathBuf> {
        if index < self.paths.len() {
            let removed = self.paths.remove(index);

            // Update current index if needed
            if let Some(current) = self.current_index {
                if current == index {
                    // Removed current document
                    self.current_document = None;
                    if self.paths.is_empty() {
                        self.current_index = None;
                    } else if current >= self.paths.len() {
                        self.current_index = Some(self.paths.len() - 1);
                    }
                } else if current > index {
                    // Adjust index after removal
                    self.current_index = Some(current - 1);
                }
            }

            Some(removed)
        } else {
            None
        }
    }

    /// Clear the entire collection.
    pub fn clear(&mut self) {
        self.paths.clear();
        self.current_index = None;
        self.current_document = None;
    }

    /// Check if there is a next document available.
    #[must_use]
    pub fn has_next(&self) -> bool {
        if let Some(current) = self.current_index {
            current + 1 < self.paths.len()
        } else {
            false
        }
    }

    /// Check if there is a previous document available.
    #[must_use]
    pub fn has_previous(&self) -> bool {
        if let Some(current) = self.current_index {
            current > 0
        } else {
            false
        }
    }

    /// Get the path at a specific index.
    #[must_use]
    pub fn path_at(&self, index: usize) -> Option<&PathBuf> {
        self.paths.get(index)
    }
}

impl Default for DocumentCollection {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_collection() {
        let collection = DocumentCollection::new();
        assert!(collection.is_empty());
        assert_eq!(collection.len(), 0);
        assert_eq!(collection.current_index(), None);
    }

    #[test]
    fn test_navigation() {
        let paths = vec![
            PathBuf::from("a.png"),
            PathBuf::from("b.png"),
            PathBuf::from("c.png"),
        ];
        let mut collection = DocumentCollection::from_paths(paths);

        assert_eq!(collection.current_index(), Some(0));
        assert_eq!(collection.next(), Some(1));
        assert_eq!(collection.next(), Some(2));
        assert_eq!(collection.next(), None); // At end
        assert_eq!(collection.previous(), Some(1));
        assert_eq!(collection.previous(), Some(0));
        assert_eq!(collection.previous(), None); // At start
    }

    #[test]
    fn test_goto() {
        let paths = vec![
            PathBuf::from("a.png"),
            PathBuf::from("b.png"),
            PathBuf::from("c.png"),
        ];
        let mut collection = DocumentCollection::from_paths(paths);

        assert!(collection.goto(2));
        assert_eq!(collection.current_index(), Some(2));
        assert!(!collection.goto(10)); // Invalid index
    }

    #[test]
    fn test_remove() {
        let paths = vec![
            PathBuf::from("a.png"),
            PathBuf::from("b.png"),
            PathBuf::from("c.png"),
        ];
        let mut collection = DocumentCollection::from_paths(paths);

        collection.goto(1);
        assert_eq!(collection.remove_at(1), Some(PathBuf::from("b.png")));
        assert_eq!(collection.len(), 2);
        assert_eq!(collection.current_index(), Some(1)); // Now points to c.png
    }
}
