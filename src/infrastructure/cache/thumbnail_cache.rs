// SPDX-License-Identifier: GPL-3.0-or-later
// src/infrastructure/cache/thumbnail_cache.rs
//
// Disk cache for document thumbnails stored in ~/.cache/noctua/

use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use image::DynamicImage;
use sha2::{Digest, Sha256};

use cosmic::widget::image::Handle as ImageHandle;

use crate::domain::document::operations::render::create_image_handle_from_image;

/// Cache directory name under ~/.cache/ for thumbnail storage.
const CACHE_DIR: &str = "noctua";

/// File extension for cached thumbnails.
const THUMBNAIL_EXT: &str = "png";

/// Thumbnail cache manager for disk-based caching.
pub struct ThumbnailCache;

impl ThumbnailCache {
    /// Load a thumbnail from disk cache.
    /// Returns None if not cached or cache is invalid.
    pub fn load(file_path: &Path, page: usize) -> Option<ImageHandle> {
        let cache_path = Self::thumbnail_path(file_path, page)?;

        log::debug!("Cache lookup: file={}, page={}", file_path.display(), page);

        if !cache_path.exists() {
            log::debug!(
                "Thumbnail not found in cache: file={} page={}",
                file_path.display(),
                page
            );
            return None;
        }

        let img = image::open(&cache_path).ok()?;
        log::debug!(
            "Thumbnail loaded from cache: file={} page={}",
            file_path.display(),
            page
        );
        Some(create_image_handle_from_image(&img))
    }

    /// Save a thumbnail to disk cache.
    pub fn save(file_path: &Path, page: usize, image: &DynamicImage) -> Option<()> {
        let dir = Self::ensure_cache_dir()?;
        let key = Self::cache_key(file_path, page)?;
        let cache_path = dir.join(format!("{key}.{THUMBNAIL_EXT}"));

        log::debug!(
            "Saving thumbnail to cache: file={}, page={}, path={}",
            file_path.display(),
            page,
            cache_path.display()
        );

        let file = fs::File::create(&cache_path).ok()?;
        let writer = BufWriter::new(file);

        let res = image.write_to(
            &mut std::io::BufWriter::new(writer),
            image::ImageFormat::Png,
        );
        match res {
            Ok(()) => {
                log::debug!(
                    "Thumbnail cached successfully: file={} page={}",
                    file_path.display(),
                    page
                );
                Some(())
            }
            Err(e) => {
                log::warn!(
                    "Failed to cache thumbnail: file={} page={}: {}",
                    file_path.display(),
                    page,
                    e
                );
                None
            }
        }
    }

    /// Clear all cached thumbnails.
    pub fn clear_cache() -> std::io::Result<()> {
        if let Some(dir) = Self::cache_dir()
            && dir.exists()
        {
            fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }

    /// Check if a thumbnail exists in cache.
    #[allow(dead_code)]
    pub fn has(file_path: &Path, page: usize) -> bool {
        Self::thumbnail_path(file_path, page).is_some_and(|p| p.exists())
    }

    // Private helper methods

    /// Get the cache directory path (~/.cache/noctua/).
    fn cache_dir() -> Option<PathBuf> {
        dirs::cache_dir().map(|p| p.join(CACHE_DIR))
    }

    /// Ensure the cache directory exists.
    fn ensure_cache_dir() -> Option<PathBuf> {
        let dir = Self::cache_dir()?;
        fs::create_dir_all(&dir).ok()?;
        Some(dir)
    }

    /// Generate a cache key from file path, modification time, and page number.
    /// Format: sha256(path + mtime + page)
    fn cache_key(file_path: &Path, page: usize) -> Option<String> {
        let metadata = fs::metadata(file_path).ok()?;
        let mtime = metadata
            .modified()
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();

        let mut hasher = Sha256::new();
        hasher.update(file_path.to_string_lossy().as_bytes());
        hasher.update(mtime.to_le_bytes());
        hasher.update(page.to_le_bytes());

        let hash = hasher.finalize();
        Some(format!("{hash:x}"))
    }

    /// Get the full path for a cached thumbnail.
    fn thumbnail_path(file_path: &Path, page: usize) -> Option<PathBuf> {
        let dir = Self::cache_dir()?;
        let key = Self::cache_key(file_path, page)?;
        Some(dir.join(format!("{key}.{THUMBNAIL_EXT}")))
    }
}
