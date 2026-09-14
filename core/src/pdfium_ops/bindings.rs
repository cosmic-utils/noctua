// SPDX-License-Identifier: GPL-3.0-or-later
// core/src/pdfium_ops/bindings.rs
//
// Process-wide Pdfium instance. pdfium's library is process-global;
// every module must share this single instance. Binding it twice hangs.

use pdfium_render::prelude::Pdfium;
use std::sync::OnceLock;

static PDFIUM: OnceLock<Option<Pdfium>> = OnceLock::new();

/// The single shared Pdfium instance for the whole process.
///
/// # Panics
/// Panics if libpdfium.so cannot be bound. Call [`try_pdfium`] first when
/// pdfium may not be available (tests, optional backends).
pub fn pdfium() -> &'static Pdfium {
    try_pdfium().expect("libpdfium.so is not available (check LD_LIBRARY_PATH)")
}

/// The shared Pdfium instance or a manager-ready error. Used by the
/// pdfium worker so a missing library fails jobs instead of panicking.
pub fn pdfium_or_err() -> Result<&'static Pdfium, crate::pdfium_ops::PdfOpsError> {
    try_pdfium().ok_or_else(|| {
        crate::pdfium_ops::PdfOpsError::Pdfium(
            "libpdfium.so is not available (check LD_LIBRARY_PATH)".to_string(),
        )
    })
}

/// Bind pdfium if possible. Returns `None` when the library is not
/// available; used by tests to skip pdfium-dependent cases gracefully.
pub fn try_pdfium() -> Option<&'static Pdfium> {
    PDFIUM
        .get_or_init(|| Pdfium::bind_to_system_library().ok().map(Pdfium::new))
        .as_ref()
}
