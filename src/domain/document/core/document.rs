// SPDX-License-Identifier: GPL-3.0-or-later
// src/domain/document/core/document.rs
//
// Core document traits and abstractions.

use cosmic::widget::image::Handle as ImageHandle;

// ============================================================================
// Type Definitions
// ============================================================================

/// Result type alias for document operations.
pub type DocResult<T> = anyhow::Result<T>;

/// Rotation state for documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rotation {
    /// No rotation (0 degrees).
    #[default]
    None,
    /// 90 degrees clockwise.
    Cw90,
    /// 180 degrees.
    Cw180,
    /// 270 degrees clockwise (90 counter-clockwise).
    Cw270,
}

impl Rotation {
    /// Rotate clockwise by 90 degrees.
    #[must_use]
    pub fn rotate_cw(self) -> Self {
        match self {
            Self::None => Self::Cw90,
            Self::Cw90 => Self::Cw180,
            Self::Cw180 => Self::Cw270,
            Self::Cw270 => Self::None,
        }
    }

    /// Rotate counter-clockwise by 90 degrees.
    #[must_use]
    pub fn rotate_ccw(self) -> Self {
        match self {
            Self::None => Self::Cw270,
            Self::Cw270 => Self::Cw180,
            Self::Cw180 => Self::Cw90,
            Self::Cw90 => Self::None,
        }
    }

    /// Convert to degrees (0, 90, 180, 270).
    #[must_use]
    pub fn to_degrees(self) -> i16 {
        match self {
            Self::None => 0,
            Self::Cw90 => 90,
            Self::Cw180 => 180,
            Self::Cw270 => 270,
        }
    }
}

/// Rotation mode: standard 90° steps or fine-grained rotation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RotationMode {
    /// Standard 90° rotation (lossless for most formats).
    Standard(Rotation),
    /// Fine-grained rotation in degrees (0.0 - 360.0) with interpolation.
    Fine(f32),
}

impl Default for RotationMode {
    fn default() -> Self {
        Self::Standard(Rotation::None)
    }
}

impl RotationMode {
    /// Convert rotation to degrees (0.0 - 360.0).
    #[must_use]
    pub fn to_degrees(self) -> f32 {
        match self {
            Self::Standard(r) => f32::from(r.to_degrees()),
            Self::Fine(deg) => deg,
        }
    }

    /// Check if rotation is a multiple of 90 degrees.
    #[must_use]
    pub fn is_multiple_of_90(self) -> bool {
        match self {
            Self::Standard(_) => true,
            Self::Fine(deg) => (deg % 90.0).abs() < 0.01,
        }
    }

    /// Check if no rotation is applied.
    #[must_use]
    pub fn is_none(self) -> bool {
        match self {
            Self::Standard(Rotation::None) => true,
            Self::Standard(_) => false,
            Self::Fine(deg) => deg.abs() < 0.01,
        }
    }

    /// Rotate clockwise by 90 degrees.
    #[must_use]
    pub fn rotate_cw(self) -> Self {
        match self {
            Self::Standard(r) => Self::Standard(r.rotate_cw()),
            Self::Fine(deg) => Self::Fine((deg + 90.0) % 360.0),
        }
    }

    /// Rotate counter-clockwise by 90 degrees.
    #[must_use]
    pub fn rotate_ccw(self) -> Self {
        match self {
            Self::Standard(r) => Self::Standard(r.rotate_ccw()),
            Self::Fine(deg) => Self::Fine((deg - 90.0 + 360.0) % 360.0),
        }
    }
}

/// Interpolation quality for fine rotation and resizing operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InterpolationQuality {
    /// Fast, nearest neighbor interpolation.
    Fast,
    /// Balanced bilinear interpolation (default).
    #[default]
    Balanced,
    /// Best quality, bicubic interpolation.
    Best,
}

/// Flip direction for documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlipDirection {
    /// Flip along the vertical axis (mirror left-right).
    Horizontal,
    /// Flip along the horizontal axis (mirror top-bottom).
    Vertical,
}

/// Current transformation state of a document.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TransformState {
    /// Current rotation mode (standard 90° or fine rotation).
    pub rotation: RotationMode,
    /// Whether flipped horizontally.
    pub flip_h: bool,
    /// Whether flipped vertically.
    pub flip_v: bool,
}

/// Output of a render operation.
#[derive(Debug, Clone)]
pub struct RenderOutput {
    /// Image handle for display.
    pub handle: ImageHandle,
    /// Rendered width in pixels.
    pub width: u32,
    /// Rendered height in pixels.
    pub height: u32,
}

/// Document metadata/information.
#[derive(Debug, Clone)]
pub struct DocumentInfo {
    /// Native width in pixels (before transforms).
    pub width: u32,
    /// Native height in pixels (before transforms).
    pub height: u32,
    /// Document format description.
    pub format: String,
}

// ============================================================================
// Traits
// ============================================================================

/// Trait for documents that can be rendered to an image.
pub trait Renderable {
    /// Render the document at the given scale factor.
    fn render(&mut self, scale: f64) -> DocResult<RenderOutput>;

    /// Get document information (dimensions, format).
    fn info(&self) -> DocumentInfo;
}

/// Trait for documents that support geometric transformations.
pub trait Transformable {
    /// Apply a standard 90° rotation.
    fn rotate(&mut self, rotation: Rotation);

    /// Flip in the given direction.
    fn flip(&mut self, direction: FlipDirection);

    /// Get the current transformation state.
    fn transform_state(&self) -> TransformState;

    /// Apply fine-grained rotation in degrees (0.0 - 360.0).
    fn rotate_fine(&mut self, _angle_degrees: f32) {
        // Default: no-op (not all formats support fine rotation)
    }

    /// Reset any accumulated fine rotation.
    fn reset_fine_rotation(&mut self) {
        // Default: no-op
    }

    /// Set interpolation quality for transformations.
    fn set_interpolation_quality(&mut self, _quality: InterpolationQuality) {
        // Default: no-op
    }
}

/// Trait for documents with multiple pages.
pub trait MultiPage {
    /// Get total number of pages.
    fn page_count(&self) -> usize;

    /// Get current page index (0-based).
    fn current_page(&self) -> usize;

    /// Navigate to a specific page.
    fn go_to_page(&mut self, page: usize) -> DocResult<()>;
}

/// Trait for multi-page documents that support thumbnail generation.
pub trait MultiPageThumbnails: MultiPage {
    /// Get thumbnail for a specific page.
    fn get_thumbnail(&mut self, page: usize) -> DocResult<Option<ImageHandle>>;

    /// Check if thumbnails are ready to be generated.
    fn thumbnails_ready(&self) -> bool;

    /// Check if all thumbnails have been loaded.
    fn thumbnails_loaded(&self) -> bool;

    /// Generate thumbnail for a specific page.
    fn generate_thumbnail_page(&mut self, page: usize) -> DocResult<()>;

    /// Generate all thumbnails.
    fn generate_all_thumbnails(&mut self) -> DocResult<()>;
}
