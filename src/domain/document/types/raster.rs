// SPDX-License-Identifier: GPL-3.0-or-later
// src/domain/document/types/raster.rs
//
// Raster image document support (PNG, JPEG, WebP, etc.).

use std::path::Path;

use image::{DynamicImage, GenericImageView, ImageReader};

use cosmic::widget::image::Handle as ImageHandle;

use crate::domain::document::core::document::{
    DocResult, DocumentInfo, FlipDirection, InterpolationQuality, Renderable, RenderOutput,
    Rotation, RotationMode, TransformState, Transformable,
};

/// Represents a raster image document (PNG, JPEG, WebP, ...).
pub struct RasterDocument {
    /// The decoded image document.
    document: DynamicImage,
    /// Native width (original, before transforms).
    native_width: u32,
    /// Native height (original, before transforms).
    native_height: u32,
    /// Current transformation state.
    transform: TransformState,
    /// Cached handle for rendering.
    handle: ImageHandle,
    /// Accumulated fine rotation angle in degrees.
    fine_rotation_angle: f32,
    /// Interpolation quality for fine rotation and resize operations.
    interpolation_quality: InterpolationQuality,
}

impl RasterDocument {
    /// Load a raster document from disk.
    pub fn open(path: &Path) -> image::ImageResult<Self> {
        let document = ImageReader::open(path)?.decode()?;
        let (native_width, native_height) = document.dimensions();
        let handle = Self::create_image_handle_from_image(&document);

        Ok(Self {
            document,
            native_width,
            native_height,
            transform: TransformState::default(),
            handle,
            fine_rotation_angle: 0.0,
            interpolation_quality: InterpolationQuality::default(),
        })
    }

    /// Returns the current pixel dimensions (width, height) after transforms.
    #[must_use]
    pub fn dimensions(&self) -> (u32, u32) {
        self.document.dimensions()
    }

    /// Get the current image handle.
    #[must_use]
    pub fn handle(&self) -> ImageHandle {
        self.handle.clone()
    }

    /// Save the current document to disk.
    #[allow(dead_code)]
    pub fn save(&self, path: &Path) -> image::ImageResult<()> {
        self.document.save(path)
    }

    /// Get the underlying `DynamicImage`.
    #[must_use]
    pub fn image(&self) -> &DynamicImage {
        &self.document
    }

    /// Get native dimensions (before transformations).
    #[must_use]
    pub fn native_dimensions(&self) -> (u32, u32) {
        (self.native_width, self.native_height)
    }

    /// Get a reference to the rendered image (for cropping from screen coordinates).
    pub fn get_rendered_image(&self) -> &DynamicImage {
        &self.document
    }

