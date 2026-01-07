// SPDX-License-Identifier: MPL-2.0
// src/main.rs

mod app;
mod config;
mod i18n;

use anyhow::Result;
use clap::Parser;
use cosmic::app::Settings;
use crate::app::Noctua;

#[derive(Parser, Debug, Clone)]
#[command(version, about)]
pub struct Args {
    /// File to open on startup
    #[arg(value_name = "FILE")]
    pub file: Option<std::path::PathBuf>,

    /// UI language (e.g. "en", "de")
    #[arg(short, long, default_value = "en")]
    pub language: String,
}

fn main() -> Result<()> {
    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    // Enable localizations to be applied.
    i18n::init(&requested_languages);

    env_logger::init();
    let args = Args::parse();

    cosmic::app::run::<Noctua>(Settings::default(), app::Flags::Args(args))
        .map_err(|e| anyhow::anyhow!(e))
}
