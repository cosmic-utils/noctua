// SPDX-License-Identifier: GPL-3.0-or-later
// core/tests/storage.rs
//
// Integration tests for document format detection and metadata.

use noctua_core_test_common as common;

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

#[test]
fn detects_javascript_in_pdf() {
    let dir = common::temp_dir("js-detect");
    let pdf = dir.join("scripted.pdf");
    std::fs::write(
        &pdf,
        b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /OpenAction << /S /JavaScript /JS (app.alert(1)) >> >>\nendobj\n",
    )
    .unwrap();

    assert!(storage::portable::contains_javascript(&pdf).unwrap());
    common::remove_dir(&dir);
}

#[test]
fn pdf_without_javascript_is_clean() {
    let dir = common::temp_dir("js-clean");
    let pdf = dir.join("plain.pdf");
    std::fs::write(&pdf, b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\n").unwrap();

    assert!(!storage::portable::contains_javascript(&pdf).unwrap());
    common::remove_dir(&dir);
}

#[test]
fn formats_file_sizes() {
    assert_eq!(storage::document::format_size(500), "500 B");
    assert_eq!(storage::document::format_size(2048), "2.0 KB");
    assert_eq!(storage::document::format_size(3 * 1024 * 1024), "3.0 MB");
}
