// SPDX-License-Identifier: GPL-3.0-or-later
// src/config.rs

use cosmic::cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};
use std::path::PathBuf;

/// Global configuration for the application.
#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 1]
pub struct AppConfig {
    /// Optional default directory to open images from.
    pub default_image_dir: Option<PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            // TODO: Use xdg dir for picture
            default_image_dir: Some(PathBuf::from("~/Pictures")),
        }
    }
}
