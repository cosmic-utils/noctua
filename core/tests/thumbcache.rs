// SPDX-License-Identifier: GPL-3.0-or-later
// core/tests/thumbcache.rs
//
// Integration tests for the freedesktop thumbnail cache.

use noctua_core_test_common as common;

use noctua_core::storage::thumbcache::{self, ThumbSize};

#[test]
fn lookup_misses_before_store() {
    let dir = common::temp_dir("thumb-miss");
    let cache = dir.join("cache");
    let img = common::make_png(&dir, "img.png", 64, 48);

    assert!(
        thumbcache::lookup_at(&cache, &img, ThumbSize::Normal)
            .unwrap()
            .is_none()
    );
    common::remove_dir(&dir);
}

#[test]
fn store_then_lookup_roundtrip() {
    let dir = common::temp_dir("thumb-store");
    let cache = dir.join("cache");
    let img = common::make_png(&dir, "img.png", 64, 48);

    let data = (64, 48, vec![255u8; 64 * 48 * 4]);
    thumbcache::store_at(&cache, &img, ThumbSize::Normal, &data).unwrap();

    let loaded = thumbcache::lookup_at(&cache, &img, ThumbSize::Normal)
        .unwrap()
        .expect("cache entry must exist after store");
    assert_eq!(loaded.0, 64);
    assert_eq!(loaded.1, 48);
    assert_eq!(loaded.2.len(), 64 * 48 * 4);
    common::remove_dir(&dir);
}

#[test]
fn get_or_create_generates_and_reuses_cache() {
    let dir = common::temp_dir("thumb-create");
    let cache = dir.join("cache");
    let img = common::make_png(&dir, "img.png", 64, 48);

    let first = thumbcache::get_or_create_at(&cache, &img, ThumbSize::Normal).unwrap();
    assert!(first.0 > 0 && first.1 > 0);

    // Second call must hit the cache: same pixel buffer length and dims.
    let second = thumbcache::get_or_create_at(&cache, &img, ThumbSize::Normal).unwrap();
    assert_eq!(first, second);
    common::remove_dir(&dir);
}

#[test]
fn get_or_create_rejects_pdfs() {
    let dir = common::temp_dir("thumb-pdf");
    let cache = dir.join("cache");
    // A fake PDF header is enough: the refusal is content-based.
    std::fs::write(dir.join("fake.pdf"), b"%PDF-1.7 fake content").unwrap();

    // PDF thumbnails must be rendered on the pdfium worker thread.
    assert!(
        thumbcache::get_or_create_at(&cache, &dir.join("fake.pdf"), ThumbSize::Normal).is_err()
    );
    common::remove_dir(&dir);
}

#[test]
fn changed_source_invalidates_cache_entry() {
    let dir = common::temp_dir("thumb-stale");
    let cache = dir.join("cache");
    let img = common::make_png(&dir, "img.png", 64, 48);

    // Pin the mtime to a known value; the cache validates against seconds
    // granularity, so a same-second rewrite would go unnoticed.
    let t0 = filetime::FileTime::from_unix_time(1_600_000_000, 0);
    filetime::set_file_mtime(&img, t0).unwrap();

    let first = thumbcache::get_or_create_at(&cache, &img, ThumbSize::Normal).unwrap();

    // Replace the source with a visually different image and a new mtime.
    let mut different = image::RgbaImage::new(64, 48);
    for pixel in different.pixels_mut() {
        *pixel = image::Rgba([200, 10, 10, 255]);
    }
    different.save(&img).unwrap();
    let t1 = filetime::FileTime::from_unix_time(1_600_000_100, 0);
    filetime::set_file_mtime(&img, t1).unwrap();

    let regenerated = thumbcache::get_or_create_at(&cache, &img, ThumbSize::Normal).unwrap();
    assert_ne!(first.2, regenerated.2, "stale entry must be regenerated");
    common::remove_dir(&dir);
}
