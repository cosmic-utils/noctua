// SPDX-License-Identifier: GPL-3.0-or-later
// src/document/update/page.rs
//
// Document page navigation logic.

use crate::document::error::DocumentError;
use crate::document::model::DocumentEntry;

pub fn select(document: &mut DocumentEntry, page: u32) -> Result<u32, DocumentError> {
    let num_pages = document.total_pages();
    if page == 0 || (num_pages > 0 && page > num_pages) {
        return Err(DocumentError::PageOutOfRange(page));
    }
    document.current_page = page;
    Ok(page)
}

pub fn next(document: &mut DocumentEntry) -> Result<u32, DocumentError> {
    let num_pages = document.total_pages();
    if num_pages > 0 && document.current_page < num_pages {
        document.current_page += 1;
        Ok(document.current_page)
    } else if num_pages == 0 {
        Err(DocumentError::InvalidOperation(
            "Document info not loaded".to_string(),
        ))
    } else {
        Err(DocumentError::PageOutOfRange(document.current_page + 1))
    }
}

pub fn previous(document: &mut DocumentEntry) -> Result<u32, DocumentError> {
    if document.current_page > 1 {
        document.current_page -= 1;
        Ok(document.current_page)
    } else {
        Err(DocumentError::PageOutOfRange(0))
    }
}

pub fn first(document: &mut DocumentEntry) -> Result<u32, DocumentError> {
    document.current_page = 1;
    Ok(1)
}

pub fn last(document: &mut DocumentEntry) -> Result<u32, DocumentError> {
    let num_pages = document.total_pages();
    if num_pages > 0 {
        document.current_page = num_pages;
        Ok(document.current_page)
    } else {
        Err(DocumentError::InvalidOperation(
            "Document info not loaded".to_string(),
        ))
    }
}
