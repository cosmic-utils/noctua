// SPDX-License-Identifier: GPL-3.0-only
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
fn detects_raster_qoi() {
    let dir = common::temp_dir("detect-qoi");
    let qoi = common::make_qoi(&dir, "img.qoi", 32, 32);

    let info = storage::document::load(&qoi).unwrap();
    assert!(matches!(info.kind, Kind::Raster(_)));
    assert_eq!(info.number_of_pages, 1);
    common::remove_dir(&dir);
}

#[test]
fn detects_raster_tga() {
    let dir = common::temp_dir("detect-tga");
    let tga = common::make_tga(&dir, "img.tga", 32, 32);

    let info = storage::document::load(&tga).unwrap();
    assert!(matches!(info.kind, Kind::Raster(_)));
    assert_eq!(info.number_of_pages, 1);
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

#[test]
fn renames_document_keeping_its_extension() {
    let dir = common::temp_dir("rename-keep-ext");
    let pdf = dir.join("old.pdf");
    std::fs::write(&pdf, b"%PDF-1.4\n").unwrap();

    let new_path = storage::document::rename_stem(&pdf, "new").unwrap();

    assert_eq!(new_path, dir.join("new.pdf"));
    assert!(!pdf.exists());
    assert!(new_path.exists());
    common::remove_dir(&dir);
}

#[test]
fn rename_drops_a_typed_extension() {
    let dir = common::temp_dir("rename-typed-ext");
    let pdf = dir.join("old.pdf");
    std::fs::write(&pdf, b"%PDF-1.4\n").unwrap();

    let new_path = storage::document::rename_stem(&pdf, "new.pdf").unwrap();

    assert_eq!(new_path, dir.join("new.pdf"));
    common::remove_dir(&dir);
}

#[test]
fn rename_refuses_to_replace_an_existing_file() {
    let dir = common::temp_dir("rename-existing");
    let source = dir.join("old.pdf");
    let taken = dir.join("taken.pdf");
    std::fs::write(&source, b"%PDF-1.4\nold").unwrap();
    std::fs::write(&taken, b"%PDF-1.4\nkept").unwrap();

    assert!(storage::document::rename_stem(&source, "taken").is_err());
    assert!(source.exists());
    assert_eq!(std::fs::read(&taken).unwrap(), b"%PDF-1.4\nkept");
    common::remove_dir(&dir);
}

#[test]
fn rename_rejects_a_name_with_a_path_separator() {
    let dir = common::temp_dir("rename-separator");
    let pdf = dir.join("old.pdf");
    std::fs::write(&pdf, b"%PDF-1.4\n").unwrap();

    assert!(storage::document::rename_stem(&pdf, "nested/new").is_err());
    assert!(pdf.exists());
    common::remove_dir(&dir);
}
