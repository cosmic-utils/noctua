// SPDX-License-Identifier: GPL-3.0-or-later
// core/tests/worker.rs
//
// Integration tests for the pdfium worker.

use noctua_core_test_common as common;

use noctua_core::render::worker::{Job, JobResult, Priority, SharedWorker, Worker};
use noctua_core::storage::thumbcache::ThumbSize;

#[test]
fn renders_page_thumbs_in_one_open() {
    let Some(pdfium) = noctua_core::pdfium_ops::try_pdfium() else {
        eprintln!("SKIP: libpdfium.so not available");
        return;
    };
    let _guard = common::pdfium_lock();
    let dir = common::temp_dir("worker-thumbs");
    let pdf = common::make_pdf(pdfium, &dir, "doc.pdf", 3);

    let worker = Worker::spawn();
    let result = worker.execute(
        Priority::Low,
        Job::RenderPageThumbs {
            path: pdf,
            pages: vec![1, 3],
            zoom: 0.2,
        },
    );
    worker.shutdown();

    match result {
        JobResult::RenderedThumbs(thumbs) => {
            let pages: Vec<u32> = thumbs.iter().map(|(page, ..)| *page).collect();
            assert_eq!(pages, vec![1, 3]);
        }
        other => panic!("expected RenderedThumbs, got {other:?}"),
    }
    common::remove_dir(&dir);
}

#[test]
fn reports_page_sizes_and_renders_with_priority() {
    let Some(pdfium) = noctua_core::pdfium_ops::try_pdfium() else {
        eprintln!("SKIP: libpdfium.so not available");
        return;
    };
    let _guard = common::pdfium_lock();
    let dir = common::temp_dir("worker-sizes");
    let pdf = common::make_pdf(pdfium, &dir, "doc.pdf", 2);

    let worker = SharedWorker::spawn();

    let sizes = worker.page_sizes(&pdf).expect("page sizes");
    assert_eq!(sizes.len(), 2);
    // A4 pages: 595.44 x 841.68 points (with tolerance for pdfium rounding).
    assert!((sizes[0].0 - 595.0).abs() < 2.0);
    assert!((sizes[0].1 - 841.0).abs() < 2.0);

    // Batch rendering accepts an explicit priority for the visible window.
    let pages = worker
        .render_pages(&pdf, &[1, 2], 0.5, Priority::VisiblePage)
        .expect("rendered pages");
    let numbers: Vec<u32> = pages.iter().map(|(page, ..)| *page).collect();
    assert_eq!(numbers, vec![1, 2]);

    common::remove_dir(&dir);
}

#[test]
fn thumbnail_caches_raster_files() {
    let dir = common::temp_dir("worker-thumb-raster");
    let png = common::make_png(&dir, "img.png", 64, 48);

    let worker = SharedWorker::spawn();
    let thumb = worker
        .thumbnail(&png, ThumbSize::Normal)
        .expect("thumbnail");
    assert_eq!((thumb.0, thumb.1), (64, 48));
    // The second call is served from the freedesktop cache.
    let cached = worker.thumbnail(&png, ThumbSize::Normal).expect("cached");
    assert_eq!(cached, thumb);
    common::remove_dir(&dir);
}

#[test]
fn thumbnail_renders_pdf_first_page() {
    let Some(pdfium) = noctua_core::pdfium_ops::try_pdfium() else {
        eprintln!("SKIP: libpdfium.so not available");
        return;
    };
    let _guard = common::pdfium_lock();
    let dir = common::temp_dir("worker-thumb-pdf");
    let pdf = common::make_pdf(pdfium, &dir, "doc.pdf", 2);

    let worker = SharedWorker::spawn();
    let thumb = worker
        .thumbnail(&pdf, ThumbSize::Normal)
        .expect("pdf thumbnail");
    assert!(thumb.0 > 0 && thumb.1 > 0);
    common::remove_dir(&dir);
}
