// SPDX-License-Identifier: GPL-3.0-or-later
// core/tests/common/src/lib.rs
//
// Shared test helpers: temp directories and generated test documents.
// A small library so each test binary links only what it uses; public
// items of a library are never flagged as dead code, which keeps the
// test suite free of `allow(dead_code)`.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

/// Serialize pdfium access across tests.
///
/// pdfium is not thread-safe and cargo runs tests on parallel threads.
/// The production design uses a single worker thread, so tests must
/// serialize pdfium access the same way.
static PDFIUM_TEST_LOCK: Mutex<()> = Mutex::new(());

/// Acquire the pdfium test lock. Hold the guard for the whole test.
pub fn pdfium_lock() -> MutexGuard<'static, ()> {
    PDFIUM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Create a unique temp directory under the system temp dir.
pub fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "noctua-test-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

/// Remove a temp directory and everything in it.
pub fn remove_dir(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

/// Generate a simple PNG image file with the given pixel size.
pub fn make_png(dir: &Path, name: &str, width: u32, height: u32) -> PathBuf {
    let path = dir.join(name);
    let mut img = image::RgbaImage::new(width, height);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        *pixel = image::Rgba([(x % 256) as u8, (y % 256) as u8, 128, 255]);
    }
    img.save(&path).expect("save png");
    path
}

/// Generate a simple SVG file.
pub fn make_svg(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(
        &path,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100">
  <rect x="10" y="10" width="180" height="80" fill="#3366cc"/>
</svg>"##,
    )
    .expect("write svg");
    path
}

/// Generate a small PDF with the given number of pages via pdfium.
/// Each page contains one text object, so pages are non-empty.
pub fn make_pdf(
    pdfium: &pdfium_render::prelude::Pdfium,
    dir: &Path,
    name: &str,
    pages: u32,
) -> PathBuf {
    use pdfium_render::prelude::*;

    let path = dir.join(name);
    let mut document = pdfium.create_new_pdf().expect("create pdf");

    let font = document.fonts_mut().helvetica();

    for i in 0..pages {
        let mut page = document
            .pages_mut()
            .create_page_at_end(PdfPagePaperSize::a4())
            .expect("create page");
        page.objects_mut()
            .create_text_object(
                PdfPoints::new(40.0),
                PdfPoints::new(700.0),
                format!("page {}", i + 1),
                font,
                PdfPoints::new(14.0),
            )
            .expect("create text object");
    }

    document.save_to_file(&path).expect("save pdf");
    path
}
