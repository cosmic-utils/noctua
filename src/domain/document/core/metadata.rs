// SPDX-License-Identifier: GPL-3.0-or-later
// src/domain/document/core/metadata.rs
//
// Document metadata structures and EXIF parsing.

use std::io::Cursor;

/// Minutes per degree for GPS coordinate conversion (DMS to decimal degrees).
const MINUTES_PER_DEGREE: f64 = 60.0;

/// Seconds per degree for GPS coordinate conversion (DMS to decimal degrees).
const SECONDS_PER_DEGREE: f64 = 3600.0;

/// Basic document metadata (always available).
#[derive(Debug, Clone)]
pub struct BasicMeta {
    /// File name (without path).
    pub file_name: String,
    /// Full file path.
    pub file_path: String,
    /// Image format as string (e.g., "PNG", "JPEG", "PDF").
    pub format: String,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// File size in bytes.
    pub file_size: u64,
    /// Color type description (e.g., "RGBA8", "RGB8", "Grayscale").
    pub color_type: String,
}

impl BasicMeta {
    /// Format file size as human-readable string.
    pub fn file_size_display(&self) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        #[allow(clippy::cast_precision_loss)]
        if self.file_size >= GB {
            let size_gb = self.file_size as f64 / GB as f64;
            format!("{size_gb:.2} GB")
        } else if self.file_size >= MB {
            let size_mb = self.file_size as f64 / MB as f64;
            format!("{size_mb:.2} MB")
        } else if self.file_size >= KB {
            let size_kb = self.file_size as f64 / KB as f64;
            format!("{size_kb:.1} KB")
        } else {
            let size = self.file_size;
            format!("{size} B")
        }
    }

    /// Format resolution as "W × H".
    pub fn resolution_display(&self) -> String {
        format!("{} × {}", self.width, self.height)
    }
}

/// EXIF metadata (optional, mainly for JPEG/TIFF).
#[derive(Debug, Clone, Default)]
pub struct ExifMeta {
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub date_time: Option<String>,
    pub exposure_time: Option<String>,
    pub f_number: Option<String>,
    pub iso: Option<u32>,
    pub focal_length: Option<String>,
    pub gps_latitude: Option<f64>,
    pub gps_longitude: Option<f64>,
}

impl ExifMeta {
    /// Parse EXIF data from raw image bytes.
    ///
    /// Extracts camera information, exposure settings, and GPS coordinates
    /// from JPEG/TIFF EXIF metadata using the kamadak-exif crate.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        use exif::{In, Reader, Tag};

        let cursor = Cursor::new(bytes);
        let exif_reader = Reader::new();
        let exif = exif_reader.read_from_container(&mut cursor.clone()).ok()?;

        let mut meta = Self::default();

        // Camera make and model
        if let Some(field) = exif.get_field(Tag::Make, In::PRIMARY) {
            meta.camera_make = Some(field.display_value().to_string().trim().to_string());
        }
        if let Some(field) = exif.get_field(Tag::Model, In::PRIMARY) {
            meta.camera_model = Some(field.display_value().to_string().trim().to_string());
        }

        // Date and time
        if let Some(field) = exif.get_field(Tag::DateTime, In::PRIMARY) {
            meta.date_time = Some(field.display_value().to_string());
        }

        // Exposure time
        if let Some(field) = exif.get_field(Tag::ExposureTime, In::PRIMARY) {
            meta.exposure_time = Some(field.display_value().to_string());
        }

        // F-number (aperture)
        if let Some(field) = exif.get_field(Tag::FNumber, In::PRIMARY) {
            meta.f_number = Some(field.display_value().to_string());
        }

        // ISO speed
        if let Some(field) = exif.get_field(Tag::PhotographicSensitivity, In::PRIMARY) {
            if let exif::Value::Short(ref vec) = field.value {
                if let Some(&iso) = vec.first() {
                    meta.iso = Some(u32::from(iso));
                }
            }
        }

        // Focal length
        if let Some(field) = exif.get_field(Tag::FocalLength, In::PRIMARY) {
            meta.focal_length = Some(field.display_value().to_string());
        }

        // GPS coordinates
        meta.gps_latitude = Self::parse_gps_coord(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef);
        meta.gps_longitude = Self::parse_gps_coord(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef);

        Some(meta)
    }

    /// Parse GPS coordinate from EXIF data (converts DMS to decimal degrees).
    fn parse_gps_coord(exif: &exif::Exif, coord_tag: exif::Tag, ref_tag: exif::Tag) -> Option<f64> {
        use exif::{In, Value};

        let coord_field = exif.get_field(coord_tag, In::PRIMARY)?;
        let ref_field = exif.get_field(ref_tag, In::PRIMARY)?;

        // Get reference (N/S for latitude, E/W for longitude)
        let reference = ref_field.display_value().to_string();

        // Parse DMS (Degrees, Minutes, Seconds) values
        if let Value::Rational(ref rationals) = coord_field.value {
            if rationals.len() >= 3 {
                let degrees = rationals[0].to_f64();
                let minutes = rationals[1].to_f64();
                let seconds = rationals[2].to_f64();

                // Convert to decimal degrees
                let mut decimal =
                    degrees + (minutes / MINUTES_PER_DEGREE) + (seconds / SECONDS_PER_DEGREE);

                // Apply sign based on hemisphere
                if reference == "S" || reference == "W" {
                    decimal = -decimal;
                }

                return Some(decimal);
            }
        }

        None
    }

    /// Combined camera make and model for display.
    pub fn camera_display(&self) -> Option<String> {
        match (&self.camera_make, &self.camera_model) {
            (Some(make), Some(model)) => {
                if model.starts_with(make) {
                    Some(model.clone())
                } else {
                    Some(format!("{make} {model}"))
                }
            }
            (Some(make), None) => Some(make.clone()),
            (None, Some(model)) => Some(model.clone()),
            (None, None) => None,
        }
    }

    /// Format GPS coordinates for display.
    pub fn gps_display(&self) -> Option<String> {
        match (self.gps_latitude, self.gps_longitude) {
            (Some(lat), Some(lon)) => Some(format!("{lat:.5}, {lon:.5}")),
            _ => None,
        }
    }
}

/// Complete document metadata container.
#[derive(Debug, Clone)]
pub struct DocumentMeta {
    pub basic: BasicMeta,
    pub exif: Option<ExifMeta>,
}
