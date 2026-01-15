// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/document/utils.rs
//
// Utility functions for document operations.

/// Set an image as desktop wallpaper using multiple fallback methods.
///
/// Expects an absolute path as string.
pub fn set_as_wallpaper(path_str: &str) {
    log::info!("Attempting to set wallpaper: {}", path_str);

    // Method 1: Try COSMIC Desktop (direct config file modification)
    if let Some(home) = dirs::home_dir() {
        let cosmic_config = home.join(".config/cosmic/com.system76.CosmicBackground/v1/all");

        if cosmic_config.exists() {
            let config_content = format!(
                r#"(
    output: "all",
    source: Path("{}"),
    filter_by_theme: true,
    rotation_frequency: 300,
    filter_method: Lanczos,
    scaling_mode: Zoom,
    sampling_method: Alphanumeric,
)"#,
                path_str
            );

            match std::fs::write(&cosmic_config, config_content) {
                Ok(_) => {
                    log::info!("✓ Wallpaper set via COSMIC config file");
                    return;
                }
                Err(e) => {
                    log::warn!("Failed to write COSMIC config: {}", e);
                }
            }
        }
    }

    // Method 2: Try wallpaper crate (supports KDE, XFCE, Windows, macOS)
    match wallpaper::set_from_path(path_str) {
        Ok(_) => {
            log::info!("✓ Wallpaper set successfully via wallpaper crate");
            return;
        }
        Err(e) => {
            log::warn!("wallpaper crate failed: {}", e);
        }
    }

    // Method 3: Try GNOME via gsettings
    let uri = format!("file://{}", path_str);
    log::info!("Trying gsettings with URI: {}", uri);

    match std::process::Command::new("gsettings")
        .args(&[
            "set",
            "org.gnome.desktop.background",
            "picture-uri",
            &uri,
        ])
        .output()
    {
        Ok(output) if output.status.success() => {
            log::info!("✓ Wallpaper set via gsettings (light mode)");

            // Also set dark mode wallpaper
            let _ = std::process::Command::new("gsettings")
                .args(&[
                    "set",
                    "org.gnome.desktop.background",
                    "picture-uri-dark",
                    &uri,
                ])
                .output();
            return;
        }
        Ok(output) => {
            log::warn!(
                "gsettings failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Err(e) => {
            log::warn!("gsettings command failed: {}", e);
        }
    }

    // Method 4: Try feh (common on tiling WMs like i3, sway)
    match std::process::Command::new("feh")
        .args(&["--bg-scale", path_str])
        .output()
    {
        Ok(output) if output.status.success() => {
            log::info!("✓ Wallpaper set via feh");
            return;
        }
        Ok(_) => {
            log::warn!("feh failed");
        }
        Err(_) => {
            log::warn!("feh not available");
        }
    }

    log::error!("✗ All methods failed to set wallpaper");
}
