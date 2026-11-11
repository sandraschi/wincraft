# WinCraft — Product Requirements Document (PRD)

**Status:** v0.1.0 shipped (2026-11-09)  
**Architecture:** Pure-Rust ArtCraft 3-tier (`wincraft-core`, `wincraft-mcp`, `wincraft-app`), stdio MCP, zero network ports.

## 1. Purpose & Consolidation Scope

Windows host diagnostics, file operations, and system administration were previously fragmented across five Python MCP servers (`windows-operations-mcp`, `system-admin-mcp`, `sysinternals-mcp`, `filesystem-mcp`, `winrar-mcp`) plus the retired Python `filecraft` scaffold. Running five Python virtual environments and four React webapps to inspect and clean up the Windows host consumed hundreds of MB of RAM.

**WinCraft** unifies these capabilities into one native Rust binary (`wincraft.exe` / `wincraft-app.exe`):
1. **Processes, Ports & RAM Weeder** (`system-admin-mcp` + `sysinternals-mcp`)
2. **High-Speed FileOps, Archives & Restart Manager Lock Inspection** (`filesystem-mcp` + `winrar-mcp` + `handle64`)
3. **NSSM-Safe Services, Autoruns, Scheduled Tasks, Registry & Event Logs** (`windows-operations-mcp`)

## 2. Core Model & Single Dispatch Bus

All operations route through `wincraft_core::dispatch(&Command, &ExecOptions) -> Result<Value, WinError>`.
- CLI (`wincraft <verb>`), MCP (`wincraft mcp`), and L3 GUI (`wincraft-app`) call the exact same `dispatch` function.
- `app_dispatch_matches_core_dispatch` verifies L1/L3 parity in CI.

## 3. Non-Goals

- **No kernel driver installation:** File-lock inspection uses the official Win32 Restart Manager (`rstrtmgr.dll`), not unsigned kernel drivers.
- **No HKLM / system-hive destruction:** `win_reg_write` is confined to `HKCU` by default and exports a `.reg` backup before every write.
- **No direct NSSM child-process killing:** `win_svc_control` uses `sc.exe stop/start` and respects NSSM service wrappers.

## 4. UI/UX Architecture & Menu Bar Standard

In accordance with [`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md):
- **Pattern 2 (Viewport-Centric Host Workstation):** Persistent process/port telemetry stage and lock inspection canvas. For service/system daemons where `File` is session-based, the menu bar implements the mandatory 5+ bookends (`Session`, `Edit`, `View`, `Insert`, `Commands`, `Window`, `Help`). Top quick button row acts as accelerators into the `View` menu. To add a new system inspector:
  1. Register a visibility toggle in the `View` or `Window` menu.
  2. Add an entry in the sidebar System / Diagnostics toolbox.
  3. Wire the action directly into the universal `Command::dispatch` bus.
- **Top Menu Bar Hierarchy:**
  - `Session`: Refresh Host Snapshot (`Ctrl+R`), Export System Diagnostics (`Ctrl+E`), Preferences (`Ctrl+,`), Quit (`Ctrl+Q`).
  - `Edit`: Copy Row / Telemetry, Select All, Search / Filter (`Ctrl+F`).
  - `View`: Process Manager, Port & Network Socket Matrix, Restart Manager File Locks, NSSM Services, Event Viewer, Theme (`Dark`/`Light`).
  - `Insert`: Add Scheduled Task, Create Service Entry, Register Environment Variable.
  - `Commands`: Kill Process Tree, Restart NSSM Service, Terminate Locking Processes, Flush Standby RAM, Query Event Logs.
  - `Window`: Reset Layout, Telemetry Viewport, Service Inspector, Diagnostics Log Dock.
  - `Help` (Mandatory 5-Dialog Suite): `Command Palette...` (`Ctrl+K`), `Keyboard Shortcuts...` (`F1`), `Project Files & Docs...`, `Licenses & Notices...`, `About WinCraft`.
- **Behavioral Mechanics:** Smooth `switch_on_hover` between top-level menus, zero-unsafe immediate-mode rendering, and headless `--smoke` parity.
