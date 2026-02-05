// SPDX-License-Identifier: GPL-3.0-or-later
// src/application/commands/transform_document.rs
//
// Transform document command: rotate, flip, and other transformations.

use crate::application::document_manager::DocumentManager;
use crate::domain::document::core::document::{DocResult, Rotation};
use crate::domain::document::operations::transform;

/// Transformation operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TransformOperation {
    /// Rotate clockwise by 90 degrees.
    RotateCw,
    /// Rotate counter-clockwise by 90 degrees.
    RotateCcw,
    /// Flip horizontally.
    FlipHorizontal,
    /// Flip vertically.
    FlipVertical,
    /// Rotate to a specific angle.
    RotateTo(Rotation),
}

/// Transform document command.
pub struct TransformDocumentCommand {
    operation: TransformOperation,
}

impl TransformDocumentCommand {
    /// Create a new transform document command.
    #[must_use]
    pub fn new(operation: TransformOperation) -> Self {
        Self { operation }
    }

    /// Execute the transform command.
    ///
    /// Uses high-level transform operations that work across all document types
    /// (Raster, Vector, Portable).
    pub fn execute(&self, manager: &mut DocumentManager) -> DocResult<()> {
        let document = manager
            .current_document_mut()
            .ok_or_else(|| anyhow::anyhow!("No document loaded"))?;

        match self.operation {
            TransformOperation::RotateCw => {
                transform::rotate_document_cw(document)?;
            }
            TransformOperation::RotateCcw => {
                transform::rotate_document_ccw(document)?;
            }
            TransformOperation::FlipHorizontal => {
                transform::flip_document_horizontal(document)?;
            }
            TransformOperation::FlipVertical => {
                transform::flip_document_vertical(document)?;
            }
            TransformOperation::RotateTo(rotation) => {
                transform::rotate_document_to(document, rotation)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_command_creation() {
        let cmd = TransformDocumentCommand::new(TransformOperation::RotateCw);
        assert_eq!(cmd.operation, TransformOperation::RotateCw);

        let cmd = TransformDocumentCommand::new(TransformOperation::FlipHorizontal);
        assert_eq!(cmd.operation, TransformOperation::FlipHorizontal);
    }
}
