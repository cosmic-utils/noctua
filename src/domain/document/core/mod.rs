// SPDX-License-Identifier: GPL-3.0-or-later
// src/domain/document/core/mod.rs
//
// Core document abstractions: traits, types, and metadata.

pub mod content;
pub mod document;
pub mod metadata;
pub mod page;

// Re-export commonly used types
pub use content::DocumentContent;
pub use metadata::DocumentMeta;
