// SPDX-License-Identifier: GPL-3.0-or-later
// core/tests/pdfium_ops.rs
//
// Integration tests for PDF operations: bind, insert, delete, move,
// rotate, annotations, save, reload.

use noctua_core_test_common as common;

use noctua_core::pdfium_ops::model::palette;
use noctua_core::pdfium_ops::{BindSource, Command, CommandResult, PdfOpsManager};
use std::path::PathBuf;

fn pdfium() -> Option<&'static pdfium_render::prelude::Pdfium> {
    noctua_core::pdfium_ops::try_pdfium()
}

/// pdfium is not thread-safe; cargo runs tests on parallel threads.
/// Every pdfium test must hold this guard for its whole duration.
fn pdfium_guard() -> std::sync::MutexGuard<'static, ()> {
    common::pdfium_lock()
}

fn sources(paths: &[PathBuf]) -> Vec<BindSource> {
    paths
        .iter()
        .map(|p| BindSource {
            path: p.clone(),
            pages: None,
        })
        .collect()
}

#[test]
fn bind_two_pdfs_concatenates_all_pages() {
    let Some(pdfium) = pdfium() else {
        eprintln!("SKIP: libpdfium.so not available");
        return;
    };
    let _guard = pdfium_guard();
    let dir = common::temp_dir("bind-two");
    let a = common::make_pdf(pdfium, &dir, "a.pdf", 2);
    let b = common::make_pdf(pdfium, &dir, "b.pdf", 3);
    let target = dir.join("out.pdf");

    let mut mgr = PdfOpsManager::new();
    let result = mgr.execute(Command::Bind {
        sources: sources(&[a, b]),
        target: target.clone(),
    });
    assert!(matches!(result, CommandResult::Ok), "{result:?}");
    assert!(matches!(
        mgr.execute(Command::PageCount),
        CommandResult::PageCount(5)
    ));
    common::remove_dir(&dir);
}

#[test]
fn bind_pdf_plus_raster_plus_svg() {
    let Some(pdfium) = pdfium() else {
        eprintln!("SKIP: libpdfium.so not available");
        return;
    };
    let _guard = pdfium_guard();
    let dir = common::temp_dir("bind-mixed");
    let a = common::make_pdf(pdfium, &dir, "a.pdf", 1);
    let png = common::make_png(&dir, "img.png", 64, 48);
    let svg = common::make_svg(&dir, "vec.svg");
    let target = dir.join("out.pdf");

    let mut mgr = PdfOpsManager::new();
    let result = mgr.execute(Command::Bind {
        sources: sources(&[a, png, svg]),
        target,
    });
    assert!(matches!(result, CommandResult::Ok), "{result:?}");
    // 1 PDF page + 1 raster page + 1 svg page
    assert!(matches!(
        mgr.execute(Command::PageCount),
        CommandResult::PageCount(3)
    ));
    common::remove_dir(&dir);
}

#[test]
fn bind_rejects_unknown_source() {
    let dir = common::temp_dir("bind-unknown");
    let junk = dir.join("junk.bin");
    std::fs::write(&junk, b"not a document").unwrap();
    let target = dir.join("out.pdf");

    let mut mgr = PdfOpsManager::new();
    let result = mgr.execute(Command::Bind {
        sources: sources(&[junk]),
        target,
    });
    assert!(matches!(result, CommandResult::Error(_)), "{result:?}");
    common::remove_dir(&dir);
}

#[test]
fn insert_pages_at_position() {
    let Some(pdfium) = pdfium() else {
        eprintln!("SKIP: libpdfium.so not available");
        return;
    };
    let _guard = pdfium_guard();
    let dir = common::temp_dir("insert");
    let base = common::make_pdf(pdfium, &dir, "base.pdf", 3);
    let extra = common::make_pdf(pdfium, &dir, "extra.pdf", 2);

    let mut mgr = PdfOpsManager::new();
    assert!(matches!(
        mgr.execute(Command::Open { path: base }),
        CommandResult::Ok
    ));
    // Insert the two extra pages before page 2 → 5 pages total.
    assert!(matches!(
        mgr.execute(Command::InsertPages {
            source: BindSource {
                path: extra,
                pages: None
            },
            at: 2,
        }),
        CommandResult::Ok
    ));
    assert!(matches!(
        mgr.execute(Command::PageCount),
        CommandResult::PageCount(5)
    ));
    common::remove_dir(&dir);
}

