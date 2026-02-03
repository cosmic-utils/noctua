// SPDX-License-Identifier: GPL-3.0-or-later
// src/domain/document/mod.rs
//
// Document domain: core abstractions, types, and operations.

pub mod collection;
pub mod core;
pub mod operations;
pub mod types;

// Re-export core abstractions (only used ones)
#[allow(unused_imports)]
pub use core::{DocumentContent, DocumentMeta};

// Note: Low-level pixel operations (apply_rotation, apply_flip, crop_image)
// are internal helpers used only by document type implementations.
// Use high-level operations above for all application and UI code.
