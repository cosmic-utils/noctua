// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/document/mod.rs
//
// Document module root: common enums and type erasure for document kinds.

pub mod file;
pub mod meta;
pub mod portable;
pub mod raster;
pub mod transform;
pub mod utils;
pub mod vector;

use cosmic::iced::widget::image as iced_image;
use cosmic::iced_renderer::graphics::image::image_rs::ImageFormat as CosmicImageFormat;
use std::fmt;
use std::path::Path;

use self::portable::PortableDocument;
use self::raster::RasterDocument;
use self::vector::VectorDocument;

/// High-level classification of documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentKind {
    Raster,
    Vector,
    Portable,
}

/// Unified document type used by the application.
pub enum DocumentContent {
    Raster(RasterDocument),
    Vector(VectorDocument),
    Portable(PortableDocument),
}

impl fmt::Debug for DocumentContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DocumentContent::Raster(_) => f.write_str("DocumentContent::Raster(..)"),
            DocumentContent::Vector(_) => f.write_str("DocumentContent::Vector(..)"),
            DocumentContent::Portable(_) => f.write_str("DocumentContent::Portable(..)"),
        }
    }
}

impl DocumentKind {
    /// Derive document kind from file extension.
    ///
    /// - `pdf`  => Portable
    /// - `svg`  => Vector
    /// - supported image extensions (via libcosmic/image_rs ImageFormat)
    ///   => Raster
    ///
    /// Returns `None` if the extension is not recognized as any supported kind.
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext_os = path.extension()?;
        let ext_str = ext_os.to_str()?;
        let ext_lower = ext_str.to_ascii_lowercase();

        match ext_lower.as_str() {
            "pdf" => return Some(DocumentKind::Portable),
            "svg" => return Some(DocumentKind::Vector),
            _ => {}
        }

        // Ask libcosmic/image_rs if this extension corresponds to a known image
        // format. If yes, we treat it as a raster document.
        if CosmicImageFormat::from_extension(ext_os).is_some() {
            return Some(DocumentKind::Raster);
        }

        None
    }
}

impl DocumentContent {
    /// Returns a cloneable image handle for rendering.
    ///
    /// This is intentionally linear: every concrete document type
    /// owns some kind of `iced_image::Handle`, and the canvas can
    /// just call `doc.handle()` without additional branching.
    pub fn handle(&self) -> iced_image::Handle {
        match self {
            DocumentContent::Raster(doc) => doc.handle.clone(),
            DocumentContent::Vector(doc) => doc.handle.clone(),
            DocumentContent::Portable(doc) => doc.handle.clone(),
        }
    }

    /// Returns the native dimensions (width, height) of the document in pixels.
    ///
    /// For raster images this is the actual pixel size.
    /// For vector/portable documents this is the rasterized size at default DPI.
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            DocumentContent::Raster(doc) => doc.dimensions(),
            DocumentContent::Vector(doc) => doc.dimensions(),
            DocumentContent::Portable(doc) => doc.dimensions(),
        }
    }
    /// Extract metadata from the document.
    /// This may involve file I/O for EXIF data, so call lazily.
    pub fn extract_meta(&self) -> meta::DocumentMeta {
        match self {
            DocumentContent::Raster(doc) => doc.extract_meta(),
            DocumentContent::Vector(doc) => doc.extract_meta(),
            DocumentContent::Portable(doc) => doc.extract_meta(),
        }
    }
}

/// Set an image file as desktop wallpaper.
///
/// This function attempts multiple methods in order:
/// 1. COSMIC Desktop (direct config file modification)
/// 2. wallpaper crate (KDE, XFCE, Windows, macOS)
/// 3. gsettings (GNOME)
/// 4. feh (tiling window managers)
///
/// The operation is performed asynchronously and logs success/failure.
pub fn set_as_wallpaper(path: &Path) {
    // Canonicalize to absolute path
    let abs_path = match path.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to canonicalize path {}: {}", path.display(), e);
            return;
        }
    };

    // Convert to string
    let path_str = match abs_path.to_str() {
        Some(s) => s.to_string(),
        None => {
            log::error!("Invalid UTF-8 in path: {}", abs_path.display());
            return;
        }
    };

    // Delegate to utils with concrete string type
    utils::set_as_wallpaper(&path_str);
}
