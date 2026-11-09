//! `wincraft-app`: L3 native `eframe`/`egui` desktop workstation for WinCraft.
//! Calls `wincraft_core::dispatch` directly in-process (zero network/port overhead).

use eframe::egui;
use serde_json::Value;
use std::path::PathBuf;
use wincraft_core::{dispatch, tools_catalog, Command, ExecOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Dashboard,
    ProcsAndPorts,
    RamWeeder,
    FileOps,
    LockInspector,
    ServicesNssm,
    StartupAndRegistry,
    DoctorAndHelp,
}

impl Page {
    fn label(self) -> &'static str {
        match self {
            Page::Dashboard => "1. Host & Disks",
            Page::ProcsAndPorts => "2. Processes & Ports",
            Page::RamWeeder => "3. RAM Weeder",
            Page::FileOps => "4. High-Speed FileOps",
            Page::LockInspector => "5. Who Locks This File?",
            Page::ServicesNssm => "6. Services & NSSM",
            Page::StartupAndRegistry => "7. Startup & Registry",
            Page::DoctorAndHelp => "8. Doctor & Tools (26)",
        }
    }
}

pub struct WinCraftApp {
    opts: ExecOptions,
    page: Page,
    status_json: Option<Value>,
    doctor_json: Option<Value>,
    procs_json: Option<Value>,
    ports_json: Option<Value>,
    weeder_json: Option<Value>,
    fileops_json: Option<Value>,
    lock_json: Option<Value>,
    services_json: Option<Value>,
    startup_json: Option<Value>,
    reg_json: Option<Value>,
    events_json: Option<Value>,
    proc_filter: String,
    port_filter: String,
    target_path: String,
    grep_query: String,
    lock_path: String,
    svc_filter: String,
    reg_key: String,
    last_message: String,
    show_help_modal: bool,
}

impl Default for WinCraftApp {
    fn default() -> Self {
        let opts = ExecOptions::default();
        let status_json = dispatch(&Command::Status, &opts).ok();
        let doctor_json = dispatch(&Command::Doctor, &opts).ok();
        Self {
            opts,
            page: Page::Dashboard,
            status_json,
            doctor_json,
            procs_json: None,
            ports_json: None,
            weeder_json: None,
            fileops_json: None,
            lock_json: None,
            services_json: None,
            startup_json: None,
            reg_json: None,
            events_json: None,
            proc_filter: String::new(),
            port_filter: String::new(),
            target_path: r"D:\Dev\repos\wincraft".to_string(),
            grep_query: "WinCraft".to_string(),
            lock_path: r"D:\Dev\repos\wincraft\Cargo.toml".to_string(),
            svc_filter: String::new(),
            reg_key: r"HKCU\Environment".to_string(),
            last_message: "Ready — L3 desktop connected in-process to wincraft-core::dispatch."
                .to_string(),
            show_help_modal: false,
        }
    }
}

impl WinCraftApp {
    pub fn exec(&mut self, cmd: &Command) -> Result<Value, String> {
        match dispatch(cmd, &self.opts) {
            Ok(v) => {
                self.last_message = format!("OK: {cmd:?}");
                Ok(v)
            }
            Err(e) => {
                let msg = e.to_string();
                self.last_message = format!("ERR: {msg}");
                Err(msg)
            }
        }
    }
}

