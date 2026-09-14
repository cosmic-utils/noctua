// SPDX-License-Identifier: GPL-3.0-or-later
// core/src/render/mod.rs
//
// UI-independent render engine.
// Loads a document and renders it to pure RGBA pixels.

use crate::storage::StorageError;
use std::path::Path;

#[cfg(feature = "pdfium-render")]
pub mod worker;

/// Content loaded from disk, kept in memory for rendering.
///
/// The render engine consumes this to produce RGBA pixels.
/// It is intentionally not serializable — it lives only in RAM.
#[derive(Debug)]
pub enum LoadedContent {
    /// Single raster image (PNG, JPEG, WebP, etc.), already decoded
    /// to straight RGBA. `load` decodes exactly once; rendering reuses
    /// these pixels instead of decoding the file again.
    Raster {
        rgba_data: Vec<u8>,
        width: u32,
        height: u32,
    },
    /// SVG document parsed into a resvg tree.
    #[cfg(feature = "resvg")]
    Svg {
        tree: Box<resvg::usvg::Tree>,
        width: f32,
        height: f32,
    },
}

/// Load a document from disk for rendering.
///
/// Supports raster images and SVG. PDFs cannot be loaded this way:
/// pdfium is not thread-safe, so PDF rendering must go through the
/// worker (`render::worker`).
pub fn load(path: &Path) -> Result<LoadedContent, RenderError> {
    #[cfg(feature = "resvg")]
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    #[cfg(feature = "resvg")]
    if ext == "svg" {
        return load_svg(path);
    }

    load_raster(path)
}

/// Read a raster image into memory, decode it once and return the
/// RGBA pixels plus dimensions.
fn load_raster(path: &Path) -> Result<LoadedContent, RenderError> {
    use image::GenericImageView;

    let data = std::fs::read(path).map_err(|e| RenderError::Storage(e.into()))?;
    let img = image::load_from_memory(&data)
        .map_err(|e| RenderError::Other(format!("Failed to decode raster: {e}")))?;
    let (width, height) = img.dimensions();
    let rgba_data = img.to_rgba8().into_raw();
    Ok(LoadedContent::Raster {
        rgba_data,
        width,
        height,
    })
}

/// Parse an SVG file into a resvg tree for rendering.
#[cfg(feature = "resvg")]
fn load_svg(path: &Path) -> Result<LoadedContent, RenderError> {
    use resvg::usvg;

    let data = std::fs::read(path).map_err(|e| RenderError::Storage(e.into()))?;
    let tree = usvg::Tree::from_data(&data, &usvg::Options::default())
        .map_err(|e| RenderError::Other(format!("Failed to parse SVG: {e}")))?;
    let size = tree.size();
    Ok(LoadedContent::Svg {
        tree: Box::new(tree),
        width: size.width(),
        height: size.height(),
    })
}

/// RGBA pixel buffer produced by the render engine.
#[derive(Debug, Clone)]
pub struct RenderedPage {
    pub width: u32,
    pub height: u32,
    pub rgba_data: Vec<u8>,
}

/// Errors that can occur during rendering.
#[derive(Debug, Clone, thiserror::Error)]
pub enum RenderError {
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("page {0} out of range")]
    PageOutOfRange(u32),

    #[error("invalid content for requested operation: {0}")]
    InvalidContent(String),

    #[error("render error: {0}")]
    Other(String),
}

/// Render a raster or SVG document at the given zoom factor.
///
/// `zoom` is the scale factor where 1.0 = 100%.
pub fn render_page(content: &LoadedContent, zoom: f32) -> Result<RenderedPage, RenderError> {
    match content {
        LoadedContent::Raster {
            rgba_data,
            width,
            height,
        } => render_raster(rgba_data, *width, *height, zoom),

        #[cfg(feature = "resvg")]
        LoadedContent::Svg {
            tree,
            width,
            height,
        } => render_svg(tree.as_ref(), *width, *height, zoom),
    }
}

/// Load a raster or SVG file from disk and render it at the given zoom.
///
/// PDFs are rejected: pdfium is not thread-safe, so PDF pages must be
/// rendered through the worker (`render::worker`).
pub fn render_path(path: &Path, zoom: f32) -> Result<RenderedPage, RenderError> {
    let content = load(path)?;
    render_page(&content, zoom)
}

/// Render a single page with a rotation in degrees (0, 90, 180, 270).
///
/// Rotation is applied to the RGBA output after rendering. Backs the
/// planned View > Rotate menu entries.
pub fn render_page_rotated(
    content: &LoadedContent,
    zoom: f32,
    rotation_degrees: u16,
    flip_h: bool,
    flip_v: bool,
) -> Result<RenderedPage, RenderError> {
    let mut page = render_page(content, zoom)?;

    let deg = rotation_degrees % 360;
    if deg != 0 {
        page = rotate_rgba(&page, deg);
    }
    if flip_h {
        page = flip_rgba_horizontal(&page);
    }
    if flip_v {
        page = flip_rgba_vertical(&page);
    }

    Ok(page)
}

// ── Raster Rendering ──

