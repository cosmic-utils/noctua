// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/message.rs
//
// Application messages: events, user actions, and internal signals.

use std::path::PathBuf;

use crate::ui::widgets::DragHandle;

#[derive(Debug, Clone)]
pub enum AppMessage {
    // File / navigation.
    #[allow(dead_code)]
    OpenPath(PathBuf),
    NextDocument,
    PrevDocument,
    GotoPage(usize),
    GenerateThumbnailPage(usize),

    // Transformations.
    RotateCW,
    RotateCCW,
    FlipHorizontal,
    FlipVertical,

    // View / zoom.
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ZoomFit,
    ViewerStateChanged {
        scale: f32,
        offset_x: f32,
        offset_y: f32,
        canvas_size: cosmic::iced::Size,
        image_size: cosmic::iced::Size,
    },

    // Pan control.
    PanLeft,
    PanRight,
    PanUp,
    PanDown,
    PanReset,

    // Tool modes.
    ToggleCropMode,
    ToggleScaleMode,

    // Crop operations.
    StartCrop,
    CancelCrop,
    ApplyCrop,
    CropDragStart {
        x: f32,
        y: f32,
        handle: DragHandle,
    },
    CropDragMove {
        x: f32,
        y: f32,
        max_x: f32,
        max_y: f32,
    },
    CropDragEnd,

    // Panels.
    ToggleContextPage(crate::ui::app::ContextPage),
    ToggleNavBar,
    OpenFormatPanel,

    // Menu.
    ToggleMainMenu,

    // Format operations.
    SetPaperFormat(super::model::PaperFormat),
    SetOrientation(super::model::Orientation),

    // Metadata.
    #[allow(dead_code)]
    RefreshMetadata,

    // Save operations.
    SaveAs,

    // Wallpaper.
    SetAsWallpaper,

    // Errors.
    #[allow(dead_code)]
    ShowError(String),
    #[allow(dead_code)]
    ClearError,

    // UI refresh.
    RefreshView,

    // Fallback.
    #[allow(dead_code)]
    NoOp,
}
