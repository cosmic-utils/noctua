// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cli/src/runner.rs
//
// Execution runner for CLI commands.

use crate::command::{CliCommand, Selector};
use noctua_core::manager::Manager;
use noctua_core::workspace::{Command as CoreCommand, CommandResult};

pub struct PatchRunner {
    pub manager: Manager,
}

impl PatchRunner {
    pub fn new(manager: Manager) -> Self {
        Self { manager }
    }

    pub fn run_all(&mut self, commands: Vec<CliCommand>) {
        let total = commands.len();
        for (i, cmd) in commands.into_iter().enumerate() {
            println!("\n[{}/{}] Executing step: {:?}", i + 1, total, cmd);

            if let Err(e) = self.execute_single(cmd) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            } else {
                println!("    -> Success!");
            }
        }
    }

    fn execute_single(&mut self, step: CliCommand) -> Result<(), String> {
        match step {
            CliCommand::Workspace(cmd) => {
                let res = self.manager.execute(cmd);
                self.handle_core_res(res)
            }
            CliCommand::Document(doc_cmd) => {
                let res = self.manager.execute(CoreCommand::DocumentCommand(doc_cmd));
                self.handle_core_res(res)
            }
            CliCommand::SelectCollection(selector) => {
                let id = match selector {
                    Selector::Index(idx) => self
                        .manager
                        .workspace()
                        .collections
                        .get_index(idx)
                        .map(|(_, c)| c.id)
                        .ok_or_else(|| format!("No collection at index {}", idx))?,
                    Selector::Name(ref name) => self
                        .manager
                        .workspace()
                        .collections
                        .values()
                        .find(|c| c.name == *name)
                        .map(|c| c.id)
                        .ok_or_else(|| format!("No collection with name '{}'", name))?,
                };
                let res = self
                    .manager
                    .execute(CoreCommand::CollectionSelect { collection_id: id });
                self.handle_core_res(res)
            }
            CliCommand::SelectDocument(selector) => {
                let doc_id = {
                    let col_id = self
                        .manager
                        .workspace()
                        .active_collection_id
                        .ok_or_else(|| "No active collection to select document in".to_string())?;

                    let col = self.manager.workspace().collections.get(&col_id).unwrap();
                    match selector {
                        Selector::Index(idx) => col
                            .documents
                            .get_index(idx)
                            .map(|(_, d)| d.id)
                            .ok_or_else(|| {
                                format!("No document at index {} in active collection", idx)
                            })?,
                        Selector::Name(ref name) => {
                            let name_str = name.as_str();
                            col.documents
                                .values()
                                .find(|d| {
                                    d.path
                                        .file_name()
                                        .map(|n| n.to_string_lossy() == name_str)
                                        .unwrap_or(false)
                                })
                                .map(|d| d.id)
                                .ok_or_else(|| format!("No document with name '{}'", name))?
                        }
                    }
                };
                let res = self.manager.execute(CoreCommand::DocumentSelect {
                    document_id: doc_id,
                });
                self.handle_core_res(res)
            }
            CliCommand::ListWorkspace => {
                let ws = self.manager.workspace();
                println!("    --- Workspace State ---");
                println!("    Name: {}", ws.name);
                println!("    Active Collection: {:?}", ws.active_collection_id);
                for col in ws.collections.values() {
                    let active_marker = if ws.active_collection_id == Some(col.id) {
                        "[ACTIVE]"
                    } else {
                        ""
                    };
                    println!(
                        "    - {} Collection: '{}' (ID: {}, Type: {:?})",
                        active_marker, col.name, col.id, col.collection_type
                    );
                    for doc in col.documents.values() {
                        let doc_marker = if col.active_document_id == Some(doc.id) {
                            "[SELECTED]"
                        } else {
                            ""
                        };
                        println!(
                            "        - {} Document: {:?} (Page: {})",
                            doc_marker, doc.path, doc.current_page
                        );
                    }
                }
                println!("    -----------------------");
                Ok(())
            }
            CliCommand::ImportDirectory(dir_path) => {
                if !dir_path.is_dir() {
                    return Err(format!("Path is not a directory: {:?}", dir_path));
                }

                let col_name = dir_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                let cmd_add = CoreCommand::CollectionAdd {
                    collection_type: noctua_core::workspace::CollectionType::Browser,
                };

                let res_add = self.manager.execute(cmd_add);
                if let CommandResult::CollectionAdded(new_id) = res_add {
                    let _ = self.manager.execute(CoreCommand::CollectionRename {
                        collection_id: new_id,
                        name: col_name,
                    });

                    let mut file_paths: Vec<_> = std::fs::read_dir(&dir_path)
                        .into_iter()
                        .flatten()
                        .flatten()
                        .map(|e| e.path())
                        .filter(|p| p.is_file())
                        .collect();
                    file_paths.sort();

                    let mut entries = Vec::new();
                    for p in file_paths {
                        let Ok(entry) = noctua_core::storage::document::load(&p) else {
                            println!(
                                "    -> Error loading file: {:?}",
                                p.file_name().unwrap_or_default()
                            );
                            continue;
                        };
                        if entry
                            .info
                            .as_ref()
                            .is_some_and(|i| matches!(i.kind, noctua_core::document::Kind::Unknown))
                        {
                            println!(
                                "    -> Skipping unsupported format: {:?}",
                                p.file_name().unwrap_or_default()
                            );
                            continue;
                        }
                        entries.push(entry);
                    }

                    if !entries.is_empty() {
                        let cmd_multi = CoreCommand::DocumentAddMultiple {
                            collection_id: new_id,
                            entries,
                        };
                        let res = self.manager.execute(cmd_multi);
                        self.handle_core_res(res)
                    } else {
                        println!("    -> Directory was empty");
                        Ok(())
                    }
                } else {
                    Err("Failed to create initial collection for import".to_string())
                }
            }
        }
    }

    fn handle_core_res(&self, res: CommandResult) -> Result<(), String> {
        match res {
            CommandResult::Error(e) => Err(format!("{:?}", e)),
            _ => Ok(()),
        }
    }
}
