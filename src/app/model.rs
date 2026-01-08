// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/model.rs
//
// Global application state.

use std::path::PathBuf;

use crate::app::document::DocumentContent;
use crate::config::AppConfig;

/// How the document is currently fitted into the window.
#[derive(Debug, Clone, Copy)]
pub enum ViewMode {
    /// Fit document to available window size.
    Fit,
    /// Display at 100% (1.0 scale).
    ActualSize,
    /// Custom zoom factor (e.g., 0.5 = 50%, 2.0 = 200%).
    Custom(f32),
}

impl ViewMode {
    /// Return the effective zoom factor for this mode.
    /// For `Fit`, returns `None` since the factor depends on window size.
    pub fn zoom_factor(&self) -> Option<f32> {
        match self {
            ViewMode::Fit => None,
            ViewMode::ActualSize => Some(1.0),
            ViewMode::Custom(z) => Some(*z),
        }
    }
}

/// Current editing / interaction mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolMode {
    None,
    Crop,
    Scale,
}

/// Pan step size in pixels per key press.
pub const PAN_STEP: f32 = 50.0;

/// Global application state.
#[derive(Debug)]
pub struct AppModel {
    /// Static configuration loaded at startup.
    pub config: AppConfig,

    /// Currently opened document (raster/vector/portable).
    pub document: Option<DocumentContent>,

    /// Path of the currently opened document, if any.
    pub current_path: Option<PathBuf>,

    /// List of files in the current folder for navigation.
    pub folder_entries: Vec<PathBuf>,

    /// Index into `folder_entries` of the current file.
    pub current_index: Option<usize>,

    /// View / zoom state.
    pub view_mode: ViewMode,

    /// Pan offset (in pixels, relative to centered position).
    pub pan_x: f32,
    pub pan_y: f32,

    /// Panel visibility.
    pub show_left_panel: bool,
    pub show_right_panel: bool,

    /// Current tool mode.
    pub tool_mode: ToolMode,

    /// Last error message to be shown in the UI, if any.
    pub error: Option<String>,
}

impl AppModel {
    /// Construct a new application state from configuration.
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            document: None,
            current_path: None,
            folder_entries: Vec::new(),
            current_index: None,
            view_mode: ViewMode::Fit,
            pan_x: 0.0,
            pan_y: 0.0,
            show_left_panel: false,
            show_right_panel: false,
            tool_mode: ToolMode::None,
            error: None,
        }
    }

    /// Helper: set an error string.
    pub fn set_error<S: Into<String>>(&mut self, msg: S) {
        self.error = Some(msg.into());
    }

    /// Helper: clear current error.
    pub fn clear_error(&mut self) {
        self.error = None;
    }

    /// Reset pan offset to center.
    pub fn reset_pan(&mut self) {
        self.pan_x = 0.0;
        self.pan_y = 0.0;
    }

    /// Get the current zoom factor, if applicable.
    pub fn zoom_factor(&self) -> Option<f32> {
        self.view_mode.zoom_factor()
    }
}
