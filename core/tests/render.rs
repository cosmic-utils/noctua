// SPDX-License-Identifier: GPL-3.0-or-later
// core/tests/render.rs
//
// Integration tests for the UI-independent render engine (raster/SVG).

use noctua_core_test_common as common;

use noctua_core::render;

#[test]
fn loads_raster_and_renders_at_zoom() {
    let dir = common::temp_dir("render-raster");
    let png = common::make_png(&dir, "img.png", 64, 32);

    let content = render::load(&png).unwrap();
    let page = render::render_page(&content, 2.0).unwrap();
    assert_eq!(page.width, 128);
    assert_eq!(page.height, 64);
    assert_eq!(page.rgba_data.len(), 128 * 64 * 4);
    common::remove_dir(&dir);
}

#[test]
fn scaled_raster_dimensions_match_buffer() {
    // 2 × 3 at 0.75 reports 2 × 2; an aspect-preserving resize would return
    // a smaller buffer, leaving the declared size and the pixels out of sync.
    let dir = common::temp_dir("render-raster-odd-zoom");
    let png = common::make_png(&dir, "img.png", 2, 3);

    let content = render::load(&png).unwrap();
    let page = render::render_page(&content, 0.75).unwrap();
    assert_eq!(page.width, 2);
    assert_eq!(page.height, 2);
    assert_eq!(page.rgba_data.len(), 2 * 2 * 4);
    common::remove_dir(&dir);
}

#[test]
fn loads_svg_and_renders() {
    let dir = common::temp_dir("render-svg");
    let svg = common::make_svg(&dir, "plan.svg");

    let content = render::load(&svg).unwrap();
    let page = render::render_page(&content, 1.0).unwrap();
    assert_eq!(page.width, 200);
    assert_eq!(page.height, 100);
    assert_eq!(page.rgba_data.len(), 200 * 100 * 4);
    common::remove_dir(&dir);
}

#[test]
fn loading_pdf_is_an_error() {
    let dir = common::temp_dir("render-pdf");
    std::fs::write(dir.join("fake.pdf"), b"%PDF-1.7 fake content").unwrap();

    // PDFs must go through the pdfium worker; load() refuses them.
    assert!(render::load(&dir.join("fake.pdf")).is_err());
    common::remove_dir(&dir);
}

#[test]
fn renders_path_directly() {
    let dir = common::temp_dir("render-path");
    let png = common::make_png(&dir, "img.png", 100, 50);

    let page = render::render_path(&png, 1.0).unwrap();
    assert_eq!(page.width, 100);
    assert_eq!(page.height, 50);
    common::remove_dir(&dir);
}

#[test]
fn rotates_rendered_page() {
    let dir = common::temp_dir("render-rotate");
    let png = common::make_png(&dir, "img.png", 64, 32);

    let content = render::load(&png).unwrap();
    let page = render::render_page_rotated(&content, 1.0, 90, false, false).unwrap();
    assert_eq!(page.width, 32);
    assert_eq!(page.height, 64);
    assert_eq!(page.rgba_data.len(), 32 * 64 * 4);
    common::remove_dir(&dir);
}
