// SPDX-License-Identifier: GPL-3.0-or-later
// src/application/queries/get_page.rs
//
// Get page query: retrieve page information from multi-page documents.
// Reserved for future CQRS pattern - currently using direct DocumentManager methods.

#![allow(dead_code)]

use cosmic::widget::image::Handle as ImageHandle;

use crate::application::document_manager::DocumentManager;
use crate::domain::document::core::document::{DocResult, Renderable};

/// Page information result.
#[derive(Debug, Clone)]
pub struct PageInfo {
    /// Page index (0-based).
    pub index: usize,
    /// Page width in pixels.
    pub width: u32,
    /// Page height in pixels.
    pub height: u32,
    /// Page thumbnail (if available).
    pub thumbnail: Option<ImageHandle>,
}

/// Get page query.
pub struct GetPageQuery {
    /// Page index to retrieve.
    page_index: usize,
}

impl GetPageQuery {
    /// Create a new get page query.
    #[must_use]
    pub fn new(page_index: usize) -> Self {
        Self { page_index }
    }

    /// Execute the query and return page information.
    pub fn execute(&self, manager: &DocumentManager) -> DocResult<Option<PageInfo>> {
        let document = match manager.current_document() {
            Some(doc) => doc,
            None => return Ok(None),
        };

        // Check if page index is valid
        if self.page_index >= document.page_count() {
            return Err(anyhow::anyhow!(
                "Invalid page index {} (document has {} pages)",
                self.page_index,
                document.page_count()
            ));
        }

        // For now, return basic info
        // TODO: Implement proper page dimension retrieval
        let info = document.info();

        Ok(Some(PageInfo {
            index: self.page_index,
            width: info.width,
            height: info.height,
            thumbnail: None, // TODO: Retrieve thumbnail from cache
        }))
    }

    /// Get the page index being queried.
    #[must_use]
    pub fn page_index(&self) -> usize {
        self.page_index
    }
}
