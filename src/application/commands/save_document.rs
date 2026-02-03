// SPDX-License-Identifier: GPL-3.0-or-later
// src/application/commands/save_document.rs
//
// Save document command: export document to a file.
// Reserved for future implementation - not yet used.

#![allow(dead_code)]

use std::path::Path;

use crate::application::document_manager::DocumentManager;
use crate::domain::document::core::document::DocResult;
use crate::domain::document::operations::export::ExportFormat;

/// Save document command.
pub struct SaveDocumentCommand {
    /// Target format for export.
    format: Option<ExportFormat>,
}

impl SaveDocumentCommand {
    /// Create a new save document command with automatic format detection.
    #[must_use]
    pub fn new() -> Self {
        Self { format: None }
    }

    /// Create a save document command with a specific format.
    #[must_use]
    pub fn with_format(format: ExportFormat) -> Self {
        Self {
            format: Some(format),
        }
    }

    /// Execute the save document command.
    pub fn execute(&self, manager: &DocumentManager, path: &Path) -> DocResult<()> {
        let _document = manager
            .current_document()
            .ok_or_else(|| anyhow::anyhow!("No document loaded"))?;

        // Detect format from path or use specified format
        let format = self
            .format
            .or_else(|| ExportFormat::from_path(path))
            .ok_or_else(|| anyhow::anyhow!("Could not determine export format"))?;

        // TODO: Implement actual save logic
        // This would involve:
        // 1. Getting the rendered image from the document
        // 2. Applying any necessary transformations
        // 3. Exporting to the target format

        log::info!("Save to {} as {:?}", path.display(), format);

        Err(anyhow::anyhow!("Save operation not yet implemented"))
    }
}

impl Default for SaveDocumentCommand {
    fn default() -> Self {
        Self::new()
    }
}