    /// Crop the document to a specified rectangular region (in-place).
    ///
    /// Coordinates are in pixels relative to the current image dimensions.
    /// The crop region is clamped to image bounds if it extends beyond.
    ///
    /// # Errors
    ///
    /// Returns an error if the crop region is completely outside the image bounds.
    pub fn crop(&mut self, x: u32, y: u32, width: u32, height: u32) -> Result<(), String> {
        let (img_width, img_height) = self.document.dimensions();

        // Validate crop region
        if x >= img_width || y >= img_height {
            return Err(format!(
                "Crop region ({}, {}) is outside image bounds ({}, {})",
                x, y, img_width, img_height
            ));
        }

        // Clamp dimensions to image bounds
        let crop_width = width.min(img_width - x);
        let crop_height = height.min(img_height - y);

        if crop_width == 0 || crop_height == 0 {
            return Err("Crop region has zero width or height".to_string());
        }

        // Apply crop
        self.document = self.document.crop_imm(x, y, crop_width, crop_height);

        // Update native dimensions to the cropped size
        self.native_width = crop_width;
        self.native_height = crop_height;

        // Reset transformations since we have a new "native" image
        self.transform = TransformState::default();
        self.fine_rotation_angle = 0.0;

        // Regenerate handle
        self.handle = Self::create_image_handle_from_image(&self.document);

        Ok(())
    }
    /// Crop the image to the specified rectangle and return as DynamicImage.
    ///
    /// This does NOT modify the document - it's used for exporting cropped images.
    pub fn crop_to_image(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<DynamicImage, String> {
        let (img_width, img_height) = self.document.dimensions();

        // Validate crop region
        if x >= img_width || y >= img_height {
            return Err(format!(
                "Crop rectangle out of bounds: {}x{} at ({}, {}) exceeds image size {}x{}",
                width, height, x, y, img_width, img_height
            ));
        }

        // Clamp dimensions to image bounds
        let crop_width = width.min(img_width - x);
        let crop_height = height.min(img_height - y);

        if crop_width == 0 || crop_height == 0 {
            return Err("Crop region has zero width or height".to_string());
        }

        let cropped = self.document.crop_imm(x, y, crop_width, crop_height);
        Ok(cropped)
    }

    /// Extract metadata for this raster document.
    ///
    /// Returns basic metadata (dimensions, format, file size) and EXIF data if available.
    pub fn extract_meta(&self, path: &Path) -> crate::domain::document::core::metadata::DocumentMeta {
        use crate::domain::document::core::metadata::{BasicMeta, DocumentMeta, ExifMeta};

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let file_path = path.to_string_lossy().to_string();

        let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        // Detect format from path
        let format = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown")
            .to_uppercase();

        let color_type = format!("{:?}", self.document.color());

        let basic = BasicMeta {
            file_name,
            file_path,
            format,
            width: self.native_width,
            height: self.native_height,
            file_size,
            color_type,
        };

        // Try to extract EXIF data
        let exif = std::fs::read(path)
            .ok()
            .and_then(|bytes| ExifMeta::from_bytes(&bytes));

        DocumentMeta { basic, exif }
    }

    /// Resize the document to specific dimensions (for format conversion).
    ///
    /// This is useful for converting images to standard paper formats (A4, US Letter, etc.).
    pub fn resize_to_format(&mut self, target_width: u32, target_height: u32) {
        use image::imageops::FilterType;

        let filter = match self.interpolation_quality {
            InterpolationQuality::Fast => FilterType::Nearest,
            InterpolationQuality::Balanced => FilterType::Triangle,
            InterpolationQuality::Best => FilterType::CatmullRom,
        };

        self.document = self
            .document
            .resize_exact(target_width, target_height, filter);
        self.handle = Self::create_image_handle_from_image(&self.document);
    }

    // Helper functions
    fn create_image_handle_from_image(img: &DynamicImage) -> ImageHandle {
        let (width, height) = img.dimensions();
        let pixels = img.to_rgba8().into_raw();
        ImageHandle::from_rgba(width, height, pixels)
    }

    fn apply_rotation(img: DynamicImage, rotation: Rotation) -> DynamicImage {
        use image::imageops::{rotate180, rotate270, rotate90};
        match rotation {
            Rotation::None => img,
            Rotation::Cw90 => DynamicImage::ImageRgba8(rotate90(&img.to_rgba8())),
            Rotation::Cw180 => DynamicImage::ImageRgba8(rotate180(&img.to_rgba8())),
            Rotation::Cw270 => DynamicImage::ImageRgba8(rotate270(&img.to_rgba8())),
        }
    }

    fn apply_flip(img: DynamicImage, direction: FlipDirection) -> DynamicImage {
        use image::imageops::{flip_horizontal, flip_vertical};
        match direction {
            FlipDirection::Horizontal => DynamicImage::ImageRgba8(flip_horizontal(&img.to_rgba8())),
            FlipDirection::Vertical => DynamicImage::ImageRgba8(flip_vertical(&img.to_rgba8())),
        }
    }
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl Renderable for RasterDocument {
    fn render(&mut self, _scale: f64) -> DocResult<RenderOutput> {
        // Raster images don't re-render at different scales (lossy),
        // we just return the current handle.
        let (width, height) = self.dimensions();
        Ok(RenderOutput {
            handle: self.handle.clone(),
            width,
            height,
        })
    }

    fn info(&self) -> DocumentInfo {
        DocumentInfo {
            width: self.native_width,
            height: self.native_height,
            format: "Raster".to_string(),
        }
    }
}

impl Transformable for RasterDocument {
    fn rotate(&mut self, rotation: Rotation) {
        // Extract current rotation in degrees
        let current_deg = match self.transform.rotation {
            RotationMode::Standard(r) => r.to_degrees(),
            RotationMode::Fine(_) => {
                // If we have fine rotation, reset it and apply standard rotation
                self.fine_rotation_angle = 0.0;
                0
            }
        };

        let new_deg = rotation.to_degrees();
        let diff_deg = (new_deg - current_deg + 360) % 360;

        if diff_deg != 0 {
            let rotation_to_apply = match diff_deg {
                90 => Rotation::Cw90,
                180 => Rotation::Cw180,
                270 => Rotation::Cw270,
                _ => unreachable!("Invalid rotation diff: {}", diff_deg),
            };
            self.document = Self::apply_rotation(
                std::mem::replace(&mut self.document, DynamicImage::new_rgb8(1, 1)),
                rotation_to_apply,
            );
        }

        // Set to standard rotation mode
        self.transform.rotation = RotationMode::Standard(rotation);
        self.handle = Self::create_image_handle_from_image(&self.document);
    }

    fn flip(&mut self, direction: FlipDirection) {
        self.document = Self::apply_flip(
            std::mem::replace(&mut self.document, DynamicImage::new_rgb8(1, 1)),
            direction,
        );
        match direction {
            FlipDirection::Horizontal => self.transform.flip_h = !self.transform.flip_h,
            FlipDirection::Vertical => self.transform.flip_v = !self.transform.flip_v,
        }
        self.handle = Self::create_image_handle_from_image(&self.document);
    }

    fn transform_state(&self) -> TransformState {
        self.transform
    }

    fn rotate_fine(&mut self, angle_degrees: f32) {
        use imageproc::geometric_transformations::{rotate_about_center, Interpolation};

        let interpolation = match self.interpolation_quality {
            InterpolationQuality::Fast => Interpolation::Nearest,
            InterpolationQuality::Balanced => Interpolation::Bilinear,
            InterpolationQuality::Best => Interpolation::Bicubic,
        };

        // Convert to RGBA8 for imageproc
        let rgba_img = self.document.to_rgba8();

        // Rotate with transparent background
        let rotated = rotate_about_center(
            &rgba_img,
            angle_degrees.to_radians(),
            interpolation,
            image::Rgba([255, 255, 255, 0]),
        );

        self.document = DynamicImage::ImageRgba8(rotated);
        self.fine_rotation_angle += angle_degrees;
        self.transform.rotation = RotationMode::Fine(self.fine_rotation_angle);
        self.handle = Self::create_image_handle_from_image(&self.document);
    }

    fn reset_fine_rotation(&mut self) {
        self.fine_rotation_angle = 0.0;
        self.transform.rotation = RotationMode::Standard(Rotation::None);
    }

    fn set_interpolation_quality(&mut self, quality: InterpolationQuality) {
        self.interpolation_quality = quality;
    }
}
