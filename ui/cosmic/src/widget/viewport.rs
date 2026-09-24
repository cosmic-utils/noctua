// SPDX-License-Identifier: GPL-3.0-only
// ui/cosmic/src/widget/viewport.rs
//
// Viewport math shared by the image viewer and, later, the annotation
// canvas. Kept separate from the widget so the app owns the state.

use cosmic::iced::{Size, Vector};

/// Clamp a pan offset so the image never scrolls out of view.
pub(crate) fn clamp_offset(offset: Vector, bounds: Size, image_size: Size) -> Vector {
    let hidden_width = (image_size.width - bounds.width / 2.0).max(0.0).round();
    let hidden_height = (image_size.height - bounds.height / 2.0).max(0.0).round();
    Vector::new(
        offset.x.clamp(-hidden_width, hidden_width),
        offset.y.clamp(-hidden_height, hidden_height),
    )
}
