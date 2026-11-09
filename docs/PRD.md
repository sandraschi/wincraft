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
