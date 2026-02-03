// SPDX-License-Identifier: GPL-3.0-or-later
// src/domain/document/operations/mod.rs
//
// Document operations: transformations, rendering, and export.

pub mod export;
pub mod render;
pub mod transform;

// Note: Low-level pixel operations (apply_rotation, apply_flip, crop_image)
// are internal helpers (pub(crate)) used only by document type implementations.
// Use high-level operations above for application and UI code.
