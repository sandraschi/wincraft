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

use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke, Vec2};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};
use wincraft_core::wiztree::{
    self, compute_hierarchical_treemap, DiskUsageSummary, LayoutRect, TreeNode, TreemapTile,
};
use wincraft_core::{
    dispatch, filter_sort_paginate, payload_to_tabular, render_tabular, tools_catalog, Command,
    ExecOptions, OutputFormat,
};

fn find_node_by_path<'a>(node: &'a TreeNode, target: &str) -> Option<&'a TreeNode> {
    if node.path.eq_ignore_ascii_case(target)
        || node
            .path
            .trim_end_matches(['\\', '/'])
            .eq_ignore_ascii_case(target.trim_end_matches(['\\', '/']))
    {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_node_by_path(child, target) {
            return Some(found);
        }
    }
    None
}

fn format_bytes(bytes: u64) -> String {
    let mb = bytes as f64 / 1_048_576.0;
    if mb >= 1024.0 {
        format!("{:.2} GB", mb / 1024.0)
    } else {
        format!("{:.1} MB", mb)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioDialog {
    CommandPalette,
    Shortcuts,
    Docs,
    Licenses,
    About,
    Safety,
    Preferences,
}

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
    DiskTreemap,
}

impl Page {
    pub const fn all() -> [Page; 10] {
        [
            Page::Dashboard,
            Page::ProcsAndPorts,
            Page::CpuAuditAndThermals,
            Page::RamWeeder,
            Page::FileOps,
            Page::LockInspector,
            Page::ServicesNssm,
            Page::StartupAndRegistry,
            Page::DoctorAndHelp,
            Page::DiskTreemap,
        ]
    }

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
            Page::DiskTreemap => "🗺 10. MFT Cushion Treemap",
        }
    }
}

pub static MENU_CATALOG: &[(&str, &str, &str)] = &[
    // File
    ("file_scan_mft", "Scan MFT Drive...", "Ctrl+M"),
    ("file_export_csv", "Export Current View (CSV)", "Ctrl+E"),
    ("file_export_md", "Export Current View (Markdown)", ""),
    ("file_export_txt", "Export Current View (Text)", ""),
    ("file_export_json", "Export Current View (JSON)", ""),
    ("file_open_backups", "Open Backups Folder", ""),
    ("file_pdf_brochure", "Open PDF Brochure", ""),
    ("file_epub_brochure", "Open EPUB Brochure", ""),
    ("file_html_brochure", "Open HTML Brochure", ""),
    ("file_relaunch_admin", "Relaunch as Admin (UAC)", ""),
    ("file_quit", "Quit", "Ctrl+Q"),
    // Edit
    ("edit_filter", "Filter Table / Cards...", "Ctrl+F"),
    ("edit_clear_filter", "Clear Filters", "Esc"),
    ("edit_toggle_readonly", "Toggle Read-Only Guard", ""),
    ("edit_toggle_confirm", "Toggle Arm Confirm Gate", ""),
    ("edit_preferences", "Preferences...", "Ctrl+,"),
    // View
    ("view_mode_cards", "View: Cards (Default)", ""),
    ("view_mode_table", "View: List / Table", ""),
    ("view_mode_json", "View: Raw JSON", ""),
    ("view_page_1", "1. Host & Disks", ""),
    ("view_page_2", "2. Processes & Ports", ""),
    ("view_page_3", "3. CPU Audit & Thermals", ""),
    ("view_page_4", "4. RAM Weeder", ""),
    ("view_page_5", "5. High-Speed FileOps", ""),
    ("view_page_6", "6. Lock Inspector", ""),
    ("view_page_7", "7. Services & NSSM", ""),
    ("view_page_8", "8. Startup & Registry", ""),
    ("view_page_9", "9. Doctor & Tools", ""),
    ("view_page_10", "10. MFT Cushion Treemap", ""),
    ("view_zoom_in", "Zoom In", "Ctrl+="),
    ("view_zoom_out", "Zoom Out", "Ctrl+-"),
    ("view_zoom_reset", "Reset Zoom (100%)", "Ctrl+1"),
    ("view_theme_dark", "Theme: Dark (Slate-950)", ""),
    ("view_theme_light", "Theme: Light", ""),
    // Commands / SysAdmin
    ("cmd_refresh_telemetry", "Refresh Telemetry & Metrics", "F5"),
    ("cmd_scan_procs", "Scan Running Processes", ""),
    ("cmd_scan_ports", "Scan Active Listening Ports", ""),
    ("cmd_scan_weeder", "Scan RAM Working Sets", ""),
    ("cmd_run_cpu_audit", "Run CPU Thermals & Audit", ""),
    ("cmd_run_doctor", "Verify Restart Manager Doctor", ""),
    ("cmd_scan_mft", "Scan Drive MFT Records", ""),
    (
        "cmd_treemap_browser",
        "Open Interactive D3 Treemap in Browser",
        "",
    ),
    // Window
    ("win_reset_layout", "Reset Panel Layout", ""),
    ("win_toggle_kpi", "Toggle Host KPI Ribbon", ""),
    ("win_toggle_safety", "Safety & Invariants Info...", ""),
    // Help
    ("help_palette", "Command Palette...", "Ctrl+K"),
    ("help_shortcuts", "Keyboard Shortcuts...", "F1"),
    ("help_getting_started", "Getting Started...", ""),
    ("help_usage", "Usage (GUI, CLI & MCP)...", ""),
    ("help_tips", "Tips & Tricks...", ""),
    ("help_pitfalls", "Common Pitfalls & Safety...", ""),
    ("help_planned", "Planned Extensions...", ""),
    ("help_catalog", "31 MCP Tools Reference...", ""),
    ("help_docs", "Project Files & Docs...", ""),
    ("help_licenses", "Licenses & Notices...", ""),
    ("help_about", "About WinCraft", ""),
];

fn switch_on_hover(ctx: &egui::Context, titles: &[egui::Response]) {
    let is_open =
        |t: &egui::Response| egui::Popup::is_id_open(ctx, egui::Popup::default_response_id(t));
    if !titles.iter().any(is_open) {
        return;
    }
    if let Some(title) = titles
        .iter()
        .find(|t| t.hovered() && !t.clicked() && !is_open(t))
    {
        egui::Popup::open_id(ctx, egui::Popup::default_response_id(title));
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
    dark_mode: bool,
    treemap_drive: String,
    treemap_summary: Option<DiskUsageSummary>,
    treemap_zoom_path: Option<String>,
    treemap_selected_tile: Option<TreemapTile>,
    active_dialog: Option<StudioDialog>,
    palette_query: String,
    docs_tab: &'static str,
    show_kpi_ribbon: bool,
    quit_requested: bool,
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
                "Ready -- Cards view active (switchable to List/JSON, sortable, filterable, paginated, exportable)."
                    .to_string(),
            dark_mode: true,
            treemap_drive: "C:\\".to_string(),
            treemap_summary: None,
            treemap_zoom_path: None,
            treemap_selected_tile: None,
            active_dialog: None,
            palette_query: String::new(),
            docs_tab: "start",
            show_kpi_ribbon: true,
            quit_requested: false,
        }
    }
}

fn harsh_visuals() -> egui::Visuals {
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
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(38, 54, 84));
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(28, 37, 52);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(48, 66, 102));
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(38, 56, 82);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.5, Color32::from_rgb(103, 232, 249));
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
    visuals.widgets.active.bg_fill = Color32::from_rgb(14, 116, 144);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);
    visuals.selection.bg_fill = Color32::from_rgb(14, 116, 144);
    visuals.selection.stroke = Stroke::new(1.0, Color32::from_rgb(103, 232, 249));
    visuals
}

fn crisp_light_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::light();
    let ink = Color32::from_rgb(15, 23, 42);
    visuals.override_text_color = Some(ink);
    visuals.weak_text_color = Some(Color32::from_rgb(51, 65, 85));
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, ink);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, ink);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5, ink);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5, Color32::WHITE);
    visuals.panel_fill = Color32::from_rgb(248, 250, 252);
    visuals.window_fill = Color32::WHITE;
    visuals.extreme_bg_color = Color32::from_rgb(241, 245, 249);
    visuals.faint_bg_color = Color32::from_rgb(226, 232, 240);
    visuals.widgets.noninteractive.bg_fill = Color32::WHITE;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(203, 213, 225));
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(226, 232, 240);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(148, 163, 184));
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(224, 242, 254);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.5, Color32::from_rgb(14, 165, 233));
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
    visuals.widgets.active.bg_fill = Color32::from_rgb(14, 116, 144);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);
    visuals.selection.bg_fill = Color32::from_rgb(186, 230, 253);
    visuals.selection.stroke = Stroke::new(1.0, Color32::from_rgb(2, 132, 199));
    visuals
}

fn apply_craft_theme(ctx: &egui::Context) {
    ctx.set_visuals(harsh_visuals());
}

