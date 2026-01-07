// SPDX-License-Identifier: MPL-2.0 OR Apache-2.0
// src/app/document/transform.rs
//
// High-level document transformations (rotate, flip, etc.).

use image::{imageops, DynamicImage};

use super::portable::PortableDocument;
use super::raster::RasterDocument;
use super::vector::VectorDocument;
use super::DocumentContent;

/// Rotate current document 90 degrees clockwise.
pub fn rotate_cw(doc: &mut DocumentContent) {
    match doc {
        DocumentContent::Raster(raster) => rotate_cw_raster(raster),
        DocumentContent::Vector(vector) => rotate_cw_vector(vector),
        DocumentContent::Portable(portable) => rotate_cw_portable(portable),
    }
}

/// Rotate current document 90 degrees counter-clockwise.
pub fn rotate_ccw(doc: &mut DocumentContent) {
    match doc {
        DocumentContent::Raster(raster) => rotate_ccw_raster(raster),
        DocumentContent::Vector(vector) => rotate_ccw_vector(vector),
        DocumentContent::Portable(portable) => rotate_ccw_portable(portable),
    }
}

/// Flip current document horizontally.
pub fn flip_horizontal(doc: &mut DocumentContent) {
    match doc {
        DocumentContent::Raster(raster) => flip_horizontal_raster(raster),
        DocumentContent::Vector(vector) => flip_horizontal_vector(vector),
        DocumentContent::Portable(portable) => flip_horizontal_portable(portable),
    }
}

/// Flip current document vertically.
pub fn flip_vertical(doc: &mut DocumentContent) {
    match doc {
        DocumentContent::Raster(raster) => flip_vertical_raster(raster),
        DocumentContent::Vector(vector) => flip_vertical_vector(vector),
        DocumentContent::Portable(portable) => flip_vertical_portable(portable),
    }
}

// --- Raster implementations ---------------------------------------------------

fn rotate_cw_raster(doc: &mut RasterDocument) {
    doc.image = DynamicImage::ImageRgba8(imageops::rotate90(&doc.image));
    doc.refresh_handle();
}

fn rotate_ccw_raster(doc: &mut RasterDocument) {
    doc.image = DynamicImage::ImageRgba8(imageops::rotate270(&doc.image));
    doc.refresh_handle();
}

fn flip_horizontal_raster(doc: &mut RasterDocument) {
    doc.image = DynamicImage::ImageRgba8(imageops::flip_horizontal(&doc.image));
    doc.refresh_handle();
}

fn flip_vertical_raster(doc: &mut RasterDocument) {
    doc.image = DynamicImage::ImageRgba8(imageops::flip_vertical(&doc.image));
    doc.refresh_handle();
}

// --- Portable implementations (operate on rendered image) ---------------------

fn rotate_cw_portable(doc: &mut PortableDocument) {
    // Keep rotation in sync for a future real PDF backend.
    doc.rotation = (doc.rotation + 90).rem_euclid(360);
    doc.rendered = DynamicImage::ImageRgba8(imageops::rotate90(&doc.rendered));
    doc.refresh_handle();
}

fn rotate_ccw_portable(doc: &mut PortableDocument) {
    doc.rotation = (doc.rotation - 90).rem_euclid(360);
    doc.rendered = DynamicImage::ImageRgba8(imageops::rotate270(&doc.rendered));
    doc.refresh_handle();
}

fn flip_horizontal_portable(doc: &mut PortableDocument) {
    doc.rendered = DynamicImage::ImageRgba8(imageops::flip_horizontal(&doc.rendered));
    doc.refresh_handle();
}

fn flip_vertical_portable(doc: &mut PortableDocument) {
    doc.rendered = DynamicImage::ImageRgba8(imageops::flip_vertical(&doc.rendered));
    doc.refresh_handle();
}

// --- Vector implementations (view-transform only, for now) --------------------

fn rotate_cw_vector(_doc: &mut VectorDocument) {
    // TODO: either update a rotation property or re-rasterize with rotation.
}

fn rotate_ccw_vector(_doc: &mut VectorDocument) {
    // TODO: either update a rotation property or re-rasterize with rotation.
}

fn flip_horizontal_vector(_doc: &mut VectorDocument) {
    // TODO: apply horizontal flip to SVG or adjust view transform.
}

fn flip_vertical_vector(_doc: &mut VectorDocument) {
    // TODO: apply vertical flip to SVG or adjust view transform.
}
