// SPDX-License-Identifier: GPL-3.0-or-later
// src/main.rs

mod app;
mod config;
mod i18n;

use clap::Parser;
use std::path::PathBuf;

use cosmic::app::Settings;

#[derive(Parser, Debug)]
#[command(name = "noctua", about = "A document viewer for the COSMIC desktop")]
struct Args {
    /// Open a document (raster, vector, portable)
    #[arg(short = 'f', long = "open-document", value_name = "PATH")]
    document: Option<PathBuf>,

    /// Open a directory and display all images
    #[arg(short = 'd', long = "open-directory", value_name = "PATH")]
    directory: Option<PathBuf>,

    /// Open a workspace file (.ws.ron)
    #[arg(short = 'w', long = "open-workspace", value_name = "PATH")]
    workspace: Option<PathBuf>,

}

fn main() -> cosmic::iced::Result {
    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    // Enable localizations to be applied.
    i18n::init(&requested_languages);

    let _args = Args::parse();


    let flags = ();

    // Start the application.
    cosmic::app::run::<app::AppModel>(Settings::default(), flags)
}
