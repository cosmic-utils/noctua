// SPDX-License-Identifier: GPL-3.0-or-later
// src/application/queries/get_document.rs
//
// Get document query: retrieve current document information.
// Reserved for future CQRS pattern - currently using direct DocumentManager methods.

#![allow(dead_code)]

use crate::application::document_manager::DocumentManager;
use crate::domain::document::core::metadata::DocumentMeta;

/// Get document query result.
#[derive(Debug)]
pub struct DocumentInfo {
    /// Document content reference.
    pub has_document: bool,
    /// Document metadata.
    pub metadata: Option<DocumentMeta>,
    /// Current page (for multi-page documents).
    pub current_page: usize,
    /// Total pages (for multi-page documents).
    pub total_pages: usize,
}

/// Get document query.
pub struct GetDocumentQuery;

impl GetDocumentQuery {
    /// Create a new get document query.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Execute the query and return document information.
    #[must_use]
    pub fn execute(&self, manager: &DocumentManager) -> DocumentInfo {
        let has_document = manager.current_document().is_some();
        let metadata = manager.current_metadata().cloned();

        let (current_page, total_pages) = if let Some(doc) = manager.current_document() {
            (doc.current_page(), doc.page_count())
        } else {
            (0, 0)
        };

        DocumentInfo {
            has_document,
            metadata,
            current_page,
            total_pages,
        }
    }
}

impl Default for GetDocumentQuery {
    fn default() -> Self {
        Self::new()
    }
}
