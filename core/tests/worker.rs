// SPDX-License-Identifier: GPL-3.0-or-later
// core/tests/worker.rs
//
// Integration tests for the pdfium worker.

mod common;

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
