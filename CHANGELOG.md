# Changelog

## [0.3.0] - 2026-11-26

First tagged release (no prior tags; workspace version 0.1.0 covered
 v0.1.0 and v0.2.0 scope below, now consolidated under v0.3.0).

### Added
- MFT storage tools (`win_mft_search` sub-second NTFS wildcard search, `win_disk_usage` space and extension summary, `win_treemap` self-contained D3 cushion HTML) and L3 Page 10 hierarchical cushion treemap with breadcrumb drill-down and in-app Relaunch as Admin.
- Publishable `cushion-treemap` crate (squarified hierarchical layout); `wincraft-core` treemap delegates to it.
- Craft menu bar standard in `wincraft-app` (`File` through `Help`), Command Palette (`Ctrl+K`), Keyboard Shortcuts (`F1`), Help Center, bottom status bar, elevation pill.
- Light theme plus dark theme (vector Sun/Moon toggle) with Ctrl+wheel zoom (0.6x to 2.5x).
- T2 floor: 29 core integration tests (57 total: 18 core unit + 29 integration + 2 mcp + 8 app).
- CI: `.github/workflows/rust-ci.yml` (fmt, clippy `-D warnings`, `cargo test --workspace`).
- README feature matrix (Implemented / Partial / Planned).
- LF guarantee: `fs_backup_write` and exports normalize CRLF/CR to LF.
- `mcpb/manifest.json` regenerated from the live server (31 tools).
- LICENSE-MIT and LICENSE-APACHE (MIT OR Apache-2.0).

### Fixed
- MCP protocol version string 2024-11-05 to 2026-07-28.
- Treemap header wrapping; Scan action always visible.

## [0.2.0] - 2026-11-11

### Added
- **CPU Hogger & Compute Audit (`win_cpu_audit` / `wincraft cpu-audit`)**: Audits running processes for high cumulative or instantaneous CPU consumption, categorizing into Precleared System, Precleared Dev, Unauthorized Hogs, Suspicious Paths (volatile `%TEMP%` / `Downloads` / system masquerading outside System32), and Known Crypto Miner Signatures (`xmrig`, `minerd`, `ethminer`, `nbminer`, etc.).
- **Host Thermal & Processor Diagnostics (`win_thermal_status` / `wincraft thermal`)**: Queries CPU load percentage, core count, logical processors, clock speeds, and thermal throttling state under high heat.
- **Dedicated Desktop Page in `wincraft-app`**: 9-page workstation featuring `Page 3: CPU Audit & Thermals` with instant reap quick actions for rogue PIDs, customizable threshold sliders for CPU seconds and CPU %, Cards/List/JSON view switching, and full tabular export.
- Expanded WinCraft tool suite from 26 to 28 MCP tools.

## [0.1.0] - 2026-11-09

### Added
- Initial Pure-Rust ArtCraft 3-tier release (`wincraft-core`, `wincraft-mcp`, `wincraft-app`) subsuming Windows host, FileOps, RAM Weeder, and Sysinternals/SysAdmin operations.
- **Native Win32 Restart Manager (`rstrtmgr.dll` FFI)**: `win_lock_inspect` identifies exact locking PIDs and application names for any file or folder without third-party drivers.
- **3-Stage Streaming Duplicate Detector**: `win_fs_dups` groups by file size, filters by 4 KB SHA-256 prefix, and streams 64 KB blocks for full FIPS 180-4 SHA-256 verification.
- **RAM Weeder**: `win_weeder_scan` and confirm-gated `win_weeder_reap` with a protected Windows/host process allowlist.
- **Fleet Port Mapper**: `win_port_map` correlates `netstat -ano` TCP/UDP PIDs against `mcp-central-docs/operations/WEBAPP_PORTS.md`.
- **Write-Safety Guards**: `win_fs_backup_write` / `win_fs_undo_backup` (`.bak` snapshots), `win_reg_write` (`.reg` backup export + `HKCU` confinement), and `win_svc_control` (NSSM-aware `sc.exe stop/start`).
- **8-Page Native Desktop Studio (`wincraft-app`)**: Host & Disks, Processes & Ports, RAM Weeder, FileOps, Lock Inspector, Services & NSSM, Startup & Registry, Doctor & Tools.
- **Cards (Default) / List / JSON View Switcher + Pagination, Sorting, Filtering & Export**: All collection outputs in `wincraft-app` and `wincraft` CLI default to **Cards** view and switch cleanly between `Cards`, `List` (table), and `JSON`, with live substring filtering, numeric-aware column sorting, pagination (`10`/`25`/`50`/`100`), and export to `CSV`, `MD` (Markdown table), `TXT`, or `JSON`.
