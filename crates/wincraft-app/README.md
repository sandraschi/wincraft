# `wincraft-app` (L3 Native egui Desktop Workstation)

Native pure-Rust Windows desktop studio (`eframe`/`egui` `0.37`) calling `wincraft_core::dispatch` in-process with zero webview, zero Node/Bun runtime, and zero open ports.

## 10 Workstation Pages

1. **Host & Disks** — Volume overview, memory telemetry, and quick-jump drive action pills.
2. **Processes & Ports** — Process table and `WEBAPP_PORTS.md`-correlated port map.
3. **CPU Audit & Thermals** — Per-core load audit and host thermal telemetry.
4. **RAM Weeder** — Zombie/orphan dev-process scanner with protected system allowlist and confirm-gated reap.
5. **File-Lock Inspector** — Native `rstrtmgr.dll` Restart Manager handle inspection.
6. **High-Speed FileOps** — Directory browser, disk usage, large files, 3-stage SHA-256 duplicates, regex grep, `.bak` writes, and archives.
7. **SysAdmin & Services** — NSSM-safe Windows service controller, startup items, and scheduled tasks.
8. **Registry & Events** — HKCU registry inspector/editor (with auto `.reg` snapshots) and Windows event log tail.
9. **Doctor, Tools & Brochure** — Environment doctor, 31-tool reference, built-in multi-tab Docs dialog, and brochure launcher.
10. **MFT Cushion Treemap** — Sub-second WizTree NTFS `$MFT` scanner, hierarchical cushion treemap with breadcrumb drill-down, interactive D3 browser export, and in-app **Relaunch as Admin** elevation.

## Menu Bar & Shortcuts

- Top menu bar: `File`, `Edit`, `View`, `Commands`, `Window`, `Help`
- `Ctrl+K` — Command Palette
- `F1` — Keyboard Shortcuts
- `Alt+A` — Arm Confirm Gate
- `Ctrl+MouseWheel` / `Ctrl+1` — UI zoom (`0.6x`–`2.5x`) and reset to `100%`
- Built-in Light/Dark theme switcher with high-contrast styling.

## Running

```powershell
cargo run -p wincraft-app
```
