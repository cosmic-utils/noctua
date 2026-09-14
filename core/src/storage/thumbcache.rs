// SPDX-License-Identifier: GPL-3.0-or-later
// core/src/storage/thumbcache.rs
//
// Thumbnail cache following the freedesktop.org specification.
//
// Cache location: ~/.cache/thumbnails/{normal,large,x-large}/
// Filename:       MD5 hash of file:// URI + .png
// PNG metadata:   Thumb::URI and Thumb::MTime for cache validation

use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use super::StorageError;

/// Thumbnail sizes defined by the freedesktop.org spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbSize {
    /// Up to 128×128 pixels, stored in `~/.cache/thumbnails/normal/`.
    Normal,
    /// Up to 256×256 pixels, stored in `~/.cache/thumbnails/large/`.
    Large,
    /// Up to 512×512 pixels, stored in `~/.cache/thumbnails/x-large/`.
    XLarge,
}

impl ThumbSize {
    fn subdir(&self) -> &str {
        match self {
            ThumbSize::Normal => "normal",
            ThumbSize::Large => "large",
            ThumbSize::XLarge => "x-large",
        }
    }

    fn max_px(&self) -> u32 {
        match self {
            ThumbSize::Normal => 128,
            ThumbSize::Large => 256,
            ThumbSize::XLarge => 512,
        }
    }
}

/// Returns `(width, height, rgba8_data)` for a thumbnail of `path`.
///
/// Loads from cache if a valid entry exists, otherwise generates and stores one.
/// Synchronous: the generator runs on the calling thread. PDFs are rejected:
/// they must be rendered on the pdfium worker (pdfium is not thread-safe);
/// use the worker's `RenderThumb` job with `lookup`/`store` instead.
pub fn get_or_create(path: &Path, size: ThumbSize) -> Result<(u32, u32, Vec<u8>), StorageError> {
    let cache_root = dirs::cache_dir()
        .ok_or_else(|| StorageError::Thumb("Cache directory not found".to_string()))?;
    get_or_create_at(&cache_root, path, size)
}

/// `get_or_create` with an explicit cache root (tests, alternative data roots).
pub fn get_or_create_at(
    cache_root: &Path,
    path: &Path,
    size: ThumbSize,
) -> Result<(u32, u32, Vec<u8>), StorageError> {
    if let Some(thumb) = lookup_at(cache_root, path, size)? {
        return Ok(thumb);
    }

    // pdfium is not thread-safe; PDF thumbnails must go through the
    // render worker, so refuse to generate them here.
    if super::document::is_pdf(path)? {
        return Err(StorageError::Thumb(
            "PDF thumbnails must be rendered via the render worker".to_string(),
        ));
    }

    let thumb = generate(path, &size)?;
    let _ = store_at(cache_root, path, size, &thumb);
    Ok(thumb)
}

/// Look up a cached thumbnail without generating one.
///
/// Returns `None` when no valid cache entry exists. Fast and cheap:
/// safe to call on the UI thread before delegating generation to the worker.
pub fn lookup(path: &Path, size: ThumbSize) -> Result<Option<(u32, u32, Vec<u8>)>, StorageError> {
    let cache_root = dirs::cache_dir()
        .ok_or_else(|| StorageError::Thumb("Cache directory not found".to_string()))?;
    lookup_at(&cache_root, path, size)
}

/// `lookup` with an explicit cache root.
pub fn lookup_at(
    cache_root: &Path,
    path: &Path,
    size: ThumbSize,
) -> Result<Option<(u32, u32, Vec<u8>)>, StorageError> {
    let uri = file_uri(path);
    let hash = format!("{:x}", md5::compute(uri.as_bytes()));
    let cache_path = cache_dir_at(cache_root, &size)?.join(format!("{}.png", hash));
    let mtime = file_mtime(path)?;

    if cache_path.exists()
        && let Ok(thumb) = load_if_valid(&cache_path, &uri, mtime)
    {
        return Ok(Some(thumb));
    }
    Ok(None)
}

/// Store a generated thumbnail in the cache.
///
/// Writes PNG metadata (`Thumb::URI`, `Thumb::MTime`) for validation.
/// Called by the worker after rendering a thumbnail.
pub fn store(
    path: &Path,
    size: ThumbSize,
    thumb: &(u32, u32, Vec<u8>),
) -> Result<(), StorageError> {
    let cache_root = dirs::cache_dir()
        .ok_or_else(|| StorageError::Thumb("Cache directory not found".to_string()))?;
    store_at(&cache_root, path, size, thumb)
}

/// `store` with an explicit cache root.
pub fn store_at(
    cache_root: &Path,
    path: &Path,
    size: ThumbSize,
    thumb: &(u32, u32, Vec<u8>),
) -> Result<(), StorageError> {
    let uri = file_uri(path);
    let hash = format!("{:x}", md5::compute(uri.as_bytes()));
    let cache_path = cache_dir_at(cache_root, &size)?.join(format!("{}.png", hash));
    let mtime = file_mtime(path)?;
    save(&cache_path, thumb, &uri, mtime)
}

// ── Helpers ──

fn cache_dir_at(cache_root: &Path, size: &ThumbSize) -> Result<PathBuf, StorageError> {
    let dir = cache_root.join("thumbnails").join(size.subdir());
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn file_uri(path: &Path) -> String {
    format!("file://{}", path.display())
}

fn file_mtime(path: &Path) -> Result<u64, StorageError> {
    let modified = std::fs::metadata(path)?
        .modified()
        .map_err(|e| StorageError::Thumb(format!("mtime error: {e}")))?;
    modified
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| StorageError::Thumb(format!("mtime before epoch: {e}")))
}

