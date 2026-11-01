// SPDX-License-Identifier: GPL-3.0-only
// core/tests/pdfium_ops.rs
//
// Integration tests for PDF operations: bind, insert, delete, move,
// rotate, annotations, save, reload.

use noctua_core_test_common as common;

use noctua_core::pdfium_ops::model::AnnotationColor;
use noctua_core::pdfium_ops::model::MarkupKind;
use noctua_core::pdfium_ops::model::palette;
use noctua_core::pdfium_ops::{BindSource, Command, CommandResult, PdfOpsManager};
use std::path::{Path, PathBuf};

fn pdfium() -> Option<&'static pdfium_render::prelude::Pdfium> {
    noctua_core::pdfium_ops::try_pdfium()
}

/// pdfium is not thread-safe; cargo runs tests on parallel threads.
/// Every pdfium test must hold this guard for its whole duration.
fn pdfium_guard() -> std::sync::MutexGuard<'static, ()> {
    common::pdfium_lock()
}

/// Read a file's bytes via the storage layer.
fn read_bytes(path: &Path) -> Vec<u8> {
    noctua_core::storage::document::open(path).expect("read file")
}

/// Build a bind source from a path: read the bytes and detect the format.
fn bind_source(path: &Path) -> BindSource {
    BindSource {
        name: path.file_name().unwrap().to_string_lossy().to_string(),
        data: read_bytes(path),
        format: noctua_core::storage::document::format(path).expect("detect format"),
        pages: None,
    }
}

fn sources(paths: &[PathBuf]) -> Vec<BindSource> {
    paths.iter().map(|p| bind_source(p)).collect()
}

/// Read a PDF and build an `Open` command.
fn open_command(path: &Path) -> Command {
    Command::Open {
        data: read_bytes(path),
        path: path.to_path_buf(),
    }
}

/// Execute `SaveAs` and write the serialized bytes to disk.
fn save_as(mgr: &mut PdfOpsManager, path: &Path) {
    match mgr.execute(Command::SaveAs {
        path: path.to_path_buf(),
    }) {
        CommandResult::Saved { data } => std::fs::write(path, data).expect("write pdf"),
        other => panic!("expected Saved, got {other:?}"),
    }
}

