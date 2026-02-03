// SPDX-License-Identifier: GPL-3.0-or-later
// src/infrastructure/system/wallpaper.rs
//
// Set desktop wallpaper across different desktop environments.

use std::path::Path;

/// Set an image as desktop wallpaper using multiple fallback methods.
///
/// Attempts the following methods in order:
/// 1. COSMIC Desktop (direct config file modification)
/// 2. wallpaper crate (KDE, XFCE, Windows, macOS)
/// 3. gsettings (GNOME)
/// 4. feh (tiling window managers)
pub fn set_as_wallpaper(path: &Path) {
    // Canonicalize to absolute path.
    let abs_path = match path.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to canonicalize path {}: {}", path.display(), e);
            return;
        }
    };

    let Some(path_str) = abs_path.to_str() else {
        log::error!("Invalid UTF-8 in path: {}", abs_path.display());
        return;
    };

    log::info!("Attempting to set wallpaper: {path_str}");

    // Method 1: Try COSMIC Desktop (direct config file modification).
    if try_cosmic_wallpaper(path_str) {
        return;
    }

    // Method 2: Try wallpaper crate (supports KDE, XFCE, Windows, macOS).
    if try_wallpaper_crate(path_str) {
        return;
    }

    // Method 3: Try GNOME via gsettings.
    if try_gsettings_wallpaper(path_str) {
        return;
    }

    // Method 4: Try feh (common on tiling WMs like i3, sway).
    if try_feh_wallpaper(path_str) {
        return;
    }

    log::error!("All methods failed to set wallpaper");
}

/// Try setting wallpaper via COSMIC config file.
fn try_cosmic_wallpaper(path_str: &str) -> bool {
    let Some(home) = dirs::home_dir() else {
        return false;
    };

    let cosmic_config = home.join(".config/cosmic/com.system76.CosmicBackground/v1/all");
    if !cosmic_config.exists() {
        return false;
    }

    let config_content = format!(
        r#"(
    output: "all",
    source: Path("{path_str}"),
    filter_by_theme: true,
    rotation_frequency: 300,
    filter_method: Lanczos,
    scaling_mode: Zoom,
    sampling_method: Alphanumeric,
)"#
    );

    match std::fs::write(&cosmic_config, config_content) {
        Ok(()) => {
            log::info!("Wallpaper set via COSMIC config");
            true
        }
        Err(e) => {
            log::warn!("Failed to write COSMIC config: {e}");
            false
        }
    }
}

/// Try setting wallpaper via wallpaper crate.
fn try_wallpaper_crate(path_str: &str) -> bool {
    match wallpaper::set_from_path(path_str) {
        Ok(()) => {
            log::info!("Wallpaper set via wallpaper crate");
            true
        }
        Err(e) => {
            log::warn!("wallpaper crate failed: {e}");
            false
        }
    }
}

/// Try setting wallpaper via GNOME gsettings.
fn try_gsettings_wallpaper(path_str: &str) -> bool {
    let uri = format!("file://{path_str}");

    let output = match std::process::Command::new("gsettings")
        .args(["set", "org.gnome.desktop.background", "picture-uri", &uri])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            log::warn!("gsettings command failed: {e}");
            return false;
        }
    };

    if !output.status.success() {
        log::warn!(
            "gsettings failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return false;
    }

    log::info!("Wallpaper set via gsettings");

    // Also set dark mode wallpaper.
    let _ = std::process::Command::new("gsettings")
        .args([
            "set",
            "org.gnome.desktop.background",
            "picture-uri-dark",
            &uri,
        ])
        .output();

    true
}

/// Try setting wallpaper via feh.
fn try_feh_wallpaper(path_str: &str) -> bool {
    let Ok(output) = std::process::Command::new("feh")
        .args(["--bg-scale", path_str])
        .output()
    else {
        log::warn!("feh not available");
        return false;
    };

    if output.status.success() {
        log::info!("Wallpaper set via feh");
        true
    } else {
        log::warn!("feh failed");
        false
    }
}
