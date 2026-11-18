//! `wincraft-app`: L3 native `eframe`/`egui` desktop workstation for WinCraft.
//! Calls `wincraft_core::dispatch` directly in-process (zero network/port overhead).
//! Features:
//! - Custom dark obsidian/cyan/emerald visual theme + live KPI telemetry ribbon
//! - Default Cards view switchable to List (table) and JSON
//! - Contextual quick actions on Cards & Rows (Browse in FileOps, Find Large Files,
//!   Inspect File Locks, Compute SHA-256, Reap Orphan PID, Control NSSM Services)
//! - Visual disk capacity & process memory progress bars + color-coded status badges
//! - Live substring filtering, numeric-aware column sorting, pagination, and CSV/MD/TXT/JSON export
//! - One-click launchers for the repository's PDF and EPUB brochures (`docs/brochure/`)

use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};
use wincraft_core::{
    dispatch, filter_sort_paginate, payload_to_tabular, render_tabular, tools_catalog, Command,
    ExecOptions, OutputFormat,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Dashboard,
    ProcsAndPorts,
    CpuAuditAndThermals,
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
            Page::Dashboard => "🖴 1. Host & Disks",
            Page::ProcsAndPorts => "⚡ 2. Processes & Ports",
            Page::CpuAuditAndThermals => "🔥 3. CPU Audit & Thermals",
            Page::RamWeeder => "🧹 4. RAM Weeder",
            Page::FileOps => "📂 5. High-Speed FileOps",
            Page::LockInspector => "🔒 6. Who Locks This File?",
            Page::ServicesNssm => "⚙ 7. Services & NSSM",
            Page::StartupAndRegistry => "🔑 8. Startup & Registry",
            Page::DoctorAndHelp => "🩺 9. Doctor, Tools & Brochure",
        }
    }
}

#[derive(Debug, Clone)]
enum PendingAction {
    BrowseMount(String),
    LargeFilesMount(String),
    DupsMount(String),
    InspectLockPath(String),
    HashFilePath(String),
    FilterProcessPid(String),
    ReapPid(u32),
    ServiceControl { name: String, action: String },
}

pub struct WinCraftApp {
    opts: ExecOptions,
    theme_applied: bool,
    page: Page,
    view_mode: OutputFormat,
    table_filter: String,
    sort_col: usize,
    sort_asc: bool,
    page_num: usize,
    page_size: usize,
    arm_confirm: bool,
    export_dir: PathBuf,
    brochure_dir: PathBuf,
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
    cpu_audit_json: Option<Value>,
    thermal_json: Option<Value>,
    proc_filter: String,
    port_filter: String,
    cpu_filter: String,
    min_cpu_sec: f64,
    min_cpu_pct: f64,
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
        let weeder_json = dispatch(&Command::WeederScan, &opts).ok();
        Self {
            opts,
            theme_applied: false,
            page: Page::Dashboard,
            view_mode: OutputFormat::Cards,
            table_filter: String::new(),
            sort_col: 0,
            sort_asc: true,
            page_num: 1,
            page_size: 25,
            arm_confirm: false,
            export_dir: PathBuf::from(r"D:\Dev\repos\wincraft\exports"),
            brochure_dir: PathBuf::from(r"D:\Dev\repos\wincraft\docs\brochure"),
            status_json,
            doctor_json,
            procs_json,
            ports_json: None,
            weeder_json,
            fileops_json: None,
            lock_json: None,
            services_json: None,
            startup_json: None,
            reg_json: None,
            events_json: None,
            cpu_audit_json: None,
            thermal_json: None,
            proc_filter: String::new(),
            port_filter: String::new(),
            cpu_filter: String::new(),
            min_cpu_sec: 30.0,
            min_cpu_pct: 5.0,
            target_path: r"D:\Dev\repos\wincraft".to_string(),
            grep_query: "WinCraft".to_string(),
            lock_path: r"D:\Dev\repos\wincraft\Cargo.toml".to_string(),
            svc_filter: String::new(),
            reg_key: r"HKCU\Environment".to_string(),
            last_message:
                "Ready — Cards view active (switchable to List/JSON, sortable, filterable, paginated, exportable)."
                    .to_string(),
            show_help_modal: false,
        }
    }
}

fn apply_craft_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    let ink = Color32::from_rgb(242, 246, 252);
    visuals.override_text_color = Some(ink);
    visuals.weak_text_color = Some(Color32::from_rgb(203, 213, 225));
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, ink);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, ink);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5, Color32::WHITE);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5, Color32::WHITE);
    visuals.panel_fill = Color32::from_rgb(13, 17, 23);
    visuals.window_fill = Color32::from_rgb(18, 23, 33);
    visuals.extreme_bg_color = Color32::from_rgb(9, 12, 17);
    visuals.faint_bg_color = Color32::from_rgb(22, 29, 41);
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(20, 26, 36);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(42, 54, 74));
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(28, 37, 52);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(38, 56, 82);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
    visuals.widgets.active.bg_fill = Color32::from_rgb(14, 116, 144);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);
    visuals.selection.bg_fill = Color32::from_rgb(14, 116, 144);
    visuals.selection.stroke = Stroke::new(1.0, Color32::from_rgb(103, 232, 249));
    ctx.set_visuals(visuals);
}

