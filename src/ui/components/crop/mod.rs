// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/crop/mod.rs
//
// Crop selection module: overlay widget and selection state.

mod selection;
mod overlay;
mod theme;

// CropRegion is part of the public API (returned by CropSelection::get_region())
// even if not directly imported by consumers
#[allow(unused_imports)]
pub use selection::{CropSelection, CropRegion, DragHandle};
pub use overlay::crop_overlay;
