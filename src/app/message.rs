// SPDX-License-Identifier: MPL-2.0 OR Apache-2.0
// src/app/message.rs
//
// Top-level application messages (events, IO, and UI signals).

use std::path::PathBuf;

/// Top-level application messages.
///
/// These are produced by:
/// - UI widgets (buttons, menus, etc.)
/// - keyboard shortcuts
/// - async tasks (file loading, etc.)
#[derive(Debug, Clone)]
pub enum AppMessage {
    /// User requested to open a single file.
    OpenPath(PathBuf),

    /// Navigate to next/previous document in the current folder.
    NextDocument,
    PrevDocument,

    /// Basic view / panel toggles.
    ToggleLeftPanel,
    ToggleRightPanel,

    /// View / zoom control.
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ZoomFit,

    /// Pan control (Ctrl + arrow keys).
    PanLeft,
    PanRight,
    PanUp,
    PanDown,
    PanReset,

    /// Editing / tool modes.
    ToggleCropMode,
    ToggleScaleMode,

    /// Document transformations.
    FlipHorizontal,
    FlipVertical,
    RotateCW,
    RotateCCW,

    /// Generic error reporting from lower layers.
    ShowError(String),
    ClearError,

    /// Fallback for unhandled or no-op cases.
    NoOp,
}
