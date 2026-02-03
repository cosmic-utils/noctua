// SPDX-License-Identifier: GPL-3.0-or-later
// src/infrastructure/loaders/mod.rs
//
// Document loaders for various formats.

pub mod document_loader;

pub mod raster_loader;
#[cfg(feature = "vector")]
pub mod svg_loader;
#[cfg(feature = "portable")]
pub mod pdf_loader;

// Re-export main types
pub use document_loader::DocumentLoaderFactory;
