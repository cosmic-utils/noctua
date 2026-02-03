// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/crop/theme.rs
//
// Theme colors for crop overlay UI elements.

/// Crop overlay opacity for darkened areas outside selection (0.0-1.0).
const CROP_OVERLAY_ALPHA: f32 = 0.5;

/// Crop overlay grid line opacity (0.0-1.0).
const CROP_GRID_ALPHA: f32 = 0.8;

use cosmic::iced::Color;

/// Get the overlay color from theme (darkened background over non-selected areas).
pub fn overlay_color(theme: &cosmic::Theme) -> Color {
    let mut c = theme.cosmic().palette.neutral_9;
    c.alpha = CROP_OVERLAY_ALPHA;
    Color::from(c)
}

/// Get the border color for the selection rectangle.
pub fn border_color(theme: &cosmic::Theme) -> Color {
    Color::from(theme.cosmic().palette.neutral_0)
}

/// Get the handle color for resize/move handles.
pub fn handle_color(theme: &cosmic::Theme) -> Color {
    Color::from(theme.cosmic().palette.neutral_0)
}

/// Get the grid color (rule of thirds, semi-transparent).
pub fn grid_color(theme: &cosmic::Theme) -> Color {
    let mut c = theme.cosmic().palette.neutral_0;
    c.alpha = CROP_GRID_ALPHA;
    Color::from(c)
}
