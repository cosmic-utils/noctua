// SPDX-License-Identifier: GPL-3.0-or-later
// src/config.rs
//
// Global configuration for the application with cosmic-config support.

use cosmic::cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};
use std::path::PathBuf;

/// Global configuration for the application.
#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 1]
pub struct AppConfig {
    /// Optional default directory to open images from.
    pub default_image_dir: Option<PathBuf>,
    /// Whether the nav bar (left panel) is visible.
    pub nav_bar_visible: bool,
    /// Whether the context drawer (right panel) is visible.
    pub context_drawer_visible: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            // TODO: Use xdg dir for picture
            default_image_dir: Some(PathBuf::from("~/Pictures")),
            nav_bar_visible: false,
            context_drawer_visible: false,
        }
    }
}
