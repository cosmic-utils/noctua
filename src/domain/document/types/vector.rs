// SPDX-License-Identifier: GPL-3.0-or-later
// src/domain/document/types/vector.rs
//
// Vector documents (SVG, etc.).

use std::path::Path;

/// Minimum pixmap size for SVG rendering (prevents zero-size pixmaps).
const MIN_PIXMAP_SIZE: u32 = 1;

use image::{DynamicImage, GenericImageView, RgbaImage};
use resvg::tiny_skia::{self, Pixmap};
use resvg::usvg::{Options, Tree};

use cosmic::widget::image::Handle as ImageHandle;

use crate::domain::document::core::document::{
    DocResult, DocumentInfo, FlipDirection, Renderable, RenderOutput, Rotation, RotationMode,
    TransformState, Transformable,
};

/// Represents a vector document such as SVG.
pub struct VectorDocument {
    /// Parsed SVG document for re-rendering at different scales.
    document: Tree,
    /// Native width of the SVG (from viewBox or width attribute).
    native_width: u32,
    /// Native height of the SVG (from viewBox or height attribute).
    native_height: u32,
    /// Current render scale (1.0 = native size).
    current_scale: f64,
    /// Accumulated transformations.
    transform: TransformState,
    /// Rasterized image at the current scale.
    pub rendered: DynamicImage,
    /// Image handle for display.
    pub handle: ImageHandle,
    /// Current rendered width.
    pub width: u32,
    /// Current rendered height.
    pub height: u32,
}

impl VectorDocument {
    /// Load a vector document from disk.
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let raw_data = std::fs::read_to_string(path)?;

        // Parse SVG with default options.
        let options = Options::default();
        let document = Tree::from_str(&raw_data, &options)?;

        // Get native size from the parsed document.
        let size = document.size();
        let native_width = size.width().ceil() as u32;
        let native_height = size.height().ceil() as u32;

        let transform = TransformState::default();

        // Render at native scale (1.0).
        let (rendered, width, height) =
            render_document(&document, native_width, native_height, 1.0, transform)?;
        let handle = Self::create_image_handle_from_image(&rendered);

        Ok(Self {
            document,
            native_width,
            native_height,
            current_scale: 1.0,
            transform,
            rendered,
            handle,
            width,
            height,
        })
    }

    /// Returns the dimensions of the rasterized representation.
    #[must_use]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Get the current image handle.
    #[must_use]
    pub fn handle(&self) -> ImageHandle {
        self.handle.clone()
    }

    /// Get native dimensions (before transformations).
    #[must_use]
    pub fn native_dimensions(&self) -> (u32, u32) {
        (self.native_width, self.native_height)
    }

    /// Extract metadata for this vector document.
    pub fn extract_meta(&self, path: &Path) -> crate::domain::document::core::metadata::DocumentMeta {
        use crate::domain::document::core::metadata::{BasicMeta, DocumentMeta};

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let file_path = path.to_string_lossy().to_string();
        let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        let basic = BasicMeta {
            file_name,
            file_path,
            format: "SVG".to_string(),
            width: self.native_width,
            height: self.native_height,
            file_size,
            color_type: "Vector".to_string(),
        };

        DocumentMeta { basic, exif: None }
    }

    /// Crop the document to the specified rectangle.
    /// Works on rendered output (raster).
    pub fn crop(&mut self, x: u32, y: u32, width: u32, height: u32) -> Result<(), String> {
        let (img_width, img_height) = self.rendered.dimensions();

        // Validate crop region
        if x >= img_width || y >= img_height {
            return Err(format!(
                "Crop region ({}, {}) is outside rendered bounds ({}, {})",
                x, y, img_width, img_height
            ));
        }

        // Clamp dimensions
        let crop_width = width.min(img_width - x);
        let crop_height = height.min(img_height - y);

        if crop_width == 0 || crop_height == 0 {
            return Err("Crop region has zero width or height".to_string());
        }

        // Crop rendered image
        self.rendered = self.rendered.crop_imm(x, y, crop_width, crop_height);
        self.width = crop_width;
        self.height = crop_height;

        // Update handle
        self.handle = Self::create_image_handle_from_image(&self.rendered);

        Ok(())
    }

    /// Re-render the SVG at a new scale, preserving transformations.
    /// Returns true if re-rendering occurred.
    #[allow(dead_code)]
    pub fn render_at_scale(&mut self, scale: f64) -> bool {
        // Skip if scale hasn't changed
        if (self.current_scale - scale).abs() < f64::EPSILON {
            return false;
        }

        match render_document(
            &self.document,
            self.native_width,
            self.native_height,
            scale,
            self.transform,
        ) {
            Ok((rendered, width, height)) => {
                self.current_scale = scale;
                self.rendered = rendered;
                self.width = width;
                self.height = height;
                self.handle = Self::create_image_handle_from_image(&self.rendered);
                true
            }
            Err(e) => {
                log::error!("Failed to re-render SVG at scale {scale}: {e}");
                false
            }
        }
    }

    /// Re-render with current scale and transform.
    fn rerender(&mut self) {
        if let Ok((rendered, width, height)) = render_document(
            &self.document,
            self.native_width,
            self.native_height,
            self.current_scale,
            self.transform,
        ) {
            self.rendered = rendered;
            self.width = width;
            self.height = height;
            self.handle = Self::create_image_handle_from_image(&self.rendered);
        }
    }

    // Helper function
    fn create_image_handle_from_image(img: &image::DynamicImage) -> ImageHandle {
        let (width, height) = img.dimensions();
        let pixels = img.to_rgba8().into_raw();
        ImageHandle::from_rgba(width, height, pixels)
    }
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl Renderable for VectorDocument {
    fn render(&mut self, scale: f64) -> DocResult<RenderOutput> {
        self.render_at_scale(scale);
        Ok(RenderOutput {
            handle: self.handle.clone(),
            width: self.width,
            height: self.height,
        })
    }

    fn info(&self) -> DocumentInfo {
        DocumentInfo {
            width: self.native_width,
            height: self.native_height,
            format: "SVG".to_string(),
        }
    }
}