#[test]
fn delete_pages() {
    let Some(pdfium) = pdfium() else {
        eprintln!("SKIP: libpdfium.so not available");
        return;
    };
    let _guard = pdfium_guard();
    let dir = common::temp_dir("delete");
    let doc = common::make_pdf(pdfium, &dir, "doc.pdf", 5);

    let mut mgr = PdfOpsManager::new();
    assert!(matches!(
        mgr.execute(Command::Open { path: doc }),
        CommandResult::Ok
    ));
    assert!(matches!(
        mgr.execute(Command::DeletePages { pages: vec![2, 4] }),
        CommandResult::Ok
    ));
    assert!(matches!(
        mgr.execute(Command::PageCount),
        CommandResult::PageCount(3)
    ));
    common::remove_dir(&dir);
}

#[test]
fn delete_out_of_range_fails() {
    let Some(pdfium) = pdfium() else {
        eprintln!("SKIP: libpdfium.so not available");
        return;
    };
    let _guard = pdfium_guard();
    let dir = common::temp_dir("delete-oob");
    let doc = common::make_pdf(pdfium, &dir, "doc.pdf", 2);

    let mut mgr = PdfOpsManager::new();
    assert!(matches!(
        mgr.execute(Command::Open { path: doc }),
        CommandResult::Ok
    ));
    assert!(matches!(
        mgr.execute(Command::DeletePages { pages: vec![9] }),
        CommandResult::Error(_)
    ));
    common::remove_dir(&dir);
}

#[test]
fn move_page_forward() {
    let Some(pdfium) = pdfium() else {
        eprintln!("SKIP: libpdfium.so not available");
        return;
    };
    let _guard = pdfium_guard();
    let dir = common::temp_dir("move-fwd");
    let doc = common::make_pdf(pdfium, &dir, "doc.pdf", 4);

    let mut mgr = PdfOpsManager::new();
    assert!(matches!(
        mgr.execute(Command::Open { path: doc }),
        CommandResult::Ok
    ));
    // Move page 1 to position 4.
    assert!(matches!(
        mgr.execute(Command::MovePage { from: 1, to: 4 }),
        CommandResult::Ok
    ));
    assert!(matches!(
        mgr.execute(Command::PageCount),
        CommandResult::PageCount(4)
    ));
    common::remove_dir(&dir);
}

#[test]
fn move_page_backward() {
    let Some(pdfium) = pdfium() else {
        eprintln!("SKIP: libpdfium.so not available");
        return;
    };
    let _guard = pdfium_guard();
    let dir = common::temp_dir("move-bwd");
    let doc = common::make_pdf(pdfium, &dir, "doc.pdf", 4);

    let mut mgr = PdfOpsManager::new();
    assert!(matches!(
        mgr.execute(Command::Open { path: doc }),
        CommandResult::Ok
    ));
    // Move page 4 to position 1.
    assert!(matches!(
        mgr.execute(Command::MovePage { from: 4, to: 1 }),
        CommandResult::Ok
    ));
    assert!(matches!(
        mgr.execute(Command::PageCount),
        CommandResult::PageCount(4)
    ));
    common::remove_dir(&dir);
}

#[test]
fn rotate_page() {
    let Some(pdfium) = pdfium() else {
        eprintln!("SKIP: libpdfium.so not available");
        return;
    };
    let _guard = pdfium_guard();
    let dir = common::temp_dir("rotate");
    let doc = common::make_pdf(pdfium, &dir, "doc.pdf", 1);

    let mut mgr = PdfOpsManager::new();
    assert!(matches!(
        mgr.execute(Command::Open { path: doc }),
        CommandResult::Ok
    ));
    assert!(matches!(
        mgr.execute(Command::RotatePage {
            page: 1,
            degrees: 90
        }),
        CommandResult::Ok
    ));
    assert!(matches!(
        mgr.execute(Command::RotatePage {
            page: 1,
            degrees: 45
        }),
        CommandResult::Error(_)
    ));
    common::remove_dir(&dir);
}

