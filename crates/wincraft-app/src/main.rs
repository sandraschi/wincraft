//! `wincraft-app`: L3 native `eframe`/`egui` desktop workstation for WinCraft.
//! Calls `wincraft_core::dispatch` directly in-process (zero network/port overhead).
//! All collection outputs default to Cards view and are switchable between Cards, List (table),
//! and JSON, with live filtering, column sorting, pagination, and one-click export to CSV, MD, TXT, or JSON.

use eframe::egui;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use wincraft_core::{
    dispatch, filter_sort_paginate, payload_to_tabular, render_tabular, tools_catalog, Command,
    ExecOptions, OutputFormat,
};

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
    view_mode: OutputFormat,
    table_filter: String,
    sort_col: usize,
    sort_asc: bool,
    page_num: usize,
    page_size: usize,
    export_dir: PathBuf,
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
        let procs_json = dispatch(
            &Command::ProcList {
                filter: None,
                limit: 100,
            },
            &opts,
        )
        .ok();
        Self {
            opts,
            page: Page::Dashboard,
            view_mode: OutputFormat::Cards, // Default is Cards per user specification
            table_filter: String::new(),
            sort_col: 0,
            sort_asc: true,
            page_num: 1,
            page_size: 25,
            export_dir: PathBuf::from(r"D:\Dev\repos\wincraft\exports"),
            status_json,
            doctor_json,
            procs_json,
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
            last_message:
                "Ready — Default view: Cards (switchable to List/JSON, paginated, sortable, filterable, exportable)."
                    .to_string(),
            show_help_modal: false,
        }
    }
}

impl WinCraftApp {
    pub fn exec(&mut self, cmd: &Command) -> Result<Value, String> {
        match dispatch(cmd, &self.opts) {
            Ok(v) => {
                self.page_num = 1;
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

    fn render_dataset_controls_and_body(
        &mut self,
        ui: &mut egui::Ui,
        dataset_slug: &str,
        payload: &Value,
    ) {
        let tab = payload_to_tabular(payload);
        let col_count = tab.headers.len().max(1);
        if self.sort_col >= col_count {
            self.sort_col = 0;
        }

        let (paged_tab, total_rows, total_pages) = filter_sort_paginate(
            tab.clone(),
            Some(self.table_filter.as_str()),
            Some(self.sort_col),
            self.sort_asc,
            self.page_num,
            self.page_size,
        );
        if self.page_num > total_pages {
            self.page_num = total_pages;
        }

        // Control Bar 1: View Mode (Cards [default] | List | JSON), Filter, and Export (CSV | MD | TXT | JSON)
        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("View:");
                if ui
                    .selectable_label(self.view_mode == OutputFormat::Cards, "🗂 Cards (Default)")
                    .clicked()
                {
                    self.view_mode = OutputFormat::Cards;
                }
                if ui
                    .selectable_label(self.view_mode == OutputFormat::List, "☰ List / Table")
                    .clicked()
                {
                    self.view_mode = OutputFormat::List;
                }
                if ui
                    .selectable_label(self.view_mode == OutputFormat::Json, "{ } JSON")
                    .clicked()
                {
                    self.view_mode = OutputFormat::Json;
                }

                ui.separator();
                ui.label("Filter:");
                let filter_resp = ui.text_edit_singleline(&mut self.table_filter);
                if filter_resp.changed() {
                    self.page_num = 1;
                }
                if !self.table_filter.is_empty() && ui.button("Clear").clicked() {
                    self.table_filter.clear();
                    self.page_num = 1;
                }

                ui.separator();
                ui.label("Export:");
                for (btn_label, ext, fmt) in [
                    ("CSV", "csv", OutputFormat::Csv),
                    ("MD", "md", OutputFormat::Md),
                    ("TXT", "txt", OutputFormat::Txt),
                    ("JSON", "json", OutputFormat::Json),
                ] {
                    if ui.button(btn_label).clicked() {
                        let _ = std::fs::create_dir_all(&self.export_dir);
                        let epoch = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        let file_path = self
                            .export_dir
                            .join(format!("{dataset_slug}_{epoch}.{ext}"));
                        let total_cap = tab.rows.len().max(1);
                        let (all_filtered, _, _) = filter_sort_paginate(
                            tab.clone(),
                            Some(self.table_filter.as_str()),
                            Some(self.sort_col),
                            self.sort_asc,
                            1,
                            total_cap,
                        );
                        let body = render_tabular(&all_filtered, payload, fmt);
                        match std::fs::write(&file_path, body) {
                            Ok(()) => {
                                self.last_message = format!(
                                    "Exported {} rows ({}) -> {}",
                                    all_filtered.rows.len(),
                                    btn_label,
                                    file_path.display()
                                );
                            }
                            Err(e) => {
                                self.last_message =
                                    format!("Export failed ({}): {e}", file_path.display());
                            }
                        }
                    }
                }
            });

            // Control Bar 2: Sorting & Pagination
            ui.horizontal_wrapped(|ui| {
                ui.label("Sort by:");
                for (idx, h) in tab.headers.iter().enumerate() {
                    let active = self.sort_col == idx;
                    let arrow = if active {
                        if self.sort_asc {
                            " ▲"
                        } else {
                            " ▼"
                        }
                    } else {
                        ""
                    };
                    if ui.selectable_label(active, format!("{h}{arrow}")).clicked() {
                        if self.sort_col == idx {
                            self.sort_asc = !self.sort_asc;
                        } else {
                            self.sort_col = idx;
                            self.sort_asc = true;
                        }
                        self.page_num = 1;
                    }
                }

                ui.separator();
                ui.label("Page size:");
                for sz in [10usize, 25, 50, 100] {
                    if ui
                        .selectable_label(self.page_size == sz, sz.to_string())
                        .clicked()
                    {
                        self.page_size = sz;
                        self.page_num = 1;
                    }
                }

                ui.separator();
                if ui.button("<< First").clicked() {
                    self.page_num = 1;
                }
                if ui.button("< Prev").clicked() && self.page_num > 1 {
                    self.page_num -= 1;
                }
                ui.label(format!(
                    "Page {} / {} ({} of {} items)",
                    self.page_num,
                    total_pages,
                    paged_tab.rows.len(),
                    total_rows
                ));
                if ui.button("Next >").clicked() && self.page_num < total_pages {
                    self.page_num += 1;
                }
                if ui.button("Last >>").clicked() {
                    self.page_num = total_pages;
                }
            });
        });

