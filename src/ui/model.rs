// SPDX-License-Identifier: GPL-3.0-or-later
// src/ui/model.rs
//
// UI state (view, tools, panels).
//
// AppModel contains ONLY UI-specific state.
// Document state lives in DocumentManager (application layer).

use cosmic::iced::Size;

use crate::ui::widgets::CropSelection;
use crate::config::AppConfig;

// =============================================================================
// View Mode
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Fit,
    ActualSize,
    Custom,
}

// =============================================================================
// Paper Format (for export/transform)
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaperFormat {
    UsLetter,
    IsoA0,
    IsoA1,
    IsoA2,
    IsoA3,
    IsoA4,
    IsoA5,
    IsoA6,
}

impl PaperFormat {
    /// Returns (width, height) in millimeters
    pub fn dimensions_mm(self) -> (u32, u32) {
        match self {
            Self::UsLetter => (216, 279), // 8.5 x 11 inches
            Self::IsoA0 => (841, 1189),
            Self::IsoA1 => (594, 841),
            Self::IsoA2 => (420, 594),
            Self::IsoA3 => (297, 420),
            Self::IsoA4 => (210, 297),
            Self::IsoA5 => (148, 210),
            Self::IsoA6 => (105, 148),
        }
    }

    /// Returns display name
    pub fn display_name(self) -> &'static str {
        match self {
            Self::UsLetter => "US Letter",
            Self::IsoA0 => "A0 (841 × 1189 mm)",
            Self::IsoA1 => "A1",
            Self::IsoA2 => "A2",
            Self::IsoA3 => "A3",
            Self::IsoA4 => "A4",
            Self::IsoA5 => "A5 (148 × 210 mm)",
            Self::IsoA6 => "A6",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    Horizontal,
    #[default]
    Vertical,
}

// =============================================================================
// Application Mode (combines tool + panel state)
// =============================================================================

/// Application mode - unified tool and panel state.
///
/// Each mode determines:
/// - Active tool behavior
/// - Right panel content
/// - Available shortcuts
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AppMode {
    /// Normal viewing mode - no active tool
    View,

    /// Crop mode with selection
    Crop { selection: CropSelection },

    /// Transform/export mode
    Transform {
        paper_format: Option<PaperFormat>,
        orientation: Orientation,
    },

    /// Fullscreen mode (all panels hidden)
    Fullscreen,
}

impl Default for AppMode {
    fn default() -> Self {
        Self::View
    }
}

impl AppMode {
    /// Get the right panel that should be shown for this mode
    pub fn right_panel(&self) -> Option<RightPanel> {
        match self {
            Self::View => Some(RightPanel::Properties),
            Self::Crop { .. } => Some(RightPanel::CropTools),
            Self::Transform { .. } => Some(RightPanel::TransformTools),
            Self::Fullscreen => None,
        }
    }

    /// Check if mode is an active tool (not View/Fullscreen)
    pub fn is_tool_active(&self) -> bool {
        matches!(self, Self::Crop { .. } | Self::Transform { .. })
    }
}

// =============================================================================
// Viewport (zoom, pan, canvas)
// =============================================================================

/// Viewport state - zoom, pan, canvas dimensions.
#[derive(Debug, Clone)]
pub struct Viewport {
    /// Current scale factor
    pub scale: f32,

    /// Pan offset X
    pub pan_x: f32,

    /// Pan offset Y
    pub pan_y: f32,

    /// Canvas size (container)
    pub canvas_size: Size,

    /// Image size (after scaling)
    pub image_size: Size,

    /// Fit mode
    pub fit_mode: ViewMode,

    /// Scroll container ID
    pub scroll_id: cosmic::widget::Id,

    /// Cached image handle for rendering (updated when document or scale changes)
    pub cached_image_handle: Option<cosmic::widget::image::Handle>,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            scale: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            canvas_size: Size::ZERO,
            image_size: Size::ZERO,
            fit_mode: ViewMode::Fit,
            scroll_id: cosmic::widget::Id::new("canvas-scroll"),
            cached_image_handle: None,
        }
    }
}

impl Viewport {
    /// Reset pan to center
    pub fn reset_pan(&mut self) {
        self.pan_x = 0.0;
        self.pan_y = 0.0;
    }
}

// =============================================================================
// Panel State
// =============================================================================

/// Panel visibility state.
#[derive(Debug, Clone, Default)]
pub struct PanelState {
    /// Left panel (thumbnails for multi-page)
    pub left: Option<LeftPanel>,

    /// Right panel (context-dependent tools/properties)
    pub right: Option<RightPanel>,
}

/// Left panel types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeftPanel {
    /// Thumbnail navigation for multi-page documents
    Thumbnails,
}

/// Right panel types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum RightPanel {
    /// Document properties and metadata
    Properties,

    /// Crop mode tools
    CropTools,

    /// Transform/export tools
    TransformTools,
}

// =============================================================================
// AppModel (UI State Only)
// =============================================================================

/// UI state for the application.
///
/// Contains ONLY UI-specific state:
/// - Current mode (view/tool)
/// - Viewport (zoom/pan)
/// - Panel visibility
/// - Transient UI state (errors, menu)
///
/// Document state (current file, metadata, etc.) lives in DocumentManager!
pub struct AppModel {
    /// Current application mode
    pub mode: AppMode,

    /// Viewport state
    pub viewport: Viewport,

    /// Panel visibility
    pub panels: PanelState,

    /// Error message to display
    pub error: Option<String>,

    /// Is main menu open?
    pub menu_open: bool,

    /// Tick counter for animations
    pub tick: u64,
}

impl AppModel {
    pub fn new(_config: AppConfig) -> Self {
        Self {
            mode: AppMode::default(),
            viewport: Viewport::default(),
            panels: PanelState::default(),
            error: None,
            menu_open: false,
            tick: 0,
        }
    }

    /// Set error message
    pub fn set_error<S: Into<String>>(&mut self, msg: S) {
        self.error = Some(msg.into());
    }

    /// Clear error message
    pub fn clear_error(&mut self) {
        self.error = None;
    }

    /// Reset viewport pan to center
    pub fn reset_pan(&mut self) {
        self.viewport.reset_pan();
    }
}
