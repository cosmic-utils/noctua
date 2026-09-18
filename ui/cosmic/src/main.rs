// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cosmic/src/main.rs
//
// Application entry point.

mod app;
mod i18n;
mod message;
mod model;
mod update;
mod view;
mod widget;

use clap::Parser;
use cosmic::app::Settings;
use std::path::PathBuf;

/// Noctua – a document viewer for the COSMIC desktop.
#[derive(Parser, Debug)]
#[command(name = "noctua", about = "A document viewer for the COSMIC desktop")]
pub struct Args {
    /// Open a folder or document on startup (skips session restore).
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,
}

fn main() -> cosmic::iced::Result {
    // Structured logging; filter at runtime via RUST_LOG. Spans log their
    // duration on close, which makes phase timings directly visible.
    // Must run first so early errors (i18n) are captured as well.
    tracing_subscriber::fmt()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("noctua_cosmic=info,noctua_core=info")
            }),
        )
        .init();

    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    i18n::init(&requested_languages);

    // Fail fast with a clear hint instead of dead PDF features.
    if noctua_core::try_pdfium().is_none() {
        tracing::warn!(
            "libpdfium.so not found: PDF rendering and previews are disabled (check LD_LIBRARY_PATH)"
        );
    }

    let args = Args::parse();

    cosmic::app::run::<model::AppModel>(Settings::default(), args)
}