fn handle_zoom(ctx: &egui::Context) {
    // egui 0.37: Ctrl+Wheel arrives as zoom_delta() != 1.0 (keyboard
    // Ctrl+Plus/Minus/0 is already handled natively by zoom_with_keyboard).
    let zoom_delta = ctx.input(|i| i.zoom_delta());
    if (zoom_delta - 1.0).abs() > f32::EPSILON {
        let next = (ctx.zoom_factor() * zoom_delta).clamp(0.6, 2.5);
        ctx.set_zoom_factor(next);
    }
}

fn theme_toggle_button(ui: &mut egui::Ui, dark_mode: &mut bool) -> bool {
    let size = egui::vec2(28.0, 28.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&resp);
        ui.painter().rect(
            rect,
            CornerRadius::same(6),
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let center = rect.center();
        let painter = ui.painter();
        if *dark_mode {
            let sun_color = Color32::from_rgb(251, 191, 36);
            painter.circle_filled(center, 4.5, sun_color);
            for i in 0..8 {
                let angle = (i as f32) * std::f32::consts::TAU / 8.0;
                let dir = egui::vec2(angle.cos(), angle.sin());
                painter.line_segment(
                    [center + dir * 6.5, center + dir * 9.2],
                    Stroke::new(1.6, sun_color),
                );
            }
        } else {
            let moon_color = Color32::from_rgb(51, 65, 85);
            painter.circle_filled(center, 6.5, moon_color);
            painter.circle_filled(center + egui::vec2(3.0, -2.2), 5.5, visuals.bg_fill);
        }
    }
    let clicked = resp.on_hover_text("Toggle Light / Dark theme").clicked();
    if clicked {
        *dark_mode = !*dark_mode;
        ui.ctx().set_visuals(if *dark_mode {
            harsh_visuals()
        } else {
            crisp_light_visuals()
        });
    }
    clicked
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

    fn menu_item(&mut self, ui: &mut egui::Ui, id: &str, label: &str) {
        if ui.button(label).clicked() {
            match self.menu_action(id) {
                Ok(_) => {}
                Err(e) => self.last_message = e,
            }
            ui.close();
        }
    }

    pub fn menu_action(&mut self, id: &str) -> Result<Value, String> {
        match id {
            "file_scan_mft" => {
                let drive = self.treemap_drive.clone();
                self.page = Page::DiskTreemap;
                match wiztree::scan_drive_usage(&drive, 5000) {
                    Ok(records) => {
                        let summary = wiztree::analyze_disk_records(&drive, &records, 50);
                        self.last_message = format!(
                            "MFT scan complete for {drive}: {} files ({:.2} GB)",
                            summary.total_files,
                            summary.total_size_bytes as f64 / 1_073_741_824.0
                        );
                        self.treemap_summary = Some(summary);
                        self.treemap_zoom_path = None;
                        self.treemap_selected_tile = None;
                        Ok(json!({ "scanned_drive": drive }))
                    }
                    Err(e) => {
                        let err = format!("MFT scan failed for {drive}: {e}");
                        self.last_message = err.clone();
                        Err(err)
                    }
                }
            }
            "file_export_csv" => {
                self.export_current_view(OutputFormat::Csv);
                Ok(json!({ "exported": "csv" }))
            }
            "file_export_md" => {
                self.export_current_view(OutputFormat::Md);
                Ok(json!({ "exported": "md" }))
            }
            "file_export_txt" => {
                self.export_current_view(OutputFormat::Txt);
                Ok(json!({ "exported": "txt" }))
            }
            "file_export_json" => {
                self.export_current_view(OutputFormat::Json);
                Ok(json!({ "exported": "json" }))
            }
            "file_open_backups" => {
                let _ = std::fs::create_dir_all(&self.export_dir);
                let dir = self.export_dir.clone();
                self.open_external_file(&dir);
                Ok(json!({ "opened": dir.display().to_string() }))
            }
            "file_pdf_brochure" => {
                let p = self.brochure_dir.join("wincraft-brochure.pdf");
                self.open_external_file(&p);
                Ok(json!({ "brochure": "pdf" }))
            }
            "file_epub_brochure" => {
                let p = self.brochure_dir.join("wincraft-brochure.epub");
                self.open_external_file(&p);
                Ok(json!({ "brochure": "epub" }))
            }
            "file_html_brochure" => {
                let p = self.brochure_dir.join("wincraft-brochure.html");
                self.open_external_file(&p);
                Ok(json!({ "brochure": "html" }))
            }
            "file_relaunch_admin" => {
                if let Ok(exe) = std::env::current_exe() {
                    let _ = ProcessCommand::new("powershell")
                        .args([
                            "-Command",
                            &format!("Start-Process '{}' -Verb RunAs", exe.display()),
                        ])
                        .spawn();
                    self.quit_requested = true;
                    Ok(json!({ "relaunch_admin": true }))
                } else {
                    Err("Failed to determine current executable path".to_string())
                }
            }
            "file_quit" => {
                self.quit_requested = true;
                Ok(json!({ "quit": true }))
            }
            "edit_filter" => {
                self.last_message = "Filter active. Type query in search field.".to_string();
                Ok(json!({ "filter": true }))
            }
            "edit_clear_filter" => {
                self.table_filter.clear();
                self.page_num = 1;
                Ok(json!({ "clear_filter": true }))
            }
            "edit_toggle_readonly" => {
                self.opts.read_only = !self.opts.read_only;
                self.last_message = format!(
                    "Read-Only Guard: {}",
                    if self.opts.read_only {
                        "ACTIVE"
                    } else {
                        "INACTIVE"
                    }
                );
                Ok(json!({ "read_only": self.opts.read_only }))
            }
            "edit_toggle_confirm" => {
                self.arm_confirm = !self.arm_confirm;
                self.last_message = format!(
                    "Arm Confirm Gate: {}",
                    if self.arm_confirm {
                        "ARMED"
                    } else {
                        "DISARMED"
                    }
                );
                Ok(json!({ "arm_confirm": self.arm_confirm }))
            }
            "edit_preferences" => {
                self.active_dialog = Some(StudioDialog::Preferences);
                Ok(json!({ "dialog": "preferences" }))
            }
            "view_mode_cards" => {
                self.view_mode = OutputFormat::Cards;
                Ok(json!({ "view_mode": "cards" }))
            }
            "view_mode_table" => {
                self.view_mode = OutputFormat::List;
                Ok(json!({ "view_mode": "table" }))
            }
            "view_mode_json" => {
                self.view_mode = OutputFormat::Json;
                Ok(json!({ "view_mode": "json" }))
            }
            "view_page_1" => {
                self.page = Page::Dashboard;
                self.page_num = 1;
                self.sort_col = 0;
                Ok(json!({ "page": 1 }))
            }
            "view_page_2" => {
                self.page = Page::ProcsAndPorts;
                self.page_num = 1;
                self.sort_col = 0;
                Ok(json!({ "page": 2 }))
            }
            "view_page_3" => {
                self.page = Page::CpuAuditAndThermals;
                self.page_num = 1;
                self.sort_col = 0;
                Ok(json!({ "page": 3 }))
            }
            "view_page_4" => {
                self.page = Page::RamWeeder;
                self.page_num = 1;
                self.sort_col = 0;
                Ok(json!({ "page": 4 }))
            }
            "view_page_5" => {
                self.page = Page::FileOps;
                self.page_num = 1;
                self.sort_col = 0;
                Ok(json!({ "page": 5 }))
            }
            "view_page_6" => {
                self.page = Page::LockInspector;
                self.page_num = 1;
                self.sort_col = 0;
                Ok(json!({ "page": 6 }))
            }
            "view_page_7" => {
                self.page = Page::ServicesNssm;
                self.page_num = 1;
                self.sort_col = 0;
                Ok(json!({ "page": 7 }))
            }
            "view_page_8" => {
                self.page = Page::StartupAndRegistry;
                self.page_num = 1;
                self.sort_col = 0;
                Ok(json!({ "page": 8 }))
            }
            "view_page_9" => {
                self.page = Page::DoctorAndHelp;
                self.page_num = 1;
                self.sort_col = 0;
                Ok(json!({ "page": 9 }))
            }
            "view_page_10" => {
                self.page = Page::DiskTreemap;
                self.page_num = 1;
                self.sort_col = 0;
                Ok(json!({ "page": 10 }))
            }
            "view_zoom_in" => Ok(json!({ "zoom": "in" })),
            "view_zoom_out" => Ok(json!({ "zoom": "out" })),
            "view_zoom_reset" => Ok(json!({ "zoom": "reset" })),
            "view_theme_dark" => {
                self.dark_mode = true;
                Ok(json!({ "dark_mode": true }))
            }
            "view_theme_light" => {
                self.dark_mode = false;
                Ok(json!({ "dark_mode": false }))
            }
            "cmd_refresh_telemetry" => {
                self.status_json = self.exec(&Command::Status).ok();
                self.weeder_json = self.exec(&Command::WeederScan).ok();
                self.last_message = "Telemetry & RAM weeder metrics refreshed.".to_string();
                Ok(json!({ "telemetry": "refreshed" }))
            }
            "cmd_scan_procs" => {
                let filter = if self.proc_filter.trim().is_empty() {
                    None
                } else {
                    Some(self.proc_filter.trim().to_string())
                };
                self.procs_json = self.exec(&Command::ProcList { filter, limit: 250 }).ok();
                self.page = Page::ProcsAndPorts;
                Ok(json!({ "scan": "procs" }))
            }
            "cmd_scan_ports" => {
                let port = self.port_filter.trim().parse::<u16>().ok();
                self.ports_json = self
                    .exec(&Command::PortMap {
                        port,
                        listening_only: true,
                    })
                    .ok();
                self.page = Page::ProcsAndPorts;
                Ok(json!({ "scan": "ports" }))
            }
            "cmd_scan_weeder" => {
                self.weeder_json = self.exec(&Command::WeederScan).ok();
                self.page = Page::RamWeeder;
                Ok(json!({ "scan": "weeder" }))
            }
            "cmd_run_cpu_audit" => {
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
                self.thermal_json = self.exec(&Command::ThermalStatus).ok();
                self.page = Page::CpuAuditAndThermals;
                Ok(json!({ "scan": "cpu_audit" }))
            }
            "cmd_run_doctor" => {
                self.doctor_json = self.exec(&Command::Doctor).ok();
                self.page = Page::DoctorAndHelp;
                Ok(json!({ "scan": "doctor" }))
            }
            "cmd_scan_mft" => self.menu_action("file_scan_mft"),
            "cmd_treemap_browser" => {
                if let Some(summary) = &self.treemap_summary {
                    let html =
                        wiztree::generate_treemap_html(&summary.path, &summary.largest_files);
                    let temp_p = std::env::temp_dir().join("wincraft_treemap.html");
                    let _ = std::fs::write(&temp_p, html);
                    let _ = ProcessCommand::new("cmd")
                        .args(["/c", "start", "", &temp_p.to_string_lossy()])
                        .spawn();
                    self.last_message =
                        format!("Opened D3 treemap in browser: {}", temp_p.display());
                    Ok(json!({ "treemap_browser": temp_p.display().to_string() }))
                } else {
                    let err = "Scan a drive first before opening treemap in browser.".to_string();
                    self.last_message = err.clone();
                    Err(err)
                }
            }
            "win_reset_layout" => {
                self.show_kpi_ribbon = true;
                self.view_mode = OutputFormat::Cards;
                self.page_num = 1;
                self.sort_col = 0;
                self.last_message = "Layout reset to defaults.".to_string();
                Ok(json!({ "layout": "reset" }))
            }
            "win_toggle_kpi" => {
                self.show_kpi_ribbon = !self.show_kpi_ribbon;
                Ok(json!({ "show_kpi": self.show_kpi_ribbon }))
            }
            "win_toggle_safety" => {
                self.active_dialog = Some(StudioDialog::Safety);
                Ok(json!({ "dialog": "safety" }))
            }
            "help_palette" => {
                self.active_dialog = Some(StudioDialog::CommandPalette);
                Ok(json!({ "dialog": "palette" }))
            }
            "help_shortcuts" => {
                self.active_dialog = Some(StudioDialog::Shortcuts);
                Ok(json!({ "dialog": "shortcuts" }))
            }
            "help_getting_started" => {
                self.docs_tab = "start";
                self.active_dialog = Some(StudioDialog::Docs);
                Ok(json!({ "dialog": "docs", "tab": "start" }))
            }
            "help_usage" => {
                self.docs_tab = "usage";
                self.active_dialog = Some(StudioDialog::Docs);
                Ok(json!({ "dialog": "docs", "tab": "usage" }))
            }
            "help_tips" => {
                self.docs_tab = "tips";
                self.active_dialog = Some(StudioDialog::Docs);
                Ok(json!({ "dialog": "docs", "tab": "tips" }))
            }
            "help_pitfalls" => {
                self.docs_tab = "pitfalls";
                self.active_dialog = Some(StudioDialog::Docs);
                Ok(json!({ "dialog": "docs", "tab": "pitfalls" }))
            }
            "help_planned" => {
                self.docs_tab = "planned";
                self.active_dialog = Some(StudioDialog::Docs);
                Ok(json!({ "dialog": "docs", "tab": "planned" }))
            }
            "help_catalog" => {
                self.docs_tab = "catalog";
                self.active_dialog = Some(StudioDialog::Docs);
                Ok(json!({ "dialog": "docs", "tab": "catalog" }))
            }
            "help_docs" => {
                self.docs_tab = "start";
                self.active_dialog = Some(StudioDialog::Docs);
                Ok(json!({ "dialog": "docs" }))
            }
            "help_licenses" => {
                self.active_dialog = Some(StudioDialog::Licenses);
                Ok(json!({ "dialog": "licenses" }))
            }
            "help_about" => {
                self.active_dialog = Some(StudioDialog::About);
                Ok(json!({ "dialog": "about" }))
            }
            other => Err(format!("unknown menu action '{other}'")),
        }
    }

    fn export_current_view(&mut self, fmt: OutputFormat) {
        let (dataset_slug, payload_opt) = match self.page {
            Page::Dashboard => (
                "disks",
                self.status_json
                    .as_ref()
                    .and_then(|st| st.get("disks"))
                    .cloned(),
            ),
            Page::ProcsAndPorts => (
                "ports",
                self.ports_json
                    .as_ref()
                    .or(self.procs_json.as_ref())
                    .cloned(),
            ),
            Page::CpuAuditAndThermals => ("cpu_audit", self.cpu_audit_json.as_ref().cloned()),
            Page::RamWeeder => ("weeder", self.weeder_json.as_ref().cloned()),
            Page::FileOps => ("fileops", self.fileops_json.as_ref().cloned()),
            Page::LockInspector => ("locks", self.lock_json.as_ref().cloned()),
            Page::ServicesNssm => ("services", self.services_json.as_ref().cloned()),
            Page::StartupAndRegistry => (
                "startup",
                self.startup_json
                    .as_ref()
                    .or(self.reg_json.as_ref())
                    .cloned(),
            ),
            Page::DoctorAndHelp => ("tools", Some(json!({ "tools": tools_catalog() }))),
            Page::DiskTreemap => ("treemap", None),
        };

        let Some(payload) = payload_opt else {
            self.last_message =
                "No active tabular dataset on current workspace to export.".to_string();
            return;
        };

        let tab = payload_to_tabular(&payload);

        let ext = fmt.extension();
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
        let body = render_tabular(&all_filtered, &payload, fmt);
        match std::fs::write(&file_path, body) {
            Ok(()) => {
                self.last_message = format!(
                    "Exported {} rows ({:?}) -> {}",
                    all_filtered.rows.len(),
                    fmt,
                    file_path.display()
                );
            }
            Err(e) => {
                self.last_message = format!("Export failed ({}): {e}", file_path.display());
            }
        }
    }

    fn render_menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("top_menubar").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                let file = ui
                    .menu_button("File", |ui| {
                        self.menu_item(ui, "file_scan_mft", "Scan MFT Drive... (Ctrl+M)");
                        ui.separator();
                        self.menu_item(ui, "file_export_csv", "Export View as CSV... (Ctrl+E)");
                        self.menu_item(ui, "file_export_md", "Export View as Markdown...");
                        self.menu_item(ui, "file_export_txt", "Export View as Text...");
                        self.menu_item(ui, "file_export_json", "Export View as JSON...");
                        ui.separator();
                        self.menu_item(ui, "file_open_backups", "Open Backups Folder");
                        ui.menu_button("Open Brochures", |ui| {
                            self.menu_item(ui, "file_pdf_brochure", "PDF Brochure");
                            self.menu_item(ui, "file_epub_brochure", "EPUB Brochure");
                            self.menu_item(ui, "file_html_brochure", "HTML5 Brochure");
                        });
                        ui.separator();
                        self.menu_item(ui, "file_relaunch_admin", "Relaunch as Admin (UAC)");
                        ui.separator();
                        self.menu_item(ui, "file_quit", "Quit (Ctrl+Q)");
                    })
                    .response;

                let edit = ui
                    .menu_button("Edit", |ui| {
                        self.menu_item(ui, "edit_filter", "Filter Table / Cards... (Ctrl+F)");
                        self.menu_item(ui, "edit_clear_filter", "Clear Filters (Esc)");
                        ui.separator();
                        let ro_label = if self.opts.read_only {
                            "Read-Only Guard: [Active]"
                        } else {
                            "Read-Only Guard: [Inactive]"
                        };
                        self.menu_item(ui, "edit_toggle_readonly", ro_label);
                        let arm_label = if self.arm_confirm {
                            "Arm Confirm Gate: [ARMED]"
                        } else {
                            "Arm Confirm Gate: [Disarmed]"
                        };
                        self.menu_item(ui, "edit_toggle_confirm", arm_label);
                        ui.separator();
                        self.menu_item(ui, "edit_preferences", "Preferences... (Ctrl+,)");
                    })
                    .response;

                let view = ui
                    .menu_button("View", |ui| {
                        ui.menu_button("View Mode", |ui| {
                            self.menu_item(ui, "view_mode_cards", "🗂 Cards (Default)");
                            self.menu_item(ui, "view_mode_table", "☰ List / Table");
                            self.menu_item(ui, "view_mode_json", "{ } Raw JSON");
                        });
                        ui.separator();
                        ui.menu_button("Workspaces", |ui| {
                            self.menu_item(ui, "view_page_1", "1. Host & Disks");
                            self.menu_item(ui, "view_page_2", "2. Processes & Ports");
                            self.menu_item(ui, "view_page_3", "3. CPU Audit & Thermals");
                            self.menu_item(ui, "view_page_4", "4. RAM Weeder");
                            self.menu_item(ui, "view_page_5", "5. High-Speed FileOps");
                            self.menu_item(ui, "view_page_6", "6. Lock Inspector");
                            self.menu_item(ui, "view_page_7", "7. Services & NSSM");
                            self.menu_item(ui, "view_page_8", "8. Startup & Registry");
                            self.menu_item(ui, "view_page_9", "9. Doctor & Catalog");
                            self.menu_item(ui, "view_page_10", "10. MFT Cushion Treemap");
                        });
                        ui.separator();
                        self.menu_item(ui, "view_zoom_in", "Zoom In (Ctrl+=)");
                        self.menu_item(ui, "view_zoom_out", "Zoom Out (Ctrl+-)");
                        self.menu_item(ui, "view_zoom_reset", "Reset Zoom (100%) (Ctrl+1)");
                        ui.separator();
                        self.menu_item(ui, "view_theme_dark", "Theme: Dark (Slate-950)");
                        self.menu_item(ui, "view_theme_light", "Theme: Light");
                    })
                    .response;

                let commands = ui
                    .menu_button("Commands", |ui| {
                        self.menu_item(
                            ui,
                            "cmd_refresh_telemetry",
                            "Refresh Telemetry & Metrics (F5)",
                        );
                        self.menu_item(ui, "cmd_scan_procs", "Scan Running Processes");
                        self.menu_item(ui, "cmd_scan_ports", "Scan Active Listening Ports");
                        self.menu_item(ui, "cmd_scan_weeder", "Scan RAM Working Sets");
                        self.menu_item(ui, "cmd_run_cpu_audit", "Run CPU Thermals & Audit");
                        self.menu_item(ui, "cmd_run_doctor", "Verify Restart Manager Doctor");
                        ui.separator();
                        self.menu_item(ui, "cmd_scan_mft", "Scan Drive MFT Records");
                        self.menu_item(ui, "cmd_treemap_browser", "Open Treemap in Browser");
                    })
                    .response;

                let window = ui
                    .menu_button("Window", |ui| {
                        self.menu_item(ui, "win_reset_layout", "Reset Panel Layout");
                        ui.separator();
                        self.menu_item(ui, "win_toggle_kpi", "Toggle Host KPI Ribbon");
                        self.menu_item(ui, "win_toggle_safety", "Safety & Invariants Info...");
                    })
                    .response;

                let help = ui
                    .menu_button("Help", |ui| {
                        self.menu_item(ui, "help_palette", "Command Palette... (Ctrl+K)");
                        self.menu_item(ui, "help_shortcuts", "Keyboard Shortcuts... (F1)");
                        ui.separator();
                        self.menu_item(ui, "help_getting_started", "Getting Started...");
                        self.menu_item(ui, "help_usage", "Usage (GUI, CLI & MCP)...");
                        self.menu_item(ui, "help_tips", "Tips & Tricks...");
                        self.menu_item(ui, "help_pitfalls", "Common Pitfalls & Safety...");
                        self.menu_item(ui, "help_planned", "Planned Extensions...");
                        self.menu_item(ui, "help_catalog", "31 MCP Tools Reference...");
                        self.menu_item(ui, "help_docs", "Project Files & Docs...");
                        ui.separator();
                        self.menu_item(ui, "file_pdf_brochure", "Open PDF Brochure");
                        self.menu_item(ui, "file_epub_brochure", "Open EPUB Brochure");
                        self.menu_item(ui, "file_html_brochure", "Open HTML5 Brochure");
                        ui.separator();
                        self.menu_item(ui, "help_licenses", "Licenses & Notices...");
                        self.menu_item(ui, "help_about", "About WinCraft");
                    })
                    .response;

                switch_on_hover(ui.ctx(), &[file, edit, view, commands, window, help]);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new("Safe Rust (0 unsafe)")
                            .color(Color32::from_rgb(34, 197, 94))
                            .small(),
                    );
                });
            });
        });
    }

    fn render_sub_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(
                RichText::new("WinCraft")
                    .color(Color32::from_rgb(103, 232, 249))
                    .strong(),
            );
            ui.label(RichText::new("v0.2.0").color(Color32::GRAY).small());

            ui.separator();

            let is_admin = wiztree::is_elevated();
            if is_admin {
                ui.label(
                    RichText::new("🛡️ [Admin]")
                        .color(Color32::from_rgb(34, 197, 94))
                        .strong(),
                );
            } else if ui
                .button(
                    RichText::new("👤 [User - Click to Elevate]")
                        .color(Color32::from_rgb(245, 158, 11))
                        .strong(),
                )
                .on_hover_text("Relaunch elevated via UAC for raw MFT direct-read access")
                .clicked()
            {
                let _ = self.menu_action("file_relaunch_admin");
            }

            ui.separator();

            let ro_label = if self.opts.read_only {
                "🔒 Read-Only: ON"
            } else {
                "🔓 Read-Only: OFF"
            };
            let ro_color = if self.opts.read_only {
                Color32::from_rgb(56, 189, 248)
            } else {
                Color32::GRAY
            };
            if ui
                .selectable_label(self.opts.read_only, RichText::new(ro_label).color(ro_color))
                .on_hover_text("Click to toggle read-only guard")
                .clicked()
            {
                let _ = self.menu_action("edit_toggle_readonly");
            }

            let arm_label = if self.arm_confirm {
                "⚠️ Confirm: ARMED"
            } else {
                "🛡️ Confirm: SAFE"
            };
            let arm_color = if self.arm_confirm {
                Color32::from_rgb(245, 158, 11)
            } else {
                Color32::GRAY
            };
            if ui
                .selectable_label(self.arm_confirm, RichText::new(arm_label).color(arm_color))
                .on_hover_text("Click to toggle mutation confirmation gate")
                .clicked()
            {
                let _ = self.menu_action("edit_toggle_confirm");
            }

            ui.separator();

            if ui
                .button("🔄 Refresh")
                .on_hover_text("Refresh all host metrics (F5)")
                .clicked()
            {
                let _ = self.menu_action("cmd_refresh_telemetry");
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let zoom_pct = (ui.ctx().zoom_factor() * 100.0).round() as i32;
                if ui
                    .button(format!("{zoom_pct}%"))
                    .on_hover_text("UI zoom (Ctrl+Wheel). Click to reset to 100%.")
                    .clicked()
                {
                    ui.ctx().set_zoom_factor(1.0);
                }

                theme_toggle_button(ui, &mut self.dark_mode);

                ui.separator();

                if ui
                    .selectable_label(self.view_mode == OutputFormat::Json, "{ } JSON")
                    .clicked()
                {
                    self.view_mode = OutputFormat::Json;
                }
                if ui
                    .selectable_label(self.view_mode == OutputFormat::List, "☰ Table")
                    .clicked()
                {
                    self.view_mode = OutputFormat::List;
                }
                if ui
                    .selectable_label(self.view_mode == OutputFormat::Cards, "🗂 Cards")
                    .clicked()
                {
                    self.view_mode = OutputFormat::Cards;
                }
                ui.label(RichText::new("View:").color(Color32::GRAY).small());
            });
        });
    }

    fn render_nav_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for p in Page::all() {
                let is_selected = self.page == p;
                let text = if is_selected {
                    RichText::new(p.label())
                        .strong()
                        .color(Color32::from_rgb(56, 189, 248))
                } else {
                    RichText::new(p.label()).color(Color32::from_rgb(203, 213, 225))
                };
                if ui.selectable_label(is_selected, text).clicked() {
                    self.page = p;
                    self.page_num = 1;
                    self.sort_col = 0;
                }
            }
        });
    }

    fn render_status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(&self.last_message)
                    .color(Color32::from_rgb(167, 243, 208))
                    .monospace()
                    .small(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("Workspace: {}", self.page.label()))
                        .color(Color32::GRAY)
                        .small(),
                );
                ui.separator();
                let admin_str = if wiztree::is_elevated() {
                    "Admin"
                } else {
                    "User"
                };
                ui.label(
                    RichText::new(format!("Mode: {admin_str}"))
                        .color(if wiztree::is_elevated() {
                            Color32::from_rgb(34, 197, 94)
                        } else {
                            Color32::from_rgb(245, 158, 11)
                        })
                        .small(),
                );
            });
        });
    }

    fn render_active_dialog(&mut self, ctx: &egui::Context) {
        let Some(dialog) = self.active_dialog else {
            return;
        };
        let title = match dialog {
            StudioDialog::CommandPalette => "Command Palette",
            StudioDialog::Shortcuts => "Keyboard Shortcuts",
            StudioDialog::Docs => "WinCraft Help & Documentation",
            StudioDialog::Licenses => "Licenses & Notices",
            StudioDialog::About => "About WinCraft",
            StudioDialog::Safety => "WinCraft Safety Invariants",
            StudioDialog::Preferences => "Preferences",
        };
        let mut open = true;
        egui::Window::new(title)
            .open(&mut open)
            .default_width(680.0)
            .default_height(480.0)
            .show(ctx, |ui| match dialog {
                StudioDialog::CommandPalette => {
                    ui.label("Type to filter commands. Press Enter or click to execute:");
                    let text_edit = ui.text_edit_singleline(&mut self.palette_query);
                    if text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let q = self.palette_query.to_ascii_lowercase();
                        if let Some((id, _, _)) = MENU_CATALOG.iter().find(|(id, label, _)| {
                            q.is_empty()
                                || label.to_ascii_lowercase().contains(&q)
                                || id.contains(&q)
                        }) {
                            let _ = self.menu_action(id);
                            self.active_dialog = None;
                            return;
                        }
                    }
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .max_height(350.0)
                        .show(ui, |ui| {
                            let q = self.palette_query.to_ascii_lowercase();
                            for (id, label, shortcut) in MENU_CATALOG {
                                if !q.is_empty()
                                    && !label.to_ascii_lowercase().contains(&q)
                                    && !id.contains(&q)
                                {
                                    continue;
                                }
                                ui.horizontal(|ui| {
                                    if ui.button(*label).clicked() {
                                        let _ = self.menu_action(id);
                                        let opens_dialog = matches!(
                                            *id,
                                            "help_palette"
                                                | "help_shortcuts"
                                                | "help_getting_started"
                                                | "help_usage"
                                                | "help_tips"
                                                | "help_pitfalls"
                                                | "help_planned"
                                                | "help_catalog"
                                                | "help_docs"
                                                | "help_licenses"
                                                | "help_about"
                                                | "win_toggle_safety"
                                                | "edit_preferences"
                                        );
                                        if !opens_dialog {
                                            self.active_dialog = None;
                                        }
                                    }
                                    if !shortcut.is_empty() {
                                        ui.label(
                                            RichText::new(*shortcut).color(Color32::GRAY).small(),
                                        );
                                    }
                                });
                            }
                        });
                }
                StudioDialog::Shortcuts => {
                    ui.heading("Keyboard Shortcuts Reference");
                    ui.separator();
                    egui::Grid::new("shortcuts_grid")
                        .striped(true)
                        .min_col_width(120.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Shortcut").strong());
                            ui.label(RichText::new("Command / Action").strong());
                            ui.end_row();

                            for (_, label, shortcut) in MENU_CATALOG {
                                if shortcut.is_empty() {
                                    continue;
                                }
                                ui.monospace(*shortcut);
                                ui.label(*label);
                                ui.end_row();
                            }
                            ui.monospace("Ctrl + MouseWheel");
                            ui.label("Global UI Zoom Factor (0.6x - 2.5x)");
                            ui.end_row();
                        });
                }
                StudioDialog::Docs => {
                    ui.horizontal(|ui| {
                        for (tab_id, tab_label) in [
                            ("start", "1. Get Started"),
                            ("functions", "2. Functions"),
                            ("usage", "3. Usage (GUI/CLI/MCP)"),
                            ("tips", "4. Tips & Tricks"),
                            ("pitfalls", "5. Pitfalls & Safety"),
                            ("planned", "6. Planned Extensions"),
                            ("catalog", "7. 31 MCP Tools"),
                        ] {
                            if ui
                                .selectable_label(self.docs_tab == tab_id, tab_label)
                                .clicked()
                            {
                                self.docs_tab = tab_id;
                            }
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .show(ui, |ui| match self.docs_tab {
                            "start" => {
                                ui.heading("Getting Started with WinCraft");
                                ui.label("WinCraft is a pure-Rust, zero-tax ArtCraft 3-tier desktop workstation and MCP server for Windows systems administration.");
                                ui.add_space(8.0);
                                ui.label("Step 1: Check host telemetry on Page 1 (Host & Disks) to inspect mount volumes and free storage.");
                                ui.label("Step 2: If on Windows with UAC, use 'Relaunch as Admin' to unlock raw NTFS direct-read MFT access.");
                                ui.label("Step 3: Scan high-speed disk usage on Page 10 (MFT Cushion Treemap) to instantly visualize space hogs.");
                                ui.label("Step 4: Use Page 4 (RAM Weeder) to find orphaned cargo test or zombie dev processes and safely reclaim RAM.");
                                ui.label("Step 5: Inspect file locks on Page 6 (Lock Inspector) via Windows Restart Manager before file operations.");
                            }
                            "functions" => {
                                ui.heading("Core Subsystems & Capabilities");
                                ui.label("• WizTree NTFS MFT Engine: Direct disk volume parsing for sub-second file cataloging and hierarchical treemaps.");
                                ui.label("• RAM Working Set Weeder: Detects zombie engines, abandoned test runners, and memory hogs with safe allowlist.");
                                ui.label("• Restart Manager Lock Inspector: FFI integration with rstrtmgr.dll to identify which processes lock any file.");
                                ui.label("• High-Speed FileOps: 3-stage streaming SHA-256 duplicate detection, fast grep, and mandatory .bak backups.");
                                ui.label("• NSSM Service & SysAdmin: Safe sc.exe control, HKCU registry editor with auto .reg snapshots.");
                            }
                            "usage" => {
                                ui.heading("Usage across Workstation, CLI & MCP");
                                ui.label("Workstation (GUI): Immediate-mode GPU rendering with Cards default view, live sorting, filtering, and export.");
                                ui.label("CLI: Run 'wincraft <verb> --format cards|list|json' for scriptable terminal output.");
                                ui.label("MCP Server: Run 'wincraft mcp' on stdio for LLM tools integration (Claude Desktop, Cursor, Antigravity).");
                            }
                            "tips" => {
                                ui.heading("Tips & Tricks for Maximum Performance");
                                ui.label("• Use Ctrl+K anytime to summon the fuzzy-matching Command Palette.");
                                ui.label("• Click on any cushion treemap tile to drill down into that directory's tree.");
                                ui.label("• Ctrl+Wheel zooms the entire interface smoothly on your GPU.");
                                ui.label("• In Cards view, click card action pills to jump directly to FileOps or process inspection.");
                            }
                            "pitfalls" => {
                                ui.heading("Common Pitfalls & Safety Invariants");
                                ui.label("• Mutating operations (weeder_reap, reg_write, svc_control) require Arm Confirm Gate.");
                                ui.label("• Read-Only Guard completely neutralizes write operations across CLI, MCP, and GUI.");
                                ui.label("• Protected processes (csrss, lsass, svchost, explorer, nssm, pwsh) can never be terminated.");
                                ui.label("• NSSM services are restarted with service manager (sc.exe stop/start), NEVER by taskkilling children.");
                            }
                            "planned" => {
                                ui.heading("Planned Extensions");
                                ui.label("- Volume Shadow Copy (VSS) snapshot browser for historical file diffing.");
                                ui.label("- Integration with DevCraft / Containercraft fleet containers.");
                            }
                            "catalog" => {
                                ui.heading("31 MCP Tools Catalog");
                                for t in tools_catalog() {
                                    ui.horizontal(|ui| {
                                        ui.monospace(
                                            RichText::new(t.name)
                                                .strong()
                                                .color(Color32::from_rgb(56, 189, 248)),
                                        );
                                        if t.confirm_gated {
                                            ui.label(
                                                RichText::new("[Mutating]")
                                                    .color(Color32::from_rgb(245, 158, 11))
                                                    .small(),
                                            );
                                        } else {
                                            ui.label(
                                                RichText::new("[Read-Only]")
                                                    .color(Color32::from_rgb(34, 197, 94))
                                                    .small(),
                                            );
                                        }
                                    });
                                    ui.label(RichText::new(t.description).small());
                                    ui.add_space(4.0);
                                }
                            }
                            _ => {}
                        });
                }
                StudioDialog::Licenses => {
                    ui.heading("Licenses & Open Source Notices");
                    ui.label("WinCraft is dual-licensed under MIT OR Apache-2.0.");
                    ui.label("Built with 100% Safe Rust: #![forbid(unsafe_code)] across wincraft-app.");
                    ui.label("Zero unwrap(), zero expect(), zero panic! in production paths.");
                }
                StudioDialog::About => {
                    ui.heading("WinCraft v0.2.0");
                    ui.label("Pure-Rust Windows Host, RAM Weeder, FileOps & SysAdmin Studio.");
                    ui.label("Part of the Storytold ArtCraft Ecosystem.");
                    ui.separator();
                    ui.label("Author: Sandra Schipal (Vienna)");
                    ui.label("Host Hardware: Goliath (AMD Ryzen 9 5900X, NVIDIA RTX 4090 24GB, 64GB RAM)");
                }
                StudioDialog::Safety => {
                    ui.heading("WinCraft Safety Invariants & Architecture");
                    ui.label("1. Default View is Cards, switchable to Table or JSON, with filtering and export.");
                    ui.label("2. Protected system processes (csrss, lsass, svchost, explorer, nssm, pwsh) can never be reaped.");
                    ui.label("3. Every file edit creates a timestamped .bak backup; registry edits export .reg snapshots.");
                    ui.label("4. NSSM services are controlled strictly via sc.exe stop/start.");
                }
                StudioDialog::Preferences => {
                    ui.heading("Workstation Preferences");
                    ui.checkbox(&mut self.dark_mode, "Dark Mode (Slate-950)");
                    ui.separator();
                    ui.label("Default Page Size:");
                    for sz in [10, 25, 50, 100] {
                        if ui
                            .selectable_label(self.page_size == sz, sz.to_string())
                            .clicked()
                        {
                            self.page_size = sz;
                            self.page_num = 1;
                        }
                    }
                    ui.separator();
                    ui.checkbox(&mut self.show_kpi_ribbon, "Show Host KPI Ribbon");
                    ui.checkbox(&mut self.opts.read_only, "Read-Only Guard");
                    ui.checkbox(&mut self.arm_confirm, "Arm Confirm Gate");
                }
            });
        if !open {
            self.active_dialog = None;
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
                                        ui.horizontal(|ui| {
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

    fn render_treemap_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let fixed_drives = wiztree::list_fixed_drives();
        let is_elevated = wiztree::is_elevated();

        // Header: title row (short, never overflows) + wrapping drive strip + always-visible actions row
        ui.horizontal(|ui| {
            ui.heading(
                RichText::new("🗺️ Hierarchical Cushion Treemap")
                    .strong()
                    .color(Color32::from_rgb(56, 189, 248)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(summary) = &self.treemap_summary {
                    let total_gb = (summary.total_size_bytes as f64) / 1_073_741_824.0;
                    ui.label(
                        RichText::new(format!(
                            "{:.2} GB Scanned ({} files)",
                            total_gb, summary.total_files
                        ))
                        .strong()
                        .color(Color32::WHITE),
                    );
                }
            });
        });

        // Drive selector strip — wraps to as many lines as needed, never pushes buttons off-screen
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Drive:").color(Color32::GRAY));
            for d in &fixed_drives {
                let can_mft = wiztree::can_read_mft(&d.root);
                let label = if can_mft {
                    format!("{} ✓ (MFT Ready)", d.root)
                } else {
                    format!("{} 🔒 (Needs Admin)", d.root)
                };
                if ui
                    .selectable_label(self.treemap_drive == d.root, label)
                    .clicked()
                {
                    self.treemap_drive = d.root.clone();
                    self.treemap_zoom_path = None;
                    self.treemap_selected_tile = None;
                }
            }
        });

        // Actions row — always on its own line so Scan can never be pushed off-screen
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(RichText::new("⚡ Scan MFT Drive").strong())
                .clicked()
            {
                let drive_target = self.treemap_drive.clone();
                match wiztree::scan_drive_usage(&drive_target, 5000) {
                    Ok(records) => {
                        let summary = wiztree::analyze_disk_records(&drive_target, &records, 50);
                        self.last_message = format!(
                            "MFT scan complete for {drive_target}: {} files scanned ({:.2} GB)",
                            summary.total_files,
                            summary.total_size_bytes as f64 / 1_073_741_824.0
                        );
                        self.treemap_summary = Some(summary);
                        self.treemap_zoom_path = None;
                        self.treemap_selected_tile = None;
                    }
                    Err(e) => {
                        self.last_message = format!("MFT scan failed for {drive_target}: {e}");
                    }
                }
            }

            if let Some(summary) = &self.treemap_summary {
                if ui.button("🌐 Open Interactive D3 in Browser").clicked() {
                    let html =
                        wiztree::generate_treemap_html(&summary.path, &summary.largest_files);
                    let temp_p = std::env::temp_dir().join("wincraft_treemap.html");
                    let _ = std::fs::write(&temp_p, html);
                    let _ = std::process::Command::new("cmd")
                        .args(["/c", "start", "", &temp_p.to_string_lossy()])
                        .spawn();
                }
            }
            ui.label(
                RichText::new("Tip: click a folder tile to zoom in, breadcrumb to zoom out.")
                    .color(Color32::DARK_GRAY)
                    .small(),
            );
        });

        if !is_elevated {
            ui.add_space(2.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("ℹ️ User Privilege Notice:").color(Color32::from_rgb(245, 158, 11)).strong());
                ui.label("Running as standard user. For instant 1-second MFT scans on restricted drives (C:\\, D:\\), click '🛡️ Relaunch as Admin' in the top header.");
            });
        }

        ui.separator();

        let Some(summary) = self.treemap_summary.clone() else {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Select a fixed drive above and click '⚡ Scan MFT Drive' to render the cushion treemap.").color(Color32::GRAY));
                ui.label(RichText::new("WizTree direct MFT reads complete in sub-seconds with zero CPU waste.").color(Color32::DARK_GRAY).small());
            });
            return;
        };

        let Some(tree_root) = &summary.file_tree else {
            ui.label("No file tree available in summary.");
            return;
        };

        let active_root = if let Some(target) = &self.treemap_zoom_path {
            match find_node_by_path(tree_root, target) {
                Some(n) => n,
                None => tree_root,
            }
        } else {
            tree_root
        };

        // Breadcrumb Navigation — wraps so deep paths never push Zoom Out off-screen
        let mut next_zoom_path: Option<Option<String>> = None;
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Location:").color(Color32::GRAY).small());
            if ui
                .button(RichText::new(format!("📁 Root ({})", tree_root.name)).small())
                .clicked()
            {
                next_zoom_path = Some(None);
            }

            if let Some(zoom_p) = &self.treemap_zoom_path {
                let relative = if zoom_p.starts_with(&tree_root.path) {
                    zoom_p[tree_root.path.len()..].trim_start_matches(['\\', '/'])
                } else {
                    zoom_p.as_str()
                };

                let mut accumulated = tree_root.path.clone();
                for part in relative.split(['\\', '/']).filter(|s| !s.is_empty()) {
                    accumulated.push('\\');
                    accumulated.push_str(part);
                    let target_path = accumulated.clone();
                    ui.label(RichText::new(">").color(Color32::DARK_GRAY).small());
                    if ui.button(RichText::new(part).small().strong()).clicked() {
                        next_zoom_path = Some(Some(target_path));
                    }
                }
            }

            if self.treemap_zoom_path.is_some() && ui.button("⬆️ Zoom Out to Root").clicked() {
                next_zoom_path = Some(None);
            }
        });
        if let Some(target) = next_zoom_path {
            self.treemap_zoom_path = target;
        }

        // Top Category Legend — wraps to second line on narrow windows
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("File Types:").color(Color32::GRAY).small());
            let legend_items = [
                ("Videos (.mp4, .mkv)", Color32::from_rgb(168, 85, 247)),
                ("Archives (.zip, .iso)", Color32::from_rgb(249, 115, 22)),
                ("Binaries (.exe, .dll)", Color32::from_rgb(34, 197, 94)),
                (
                    "AI / Data (.safetensors, .db)",
                    Color32::from_rgb(236, 72, 153),
                ),
                ("Docs (.pdf, .docx)", Color32::from_rgb(56, 189, 248)),
            ];
            for (name, color) in legend_items {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, color);
                    ui.label(RichText::new(name).small());
                });
            }
        });

        ui.add_space(4.0);

        let inspector_height = 44.0;
        let available_rect = ui.available_rect_before_wrap();
        let treemap_rect = egui::Rect::from_min_size(
            available_rect.min,
            Vec2::new(
                available_rect.width(),
                (available_rect.height() - inspector_height).max(140.0),
            ),
        );

        let canvas_response = ui.allocate_rect(treemap_rect, egui::Sense::click());
        let painter = ui.painter_at(treemap_rect);

        // Draw dark canvas background
        painter.rect_filled(treemap_rect, 4.0, Color32::from_rgb(11, 15, 25));
        painter.rect_stroke(
            treemap_rect,
            4.0,
            egui::Stroke::new(1.0, Color32::from_rgb(30, 41, 59)),
            egui::StrokeKind::Outside,
        );

        let pointer_pos = ctx.pointer_hover_pos();
        let mut hovered_tile: Option<TreemapTile> = None;
        let mut hovered_folder: Option<TreemapTile> = None;
        let mut new_selected_tile: Option<TreemapTile> = None;
        let mut new_zoom_folder: Option<String> = None;

        let bounds = LayoutRect {
            x: treemap_rect.min.x,
            y: treemap_rect.min.y,
            w: treemap_rect.width(),
            h: treemap_rect.height(),
        };

        let layout = compute_hierarchical_treemap(active_root, bounds, 5);

        // Draw Folder Enveloping Containers
        for folder in &layout.folder_tiles {
            let rect = egui::Rect::from_min_size(
                egui::pos2(folder.rect.x, folder.rect.y),
                Vec2::new(folder.rect.w, folder.rect.h),
            );

            if rect.width() < 4.0 || rect.height() < 4.0 {
                continue;
            }

            let is_f_hovered = pointer_pos.is_some_and(|pos| rect.contains(pos));
            if is_f_hovered {
                hovered_folder = Some(folder.clone());
                if canvas_response.clicked() {
                    new_zoom_folder = Some(folder.path.clone());
                }
            }

            painter.rect_filled(rect, 3.0, Color32::from_rgb(15, 23, 42));
            painter.rect_stroke(
                rect,
                3.0,
                egui::Stroke::new(1.2, Color32::from_rgb(51, 65, 85)),
                egui::StrokeKind::Outside,
            );

            if rect.width() > 65.0 && rect.height() > 36.0 {
                let header_rect =
                    egui::Rect::from_min_size(rect.min, Vec2::new(rect.width(), 14.0));
                painter.rect_filled(header_rect, 2.0, Color32::from_rgb(30, 41, 59));

                let header_clip = painter.with_clip_rect(header_rect);
                header_clip.text(
                    header_rect.left_top() + Vec2::new(4.0, 1.0),
                    egui::Align2::LEFT_TOP,
                    format!("📁 {}", folder.name),
                    egui::FontId::proportional(10.0),
                    Color32::from_rgb(203, 213, 225),
                );

                let size_str = format_bytes(folder.size);
                header_clip.text(
                    header_rect.right_top() - Vec2::new(4.0, -1.0),
                    egui::Align2::RIGHT_TOP,
                    size_str,
                    egui::FontId::proportional(9.0),
                    Color32::from_rgb(148, 163, 184),
                );
            }
        }

        // Draw File Cushions
        for file in &layout.file_tiles {
            let rect = egui::Rect::from_min_size(
                egui::pos2(file.rect.x, file.rect.y),
                Vec2::new(file.rect.w, file.rect.h),
            );

            let tile_rect = rect.shrink(1.0);
            if tile_rect.width() <= 1.0 || tile_rect.height() <= 1.0 {
                continue;
            }

            let is_hovered = pointer_pos.is_some_and(|pos| tile_rect.contains(pos));
            if is_hovered {
                hovered_tile = Some(file.clone());
                if canvas_response.clicked() {
                    new_selected_tile = Some(file.clone());
                }
                if canvas_response.double_clicked() {
                    let _ = std::process::Command::new("explorer")
                        .args(["/select,", &file.path])
                        .spawn();
                }
            }

            let base_color =
                Color32::from_rgb(file.color_rgb[0], file.color_rgb[1], file.color_rgb[2]);

            painter.rect_filled(tile_rect, 2.0, base_color);

            // 3D Cushion Bevel borders
            painter.line_segment(
                [tile_rect.left_top(), tile_rect.right_top()],
                egui::Stroke::new(1.2, Color32::from_rgba_unmultiplied(255, 255, 255, 80)),
            );
            painter.line_segment(
                [tile_rect.left_top(), tile_rect.left_bottom()],
                egui::Stroke::new(1.2, Color32::from_rgba_unmultiplied(255, 255, 255, 65)),
            );
            painter.line_segment(
                [tile_rect.left_bottom(), tile_rect.right_bottom()],
                egui::Stroke::new(1.2, Color32::from_rgba_unmultiplied(0, 0, 0, 110)),
            );
            painter.line_segment(
                [tile_rect.right_top(), tile_rect.right_bottom()],
                egui::Stroke::new(1.2, Color32::from_rgba_unmultiplied(0, 0, 0, 110)),
            );

            if tile_rect.width() > 18.0 && tile_rect.height() > 18.0 {
                let shine_w = (tile_rect.width() * 0.45).min(40.0);
                let shine_h = (tile_rect.height() * 0.45).min(40.0);
                let shine_rect = egui::Rect::from_min_size(
                    tile_rect.min + Vec2::splat(1.5),
                    Vec2::new(shine_w, shine_h),
                );
                painter.rect_filled(
                    shine_rect,
                    2.0,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 25),
                );
            }

            let is_selected = self
                .treemap_selected_tile
                .as_ref()
                .is_some_and(|s| s.path == file.path);
            if is_hovered {
                painter.rect_stroke(
                    tile_rect,
                    2.0,
                    egui::Stroke::new(2.5, Color32::from_rgb(250, 204, 21)),
                    egui::StrokeKind::Outside,
                );
            } else if is_selected {
                painter.rect_stroke(
                    tile_rect,
                    2.0,
                    egui::Stroke::new(2.0, Color32::from_rgb(56, 189, 248)),
                    egui::StrokeKind::Outside,
                );
            }

            if tile_rect.width() > 50.0 && tile_rect.height() > 22.0 {
                let text_clip = painter.with_clip_rect(tile_rect);
                let size_str = format_bytes(file.size);

                let label_pos = tile_rect.left_top() + Vec2::new(5.0, 3.0);
                text_clip.text(
                    label_pos + Vec2::splat(1.0),
                    egui::Align2::LEFT_TOP,
                    &file.name,
                    egui::FontId::proportional(11.0),
                    Color32::from_black_alpha(200),
                );
                text_clip.text(
                    label_pos,
                    egui::Align2::LEFT_TOP,
                    &file.name,
                    egui::FontId::proportional(11.0),
                    Color32::WHITE,
                );

                if tile_rect.width() > 70.0 && tile_rect.height() > 38.0 {
                    let size_pos = tile_rect.right_bottom() - Vec2::new(5.0, 3.0);
                    text_clip.text(
                        size_pos + Vec2::splat(1.0),
                        egui::Align2::RIGHT_BOTTOM,
                        &size_str,
                        egui::FontId::proportional(10.0),
                        Color32::from_black_alpha(200),
                    );
                    text_clip.text(
                        size_pos,
                        egui::Align2::RIGHT_BOTTOM,
                        &size_str,
                        egui::FontId::proportional(10.0),
                        Color32::from_rgb(226, 232, 240),
                    );
                }
            }
        }

        // Draw Enclosing Parent Folder Outline on File Hover
        if let Some(hovered_f) = &hovered_tile {
            if let Some(parent) = layout
                .folder_tiles
                .iter()
                .find(|p| p.path == hovered_f.parent_path)
            {
                let p_rect = egui::Rect::from_min_size(
                    egui::pos2(parent.rect.x, parent.rect.y),
                    Vec2::new(parent.rect.w, parent.rect.h),
                );
                painter.rect_stroke(
                    p_rect,
                    3.0,
                    egui::Stroke::new(2.0, Color32::from_rgb(56, 189, 248)),
                    egui::StrokeKind::Outside,
                );
            }
        }

        if let Some(st) = new_selected_tile {
            self.treemap_selected_tile = Some(st);
        }
        if let Some(zf) = new_zoom_folder {
            self.treemap_zoom_path = Some(zf);
        }

        // Tooltip
        if let Some(hovered) = &hovered_tile {
            let size_str = format_bytes(hovered.size);
            if let Some(pos) = pointer_pos {
                egui::Area::new(egui::Id::unique("treemap_hover_tip"))
                    .fixed_pos(pos + Vec2::new(14.0, 14.0))
                    .order(egui::Order::Tooltip)
                    .show(ctx, |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.strong(&hovered.name);
                            ui.label(
                                RichText::new(format!("Path: {}", hovered.path))
                                    .color(Color32::from_rgb(148, 163, 184)),
                            );
                            ui.label(
                                RichText::new(format!("Enclosing Folder: {}", hovered.parent_path))
                                    .color(Color32::from_rgb(56, 189, 248))
                                    .small(),
                            );
                            ui.horizontal(|ui| {
                                ui.label("Size:");
                                ui.label(
                                    RichText::new(format!("{} ({} bytes)", size_str, hovered.size))
                                        .color(Color32::from_rgb(56, 189, 248))
                                        .strong(),
                                );
                            });
                            if !hovered.extension.is_empty() {
                                ui.label(format!("Extension: .{}", hovered.extension));
                            }
                        });
                    });
            }
        } else if let Some(hf) = &hovered_folder {
            let size_str = format_bytes(hf.size);
            if let Some(pos) = pointer_pos {
                egui::Area::new(egui::Id::unique("treemap_folder_tip"))
                    .fixed_pos(pos + Vec2::new(14.0, 14.0))
                    .order(egui::Order::Tooltip)
                    .show(ctx, |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.strong(format!("📁 {}", hf.name));
                            ui.label(
                                RichText::new(format!("Path: {}", hf.path))
                                    .color(Color32::from_rgb(148, 163, 184)),
                            );
                            ui.horizontal(|ui| {
                                ui.label("Cumulative Size:");
                                ui.label(
                                    RichText::new(format!("{} ({} bytes)", size_str, hf.size))
                                        .color(Color32::from_rgb(56, 189, 248))
                                        .strong(),
                                );
                            });
                            ui.label(
                                RichText::new("💡 Click folder to zoom into contents")
                                    .color(Color32::GRAY)
                                    .small(),
                            );
                        });
                    });
            }
        }

        // Bottom Inspector Bar — two wrapped rows so long paths never push buttons off-screen
        ui.add_space(4.0);
        ui.vertical(|ui| {
            if let Some(tile) = &self.treemap_selected_tile {
                ui.horizontal_wrapped(|ui| {
                    let size_str = format_bytes(tile.size);
                    let icon = if tile.is_folder { "📁" } else { "📄" };
                    ui.label(RichText::new(format!("{icon} {}", tile.name)).strong());
                    ui.label(
                        RichText::new(format!("• {size_str}"))
                            .color(Color32::from_rgb(56, 189, 248))
                            .strong(),
                    );
                    ui.label(
                        RichText::new(format!("• {}", tile.path))
                            .color(Color32::from_rgb(148, 163, 184))
                            .small(),
                    );
                });
                ui.horizontal_wrapped(|ui| {
                    if tile.is_folder && ui.button("🔍 Zoom In").clicked() {
                        self.treemap_zoom_path = Some(tile.path.clone());
                    }
                    if ui
                        .button("🔒 Who Locks This?")
                        .on_hover_text("Switch to Restart Manager Lock Inspector for this path")
                        .clicked()
                    {
                        self.lock_path = tile.path.clone();
                        self.page = Page::LockInspector;
                    }
                    if ui.button("📋 Copy Path").clicked() {
                        ui.ctx().copy_text(tile.path.clone());
                    }
                    if ui.button("📂 Reveal in Explorer").clicked() {
                        let _ = std::process::Command::new("explorer")
                            .args(["/select,", &tile.path])
                            .spawn();
                    }
                });
            } else {
                ui.label(
                    RichText::new("Hover or click any cushion tile or folder to inspect.")
                        .color(Color32::GRAY),
                );
            }
        });
    }
}