/// Read the text of every page of a PDF in page order, trimmed. Used to
/// assert that page-reordering commands produce the expected order.
fn page_texts(pdfium: &pdfium_render::prelude::Pdfium, path: &Path) -> Vec<String> {
    pdfium
        .load_pdf_from_file(path, None)
        .expect("load pdf")
        .pages()
        .iter()
        .map(|page| page.text().map(|text| text.all()).unwrap_or_default())
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
    assert!(matches!(result, CommandResult::Saved { .. }), "{result:?}");
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
    assert!(matches!(result, CommandResult::Saved { .. }), "{result:?}");
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
        mgr.execute(open_command(&base)),
        CommandResult::Ok
    ));
    // Insert the two extra pages before page 2 → 5 pages total.
    assert!(matches!(
        mgr.execute(Command::InsertPages {
            source: bind_source(&extra),
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
    assert!(matches!(mgr.execute(open_command(&doc)), CommandResult::Ok));
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
    assert!(matches!(mgr.execute(open_command(&doc)), CommandResult::Ok));
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
    let saved = dir.join("moved.pdf");

    let mut mgr = PdfOpsManager::new();
    assert!(matches!(mgr.execute(open_command(&doc)), CommandResult::Ok));
    // Move page 1 to position 4.
    assert!(matches!(
        mgr.execute(Command::MovePage { from: 1, to: 4 }),
        CommandResult::Ok
    ));
    assert!(matches!(
        mgr.execute(Command::PageCount),
        CommandResult::PageCount(4)
    ));
    // Persist the result and verify the page order, not just the count.
    save_as(&mut mgr, &saved);
    assert_eq!(
        page_texts(pdfium, &saved),
        ["page 2", "page 3", "page 4", "page 1"]
    );
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
    let saved = dir.join("moved.pdf");

    let mut mgr = PdfOpsManager::new();
    assert!(matches!(mgr.execute(open_command(&doc)), CommandResult::Ok));
    // Move page 4 to position 1.
    assert!(matches!(
        mgr.execute(Command::MovePage { from: 4, to: 1 }),
        CommandResult::Ok
    ));
    assert!(matches!(
        mgr.execute(Command::PageCount),
        CommandResult::PageCount(4)
    ));
    // Persist the result and verify the page order, not just the count.
    save_as(&mut mgr, &saved);
    assert_eq!(
        page_texts(pdfium, &saved),
        ["page 4", "page 1", "page 2", "page 3"]
    );
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
    assert!(matches!(mgr.execute(open_command(&doc)), CommandResult::Ok));
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
fn rotate_page_relative_cycles_orientation() {
    let Some(pdfium) = pdfium() else {
        eprintln!("SKIP: libpdfium.so not available");
        return;
    };
    let _guard = pdfium_guard();
    let dir = common::temp_dir("rotate-relative");
    let doc = common::make_pdf(pdfium, &dir, "doc.pdf", 1);

    let mut mgr = PdfOpsManager::new();
    assert!(matches!(mgr.execute(open_command(&doc)), CommandResult::Ok));

    let sizes = |mgr: &mut PdfOpsManager| match mgr.execute(Command::PageSizes) {
        CommandResult::PageSizes(sizes) => sizes[0],
        other => panic!("expected PageSizes, got {other:?}"),
    };

    // A4 portrait starts wider than it is tall.
    let (w, h) = sizes(&mut mgr);
    assert!(h > w);

    assert!(matches!(
        mgr.execute(Command::RotatePageRelative {
            page: 1,
            clockwise: true
        }),
        CommandResult::Ok
    ));
    let (w, h) = sizes(&mut mgr);
    assert!(w > h, "clockwise quarter-turn should swap orientation");

    assert!(matches!(
        mgr.execute(Command::RotatePageRelative {
            page: 1,
            clockwise: false
        }),
        CommandResult::Ok
    ));
    let (w, h) = sizes(&mut mgr);
    assert!(
        h > w,
        "counter-clockwise quarter-turn should restore orientation"
    );

    assert!(matches!(
        mgr.execute(Command::RotatePageRelative {
            page: 0,
            clockwise: true
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
    assert!(matches!(mgr.execute(open_command(&doc)), CommandResult::Ok));
    assert!(matches!(
        mgr.execute(Command::AddFreeTextAnnotation {
            page: 1,
            text: "test note".to_string(),
            font_size: 16.0,
            color: palette::RED,
            position: (40.0, 70.0),
            max_width: 160.0,
        }),
        CommandResult::FreeTextWritten { .. }
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
        mgr.execute(Command::AddMarkupAnnotation {
            page: 2,
            kind: MarkupKind::Highlight,
            color: palette::YELLOW,
            x: 200.0,
            y: 300.0,
            width: 120.0,
            height: 20.0,
        }),
        CommandResult::Ok
    ));
    assert!(matches!(
        mgr.execute(Command::AddMarkupAnnotation {
            page: 2,
            kind: MarkupKind::Underline,
            color: palette::BLUE,
            x: 200.0,
            y: 280.0,
            width: 120.0,
            height: 20.0,
        }),
        CommandResult::Ok
    ));
    assert!(mgr.dirty());
    save_as(&mut mgr, &saved);
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
    assert!(matches!(mgr.execute(open_command(&doc)), CommandResult::Ok));
    save_as(&mut mgr, &saved);

    // Reload from disk and check the page count persisted.
    let mut reloaded = PdfOpsManager::new();
    assert!(matches!(
        reloaded.execute(open_command(&saved)),
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
    assert!(matches!(mgr.execute(open_command(&doc)), CommandResult::Ok));
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
fn render_thumbnail_fits_size_box() {
    let Some(pdfium) = pdfium() else {
        eprintln!("SKIP: libpdfium.so not available");
        return;
    };
    let _guard = pdfium_guard();
    let dir = common::temp_dir("render-thumb");
    let doc = common::make_pdf(pdfium, &dir, "doc.pdf", 1);

    let mut mgr = PdfOpsManager::new();
    assert!(matches!(mgr.execute(open_command(&doc)), CommandResult::Ok));
    match mgr.execute(Command::RenderThumbnail {
        page: 1,
        max_px: 128,
    }) {
        CommandResult::Rendered {
            width,
            height,
            rgba_data,
        } => {
            assert!(width > 0 && width <= 128, "width {width}");
            assert!(height > 0 && height <= 128, "height {height}");
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

/// A free text annotation wraps inside its width, grows downward from the
/// click and is drawn without a frame.
#[test]
fn free_text_wraps_without_a_frame() {
    let Some(pdfium) = pdfium() else {
        eprintln!("SKIP: libpdfium.so not available");
        return;
    };
    let _guard = pdfium_guard();
    let dir = common::temp_dir("free-text");
    let doc = common::make_pdf(pdfium, &dir, "doc.pdf", 1);

    let mut mgr = PdfOpsManager::new();
    assert!(matches!(mgr.execute(open_command(&doc)), CommandResult::Ok));

    let index = match mgr.execute(Command::AddFreeTextAnnotation {
        page: 1,
        text: "Lorem ipsum dolor sit amet, consectetur adipiscing elit sed do eiusmod".to_string(),
        font_size: 12.0,
        color: AnnotationColor::rgb(255, 0, 0),
        position: (60.0, 700.0),
        max_width: 200.0,
    }) {
        CommandResult::FreeTextWritten { index, .. } => index.expect("created index"),
        other => panic!("expected FreeTextWritten, got {other:?}"),
    };
    assert_eq!(index, 0);

    let annotations = match mgr.execute(Command::ListFreeTextAnnotations { page: 1 }) {
        CommandResult::FreeTextAnnotations(list) => list,
        other => panic!("expected FreeTextAnnotations, got {other:?}"),
    };
    assert_eq!(annotations.len(), 1);
    let annotation = &annotations[0];
    assert!(
        annotation.width > 100.0 && annotation.width <= 200.0,
        "width {}",
        annotation.width
    );
    // More than one line, and the box hangs below the click at its top edge.
    assert!(
        annotation.height > 12.0 * 1.2,
        "height {}",
        annotation.height
    );
    let top = annotation.y + annotation.height;
    assert!((top - 700.0).abs() < 0.5, "top {top}");

    let (width, height, rgba) = match mgr.execute(Command::RenderPage { page: 1, zoom: 1.0 }) {
        CommandResult::Rendered {
            width,
            height,
            rgba_data,
        } => (width, height, rgba_data),
        other => panic!("expected Rendered, got {other:?}"),
    };

    // A frame would paint the box's top edge across its whole width; the text
    // itself starts one padding and one ascent below it.
    let top_row = height - 700;
    let mut edge_pixels = 0;
    let mut text_pixels = 0;
    for y in 0..40_u32 {
        for x in 60..(60 + annotation.width as u32) {
            let i = (((top_row + y) * width + x) * 4) as usize;
            if rgba[i] < 240 || rgba[i + 1] < 240 || rgba[i + 2] < 240 {
                if y <= 2 {
                    edge_pixels += 1;
                } else {
                    text_pixels += 1;
                }
            }
        }
    }
    assert!(
        edge_pixels * 4 < 2 * annotation.width as u32,
        "the top edge must stay empty, got {edge_pixels} pixels"
    );
    assert!(text_pixels > 0, "the text must be rendered");

    common::remove_dir(&dir);
}

/// Re-writing an annotation with its own box width as wrap width must not
/// re-flow the text: clicking an existing annotation starts an edit, and every
/// click used to re-wrap the paragraph.
#[test]
fn free_text_edit_keeps_its_layout() {
    let Some(pdfium) = pdfium() else {
        eprintln!("SKIP: libpdfium.so not available");
        return;
    };
    let _guard = pdfium_guard();
    let dir = common::temp_dir("free-text-edit");
    let doc = common::make_pdf(pdfium, &dir, "doc.pdf", 1);

    let mut mgr = PdfOpsManager::new();
    assert!(matches!(mgr.execute(open_command(&doc)), CommandResult::Ok));

    let text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit sed do eiusmod";
    assert!(matches!(
        mgr.execute(Command::AddFreeTextAnnotation {
            page: 1,
            text: text.to_string(),
            font_size: 12.0,
            color: AnnotationColor::rgb(0, 0, 0),
            position: (60.0, 700.0),
            max_width: 200.0,
        }),
        CommandResult::FreeTextWritten { .. }
    ));

    let read =
        |mgr: &mut PdfOpsManager| match mgr.execute(Command::ListFreeTextAnnotations { page: 1 }) {
            CommandResult::FreeTextAnnotations(mut list) => list.remove(0),
            other => panic!("expected FreeTextAnnotations, got {other:?}"),
        };

    let before = read(&mut mgr);
    for _ in 0..3 {
        let current = read(&mut mgr);
        assert!(matches!(
            mgr.execute(Command::UpdateFreeText {
                page: 1,
                index: current.index,
                text: text.to_string(),
                font_size: 12.0,
                color: AnnotationColor::rgb(0, 0, 0),
                position: (current.x, current.y + current.height),
                max_width: current.width,
                cursor: text.chars().count(),
            }),
            CommandResult::FreeTextWritten { .. }
        ));
    }
    let after = read(&mut mgr);

    assert!(
        (after.width - before.width).abs() < 0.5,
        "width {} -> {}",
        before.width,
        after.width
    );
    assert!(
        (after.height - before.height).abs() < 0.5,
        "height {} -> {}",
        before.height,
        after.height
    );

    common::remove_dir(&dir);
}

/// The insertion marker follows the cursor index, including across wrapped
/// lines.
#[test]
fn free_text_caret_follows_the_cursor() {
    let Some(pdfium) = pdfium() else {
        eprintln!("SKIP: libpdfium.so not available");
        return;
    };
    let _guard = pdfium_guard();
    let dir = common::temp_dir("free-text-caret");
    let doc = common::make_pdf(pdfium, &dir, "doc.pdf", 1);

    let mut mgr = PdfOpsManager::new();
    assert!(matches!(mgr.execute(open_command(&doc)), CommandResult::Ok));

    let text = "alpha beta gamma delta epsilon zeta eta theta";
    let created = mgr.execute(Command::AddFreeTextAnnotation {
        page: 1,
        text: text.to_string(),
        font_size: 12.0,
        color: AnnotationColor::rgb(0, 0, 0),
        position: (60.0, 700.0),
        max_width: 100.0,
    });
    let CommandResult::FreeTextWritten { caret: end, .. } = created else {
        panic!("create failed");
    };

    let moved = mgr.execute(Command::UpdateFreeText {
        page: 1,
        index: 0,
        text: text.to_string(),
        font_size: 12.0,
        color: AnnotationColor::rgb(0, 0, 0),
        position: (60.0, 700.0),
        max_width: 100.0,
        cursor: 0,
    });
    let CommandResult::FreeTextWritten { caret: start, .. } = moved else {
        panic!("update failed");
    };

    // The caret starts at the box's left padding and sits on the first line.
    assert!((start.0 - 62.0).abs() < 0.5, "x {}", start.0);
    assert!(start.1 > end.1, "start {} end {}", start.1, end.1);
    assert!(start.0 < end.0, "start {} end {}", start.0, end.0);

    common::remove_dir(&dir);
}
