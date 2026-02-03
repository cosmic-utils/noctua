// SPDX-License-Identifier: GPL-3.0-or-later
// src/infrastructure/mod.rs
//
// Infrastructure layer: external dependencies, loaders, cache, and filesystem.

pub mod cache;
pub mod filesystem;
pub mod loaders;
pub mod system;

// Re-export loader factory
#[allow(unused_imports)]
pub use loaders::DocumentLoaderFactory;
