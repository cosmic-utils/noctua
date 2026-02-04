// SPDX-License-Identifier: GPL-3.0-or-later
// src/domain/mod.rs
//
// Domain layer: business logic, document abstractions, and viewport management.

pub mod document;

// Re-export core document types
#[allow(unused_imports)]
pub use document::core::content::DocumentContent;
#[allow(unused_imports)]
pub use document::core::metadata::DocumentMeta;

// Note: Viewport and error handling were removed to reduce code bloat.
// - Viewport: Was 865 lines of unused code (planned feature)
// - Domain Errors: Not integrated, anyhow::Result is sufficient
//
// Low-level pixel operations (apply_rotation, apply_flip, crop_image)
// are internal helpers used only by document type implementations.
// Use high-level operations for all application and UI code.
