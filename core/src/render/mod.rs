// SPDX-License-Identifier: GPL-3.0-or-later
// core/src/render/mod.rs
//
// UI-independent render engine.
// Takes a LoadedContent + page + zoom, returns pure RGBA pixels.

use crate::document::PageInfo;
use crate::storage::StorageError;
#[cfg(not(any(feature = "resvg", feature = "pdfium-render")))]
use std::marker::PhantomData;

#[cfg(feature = "pdfium-render")]
pub mod worker;

/// Content loaded from disk, kept in memory for rendering.
///
/// The render engine consumes this to produce RGBA pixels.
/// It is intentionally not serializable — it lives only in RAM.
#[derive(Debug)]
pub enum LoadedContent<'a> {
    /// Single raster image (PNG, JPEG, WebP, etc.).
    Raster {
        data: Vec<u8>,
        width: u32,
        height: u32,
    },
    /// SVG document parsed into a resvg tree.
    #[cfg(feature = "resvg")]
    Svg {
        tree: resvg::usvg::Tree,
        width: f32,
        height: f32,
    },
    /// PDF document loaded via pdfium.
    #[cfg(feature = "pdfium-render")]
    Pdf {
        document: pdfium_render::prelude::PdfDocument<'a>,
    },
    /// Placeholder variant that carries the lifetime when no render features are active.
    #[cfg(not(any(feature = "resvg", feature = "pdfium-render")))]
    _Phantom { _marker: PhantomData<&'a ()> },
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

/// Render a single page of a document at the given zoom factor.
///
/// `page` is 1-based. For raster and SVG documents, `page` must be 1.
/// `zoom` is the scale factor where 1.0 = 100%.
pub fn render_page(
    content: &LoadedContent,
    page: u32,
    zoom: f32,
) -> Result<RenderedPage, RenderError> {
    let _ = page; // Used only in pdfium-render path; silences warning otherwise.

    match content {
        LoadedContent::Raster {
            data,
            width,
            height,
        } => render_raster(data, *width, *height, zoom),

        #[cfg(feature = "resvg")]
        LoadedContent::Svg {
            tree,
            width,
            height,
        } => render_svg(tree, *width, *height, zoom),

        #[cfg(feature = "pdfium-render")]
        LoadedContent::Pdf { document } => render_pdf(document, page, zoom),

        #[cfg(not(any(feature = "resvg", feature = "pdfium-render")))]
        _ => render_placeholder(800, 600, zoom),
    }
}

/// Render a single page with a rotation in degrees (0, 90, 180, 270).
///
/// Rotation is applied to the RGBA output after rendering.
pub fn render_page_rotated(
    content: &LoadedContent,
    page: u32,
    zoom: f32,
    rotation_degrees: u16,
    flip_h: bool,
    flip_v: bool,
) -> Result<RenderedPage, RenderError> {
    let mut page = render_page(content, page, zoom)?;

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

/// Get page layout info for a loaded document.
///
/// For raster/SVG this returns a single page. For PDF it returns all pages.
pub fn page_infos(content: &LoadedContent) -> Vec<PageInfo> {
    match content {
        LoadedContent::Raster { width, height, .. } => {
            vec![PageInfo {
                width_pt: *width as f32,
                height_pt: *height as f32,
            }]
        }

        #[cfg(feature = "resvg")]
        LoadedContent::Svg { width, height, .. } => {
            vec![PageInfo {
                width_pt: *width,
                height_pt: *height,
            }]
        }

        #[cfg(feature = "pdfium-render")]
        LoadedContent::Pdf { document } => {
            let pages = document.pages();
            (0..pages.len())
                .filter_map(|i| pages.get(i).ok())
                .map(|p| PageInfo {
                    width_pt: p.width().value,
                    height_pt: p.height().value,
                })
                .collect()
        }

        #[cfg(not(any(feature = "resvg", feature = "pdfium-render")))]
        _ => vec![PageInfo {
            width_pt: 800.0,
            height_pt: 600.0,
        }],
    }
}

// ── Raster Rendering ──

fn render_raster(
    data: &[u8],
    width: u32,
    height: u32,
    zoom: f32,
) -> Result<RenderedPage, RenderError> {
    use image::imageops::FilterType;

    let img = image::load_from_memory(data)
        .map_err(|e| RenderError::Other(format!("Failed to decode raster: {e}")))?;

    if (zoom - 1.0).abs() < f32::EPSILON {
        let rgba = img.to_rgba8();
        return Ok(RenderedPage {
            width,
            height,
            rgba_data: rgba.into_raw(),
        });
    }

    let w = ((width as f32) * zoom).round() as u32;
    let h = ((height as f32) * zoom).round() as u32;
    let scaled = img.resize(w, h, FilterType::Triangle);
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

// ── PDF Rendering ──

#[cfg(feature = "pdfium-render")]
fn render_pdf(
    document: &pdfium_render::prelude::PdfDocument,
    page: u32,
    zoom: f32,
) -> Result<RenderedPage, RenderError> {
    use pdfium_render::prelude::*;

    let pdf_page = document
        .pages()
        .get((page - 1) as u16)
        .map_err(|e| RenderError::Other(format!("Failed to get PDF page {page}: {e}")))?;

    let page_w = pdf_page.width().value;
    let page_h = pdf_page.height().value;

    let target_w = ((page_w * zoom).ceil() as i32).max(1);
    let target_h = ((page_h * zoom).ceil() as i32).max(1);

    let config = PdfRenderConfig::new()
        .set_target_width(target_w)
        .set_maximum_height(target_h);

    let bitmap = pdf_page
        .render_with_config(&config)
        .map_err(|e| RenderError::Other(format!("Failed to render PDF page: {e}")))?;

    let image = bitmap.as_image();
    let rgba = image.to_rgba8();

    Ok(RenderedPage {
        width: rgba.width(),
        height: rgba.height(),
        rgba_data: rgba.into_raw(),
    })
}

// ── Placeholder (no backend available) ──

#[cfg(not(any(feature = "resvg", feature = "pdfium-render")))]
fn render_placeholder(w: u32, _h: u32, zoom: f32) -> Result<RenderedPage, RenderError> {
    let w = ((w as f32) * zoom).round() as u32;
    let h = 60u32; // Fixed height signals "unsupported" to the consumer
    let bg = [40u8, 42, 54, 255]; // Dark background
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..(w * h) {
        data.extend_from_slice(&bg);
    }
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