fn render_raster(
    rgba_data: &[u8],
    width: u32,
    height: u32,
    zoom: f32,
) -> Result<RenderedPage, RenderError> {
    use image::imageops::FilterType;

    if (zoom - 1.0).abs() < f32::EPSILON {
        // No scaling: reuse the decoded pixels without another pass.
        return Ok(RenderedPage {
            width,
            height,
            rgba_data: rgba_data.to_vec(),
        });
    }

    let img = image::RgbaImage::from_raw(width, height, rgba_data.to_vec())
        .ok_or_else(|| RenderError::Other("Failed to construct image buffer".to_string()))?;
    let w = (((width as f32) * zoom).round() as u32).max(1);
    let h = (((height as f32) * zoom).round() as u32).max(1);
    // resize_exact, not resize: resize re-fits the aspect ratio and can
    // return dimensions other than the ones reported to the caller, so the
    // declared size would no longer match the pixel buffer.
    let scaled = image::DynamicImage::ImageRgba8(img).resize_exact(w, h, FilterType::Triangle);
    let rgba = scaled.to_rgba8();

    Ok(RenderedPage {
        width: w,
        height: h,
        rgba_data: rgba.into_raw(),
    })
}

// ── SVG Rendering ──

#[cfg(feature = "resvg")]
fn render_svg(
    tree: &resvg::usvg::Tree,
    width: f32,
    height: f32,
    zoom: f32,
) -> Result<RenderedPage, RenderError> {
    use resvg::tiny_skia;

    let w = ((width * zoom).ceil() as u32).max(1);
    let h = ((height * zoom).ceil() as u32).max(1);

    let mut pixmap = tiny_skia::Pixmap::new(w, h)
        .ok_or_else(|| RenderError::Other("Failed to allocate pixmap".to_string()))?;

    let transform = if (zoom - 1.0).abs() > f32::EPSILON {
        tiny_skia::Transform::from_scale(zoom, zoom)
    } else {
        tiny_skia::Transform::identity()
    };

    resvg::render(tree, transform, &mut pixmap.as_mut());

    // Convert premultiplied RGBA (tiny_skia) to straight RGBA.
    let data: Vec<u8> = pixmap
        .data()
        .chunks(4)
        .flat_map(|p| {
            let a = p[3];
            if a == 0 {
                return [0u8, 0, 0, 0];
            }
            let inv_a = 255.0 / a as f32;
            [
                (p[0] as f32 * inv_a).round().min(255.0) as u8,
                (p[1] as f32 * inv_a).round().min(255.0) as u8,
                (p[2] as f32 * inv_a).round().min(255.0) as u8,
                a,
            ]
        })
        .collect();

    Ok(RenderedPage {
        width: w,
        height: h,
        rgba_data: data,
    })
}

// ── Pixel Transformations (Rotation & Flip) ──

/// Rotate RGBA pixel data by 90, 180, or 270 degrees.
fn rotate_rgba(page: &RenderedPage, degrees: u16) -> RenderedPage {
    match degrees {
        90 => {
            let mut data = vec![0u8; page.rgba_data.len()];
            for y in 0..page.height {
                for x in 0..page.width {
                    let src = ((y * page.width + x) * 4) as usize;
                    let dst_x = page.height - 1 - y;
                    let dst_y = x;
                    let dst = ((dst_y * page.height + dst_x) * 4) as usize;
                    data[dst..dst + 4].copy_from_slice(&page.rgba_data[src..src + 4]);
                }
            }
            RenderedPage {
                width: page.height,
                height: page.width,
                rgba_data: data,
            }
        }
        180 => {
            let mut data = vec![0u8; page.rgba_data.len()];
            for y in 0..page.height {
                for x in 0..page.width {
                    let src = ((y * page.width + x) * 4) as usize;
                    let dst_x = page.width - 1 - x;
                    let dst_y = page.height - 1 - y;
                    let dst = ((dst_y * page.width + dst_x) * 4) as usize;
                    data[dst..dst + 4].copy_from_slice(&page.rgba_data[src..src + 4]);
                }
            }
            RenderedPage {
                width: page.width,
                height: page.height,
                rgba_data: data,
            }
        }
        270 => {
            let mut data = vec![0u8; page.rgba_data.len()];
            for y in 0..page.height {
                for x in 0..page.width {
                    let src = ((y * page.width + x) * 4) as usize;
                    let dst_x = y;
                    let dst_y = page.width - 1 - x;
                    let dst = ((dst_y * page.height + dst_x) * 4) as usize;
                    data[dst..dst + 4].copy_from_slice(&page.rgba_data[src..src + 4]);
                }
            }
            RenderedPage {
                width: page.height,
                height: page.width,
                rgba_data: data,
            }
        }
        _ => page.clone(),
    }
}

/// Flip RGBA pixel data horizontally.
fn flip_rgba_horizontal(page: &RenderedPage) -> RenderedPage {
    let mut data = vec![0u8; page.rgba_data.len()];
    let row_bytes = page.width as usize * 4;
    for y in 0..page.height as usize {
        let src_row = &page.rgba_data[y * row_bytes..(y + 1) * row_bytes];
        let dst_row = &mut data[y * row_bytes..(y + 1) * row_bytes];
        for x in 0..page.width as usize {
            let src = x * 4;
            let dst = (page.width as usize - 1 - x) * 4;
            dst_row[dst..dst + 4].copy_from_slice(&src_row[src..src + 4]);
        }
    }
    RenderedPage {
        width: page.width,
        height: page.height,
        rgba_data: data,
    }
}

/// Flip RGBA pixel data vertically.
fn flip_rgba_vertical(page: &RenderedPage) -> RenderedPage {
    let mut data = vec![0u8; page.rgba_data.len()];
    let row_bytes = page.width as usize * 4;
    for y in 0..page.height as usize {
        let src_row = &page.rgba_data[y * row_bytes..(y + 1) * row_bytes];
        let dst_row = &mut data
            [(page.height as usize - 1 - y) * row_bytes..(page.height as usize - y) * row_bytes];
        dst_row.copy_from_slice(src_row);
    }
    RenderedPage {
        width: page.width,
        height: page.height,
        rgba_data: data,
    }
}
