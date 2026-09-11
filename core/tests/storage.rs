// SPDX-License-Identifier: GPL-3.0-or-later
// core/tests/storage.rs
//
// Integration tests for document format detection and metadata.

mod common;

use noctua_core::document::Kind;
use noctua_core::storage;

#[test]
fn detects_raster_png() {
    let dir = common::temp_dir("detect-png");
    let png = common::make_png(&dir, "img.png", 64, 48);

    let info = storage::document::load(&png).unwrap();
    assert!(matches!(info.kind, Kind::Raster(_)));
    assert_eq!(info.number_of_pages, 1);
    assert!(info.file_size_bytes > 0);
    common::remove_dir(&dir);
}

#[test]
fn detects_vector_svg() {
    let dir = common::temp_dir("detect-svg");
    let svg = common::make_svg(&dir, "vec.svg");

    let info = storage::document::load(&svg).unwrap();
    assert!(matches!(info.kind, Kind::Vector(_)));
    common::remove_dir(&dir);
}

#[test]
fn detects_portable_pdf() {
    let Some(pdfium) = noctua_core::pdfium_ops::try_pdfium() else {
        eprintln!("SKIP: libpdfium.so not available");
        return;
    };
    // pdfium is not thread-safe; serialize with the other pdfium tests.
    let _guard = common::pdfium_lock();
    let dir = common::temp_dir("detect-pdf");
    let pdf = common::make_pdf(pdfium, &dir, "doc.pdf", 2);

    let info = storage::document::load(&pdf).unwrap();
    assert!(matches!(info.kind, Kind::Portable(_)));
    assert_eq!(info.number_of_pages, 2);
    common::remove_dir(&dir);
}

#[test]
fn unknown_format_is_unknown() {
    let dir = common::temp_dir("detect-unknown");
    let junk = dir.join("file.bin");
    std::fs::write(&junk, b"random bytes").unwrap();

    let info = storage::document::load(&junk).unwrap();
    assert!(matches!(info.kind, Kind::Unknown));
    common::remove_dir(&dir);
}
