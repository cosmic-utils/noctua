// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/message.rs
//
// All application messages (events, user actions, signals).

use std::path::PathBuf;

use crate::app::ContextPage;

/// Messages emitted by user actions, async I/O, or internal signals.
#[derive(Debug, Clone)]
pub enum AppMessage {
    // === File / Navigation ===
    /// Open a file at the given path.
    OpenPath(PathBuf),
    /// Navigate to the next document in folder.
    NextDocument,
    /// Navigate to the previous document in folder.
    PrevDocument,

    // === Transformations ===
    /// Rotate 90° clockwise.
    RotateCW,
    /// Rotate 90° counter-clockwise.
    RotateCCW,
    /// Flip horizontally (mirror).
    FlipHorizontal,
    /// Flip vertically.
    FlipVertical,

    // === Zoom ===
    /// Zoom in by a fixed step.
    ZoomIn,
    /// Zoom out by a fixed step.
    ZoomOut,
    /// Reset zoom to 100%.
    ZoomReset,
    /// Fit document to window.
    ZoomFit,

    // === Pan ===
    /// Pan image left.
    PanLeft,
    /// Pan image right.
    PanRight,
    /// Pan image up.
    PanUp,
    /// Pan image down.
    PanDown,
    /// Reset pan to center.
    PanReset,

    // === Tool Modes ===
    /// Toggle crop mode.
    ToggleCropMode,
    /// Toggle scale mode.
    ToggleScaleMode,

    // === Panels (COSMIC-managed) ===
    /// Toggle a context drawer page.
    ToggleContextPage(ContextPage),
    /// Toggle the nav bar (left panel) visibility.
    ToggleNavBar,

    // === Metadata ===
    /// Refresh metadata from the current document.
    RefreshMetadata,

    // === Errors ===
    /// Display an error message.
    ShowError(String),
    /// Clear the current error.
    ClearError,

    /// Fallback for unhandled or no-op cases.
    NoOp,
}