fn badge(ui: &mut egui::Ui, text: &str, fg: Color32, bg: Color32) {
    egui::Frame::new()
        .fill(bg)
        .stroke(Stroke::new(1.0, fg.gamma_multiply(0.5)))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(fg).small().strong());
        });
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

    fn open_external_file(&mut self, path: &std::path::Path) {
        if !path.exists() {
            self.last_message = format!("File not found: {}", path.display());
            return;
        }
        let res = ProcessCommand::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .spawn();
        match res {
            Ok(_) => {
                self.last_message = format!("Opened {}", path.display());
            }
            Err(e) => {
                self.last_message = format!("Failed to open {}: {e}", path.display());
            }
        }
    }

    fn handle_pending_action(&mut self, action: PendingAction) {
        match action {
            PendingAction::BrowseMount(mount) => {
                self.target_path = mount.clone();
                self.page = Page::FileOps;
                self.fileops_json = self
                    .exec(&Command::FsList {
                        path: PathBuf::from(&mount),
                        recursive: false,
                        depth: 1,
                    })
                    .ok();
            }
            PendingAction::LargeFilesMount(mount) => {
                self.target_path = mount.clone();
                self.page = Page::FileOps;
                self.fileops_json = self
                    .exec(&Command::FsLarge {
                        path: PathBuf::from(&mount),
                        min_size_bytes: 10 * 1024 * 1024,
                        limit: 50,
                        skip_build_dirs: true,
                    })
                    .ok();
            }
            PendingAction::DupsMount(mount) => {
                self.target_path = mount.clone();
                self.page = Page::FileOps;
                self.fileops_json = self
                    .exec(&Command::FsDups {
                        path: PathBuf::from(&mount),
                        skip_build_dirs: true,
                    })
                    .ok();
            }
            PendingAction::InspectLockPath(path) => {
                self.lock_path = path.clone();
                self.page = Page::LockInspector;
                self.lock_json = self
                    .exec(&Command::LockInspect {
                        path: PathBuf::from(&path),
                    })
                    .ok();
            }
            PendingAction::HashFilePath(path) => {
                if let Ok(res) = self.exec(&Command::FsHash {
                    path: PathBuf::from(&path),
                }) {
                    let sha = res
                        .get("sha256")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    self.last_message = format!("SHA-256 ({path}): {sha}");
                }
            }
            PendingAction::FilterProcessPid(pid) => {
                self.table_filter = pid;
                self.page_num = 1;
            }
            PendingAction::ReapPid(pid) => {
                if let Ok(res) = self.exec(&Command::WeederReap {
                    pid,
                    confirm: self.arm_confirm,
                }) {
                    self.last_message = format!("Reaped PID {pid}: {res}");
                    self.weeder_json = self.exec(&Command::WeederScan).ok();
                }
            }
            PendingAction::ServiceControl { name, action } => {
                if let Ok(res) = self.exec(&Command::SvcControl {
                    name: name.clone(),
                    action: action.clone(),
                    confirm: self.arm_confirm,
                }) {
                    let ok = res
                        .get("success")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    self.last_message = format!("Service {name} ({action}): success={ok}");
                }
            }
        }
    }

    fn render_kpi_ribbon(&mut self, ui: &mut egui::Ui) {
        let (disks_count, max_disk_pct, procs_count, weeder_count, weeder_mb, dead_path_count) =
            if let Some(ref st) = self.status_json {
                let disks = st.get("disks").and_then(|v| v.as_array());
                let d_cnt = disks.map(|a| a.len()).unwrap_or(0);
                let max_pct = disks
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|d| d.get("used_percent").and_then(|v| v.as_f64()))
                            .fold(0.0_f64, f64::max)
                    })
                    .unwrap_or(0.0);
                let p_cnt = st
                    .get("running_processes")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let w_cnt = st
                    .pointer("/weeder_summary/candidate_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let w_mb = st
                    .pointer("/weeder_summary/reclaimable_mb")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let dp_cnt = st
                    .pointer("/path_health/dead_entries_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                (d_cnt, max_pct, p_cnt, w_cnt, w_mb, dp_cnt)
            } else {
                (0, 0.0, 0, 0, 0.0, 0)
            };

        ui.horizontal_wrapped(|ui| {
            let disk_color = if max_disk_pct >= 95.0 {
                Color32::from_rgb(248, 113, 113)
            } else if max_disk_pct >= 85.0 {
                Color32::from_rgb(251, 191, 36)
            } else {
                Color32::from_rgb(52, 211, 153)
            };
            badge(
                ui,
                &format!("🖴 {disks_count} Volumes (peak {max_disk_pct:.1}%)"),
                disk_color,
                Color32::from_rgb(20, 30, 42),
            );
            badge(
                ui,
                &format!("⚡ {procs_count} Processes"),
                Color32::from_rgb(56, 189, 248),
                Color32::from_rgb(17, 28, 44),
            );
            let weeder_color = if weeder_count > 0 {
                Color32::from_rgb(251, 191, 36)
            } else {
                Color32::from_rgb(52, 211, 153)
            };
            badge(
                ui,
                &format!("🧹 Weeder: {weeder_count} orphans ({weeder_mb:.1} MB)"),
                weeder_color,
                Color32::from_rgb(28, 25, 18),
            );
            let path_color = if dead_path_count > 0 {
                Color32::from_rgb(251, 191, 36)
            } else {
                Color32::from_rgb(52, 211, 153)
            };
            badge(
                ui,
                &format!("🛣 Dead PATH entries: {dead_path_count}"),
                path_color,
                Color32::from_rgb(22, 27, 36),
            );
            if self.opts.read_only {
                badge(
                    ui,
                    "🛡 READ-ONLY LOCKED",
                    Color32::from_rgb(251, 191, 36),
                    Color32::from_rgb(45, 30, 12),
                );
            } else if self.arm_confirm {
                badge(
                    ui,
                    "⚠ MUTATIONS ARMED (confirm=true)",
                    Color32::from_rgb(248, 113, 113),
                    Color32::from_rgb(48, 18, 22),
                );
            } else {
                badge(
                    ui,
                    "🛡 SAFE MODE (confirm=false)",
                    Color32::from_rgb(52, 211, 153),
                    Color32::from_rgb(14, 36, 28),
                );
            }
        });
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
        self.page_num = self.page_num.max(1);

        let (mut paged_tab, total_rows, total_pages) = filter_sort_paginate(
            tab.clone(),
            Some(self.table_filter.as_str()),
            Some(self.sort_col),
            self.sort_asc,
            self.page_num,
            self.page_size,
        );
        if self.page_num > total_pages {
            self.page_num = total_pages.max(1);
            let (repaged, _, _) = filter_sort_paginate(
                tab.clone(),
                Some(self.table_filter.as_str()),
                Some(self.sort_col),
                self.sort_asc,
                self.page_num,
                self.page_size,
            );
            paged_tab = repaged;
        }

        // Control Bar 1: View Mode (Cards [default] | List | JSON), Filter, and Export (CSV | MD | TXT | JSON)
        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("View:").strong());
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
                ui.label(RichText::new("🔍 Filter:").strong());
                let filter_resp = ui.text_edit_singleline(&mut self.table_filter);
                if filter_resp.changed() {
                    self.page_num = 1;
                }
                if !self.table_filter.is_empty() && ui.button("✕ Clear").clicked() {
                    self.table_filter.clear();
                    self.page_num = 1;
                }

                ui.separator();
                ui.label(RichText::new("📤 Export:").strong());
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
                if ui.button("📂 Open Exports Folder").clicked() {
                    let _ = std::fs::create_dir_all(&self.export_dir);
                    let dir = self.export_dir.clone();
                    self.open_external_file(&dir);
                }
            });

            // Control Bar 2: Sorting & Pagination
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("⇅ Sort:").strong());
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
                ui.label(RichText::new("Rows/Page:").strong());
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
                if ui.button("⏮ First").clicked() {
                    self.page_num = 1;
                }
                if ui.button("◀ Prev").clicked() && self.page_num > 1 {
                    self.page_num -= 1;
                }
                badge(
                    ui,
                    &format!(
                        "Page {} / {} ({} of {} items)",
                        self.page_num,
                        total_pages,
                        paged_tab.rows.len(),
                        total_rows
                    ),
                    Color32::from_rgb(103, 232, 249),
                    Color32::from_rgb(15, 28, 42),
                );
                if ui.button("Next ▶").clicked() && self.page_num < total_pages {
                    self.page_num += 1;
                }
                if ui.button("Last ⏭").clicked() {
                    self.page_num = total_pages;
                }
            });
        });

        ui.add_space(6.0);

        let mut pending: Option<PendingAction> = None;

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
                    .spacing([14.0, 7.0])
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
                            if ui
                                .button(RichText::new(format!("{h}{arrow}")).strong())
                                .clicked()
                            {
                                if self.sort_col == idx {
                                    self.sort_asc = !self.sort_asc;
                                } else {
                                    self.sort_col = idx;
                                    self.sort_asc = true;
                                }
                                self.page_num = 1;
                            }
                        }
                        ui.label(RichText::new("Quick Actions").strong());
                        ui.end_row();

                        for row in &paged_tab.rows {
                            for cell in row {
                                ui.monospace(cell);
                            }
                            ui.horizontal(|ui| {
                                Self::render_row_quick_actions(ui, dataset_slug, row, &mut pending);
                            });
                            ui.end_row();
                        }
                    });
            }
            OutputFormat::Cards | OutputFormat::Csv | OutputFormat::Md | OutputFormat::Txt => {
                if paged_tab.rows.is_empty() {
                    ui.label(
                        RichText::new("No items to display (empty dataset or filter matched nothing). Try clearing the filter or switching to List view.")
                            .color(Color32::from_rgb(148, 163, 184)),
                    );
                }
                let start_idx = (self.page_num.saturating_sub(1)) * self.page_size + 1;
                // Responsive columns: 1 column on narrow windows, 2 on wide.
                // Fixed 510px min-width previously pushed the 2nd column off-screen
                // (vertical-only ScrollArea => clipped cards looked like "cards broken").
                let avail = ui.available_width();
                let cols = if avail < 720.0 { 1 } else { 2 };
                let card_w =
                    ((avail - (cols as f32 - 1.0) * 14.0) / cols as f32 - 18.0).clamp(260.0, 620.0);
                egui::Grid::new(format!("cards_{dataset_slug}"))
                    .num_columns(cols)
                    .spacing([14.0, 12.0])
                    .show(ui, |ui| {
                        for (i, row) in paged_tab.rows.iter().enumerate() {
                            ui.group(|ui| {
                                ui.set_min_width(card_w);
                                ui.set_max_width(card_w);
                                // Grid cells are LeftToRight: force a vertical
                                // stack or every block lands side-by-side and
                                // field rows collapse into a sliver.
                                ui.vertical(|ui| {
                                    let title_col = row.first().map(|s| s.as_str()).unwrap_or("-");
                                    let second_col = row.get(1).map(|s| s.as_str()).unwrap_or("");

                                    // Card Header with status badges
                                    ui.horizontal_wrapped(|ui| {
                                        badge(
                                            ui,
                                            &format!("#{}", start_idx + i),
                                            Color32::from_rgb(103, 232, 249),
                                            Color32::from_rgb(14, 33, 48),
                                        );
                                        ui.label(
                                            RichText::new(title_col)
                                                .size(15.5)
                                                .strong()
                                                .color(Color32::from_rgb(240, 246, 252)),
                                        );
                                        if !second_col.is_empty() {
                                            ui.label(
                                                RichText::new(format!("· {second_col}"))
                                                    .monospace()
                                                    .color(Color32::from_rgb(148, 163, 184)),
                                            );
                                        }
                                        Self::render_card_badges(ui, dataset_slug, row);
                                    });

                                    // Optional visual gauge for Disks & Processes
                                    Self::render_card_gauge(ui, dataset_slug, row);

                                    ui.separator();
                                    for (h, val) in paged_tab.headers.iter().zip(row.iter()) {
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(
                                                RichText::new(format!("{h}:"))
                                                    .color(Color32::from_rgb(148, 163, 184)),
                                            );
                                            ui.monospace(
                                                RichText::new(val)
                                                    .color(Color32::from_rgb(226, 232, 240)),
                                            );
                                        });
                                    }

                                    // Contextual Quick Actions footer
                                    ui.separator();
                                    ui.horizontal_wrapped(|ui| {
                                        Self::render_row_quick_actions(
                                            ui,
                                            dataset_slug,
                                            row,
                                            &mut pending,
                                        );
                                    });
                                }); // end vertical card stack
                            });
                            if (i + 1) % cols == 0 {
                                ui.end_row();
                            }
                        }
                        // Close a trailing incomplete row so a lone last card still renders.
                        if paged_tab.rows.len() % cols != 0 {
                            ui.end_row();
                        }
                    });
            }
        }

        if let Some(act) = pending {
            self.handle_pending_action(act);
        }
    }

    fn render_card_badges(ui: &mut egui::Ui, dataset_slug: &str, row: &[String]) {
        match dataset_slug {
            "services" => {
                let state = row.get(2).map(|s| s.as_str()).unwrap_or("");
                if state.eq_ignore_ascii_case("RUNNING") {
                    badge(
                        ui,
                        "● RUNNING",
                        Color32::from_rgb(52, 211, 153),
                        Color32::from_rgb(12, 38, 28),
                    );
                } else {
                    badge(
                        ui,
                        state,
                        Color32::from_rgb(148, 163, 184),
                        Color32::from_rgb(30, 36, 46),
                    );
                }
            }
            "processes" => {
                let prot = row.get(5).map(|s| s.as_str()).unwrap_or("false");
                if prot == "true" {
                    badge(
                        ui,
                        "🛡 Protected Host Process",
                        Color32::from_rgb(251, 191, 36),
                        Color32::from_rgb(44, 32, 12),
                    );
                }
            }
            "ports" => {
                let fleet_svc = row.get(5).map(|s| s.as_str()).unwrap_or("");
                if !fleet_svc.is_empty() && fleet_svc != "-" {
                    badge(
                        ui,
                        &format!("★ Fleet: {fleet_svc}"),
                        Color32::from_rgb(251, 191, 36),
                        Color32::from_rgb(46, 34, 12),
                    );
                }
            }
            _ => {}
        }
    }

    fn render_card_gauge(ui: &mut egui::Ui, dataset_slug: &str, row: &[String]) {
        if dataset_slug == "disks" {
            if let Some(pct) = row.get(1).and_then(|s| s.parse::<f32>().ok()) {
                let frac = (pct / 100.0).clamp(0.0, 1.0);
                let fill = if pct >= 95.0 {
                    Color32::from_rgb(239, 68, 68)
                } else if pct >= 85.0 {
                    Color32::from_rgb(245, 158, 11)
                } else {
                    Color32::from_rgb(16, 185, 129)
                };
                let free_gb = row.get(2).map(|s| s.as_str()).unwrap_or("?");
                let total_gb = row.get(3).map(|s| s.as_str()).unwrap_or("?");
                ui.add(egui::ProgressBar::new(frac).fill(fill).text(format!(
                    "{pct:.1}% used ({free_gb} GB free of {total_gb} GB)"
                )));
            }
        } else if dataset_slug == "processes" || dataset_slug == "weeder" {
            if let Some(mem_mb) = row.get(2).and_then(|s| s.parse::<f32>().ok()) {
                let frac = (mem_mb / 4096.0).clamp(0.02, 1.0);
                ui.add(
                    egui::ProgressBar::new(frac)
                        .fill(Color32::from_rgb(14, 165, 233))
                        .text(format!("{mem_mb:.1} MB working set")),
                );
            }
        }
    }

    fn render_row_quick_actions(
        ui: &mut egui::Ui,
        dataset_slug: &str,
        row: &[String],
        pending: &mut Option<PendingAction>,
    ) {
        match dataset_slug {
            "disks" => {
                if let Some(mount) = row.first() {
                    if ui.small_button("📂 Browse").clicked() {
                        *pending = Some(PendingAction::BrowseMount(mount.clone()));
                    }
                    if ui.small_button("🔍 Large Files").clicked() {
                        *pending = Some(PendingAction::LargeFilesMount(mount.clone()));
                    }
                    if ui.small_button("🧬 Duplicates").clicked() {
                        *pending = Some(PendingAction::DupsMount(mount.clone()));
                    }
                }
            }
            "processes" | "weeder" => {
                let pid_str = row.first().cloned().unwrap_or_default();
                let prot = row.get(5).map(|s| s.as_str()).unwrap_or("false") == "true";
                if ui.small_button("🔎 Filter PID").clicked() {
                    *pending = Some(PendingAction::FilterProcessPid(pid_str.clone()));
                }
                if !prot {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        if ui.small_button("🧹 Reap PID").clicked() {
                            *pending = Some(PendingAction::ReapPid(pid));
                        }
                    }
                }
            }
            "audit" => {
                let pid_str = row.first().cloned().unwrap_or_default();
                let status_str = row.get(2).map(|s| s.as_str()).unwrap_or("");
                if ui.small_button("🔎 Filter PID").clicked() {
                    *pending = Some(PendingAction::FilterProcessPid(pid_str.clone()));
                }
                if status_str.contains("UNAUTHORIZED") || status_str.contains("SUSPICIOUS") {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        if ui.small_button("🧹 Reap Rogue PID").clicked() {
                            *pending = Some(PendingAction::ReapPid(pid));
                        }
                    }
                }
            }
            "ports" => {
                if let Some(pid_str) = row.get(4) {
                    if ui.small_button("🔎 Filter Owning PID").clicked() {
                        *pending = Some(PendingAction::FilterProcessPid(pid_str.clone()));
                    }
                }
            }
            "fileops" => {
                // Path is column 0 in items/files/matches
                if let Some(p) = row.first() {
                    if p.contains('\\') || p.contains('/') {
                        if ui.small_button("🔒 Who Locks This?").clicked() {
                            *pending = Some(PendingAction::InspectLockPath(p.clone()));
                        }
                        if ui.small_button("#️⃣ SHA-256").clicked() {
                            *pending = Some(PendingAction::HashFilePath(p.clone()));
                        }
                    }
                }
            }
            "services" => {
                if let Some(svc_name) = row.first() {
                    if ui.small_button("🔄 Restart (sc.exe)").clicked() {
                        *pending = Some(PendingAction::ServiceControl {
                            name: svc_name.clone(),
                            action: "restart".to_string(),
                        });
                    }
                    if ui.small_button("▶ Start").clicked() {
                        *pending = Some(PendingAction::ServiceControl {
                            name: svc_name.clone(),
                            action: "start".to_string(),
                        });
                    }
                    if ui.small_button("⏹ Stop").clicked() {
                        *pending = Some(PendingAction::ServiceControl {
                            name: svc_name.clone(),
                            action: "stop".to_string(),
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

impl eframe::App for WinCraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !self.theme_applied {
            apply_craft_theme(ui.ctx());
            self.theme_applied = true;
        }

        // Top Header Bar
        ui.horizontal_wrapped(|ui| {
            ui.heading(
                RichText::new("⚡ WinCraft v0.1.0")
                    .color(Color32::from_rgb(103, 232, 249))
                    .strong(),
            );
            ui.separator();
            ui.checkbox(&mut self.opts.read_only, "Read-Only Guard");
            ui.checkbox(&mut self.arm_confirm, "Arm Confirm Gate");
            ui.separator();
            if ui.button("🔄 Refresh Telemetry").clicked() {
                self.status_json = self.exec(&Command::Status).ok();
                self.weeder_json = self.exec(&Command::WeederScan).ok();
            }
            if ui.button("📖 PDF Brochure").clicked() {
                let p = self.brochure_dir.join("wincraft-brochure.pdf");
                self.open_external_file(&p);
            }
            if ui.button("📚 EPUB Brochure").clicked() {
                let p = self.brochure_dir.join("wincraft-brochure.epub");
                self.open_external_file(&p);
            }
            if ui
                .button(if self.show_help_modal {
                    "✕ Hide Safety"
                } else {
                    "? Safety & Architecture"
                })
                .clicked()
            {
                self.show_help_modal = !self.show_help_modal;
            }
            ui.separator();
            ui.label(
                RichText::new(&self.last_message)
                    .color(Color32::from_rgb(167, 243, 208))
                    .monospace(),
            );
        });

        ui.add_space(3.0);
        self.render_kpi_ribbon(ui);
        ui.separator();

        // Navigation Bar
        ui.horizontal_wrapped(|ui| {
            for p in [
                Page::Dashboard,
                Page::ProcsAndPorts,
                Page::CpuAuditAndThermals,
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
                ui.heading("WinCraft Safety Invariants & ArtCraft 3-Tier Architecture");
                ui.label("WinCraft subsumes windows-operations-mcp, system-admin-mcp, sysinternals-mcp, filesystem-mcp, and winrar-mcp into one pure-Rust ArtCraft 3-tier binary.");
                ui.label("1. Default View is Cards, switchable at any time to List/Table or JSON, with live filtering, numeric-aware sorting, pagination, and CSV/MD/TXT/JSON export.");
                ui.label("2. Protected system processes (csrss, lsass, svchost, explorer, nssm, pwsh, etc.) can never be reaped.");
                ui.label("3. Every file edit creates a timestamped .bak backup; every HKCU registry edit exports a .reg snapshot first.");
                ui.label("4. NSSM services are controlled strictly via sc.exe stop/start.");
            });
        }

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| match self.page {
            Page::Dashboard => {
                ui.heading("Host Overview & Logical Disk Volumes (Click a Card Action to Inspect in FileOps)");
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
                    if ui.button("⚡ Scan Processes").clicked() {
                        let filter = if self.proc_filter.trim().is_empty() {
                            None
                        } else {
                            Some(self.proc_filter.trim().to_string())
                        };
                        self.procs_json =
                            self.exec(&Command::ProcList { filter, limit: 250 }).ok();
                    }
                    ui.separator();
                    ui.label("Port:");
                    ui.text_edit_singleline(&mut self.port_filter);
                    if ui.button("🔌 Scan Listening Ports").clicked() {
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
                    ui.label(RichText::new("Listening TCP/UDP Ports:").strong());
                    self.render_dataset_controls_and_body(ui, "ports", &ports);
                }
                if let Some(procs) = self.procs_json.clone() {
                    ui.separator();
                    ui.label(RichText::new("Running Processes:").strong());
                    self.render_dataset_controls_and_body(ui, "processes", &procs);
                }
            }
            Page::CpuAuditAndThermals => {
                ui.heading("CPU Hogger & Compute Audit + Host Thermals (Rogue Process & Miner Detector)");
                ui.horizontal_wrapped(|ui| {
                    if ui.button("🔥 Run CPU Hogger Audit").clicked() {
                        let filter = if self.cpu_filter.trim().is_empty() {
                            None
                        } else {
                            Some(self.cpu_filter.trim().to_string())
                        };
                        self.cpu_audit_json = self
                            .exec(&Command::CpuAudit {
                                filter,
                                min_cpu_seconds: Some(self.min_cpu_sec),
                                min_cpu_percent: Some(self.min_cpu_pct),
                            })
                            .ok();
                    }
                    if ui.button("🌡 Query Host Thermals").clicked() {
                        self.thermal_json = self.exec(&Command::ThermalStatus).ok();
                    }
                    ui.separator();
                    ui.label("Filter:");
                    ui.text_edit_singleline(&mut self.cpu_filter);
                    ui.label("Min CPU (s):");
                    ui.add(
                        egui::DragValue::new(&mut self.min_cpu_sec)
                            .range(0.0..=3600.0)
                            .speed(5.0),
                    );
                    ui.label("Min CPU (%):");
                    ui.add(
                        egui::DragValue::new(&mut self.min_cpu_pct)
                            .range(0.0..=100.0)
                            .speed(1.0),
                    );
                });
                if let Some(ref therm) = self.thermal_json.clone() {
                    ui.separator();
                    ui.label(RichText::new("Host Processor & Thermal Metrics:").strong());
                    self.render_dataset_controls_and_body(ui, "thermal", therm);
                }
                if let Some(ref audit) = self.cpu_audit_json.clone() {
                    ui.separator();
                    ui.label(
                        RichText::new("Compute Audit Candidates & Rogue Process Detection:")
                            .strong(),
                    );
                    self.render_dataset_controls_and_body(ui, "audit", audit);
                }
            }
            Page::RamWeeder => {
                ui.heading("RAM Weeder — Orphan Cargo Test & Zombie Engine Detector");
                ui.horizontal(|ui| {
                    if ui.button("🧹 Run Weeder Scan").clicked() {
                        self.weeder_json = self.exec(&Command::WeederScan).ok();
                    }
                    ui.label("Tip: Enable 'Arm Confirm Gate' in the top bar before clicking 'Reap PID' on an orphan process card.");
                });
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
                ui.horizontal_wrapped(|ui| {
                    let p = PathBuf::from(self.target_path.trim());
                    if ui.button("📂 List Directory").clicked() {
                        self.fileops_json = self
                            .exec(&Command::FsList {
                                path: p.clone(),
                                recursive: true,
                                depth: 2,
                            })
                            .ok();
                    }
                    if ui.button("📊 Disk Usage (DU)").clicked() {
                        self.fileops_json = self
                            .exec(&Command::FsDu {
                                path: p.clone(),
                                skip_build_dirs: true,
                            })
                            .ok();
                    }
                    if ui.button("🧬 Find Duplicates (3-Stage SHA-256)").clicked() {
                        self.fileops_json = self
                            .exec(&Command::FsDups {
                                path: p.clone(),
                                skip_build_dirs: true,
                            })
                            .ok();
                    }
                    if ui.button("🐘 Find Largest Files").clicked() {
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
                    if ui.button("🔍 Search Text").clicked() {
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
                    if ui.button("🔒 Inspect Locks").clicked() {
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
                    if ui.button("⚙ Query Services").clicked() {
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
                    if ui.button("🚀 List Startup Autoruns").clicked() {
                        self.startup_json = self.exec(&Command::StartupList).ok();
                    }
                    if ui.button("📜 Tail System Event Log").clicked() {
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
                    if ui.button("🔑 Read Key").clicked() {
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
                ui.heading("Toolchain Doctor, Brochure Assets & 28 MCP Tools Catalog");
                ui.horizontal_wrapped(|ui| {
                    if ui.button("🩺 Re-run Doctor").clicked() {
                        self.doctor_json = self.exec(&Command::Doctor).ok();
                    }
                    if ui.button("📖 Open PDF Brochure").clicked() {
                        let p = self.brochure_dir.join("wincraft-brochure.pdf");
                        self.open_external_file(&p);
                    }
                    if ui.button("📚 Open EPUB Brochure").clicked() {
                        let p = self.brochure_dir.join("wincraft-brochure.epub");
                        self.open_external_file(&p);
                    }
                    if ui.button("🌐 Open HTML Brochure").clicked() {
                        let p = self.brochure_dir.join("wincraft-brochure.html");
                        self.open_external_file(&p);
                    }
                });
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
            .with_inner_size([1280.0, 820.0]),
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

    /// Headless layout regression test for the Cards grid (no GPU needed:
    /// `run_pass` only produces shapes). Fails if a card's field label and
    /// value wrap onto separate lines (the "vertical sliver" bug) instead of
    /// sharing one row.
    #[test]
    fn cards_grid_field_label_and_value_share_a_line() {
        let payload = json!({
            "volumes": [
                {"mount": "C:\\", "used_percent": "94.7", "free_gb": "98.07", "total_gb": "1862.2"},
                {"mount": "D:\\", "used_percent": "93.4", "free_gb": "122.75", "total_gb": "1862.91"}
            ]
        });
        let mut app = WinCraftApp {
            view_mode: OutputFormat::Cards,
            ..Default::default()
        };

        let ctx = egui::Context::default();
        let make_raw = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1920.0, 1080.0),
            )),
            ..Default::default()
        };
        // Passes 1-2 warm fonts/layout; the final pass is inspected.
        for _ in 0..2 {
            let output = ctx.run_pass(make_raw(), |ctx| {
                egui::Area::new(egui::Id::unique("cards_layout_test"))
                    .fixed_pos(egui::Pos2::ZERO)
                    .show(ctx, |ui| {
                        ui.set_width(1920.0);
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            app.render_dataset_controls_and_body(ui, "disks", &payload);
                        });
                    });
            });
            std::mem::forget(output.textures_delta);
        }
        // Re-run once more to collect positions for the assertions below.
        let output = ctx.run_pass(make_raw(), |ctx| {
            egui::Area::new(egui::Id::unique("cards_layout_test"))
                .fixed_pos(egui::Pos2::ZERO)
                .show(ctx, |ui| {
                    ui.set_width(1920.0);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        app.render_dataset_controls_and_body(ui, "disks", &payload);
                    });
                });
        });
        std::mem::forget(output.textures_delta);

        let mut mount_pos: Option<(f32, f32)> = None;
        let mut values: Vec<(f32, f32)> = Vec::new();
        let mut all: Vec<String> = Vec::new();
        for clipped in &output.shapes {
            if let egui::Shape::Text(ts) = &clipped.shape {
                let text = ts.galley.text();
                all.push(format!(
                    "y={:6.1} x={:6.1} w={:6.1} h={:5.1} {text:?}",
                    ts.pos.y,
                    ts.pos.x,
                    ts.galley.size().x,
                    ts.galley.size().y
                ));
                if text == "Mount:" && mount_pos.is_none() {
                    mount_pos = Some((ts.pos.x, ts.pos.y));
                } else if text == "C:\\" {
                    values.push((ts.pos.x, ts.pos.y));
                }
            }
        }
        all.sort();
        assert!(
            mount_pos.is_some() && !values.is_empty(),
            "expected the 'Mount:' label and a 'C:\\' value to be laid out"
        );
        let (mx, my) = mount_pos.unwrap_or((0.0, 0.0));
        // The field value must sit on the same line, just right of its label.
        // (Matching by y alone is ambiguous: the card title is also "C:\\".)
        let shares_line = values
            .iter()
            .any(|(x, y)| (y - my).abs() < 4.0 && *x - mx > 10.0 && *x - mx < 400.0);
        assert!(
            shares_line,
            "card field label at ({mx:.1}, {my:.1}) has no value sharing its line; layout:\n{}",
            all.join("\n")
        );
    }

    /// Throwaway bisection harness (not part of the suite): minimal
    /// group-in-grid card that dumps text geometry so the wrap collapse
    /// can be bisected without touching prod code paths.
    #[test]
    fn cards_minimal_repro_probes() {
        let ctx = egui::Context::default();
        let make_raw = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1920.0, 1080.0),
            )),
            ..Default::default()
        };
        for _ in 0..3 {
            let output = ctx.run_pass(make_raw(), |ctx| {
                egui::Area::new(egui::Id::unique("cards_min_repro"))
                    .fixed_pos(egui::Pos2::ZERO)
                    .show(ctx, |ui| {
                        ui.set_width(1920.0);
                        ui.horizontal(|ui| {
                            eprintln!("H-ROW avail0={:.1}", ui.available_width());
                            ui.label("AAA");
                            eprintln!("H-ROW avail1={:.1}", ui.available_width());
                            ui.label("BBB");
                            eprintln!("H-ROW avail2={:.1}", ui.available_width());
                        });
                        ui.horizontal_wrapped(|ui| {
                            eprintln!("W-ROW avail0={:.1}", ui.available_width());
                            ui.label("AAA");
                            eprintln!("W-ROW avail1={:.1}", ui.available_width());
                            ui.label("BBB");
                            eprintln!("W-ROW avail2={:.1}", ui.available_width());
                        });
                        egui::Grid::new("repro_grid").num_columns(2).show(ui, |ui| {
                            for _ in 0..2 {
                                ui.group(|ui| {
                                    ui.set_min_width(620.0);
                                    ui.set_max_width(620.0);
                                    ui.vertical(|ui| {
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label("Mount:");
                                            ui.monospace("C:\\");
                                        });
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label("L2A");
                                            ui.label("L2B");
                                        });
                                        ui.horizontal_wrapped(|ui| {
                                            ui.small_button("BTN");
                                            ui.label("after-btn");
                                        });
                                    });
                                });
                                ui.end_row();
                            }
                        });
                    });
            });
            std::mem::forget(output.textures_delta);
            let mut dump: Vec<String> = Vec::new();
            for clipped in &output.shapes {
                if let egui::Shape::Text(ts) = &clipped.shape {
                    dump.push(format!(
                        "y={:6.1} x={:6.1} w={:6.1} {:?}",
                        ts.pos.y,
                        ts.pos.x,
                        ts.galley.size().x,
                        ts.galley.text()
                    ));
                }
            }
            dump.sort();
            for line in &dump {
                eprintln!("MINI {line}");
            }
        }
    }
}
