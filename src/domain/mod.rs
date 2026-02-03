// SPDX-License-Identifier: GPL-3.0-or-later
// src/domain/mod.rs
//
// Domain layer: business logic, document abstractions, and viewport management.

pub mod document;
pub mod errors;
pub mod viewport;

// Re-export core document types
#[allow(unused_imports)]
pub use document::core::content::DocumentContent;
#[allow(unused_imports)]
pub use document::core::metadata::DocumentMeta;

// Note: Low-level pixel operations (apply_rotation, apply_flip, crop_image)
// are internal helpers used only by document type implementations.
// Use high-level operations above for all application and UI code.
