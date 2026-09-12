// SPDX-License-Identifier: GPL-3.0-or-later
// core/tests/browser.rs
//
// Integration tests for browser mode folder listing.

mod common;

use noctua_core::storage;

#[test]
fn lists_supported_documents_sorted() {
    let dir = common::temp_dir("browser-sorted");
    common::make_png(&dir, "b.png", 32, 32);
    common::make_png(&dir, "A.png", 32, 32);
    common::make_svg(&dir, "c.svg");

    let entries = storage::browser::list_documents(&dir).unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    // Case-insensitive sort: A.png, b.png, c.svg
    assert_eq!(names, vec!["A.png", "b.png", "c.svg"]);
    common::remove_dir(&dir);
}

#[test]
fn skips_unsupported_and_hidden_files() {
    let dir = common::temp_dir("browser-filter");
    common::make_png(&dir, "ok.png", 32, 32);
    std::fs::write(dir.join("junk.txt"), b"not a document").unwrap();
    std::fs::write(dir.join(".hidden.png"), b"").unwrap();
    std::fs::create_dir(dir.join("subdir")).unwrap();

    let entries = storage::browser::list_documents(&dir).unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["ok.png"]);
    common::remove_dir(&dir);
}

#[test]
fn lists_pdfs_when_pdfium_is_available() {
    let Some(pdfium) = noctua_core::pdfium_ops::try_pdfium() else {
        eprintln!("SKIP: libpdfium.so not available");
        return;
    };
    let _guard = common::pdfium_lock();
    let dir = common::temp_dir("browser-pdf");
    common::make_pdf(pdfium, &dir, "doc.pdf", 1);
    common::make_png(&dir, "img.png", 32, 32);

    let entries = storage::browser::list_documents(&dir).unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["doc.pdf", "img.png"]);
    common::remove_dir(&dir);
}

#[test]
fn missing_directory_is_an_error() {
    let dir = common::temp_dir("browser-missing");
    let missing = dir.join("does-not-exist");
    assert!(storage::browser::list_documents(&missing).is_err());
    common::remove_dir(&dir);
}

#[test]
fn display_name_uses_last_component() {
    let dir = common::temp_dir("browser-name");
    let file = dir.join("photo.jpg");
    std::fs::write(&file, b"x").unwrap();

    assert_eq!(storage::browser::display_name(&file), "photo.jpg");
    assert_eq!(
        storage::browser::display_name(&dir),
        dir.file_name().unwrap().to_string_lossy()
    );
    common::remove_dir(&dir);
}

#[test]
fn detects_pdf_by_magic_bytes() {
    let dir = common::temp_dir("browser-ispdf");
    // A fake PDF header is enough: detection is content-based, not extension-based.
    std::fs::write(dir.join("fake.pdf"), b"%PDF-1.7 fake content").unwrap();
    std::fs::write(dir.join("doc.txt"), b"%PDF-1.7").unwrap();
    common::make_png(&dir, "img.png", 32, 32);

    assert!(storage::document::is_pdf(&dir.join("fake.pdf")).unwrap());
    // The .txt file also carries the PDF magic bytes — detection trusts content.
    assert!(storage::document::is_pdf(&dir.join("doc.txt")).unwrap());
    assert!(!storage::document::is_pdf(&dir.join("img.png")).unwrap());
    common::remove_dir(&dir);
}
