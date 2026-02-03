// SPDX-License-Identifier: GPL-3.0-or-later
// src/domain/document/operations/export.rs
//
// Document export operations to various formats.

use std::path::Path;

use image::DynamicImage;

use crate::domain::document::core::document::DocResult;

/// Supported export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// PNG format (lossless).
    Png,
    /// JPEG format (lossy).
    Jpeg,
    /// WebP format.
    WebP,
    /// PDF format.
    Pdf,
    /// SVG format (for vector documents).
    Svg,
}

impl ExportFormat {
    /// Get file extension for this format.
    #[must_use]
    pub fn extension(&self) -> &str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::WebP => "webp",
            Self::Pdf => "pdf",
            Self::Svg => "svg",
        }
    }

    /// Get MIME type for this format.
    #[must_use]
    pub fn mime_type(&self) -> &str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::WebP => "image/webp",
            Self::Pdf => "application/pdf",
            Self::Svg => "image/svg+xml",
        }
    }

    /// Detect format from file extension.
    #[must_use]
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_lowercase();
        match ext.as_str() {
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "webp" => Some(Self::WebP),
            "pdf" => Some(Self::Pdf),
            "svg" => Some(Self::Svg),
            _ => None,
        }
    }
}

/// Export options for image formats.
#[derive(Debug, Clone)]
pub struct ImageExportOptions {
    /// Quality setting (0-100) for lossy formats.
    pub quality: u8,
    /// Whether to preserve metadata (EXIF, etc.).
    pub preserve_metadata: bool,
}

impl Default for ImageExportOptions {
    fn default() -> Self {
        Self {
            quality: 90,
            preserve_metadata: true,
        }
    }
}

/// Export a raster image to a file.
///
/// This function handles format-specific encoding and options.
pub fn export_image(
    img: &DynamicImage,
    path: &Path,
    format: ExportFormat,
    _options: &ImageExportOptions,
) -> DocResult<()> {
    match format {
        ExportFormat::Png => {
            img.save_with_format(path, image::ImageFormat::Png)?;
        }
        ExportFormat::Jpeg => {
            // TODO: Apply quality settings
            img.save_with_format(path, image::ImageFormat::Jpeg)?;
        }
        ExportFormat::WebP => {
            img.save_with_format(path, image::ImageFormat::WebP)?;
        }
        ExportFormat::Pdf | ExportFormat::Svg => {
            return Err(anyhow::anyhow!(
                "Export to {} not yet implemented",
                format.extension()
            ));
        }
    }

    Ok(())
}

/// Export a document to a standard paper format (A4, Letter, etc.).
///
/// This function resizes the document to fit the target format while maintaining
/// aspect ratio, then exports it.
pub fn export_to_paper_format(
    img: &DynamicImage,
    path: &Path,
    target_width: u32,
    target_height: u32,
    format: ExportFormat,
) -> DocResult<()> {
    use image::imageops::FilterType;

    // Resize to fit target dimensions
    let resized = img.resize(target_width, target_height, FilterType::Lanczos3);

    // Export with default options
    let options = ImageExportOptions::default();
    export_image(&resized, path, format, &options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_extension() {
        assert_eq!(ExportFormat::Png.extension(), "png");
        assert_eq!(ExportFormat::Jpeg.extension(), "jpg");
        assert_eq!(ExportFormat::Pdf.extension(), "pdf");
    }

    #[test]
    fn test_format_from_path() {
        assert_eq!(
            ExportFormat::from_path(Path::new("test.png")),
            Some(ExportFormat::Png)
        );
        assert_eq!(
            ExportFormat::from_path(Path::new("test.JPG")),
            Some(ExportFormat::Jpeg)
        );
        assert_eq!(ExportFormat::from_path(Path::new("test.txt")), None);
    }
}
