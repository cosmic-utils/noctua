// SPDX-License-Identifier: GPL-3.0-only
// ui/cosmic/src/config.rs
//
// Application settings via cosmic-config, persisted under ~/.config/noctua/.

use cosmic::cosmic_config::{
    self, Config, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry,
};
use serde::{Deserialize, Serialize};

/// Reverse-DNS application id, matching `Application::APP_ID` in app.rs.
const APP_ID: &str = "org.codeberg.wfx.Noctua";

/// Noctua settings, persisted by cosmic-config.
#[derive(CosmicConfigEntry, Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[version = 1]
pub struct NoctuaConfig {
    /// Cap the long edge of displayed images at this many pixels. `None`
    /// renders at native resolution; `Some(2048)` matches iced's atlas size
    /// and trades 1:1 sharpness for lower memory use.
    pub max_texture_edge: Option<u32>,
}

/// Open the cosmic-config handle for Noctua.
pub fn config() -> Result<Config, cosmic_config::Error> {
    Config::new(APP_ID, NoctuaConfig::VERSION)
}
