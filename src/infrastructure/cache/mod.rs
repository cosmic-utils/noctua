// SPDX-License-Identifier: GPL-3.0-or-later
// src/infrastructure/cache/mod.rs
//
// Cache infrastructure: thumbnail and document caching.

pub mod thumbnail_cache;

// Re-export ThumbnailCache
pub use thumbnail_cache::ThumbnailCache;
