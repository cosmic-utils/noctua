// SPDX-License-Identifier: GPL-3.0-or-later
// src/infrastructure/system/mod.rs
//
// System integration: wallpaper, desktop environment utilities.

pub mod wallpaper;

// Re-export wallpaper function
pub use wallpaper::set_as_wallpaper;
