// SPDX-License-Identifier: GPL-3.0-or-later
// src/document/format.rs
//
// Document formats and geometry calculations.

/// Represents a physical paper size in millimeters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaperSize {
    pub width_mm: f32,
    pub height_mm: f32,
}

impl PaperSize {
    // Some common paper sizes
    pub const A4: Self = Self {
        width_mm: 210.0,
        height_mm: 297.0,
    };
    pub const A5: Self = Self {
        width_mm: 148.0,
        height_mm: 210.0,
    };
    pub const LETTER: Self = Self {
        width_mm: 215.9,
        height_mm: 279.4,
    };

    // let custom = PaperSize::new(100.0, 200.0);
    pub fn new(width_mm: f32, height_mm: f32) -> Self {
        Self {
            width_mm,
            height_mm,
        }
    }

    /// Converts the physical dimensions to pixels based on the given DPI.
    /// Formula: mm * DPI / 25.4 = px
    pub fn to_pixels(&self, dpi: f32) -> (u32, u32) {
        let width_px = (self.width_mm * dpi / 25.4).round() as u32;
        let height_px = (self.height_mm * dpi / 25.4).round() as u32;

        (width_px, height_px)
    }
}
