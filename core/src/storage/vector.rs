// SPDX-License-Identifier: GPL-3.0-or-later
// src/storage/vector.rs
//
// Vector graphics (SVG) metadata extraction.

use std::fs;
use std::path::Path;

use crate::document::Vector;
use crate::storage::StorageError;

/// Load SVG metadata without parsing the entire document or rendering.
///
/// Extracts viewbox dimensions, SVG version, and other vector-specific properties.
/// Returns a `Vector` struct and the number of pages (always 1 for SVG).
pub fn load_svg_metadata(path: &Path) -> Result<(Vector, u32), StorageError> {
    #[cfg(feature = "resvg")]
    {
        use resvg::usvg::{self, Options};

        let svg_data = fs::read(path)?;

        let opt = Options {
            // Don't load external resources for metadata extraction
            resources_dir: None,
            font_family: "sans-serif".to_string(),
            font_size: 12.0,
            languages: vec!["en".to_string()],
            shape_rendering: usvg::ShapeRendering::GeometricPrecision,
            text_rendering: usvg::TextRendering::OptimizeLegibility,
            image_rendering: usvg::ImageRendering::OptimizeQuality,
            default_size: usvg::Size::from_wh(100.0, 100.0).unwrap(),
            ..Default::default()
        };

        let tree = usvg::Tree::from_data(&svg_data, &opt)
            .map_err(|e| StorageError::Document(format!("Failed to parse SVG: {}", e)))?;

        let root = tree.root();
        // In usvg 0.45, the parsed Group does not retain raw XML attributes.
        // Viewbox dimensions come from the resolved bounding box.
        let viewbox = root.abs_bounding_box();
        let viewbox_width = viewbox.width();
        let viewbox_height = viewbox.height();

        let vector = Vector {
            format: "SVG".to_string(),
            viewbox_width,
            viewbox_height,
        };

        Ok((vector, 1)) // SVG documents always have 1 page
    }

    #[cfg(not(feature = "resvg"))]
    {
        // Fall back to basic parsing if resvg is not available
        load_svg_metadata_basic(path)
    }
}

/// Minimal SVG metadata extraction without external dependencies.
///
/// Parses the SVG file as text to extract basic viewbox and dimensions.
/// This is less accurate than resvg but doesn't require the dependency.
pub fn load_svg_metadata_basic(path: &Path) -> Result<(Vector, u32), StorageError> {
    // Read the SVG file as text
    let content = fs::read_to_string(path)?;

    // Default viewbox dimensions
    let mut viewbox_width = 100.0;
    let mut viewbox_height = 100.0;

    // Try to find viewBox attribute
    if let Some(viewbox_start) = content.find("viewBox=\"") {
        let viewbox_sub = &content[viewbox_start + 9..]; // Skip "viewBox=\""
        if let Some(viewbox_end) = viewbox_sub.find('"') {
            let viewbox_str = &viewbox_sub[..viewbox_end];
            let parts: Vec<&str> = viewbox_str.split_whitespace().collect();
            if parts.len() >= 4 {
                // viewBox="x y width height"
                if let (Ok(w), Ok(h)) = (parts[2].parse::<f32>(), parts[3].parse::<f32>()) {
                    viewbox_width = w;
                    viewbox_height = h;
                }
            }
        }
    } else {
        // If no viewBox, try width and height attributes
        if let Some(width_start) = content.find("width=\"") {
            let width_sub = &content[width_start + 7..]; // Skip "width=\""
            if let Some(width_end) = width_sub.find('"') {
                let width_str = &width_sub[..width_end];
                if let Ok(w) = parse_svg_length(width_str) {
                    viewbox_width = w;
                }
            }
        }

        if let Some(height_start) = content.find("height=\"") {
            let height_sub = &content[height_start + 8..]; // Skip "height=\""
            if let Some(height_end) = height_sub.find('"') {
                let height_str = &height_sub[..height_end];
                if let Ok(h) = parse_svg_length(height_str) {
                    viewbox_height = h;
                }
            }
        }
    }

    let vector = Vector {
        format: "SVG".to_string(),
        viewbox_width,
        viewbox_height,
    };

    Ok((vector, 1)) // SVG documents always have 1 page
}

/// Parse SVG length values (e.g., "100", "100px", "10cm", "50%", "1.5in").
///
/// Returns pixels as f32. This is a simplified parser that handles common cases.
fn parse_svg_length(length_str: &str) -> Result<f32, StorageError> {
    // Remove whitespace
    let trimmed = length_str.trim();

    if let Some(stripped) = trimmed.strip_suffix('%')
        && let Ok(value) = stripped.parse::<f32>()
    {
        return Ok(value); // Percentage values are returned as-is
    }

    // Extract numeric part
    let numeric_part = trimmed
        .chars()
        .take_while(|c| c.is_numeric() || *c == '.' || *c == '-')
        .collect::<String>();

    if numeric_part.is_empty() {
        return Ok(0.0);
    }

    let value = numeric_part
        .parse::<f32>()
        .map_err(|e| StorageError::Document(format!("Failed to parse SVG length: {}", e)))?;

    // Handle units
    let unit = trimmed[numeric_part.len()..].trim();

    match unit {
        "px" | "" => Ok(value),                  // Pixels (default)
        "in" => Ok(value * 96.0),                // Inches to pixels (96 DPI)
        "cm" => Ok(value * 37.795_277),          // Centimeters to pixels
        "mm" => Ok(value * 3.779_527_7),         // Millimeters to pixels
        "pt" => Ok(value * 1.333_333_4),         // Points to pixels
        "pc" => Ok(value * 16.0),                // Picas to pixels
        "em" | "ex" | "rem" => Ok(value * 16.0), // Font units (approximation)
        _ => Ok(value),                          // Unknown unit, assume pixels
    }
}