impl eframe::App for WinCraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.horizontal(|ui| {
            ui.heading("WinCraft v0.1.0");
            ui.separator();
            ui.checkbox(&mut self.opts.read_only, "Read-Only Guard");
            ui.separator();
            if ui.button("Refresh Status").clicked() {
                self.status_json = self.exec(&Command::Status).ok();
            }
            if ui
                .button(if self.show_help_modal {
                    "Hide Safety & Help"
                } else {
                    "? Help & Safety"
                })
                .clicked()
            {
                self.show_help_modal = !self.show_help_modal;
            }
            ui.separator();
            ui.label(&self.last_message);
        });

        ui.separator();

        ui.horizontal_wrapped(|ui| {
            for p in [
                Page::Dashboard,
                Page::ProcsAndPorts,
                Page::RamWeeder,
                Page::FileOps,
                Page::LockInspector,
                Page::ServicesNssm,
                Page::StartupAndRegistry,
                Page::DoctorAndHelp,
            ] {
                if ui.selectable_label(self.page == p, p.label()).clicked() {
                    self.page = p;
                }
            }
        });

        if self.show_help_modal {
            ui.separator();
            ui.group(|ui| {
                ui.heading("WinCraft Safety & Architecture");
                ui.label("WinCraft subsumes windows-operations-mcp, system-admin-mcp, sysinternals-mcp, filesystem-mcp, and winrar-mcp into one pure-Rust ArtCraft 3-tier binary.");
                ui.label("1. Every mutating verb (weeder_reap, fs_backup_write, fs_undo_backup, fs_archive_create, svc_control, reg_write) requires confirm=true.");
                ui.label("2. Protected system processes (csrss, lsass, svchost, explorer, nssm, pwsh, etc.) can never be reaped.");
                ui.label("3. Every file edit creates a timestamped .bak backup; every HKCU registry edit exports a .reg snapshot first.");
                ui.label("4. NSSM services are controlled strictly via sc.exe stop/start.");
            });
        }

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| match self.page {
            Page::Dashboard => {
                ui.heading("Host Overview, Logical Volumes & PATH Audit");
                if let Some(ref st) = self.status_json {
                    ui.monospace(
                        serde_json::to_string_pretty(st).unwrap_or_else(|_| "{}".to_string()),
                    );
                }
            }
            Page::ProcsAndPorts => {
                ui.heading("Processes & Fleet Port Map (WEBAPP_PORTS.md Correlated)");
                ui.horizontal(|ui| {
                    ui.label("Process filter:");
                    ui.text_edit_singleline(&mut self.proc_filter);
                    if ui.button("Scan Processes").clicked() {
                        let filter = if self.proc_filter.trim().is_empty() {
                            None
                        } else {
                            Some(self.proc_filter.trim().to_string())
                        };
                        self.procs_json = self.exec(&Command::ProcList { filter, limit: 50 }).ok();
                    }
                    ui.separator();
                    ui.label("Port:");
                    ui.text_edit_singleline(&mut self.port_filter);
                    if ui.button("Scan Listening Ports").clicked() {
                        let port = self.port_filter.trim().parse::<u16>().ok();
                        self.ports_json = self
                            .exec(&Command::PortMap {
                                port,
                                listening_only: true,
                            })
                            .ok();
                    }
                });
                if let Some(ref ports) = self.ports_json {
                    ui.collapsing("Listening Ports", |ui| {
                        ui.monospace(serde_json::to_string_pretty(ports).unwrap_or_default());
                    });
                }
                if let Some(ref procs) = self.procs_json {
                    ui.collapsing("Top Processes by Memory", |ui| {
                        ui.monospace(serde_json::to_string_pretty(procs).unwrap_or_default());
                    });
                }
            }
            Page::RamWeeder => {
                ui.heading("RAM Weeder — Orphan Cargo Test & Zombie Engine Detector");
                if ui.button("Run Weeder Scan").clicked() {
                    self.weeder_json = self.exec(&Command::WeederScan).ok();
                }
                if let Some(ref w) = self.weeder_json {
                    ui.monospace(serde_json::to_string_pretty(w).unwrap_or_default());
                }
            }
            Page::FileOps => {
                ui.heading("High-Speed FileOps (DU, Large Files, 3-Stage Duplicates, Grep)");
                ui.horizontal(|ui| {
                    ui.label("Directory:");
                    ui.text_edit_singleline(&mut self.target_path);
                });
                ui.horizontal(|ui| {
                    let p = PathBuf::from(self.target_path.trim());
                    if ui.button("Disk Usage (DU)").clicked() {
                        self.fileops_json = self
                            .exec(&Command::FsDu {
                                path: p.clone(),
                                skip_build_dirs: true,
                            })
                            .ok();
                    }
                    if ui.button("Find Duplicates (3-Stage SHA-256)").clicked() {
                        self.fileops_json = self
                            .exec(&Command::FsDups {
                                path: p.clone(),
                                skip_build_dirs: true,
                            })
                            .ok();
                    }
                    if ui.button("Find Largest Files").clicked() {
                        self.fileops_json = self
                            .exec(&Command::FsLarge {
                                path: p.clone(),
                                min_size_bytes: 65536,
                                limit: 25,
                                skip_build_dirs: true,
                            })
                            .ok();
                    }
                    ui.label("Grep:");
                    ui.text_edit_singleline(&mut self.grep_query);
                    if ui.button("Search").clicked() {
                        let q = self.grep_query.clone();
                        self.fileops_json = self
                            .exec(&Command::FsGrep {
                                path: p,
                                query: q,
                                ext: None,
                                max_matches: 50,
                            })
                            .ok();
                    }
                });
                if let Some(ref f) = self.fileops_json {
                    ui.monospace(serde_json::to_string_pretty(f).unwrap_or_default());
                }
            }
            Page::LockInspector => {
                ui.heading("Who Locks This File? (Native Windows Restart Manager rstrtmgr.dll)");
                ui.horizontal(|ui| {
                    ui.label("File or Directory Path:");
                    ui.text_edit_singleline(&mut self.lock_path);
                    if ui.button("Inspect Locks").clicked() {
                        let p = PathBuf::from(self.lock_path.trim());
                        self.lock_json = self.exec(&Command::LockInspect { path: p }).ok();
                    }
                });
                if let Some(ref l) = self.lock_json {
                    ui.monospace(serde_json::to_string_pretty(l).unwrap_or_default());
                }
            }
            Page::ServicesNssm => {
                ui.heading("Windows Services & NSSM-Safe Controller");
                ui.horizontal(|ui| {
                    ui.label("Service filter:");
                    ui.text_edit_singleline(&mut self.svc_filter);
                    if ui.button("Query Services").clicked() {
                        let filter = if self.svc_filter.trim().is_empty() {
                            None
                        } else {
                            Some(self.svc_filter.trim().to_string())
                        };
                        self.services_json = self
                            .exec(&Command::SvcList {
                                filter,
                                state: None,
                                limit: 100,
                            })
                            .ok();
                    }
                });
                if let Some(ref s) = self.services_json {
                    ui.monospace(serde_json::to_string_pretty(s).unwrap_or_default());
                }
            }
            Page::StartupAndRegistry => {
                ui.heading("Autoruns, Scheduled Tasks, Registry & Event Logs");
                ui.horizontal(|ui| {
                    if ui.button("List Startup Autoruns").clicked() {
                        self.startup_json = self.exec(&Command::StartupList).ok();
                    }
                    if ui.button("Tail System Event Log").clicked() {
                        self.events_json = self
                            .exec(&Command::EventTail {
                                channel: "System".to_string(),
                                count: 10,
                            })
                            .ok();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Registry Key:");
                    ui.text_edit_singleline(&mut self.reg_key);
                    if ui.button("Read Key").clicked() {
                        let k = self.reg_key.trim().to_string();
                        self.reg_json = self
                            .exec(&Command::RegRead {
                                key: k,
                                value: None,
                            })
                            .ok();
                    }
                });
                if let Some(ref r) = self.reg_json {
                    ui.monospace(serde_json::to_string_pretty(r).unwrap_or_default());
                }
                if let Some(ref st) = self.startup_json {
                    ui.monospace(serde_json::to_string_pretty(st).unwrap_or_default());
                }
                if let Some(ref ev) = self.events_json {
                    ui.monospace(serde_json::to_string_pretty(ev).unwrap_or_default());
                }
            }
            Page::DoctorAndHelp => {
                ui.heading("Toolchain Doctor & 26 MCP Tools Catalog");
                if ui.button("Re-run Doctor").clicked() {
                    self.doctor_json = self.exec(&Command::Doctor).ok();
                }
                if let Some(ref doc) = self.doctor_json {
                    ui.monospace(serde_json::to_string_pretty(doc).unwrap_or_default());
                }
                ui.separator();
                for t in tools_catalog() {
                    ui.label(format!(
                        "• {} [{}] (read_only={}, confirm={}) — {}",
                        t.name, t.domain, t.read_only, t.confirm_gated, t.description
                    ));
                }
            }
        });
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--smoke") {
        let mut app = WinCraftApp::default();
        let ok = app.exec(&Command::Status).is_ok() && app.exec(&Command::Doctor).is_ok();
        if ok {
            println!("smoke: ok (wincraft-app 0.1.0)");
            std::process::exit(0);
        } else {
            eprintln!("smoke: failed");
            std::process::exit(1);
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("WinCraft — Windows Host, RAM Weeder, FileOps & SysAdmin Studio")
            .with_inner_size([1180.0, 760.0]),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "wincraft-app",
        native_options,
        Box::new(|_cc| Ok(Box::new(WinCraftApp::default()))),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_dispatch_matches_core_dispatch() {
        let mut app = WinCraftApp::default();
        let core_tools = dispatch(&Command::Tools, &ExecOptions::default());
        let app_tools = app.exec(&Command::Tools);
        assert!(core_tools.is_ok());
        assert!(app_tools.is_ok());
        assert_eq!(core_tools.ok(), app_tools.ok());
    }
}