/// Calculate the uniform scale factor to fit (w, h) inside a (max_w, max_h) box.
fn calculate_fit_scale(w: f64, h: f64, max_w: f64, max_h: f64) -> f64 {
    let scale_w = max_w / w;
    let scale_h = max_h / h;
    scale_w.min(scale_h).min(1.0)
}

// ── Cache I/O ──

fn load_if_valid(
    path: &Path,
    expected_uri: &str,
    expected_mtime: u64,
) -> Result<(u32, u32, Vec<u8>), StorageError> {
    let file = std::fs::File::open(path)?;
    let decoder = png::Decoder::new(file);
    let mut reader = decoder
        .read_info()
        .map_err(|e| StorageError::Thumb(format!("PNG decode error: {e}")))?;

    let info = reader.info();

    let uri_valid = info
        .uncompressed_latin1_text
        .iter()
        .any(|c| c.keyword == "Thumb::URI" && c.text == expected_uri);

    let mtime_valid = info
        .uncompressed_latin1_text
        .iter()
        .any(|c| c.keyword == "Thumb::MTime" && c.text.parse::<u64>().ok() == Some(expected_mtime));

    if !uri_valid || !mtime_valid {
        return Err(StorageError::Thumb(
            "Stale or invalid cache entry".to_string(),
        ));
    }

    let mut buf = vec![0u8; reader.output_buffer_size()];
    let frame = reader
        .next_frame(&mut buf)
        .map_err(|e| StorageError::Thumb(format!("PNG frame error: {e}")))?;

    // A cache entry without pixel data is corrupt; reject it so the
    // caller regenerates the thumbnail.
    if frame.width == 0 || frame.height == 0 || frame.buffer_size() == 0 {
        return Err(StorageError::Thumb("Empty thumbnail frame".to_string()));
    }

    Ok((
        frame.width,
        frame.height,
        buf[..frame.buffer_size()].to_vec(),
    ))
}

fn save(
    path: &Path,
    (w, h, rgba): &(u32, u32, Vec<u8>),
    uri: &str,
    mtime: u64,
) -> Result<(), StorageError> {
    let file = std::fs::File::create(path)?;
    let writer = BufWriter::new(file);

    let mut encoder = png::Encoder::new(writer, *w, *h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .add_text_chunk("Thumb::URI".to_string(), uri.to_string())
        .map_err(|e| StorageError::Thumb(format!("PNG text chunk error: {e}")))?;
    encoder
        .add_text_chunk("Thumb::MTime".to_string(), mtime.to_string())
        .map_err(|e| StorageError::Thumb(format!("PNG text chunk error: {e}")))?;

    let mut writer = encoder
        .write_header()
        .map_err(|e| StorageError::Thumb(format!("PNG header error: {e}")))?;
    writer
        .write_image_data(rgba)
        .map_err(|e| StorageError::Thumb(format!("PNG write error: {e}")))?;

    Ok(())
}

// ── Generation ──

fn generate(path: &Path, size: &ThumbSize) -> Result<(u32, u32, Vec<u8>), StorageError> {
    #[cfg(feature = "resvg")]
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    #[cfg(feature = "resvg")]
    if ext == "svg" {
        return generate_svg(path, size);
    }

    generate_raster(path, size)
}

fn generate_raster(path: &Path, size: &ThumbSize) -> Result<(u32, u32, Vec<u8>), StorageError> {
    use image::{GenericImageView, imageops::FilterType};

    let img =
        image::open(path).map_err(|e| StorageError::Thumb(format!("Failed to open image: {e}")))?;
    let (iw, ih) = img.dimensions();
    let max = size.max_px();
    let scale = calculate_fit_scale(iw as f64, ih as f64, max as f64, max as f64);
    let w = ((iw as f64) * scale) as u32;
    let h = ((ih as f64) * scale) as u32;
    // resize_exact, not resize: resize re-fits the aspect ratio and can
    // return different dimensions, which would not match the header the
    // PNG encoder writes — producing a cache entry without pixel data.
    let thumb = img.resize_exact(w, h, FilterType::Triangle);
    let data = thumb.to_rgba8().into_raw();
    Ok((w, h, data))
}

/// Render an SVG file to a thumbnail using resvg.
///
/// tiny_skia stores pixels in premultiplied RGBA; we convert to straight
/// alpha before storing so the consumer receives correct colour values.
#[cfg(feature = "resvg")]
fn generate_svg(path: &Path, size: &ThumbSize) -> Result<(u32, u32, Vec<u8>), StorageError> {
    use resvg::{tiny_skia, usvg};

    let data =
        std::fs::read(path).map_err(|e| StorageError::Thumb(format!("Failed to read SVG: {e}")))?;
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(&data, &options)
        .map_err(|e| StorageError::Thumb(format!("Failed to parse SVG: {e}")))?;

    let svg_size = tree.size();
    let max = size.max_px();
    let scale = calculate_fit_scale(
        svg_size.width() as f64,
        svg_size.height() as f64,
        max as f64,
        max as f64,
    ) as f32;

    let w = ((svg_size.width() * scale).ceil() as u32).max(1);
    let h = ((svg_size.height() * scale).ceil() as u32).max(1);

    let mut pixmap = tiny_skia::Pixmap::new(w, h)
        .ok_or_else(|| StorageError::Thumb("Failed to allocate pixmap".to_string()))?;

    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

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

    Ok((w, h, data))
}