#[test]
fn add_annotations_and_save() {
    let Some(pdfium) = pdfium() else {
        eprintln!("SKIP: libpdfium.so not available");
        return;
    };
    let _guard = pdfium_guard();
    let dir = common::temp_dir("annotate");
    let doc = common::make_pdf(pdfium, &dir, "doc.pdf", 2);
    let saved = dir.join("saved.pdf");

    let mut mgr = PdfOpsManager::new();
    assert!(matches!(
        mgr.execute(Command::Open { path: doc }),
        CommandResult::Ok
    ));
    assert!(matches!(
        mgr.execute(Command::AddTextAnnotation {
            page: 1,
            text: "test note".to_string(),
            x: 40.0,
            y: 40.0,
            width: 60.0,
            height: 30.0,
        }),
        CommandResult::Ok
    ));
    assert!(matches!(
        mgr.execute(Command::AddInkAnnotation {
            page: 1,
            color: palette::RED,
            points: vec![(50.0, 50.0), (80.0, 70.0), (110.0, 55.0)],
            x: 40.0,
            y: 40.0,
            width: 100.0,
            height: 60.0,
        }),
        CommandResult::Ok
    ));
    assert!(matches!(
        mgr.execute(Command::AddHighlightAnnotation {
            page: 2,
            color: palette::YELLOW,
            x: 200.0,
            y: 300.0,
            width: 120.0,
            height: 20.0,
        }),
        CommandResult::Ok
    ));
    assert!(mgr.dirty());
    assert!(matches!(
        mgr.execute(Command::SaveAs { path: saved }),
        CommandResult::Ok
    ));
    assert!(!mgr.dirty());
    common::remove_dir(&dir);
}

#[test]
fn save_keeps_page_count_after_reload() {
    let Some(pdfium) = pdfium() else {
        eprintln!("SKIP: libpdfium.so not available");
        return;
    };
    let _guard = pdfium_guard();
    let dir = common::temp_dir("save-reload");
    let doc = common::make_pdf(pdfium, &dir, "doc.pdf", 3);
    let saved = dir.join("saved.pdf");

    let mut mgr = PdfOpsManager::new();
    assert!(matches!(
        mgr.execute(Command::Open { path: doc }),
        CommandResult::Ok
    ));
    assert!(matches!(
        mgr.execute(Command::SaveAs {
            path: saved.clone()
        }),
        CommandResult::Ok
    ));

    // Reload from disk and check the page count persisted.
    let mut reloaded = PdfOpsManager::new();
    assert!(matches!(
        reloaded.execute(Command::Open { path: saved }),
        CommandResult::Ok
    ));
    assert!(matches!(
        reloaded.execute(Command::PageCount),
        CommandResult::PageCount(3)
    ));
    common::remove_dir(&dir);
}

#[test]
fn render_page_returns_pixels() {
    let Some(pdfium) = pdfium() else {
        eprintln!("SKIP: libpdfium.so not available");
        return;
    };
    let _guard = pdfium_guard();
    let dir = common::temp_dir("render");
    let doc = common::make_pdf(pdfium, &dir, "doc.pdf", 1);

    let mut mgr = PdfOpsManager::new();
    assert!(matches!(
        mgr.execute(Command::Open { path: doc }),
        CommandResult::Ok
    ));
    match mgr.execute(Command::RenderPage { page: 1, zoom: 1.0 }) {
        CommandResult::Rendered {
            width,
            height,
            rgba_data,
        } => {
            assert!(width > 0 && height > 0);
            assert_eq!(rgba_data.len(), (width * height * 4) as usize);
        }
        other => panic!("expected Rendered, got {other:?}"),
    }
    common::remove_dir(&dir);
}

#[test]
fn operations_without_open_document_fail() {
    let mut mgr = PdfOpsManager::new();
    assert!(matches!(
        mgr.execute(Command::PageCount),
        CommandResult::Error(_)
    ));
    assert!(matches!(
        mgr.execute(Command::Save),
        CommandResult::Error(_)
    ));
}
