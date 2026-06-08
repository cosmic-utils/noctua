// SPDX-License-Identifier: GPL-3.0-or-later
// src/main.rs
//
// Entry point for the Noctua headless CLI.

mod command;
mod runner;

use crate::command::CliCommand;
use crate::runner::PatchRunner;
use clap::Parser;
use noctua_core::manager::Manager;
use noctua_core::storage;
use noctua_core::workspace::Workspace;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "noctua-cli",
    about = "Headless CLI for Noctua Workspace Operations"
)]
struct Args {
    /// Execute a RON patch file containing commands
    #[arg(short = 'p', long = "patch", value_name = "FILE")]
    patch_file: PathBuf,

    /// Optional target workspace to apply the patch onto. If not provided, starts with empty workspace.
    #[arg(short = 'w', long = "workspace", value_name = "FILE")]
    workspace_file: Option<PathBuf>,

    /// Save the resulting workspace to this file after applying the patch.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    output_workspace_file: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();

    println!("Starting Noctua CLI...");

    // 1. Initialize Workspace
    let mut workspace = Workspace::default();

    if let Some(ws_path) = &args.workspace_file {
        println!("Loading workspace from: {:?}", ws_path);
        match storage::workspace::load::<Workspace>(ws_path) {
            Ok(loaded_ws) => workspace = loaded_ws,
            Err(e) => {
                eprintln!("Failed to load workspace: {:?}", e);
                std::process::exit(1);
            }
        }
    } else {
        println!("Starting with empty default workspace.");
    }

    let manager = Manager::from_workspace(workspace);
    let mut runner = PatchRunner::new(manager);

    // 2. Load commands from patch file
    println!("Reading patch file: {:?}", args.patch_file);
    let content = match fs::read_to_string(&args.patch_file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read patch file: {:?}", e);
            std::process::exit(1);
        }
    };

    let steps: Vec<CliCommand> = match ron::from_str(&content) {
        Ok(cmds) => cmds,
        Err(e) => {
            eprintln!("Failed to parse RON commands from patch: {:?}", e);
            std::process::exit(1);
        }
    };

    // 3. Execute Commands
    runner.run_all(steps);

    // 4. Save state if output is requested
    if let Some(out_path) = &args.output_workspace_file {
        println!("Saving workspace state to: {:?}", out_path);
        if let Err(e) = storage::workspace::save(runner.manager.workspace(), out_path) {
            eprintln!("Failed to save output workspace: {:?}", e);
            std::process::exit(1);
        }
    }

    println!("Patch applied successfully.");
}