        ui.add_space(4.0);

        // Dataset Content Rendering
        match self.view_mode {
            OutputFormat::Json => {
                ui.monospace(
                    serde_json::to_string_pretty(payload).unwrap_or_else(|_| "{}".to_string()),
                );
            }
            OutputFormat::List => {
                egui::Grid::new(format!("grid_{dataset_slug}"))
                    .striped(true)
                    .spacing([14.0, 6.0])
                    .show(ui, |ui| {
                        for (idx, h) in paged_tab.headers.iter().enumerate() {
                            let arrow = if self.sort_col == idx {
                                if self.sort_asc {
                                    " ▲"
                                } else {
                                    " ▼"
                                }
                            } else {
                                ""
                            };
                            if ui.button(format!("{h}{arrow}")).clicked() {
                                if self.sort_col == idx {
                                    self.sort_asc = !self.sort_asc;
                                } else {
                                    self.sort_col = idx;
                                    self.sort_asc = true;
                                }
                                self.page_num = 1;
                            }
                        }
                        ui.end_row();

                        for row in &paged_tab.rows {
                            for cell in row {
                                ui.monospace(cell);
                            }
                            ui.end_row();
                        }
                    });
            }
            OutputFormat::Cards | OutputFormat::Csv | OutputFormat::Md | OutputFormat::Txt => {
                // Multi-card responsive grid (2 cards per row)
                let start_idx = (self.page_num.saturating_sub(1)) * self.page_size + 1;
                egui::Grid::new(format!("cards_{dataset_slug}"))
                    .num_columns(2)
                    .spacing([12.0, 10.0])
                    .show(ui, |ui| {
                        for (i, row) in paged_tab.rows.iter().enumerate() {
                            ui.group(|ui| {
                                ui.set_min_width(480.0);
                                let title_col = row.first().map(|s| s.as_str()).unwrap_or("-");
                                let second_col = row.get(1).map(|s| s.as_str()).unwrap_or("");
                                ui.horizontal(|ui| {
                                    ui.strong(format!("#{} {}", start_idx + i, title_col));
                                    if !second_col.is_empty() {
                                        ui.separator();
                                        ui.monospace(second_col);
                                    }
                                });
                                ui.separator();
                                for (h, val) in paged_tab.headers.iter().zip(row.iter()) {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(format!("{h}:"));
                                        ui.monospace(val);
                                    });
                                }
                            });
                            if i % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });
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
                    self.page_num = 1;
                    self.sort_col = 0;
                }
            }
        });

        if self.show_help_modal {
            ui.separator();
            ui.group(|ui| {
                ui.heading("WinCraft Safety & Architecture");
                ui.label("WinCraft subsumes windows-operations-mcp, system-admin-mcp, sysinternals-mcp, filesystem-mcp, and winrar-mcp into one pure-Rust ArtCraft 3-tier binary.");
                ui.label("1. Outputs default to Cards and can be switched at any time between Cards, List/Table, and JSON, with live filtering, column sorting, pagination, and CSV/MD/TXT/JSON export.");
                ui.label("2. Protected system processes (csrss, lsass, svchost, explorer, nssm, pwsh, etc.) can never be reaped.");
                ui.label("3. Every file edit creates a timestamped .bak backup; every HKCU registry edit exports a .reg snapshot first.");
                ui.label("4. NSSM services are controlled strictly via sc.exe stop/start.");
            });
        }

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| match self.page {
            Page::Dashboard => {
                ui.heading("Host Overview & Logical Disk Volumes");
                if let Some(st) = self.status_json.clone() {
                    if let Some(disks) = st.get("disks") {
                        let wrapped = json!({ "volumes": disks });
                        self.render_dataset_controls_and_body(ui, "disks", &wrapped);
                    } else {
                        self.render_dataset_controls_and_body(ui, "status", &st);
                    }
                }
            }
            Page::ProcsAndPorts => {
                ui.heading("Processes & Fleet Port Map (WEBAPP_PORTS.md Correlated)");
                ui.horizontal(|ui| {
                    ui.label("Process query:");
                    ui.text_edit_singleline(&mut self.proc_filter);
                    if ui.button("Scan Processes").clicked() {
                        let filter = if self.proc_filter.trim().is_empty() {
                            None
                        } else {
                            Some(self.proc_filter.trim().to_string())
                        };
                        self.procs_json = self.exec(&Command::ProcList { filter, limit: 200 }).ok();
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
                if let Some(ports) = self.ports_json.clone() {
                    ui.separator();
                    ui.label("Listening Ports:");
                    self.render_dataset_controls_and_body(ui, "ports", &ports);
                }
                if let Some(procs) = self.procs_json.clone() {
                    ui.separator();
                    ui.label("Processes:");
                    self.render_dataset_controls_and_body(ui, "processes", &procs);
                }
            }
            Page::RamWeeder => {
                ui.heading("RAM Weeder — Orphan Cargo Test & Zombie Engine Detector");
                if ui.button("Run Weeder Scan").clicked() {
                    self.weeder_json = self.exec(&Command::WeederScan).ok();
                }
                if let Some(w) = self.weeder_json.clone() {
                    self.render_dataset_controls_and_body(ui, "weeder", &w);
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
                    if ui.button("List Directory").clicked() {
                        self.fileops_json = self
                            .exec(&Command::FsList {
                                path: p.clone(),
                                recursive: true,
                                depth: 2,
                            })
                            .ok();
                    }
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
                                min_size_bytes: 16384,
                                limit: 100,
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
                                max_matches: 200,
                            })
                            .ok();
                    }
                });
                if let Some(f) = self.fileops_json.clone() {
                    self.render_dataset_controls_and_body(ui, "fileops", &f);
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
                if let Some(l) = self.lock_json.clone() {
                    self.render_dataset_controls_and_body(ui, "locks", &l);
                }
            }
            Page::ServicesNssm => {
                ui.heading("Windows Services & NSSM-Safe Controller");
                ui.horizontal(|ui| {
                    ui.label("Service query:");
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
                                limit: 300,
                            })
                            .ok();
                    }
                });
                if let Some(s) = self.services_json.clone() {
                    self.render_dataset_controls_and_body(ui, "services", &s);
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
                                count: 25,
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
                if let Some(r) = self.reg_json.clone() {
                    self.render_dataset_controls_and_body(ui, "registry", &r);
                }
                if let Some(st) = self.startup_json.clone() {
                    self.render_dataset_controls_and_body(ui, "startup", &st);
                }
                if let Some(ev) = self.events_json.clone() {
                    self.render_dataset_controls_and_body(ui, "events", &ev);
                }
            }
            Page::DoctorAndHelp => {
                ui.heading("Toolchain Doctor & 26 MCP Tools Catalog");
                if ui.button("Re-run Doctor").clicked() {
                    self.doctor_json = self.exec(&Command::Doctor).ok();
                }
                let tools_val = json!({ "tools": tools_catalog() });
                self.render_dataset_controls_and_body(ui, "tools", &tools_val);
            }
        });
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--smoke") {
        let mut app = WinCraftApp::default();
        let ok = app.exec(&Command::Status).is_ok()
            && app.exec(&Command::Doctor).is_ok()
            && app.view_mode == OutputFormat::Cards;
        if ok {
            println!("smoke: ok (wincraft-app 0.1.0, default_view=cards)");
            std::process::exit(0);
        } else {
            eprintln!("smoke: failed");
            std::process::exit(1);
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("WinCraft — Windows Host, RAM Weeder, FileOps & SysAdmin Studio")
            .with_inner_size([1240.0, 800.0]),
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
    fn app_dispatch_matches_core_dispatch_and_defaults_to_cards() {
        let mut app = WinCraftApp::default();
        assert_eq!(app.view_mode, OutputFormat::Cards);
        let core_tools = dispatch(&Command::Tools, &ExecOptions::default());
        let app_tools = app.exec(&Command::Tools);
        assert!(core_tools.is_ok());
        assert!(app_tools.is_ok());
        assert_eq!(core_tools.ok(), app_tools.ok());
    }
}