impl eframe::App for WinCraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        handle_zoom(&ctx);
        if !self.theme_applied {
            apply_craft_theme(&ctx);
            self.theme_applied = true;
        }

        // Global hotkeys
        let mut trigger_action: Option<&'static str> = None;
        let mut zoom_delta = 0.0_f32;
        ctx.input(|i| {
            if (i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(egui::Key::K) {
                trigger_action = Some("help_palette");
            }
            if i.key_pressed(egui::Key::F1) {
                trigger_action = Some("help_shortcuts");
            }
            if i.key_pressed(egui::Key::F5) {
                trigger_action = Some("cmd_refresh_telemetry");
            }
            if (i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(egui::Key::Q) {
                trigger_action = Some("file_quit");
            }
            if (i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(egui::Key::Num1) {
                zoom_delta = 1.0;
            }
            if (i.modifiers.ctrl || i.modifiers.command)
                && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals))
            {
                zoom_delta = 1.1;
            }
            if (i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(egui::Key::Minus) {
                zoom_delta = 1.0 / 1.1;
            }
            if (i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(egui::Key::M) {
                trigger_action = Some("file_scan_mft");
            }
            if (i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(egui::Key::Comma) {
                trigger_action = Some("edit_preferences");
            }
        });

        if let Some(action) = trigger_action {
            let _ = self.menu_action(action);
        }
        if zoom_delta > 0.0 {
            if (zoom_delta - 1.0).abs() < f32::EPSILON {
                ctx.set_zoom_factor(1.0);
            } else {
                let next = (ctx.zoom_factor() * zoom_delta).clamp(0.6, 2.5);
                ctx.set_zoom_factor(next);
            }
        }

        if self.quit_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            self.quit_requested = false;
        }

        // Top Menu Bar (Pinned)
        self.render_menu_bar(ui);

        // Desktop Sub-Header Bar (Title, Elevation badge, Guard pills, Refresh, View Mode, Theme, Zoom)
        self.render_sub_header(ui);
        ui.separator();

        // Optional Host KPI Ribbon
        if self.show_kpi_ribbon {
            self.render_kpi_ribbon(ui);
            ui.separator();
        }

        // Navigation Bar
        self.render_nav_bar(ui);
        ui.separator();

        // Active Dialogs Modal (Command Palette, Shortcuts, Docs, About, etc.)
        self.render_active_dialog(&ctx);

        // Bottom Status Bar
        self.render_status_bar(ui);
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
                ui.heading("RAM Weeder -- Orphan Cargo Test & Zombie Engine Detector");
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
                ui.heading("Toolchain Doctor, Brochure Assets & 31 MCP Tools Catalog");
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
            Page::DiskTreemap => {
                let ctx = ui.ctx().clone();
                self.render_treemap_page(ui, &ctx);
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
            .with_title("WinCraft -- Windows Host, RAM Weeder, FileOps & SysAdmin Studio")
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
        assert!(app.dark_mode);
        let core_tools = dispatch(&Command::Tools, &ExecOptions::default());
        let app_tools = app.exec(&Command::Tools);
        assert!(core_tools.is_ok());
        assert!(app_tools.is_ok());
        assert_eq!(core_tools.ok(), app_tools.ok());
    }

    #[test]
    fn dark_and_light_visuals_differ_with_high_contrast() {
        let dark = harsh_visuals();
        let light = crisp_light_visuals();
        assert_ne!(dark.panel_fill, light.panel_fill);
        assert_eq!(dark.panel_fill, Color32::from_rgb(13, 17, 23));
        assert_eq!(light.panel_fill, Color32::from_rgb(248, 250, 252));
        assert_eq!(
            light.override_text_color,
            Some(Color32::from_rgb(15, 23, 42))
        );
    }

    #[test]
    fn zoom_factor_clamp_stays_within_bounds() {
        for z in [0.1_f32, 0.6, 1.0, 2.5, 5.0] {
            let clamped = z.clamp(0.6, 2.5);
            assert!((0.6..=2.5).contains(&clamped));
        }
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
                                            let _ = ui.small_button("BTN");
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

    #[test]
    fn menu_catalog_contains_standard_entries_and_pages() {
        assert!(MENU_CATALOG.len() >= 30);
        let ids: Vec<&str> = MENU_CATALOG.iter().map(|(id, _, _)| *id).collect();
        assert!(ids.contains(&"file_scan_mft"));
        assert!(ids.contains(&"file_export_csv"));
        assert!(ids.contains(&"file_relaunch_admin"));
        assert!(ids.contains(&"file_quit"));
        assert!(ids.contains(&"edit_filter"));
        assert!(ids.contains(&"edit_toggle_readonly"));
        assert!(ids.contains(&"edit_toggle_confirm"));
        assert!(ids.contains(&"view_mode_cards"));
        assert!(ids.contains(&"view_mode_table"));
        assert!(ids.contains(&"view_mode_json"));
        assert!(ids.contains(&"cmd_refresh_telemetry"));
        assert!(ids.contains(&"help_palette"));
        assert!(ids.contains(&"help_shortcuts"));
        assert!(ids.contains(&"help_about"));
        assert_eq!(Page::all().len(), 10);
    }

    #[test]
    fn menu_actions_update_app_state() {
        let mut app = WinCraftApp::default();

        assert_eq!(app.view_mode, OutputFormat::Cards);
        assert!(app.menu_action("view_mode_table").is_ok());
        assert_eq!(app.view_mode, OutputFormat::List);
        assert!(app.menu_action("view_mode_json").is_ok());
        assert_eq!(app.view_mode, OutputFormat::Json);
        assert!(app.menu_action("view_mode_cards").is_ok());
        assert_eq!(app.view_mode, OutputFormat::Cards);

        assert!(!app.opts.read_only);
        assert!(app.menu_action("edit_toggle_readonly").is_ok());
        assert!(app.opts.read_only);

        assert!(!app.arm_confirm);
        assert!(app.menu_action("edit_toggle_confirm").is_ok());
        assert!(app.arm_confirm);

        assert_eq!(app.page, Page::Dashboard);
        assert!(app.menu_action("view_page_10").is_ok());
        assert_eq!(app.page, Page::DiskTreemap);

        assert_eq!(app.active_dialog, None);
        assert!(app.menu_action("help_palette").is_ok());
        assert_eq!(app.active_dialog, Some(StudioDialog::CommandPalette));
        assert!(app.menu_action("help_shortcuts").is_ok());
        assert_eq!(app.active_dialog, Some(StudioDialog::Shortcuts));
        assert!(app.menu_action("help_about").is_ok());
        assert_eq!(app.active_dialog, Some(StudioDialog::About));
    }

    #[test]
    fn ascii_hygiene_zero_em_dashes_in_source() {
        let src = include_str!("main.rs");
        assert!(
            !src.contains('\u{2014}'),
            "main.rs must not contain em dashes (U+2014)"
        );
    }
}
