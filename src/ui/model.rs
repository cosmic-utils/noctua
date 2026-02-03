// SPDX-License-Identifier: GPL-3.0-or-later
// src/ui/model.rs
//
// UI state (view, tools, panels).

use cosmic::iced::Size;

use crate::ui::components::crop::CropSelection;
use crate::config::AppConfig;

// =============================================================================
// Enums
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Fit,
    ActualSize,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolMode {
    None,
    Crop,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavPanel {
    None,
    Pages,
    Format,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

// =============================================================================
// Model
// =============================================================================

/// UI state for the application.
///
/// This struct holds only UI-related state (view, tools, panels).
/// Document data is managed by DocumentManager in the application layer.
/// Cached render data is stored here for performance.
pub struct AppModel {
    // Cached rendering data (read-only from DocumentManager)
    pub current_image_handle: Option<cosmic::widget::image::Handle>,
    pub current_dimensions: Option<(u32, u32)>,
    pub current_page: Option<usize>,
    pub page_count: Option<usize>,

    // Cached metadata (read-only)
    pub metadata: Option<crate::domain::document::core::metadata::DocumentMeta>,

    // Navigation info (read-only)
    pub current_path: Option<std::path::PathBuf>,
    pub current_index: Option<usize>,
    pub folder_count: usize,

    // View state
    pub view_mode: ViewMode,
    pub pan_x: f32,
    pub pan_y: f32,
    pub scale: f32,
    pub canvas_size: Size,
    pub image_size: Size,

    // Tool state
    pub tool_mode: ToolMode,
    pub crop_selection: CropSelection,

    // Format settings (for export)
    pub paper_format: Option<PaperFormat>,
    pub orientation: Orientation,

    // UI panels
    pub active_nav_panel: NavPanel,
    pub last_nav_panel: Option<NavPanel>,
    pub menu_open: bool,

    // UI feedback
    pub error: Option<String>,
    pub tick: u64,
}

impl AppModel {
    pub fn new(_config: AppConfig) -> Self {
        Self {
            // Cached data
            current_image_handle: None,
            current_dimensions: None,
            current_page: None,
            page_count: None,
            metadata: None,
            current_path: None,
            current_index: None,
            folder_count: 0,
            // View state
            view_mode: ViewMode::Fit,
            pan_x: 0.0,
            pan_y: 0.0,
            scale: 1.0,
            canvas_size: Size::ZERO,
            image_size: Size::ZERO,
            // Tool state
            tool_mode: ToolMode::None,
            crop_selection: CropSelection::default(),
            // Format settings
            paper_format: None,
            orientation: Orientation::Vertical,
            // UI panels
            active_nav_panel: NavPanel::None,
            last_nav_panel: None,
            menu_open: false,
            // UI feedback
            error: None,
            tick: 0,
        }
    }

    pub fn set_error<S: Into<String>>(&mut self, msg: S) {
        self.error = Some(msg.into());
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }

    pub fn reset_pan(&mut self) {
        self.pan_x = 0.0;
        self.pan_y = 0.0;
    }
}