impl Transformable for VectorDocument {
    fn rotate(&mut self, rotation: Rotation) {
        self.transform.rotation = RotationMode::Standard(rotation);
        self.rerender();
    }

    fn flip(&mut self, direction: FlipDirection) {
        match direction {
            FlipDirection::Horizontal => self.transform.flip_h = !self.transform.flip_h,
            FlipDirection::Vertical => self.transform.flip_v = !self.transform.flip_v,
        }
        self.rerender();
    }

    fn transform_state(&self) -> TransformState {
        self.transform
    }
}

/// Render the SVG document at a given scale with transformations.
fn render_document(
    document: &Tree,
    native_width: u32,
    native_height: u32,
    scale: f64,
    transform: TransformState,
) -> anyhow::Result<(DynamicImage, u32, u32)> {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let width = ((f64::from(native_width) * scale).ceil() as u32).max(MIN_PIXMAP_SIZE);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let height = ((f64::from(native_height) * scale).ceil() as u32).max(MIN_PIXMAP_SIZE);

    let mut pixmap =
        Pixmap::new(width, height).ok_or_else(|| anyhow::anyhow!("Failed to create pixmap"))?;

    #[allow(clippy::cast_possible_truncation)]
    let scale_f32 = scale as f32;
    let ts = tiny_skia::Transform::from_scale(scale_f32, scale_f32);
    resvg::render(document, ts, &mut pixmap.as_mut());

    let mut image = pixmap_to_dynamic_image(&pixmap);

    // Apply flip transformations using shared utilities
    if transform.flip_h {
        image = crate::domain::document::operations::transform::apply_flip(
            image,
            FlipDirection::Horizontal,
        );
    }
    if transform.flip_v {
        image = crate::domain::document::operations::transform::apply_flip(
            image,
            FlipDirection::Vertical,
        );
    }

    // Apply rotation using shared utilities
    image = match transform.rotation {
        RotationMode::Standard(rotation) => {
            crate::domain::document::operations::transform::apply_rotation(image, rotation)
        }
        RotationMode::Fine(_) => {
            // For vector documents, fine rotation is handled differently
            // For now, we just render without rotation
            // TODO: Implement fine rotation support for vector documents
            image
        }
    };

    let final_width = image.width();
    let final_height = image.height();

    Ok((image, final_width, final_height))
}

/// Convert a `tiny_skia` Pixmap to a `DynamicImage`.
fn pixmap_to_dynamic_image(pixmap: &Pixmap) -> DynamicImage {
    let width = pixmap.width();
    let height = pixmap.height();

    // tiny_skia uses premultiplied alpha, we need to unpremultiply for image crate
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for pixel in pixmap.pixels() {
        let a = pixel.alpha();
        if a == 0 {
            pixels.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            // Unpremultiply: color = premultiplied_color * 255 / alpha
            let r = (u16::from(pixel.red()) * 255 / u16::from(a)) as u8;
            let g = (u16::from(pixel.green()) * 255 / u16::from(a)) as u8;
            let b = (u16::from(pixel.blue()) * 255 / u16::from(a)) as u8;
            pixels.extend_from_slice(&[r, g, b, a]);
        }
    }

    let rgba_image = RgbaImage::from_raw(width, height, pixels)
        .expect("Failed to create RgbaImage from pixmap data");

    DynamicImage::ImageRgba8(rgba_image)
}
