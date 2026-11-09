# WinCraft — AGENTS.md

## Architecture

Pure-Rust ArtCraft 3-tier workspace:
- **L1 `crates/wincraft-core`**: Single `dispatch(cmd, opts)` bus. Native Win32 `rstrtmgr.dll` Restart Manager file-lock inspector, `kernel32.dll` logical volume inspector, 3-stage streaming SHA-256 duplicate finder, RAM Weeder, `WEBAPP_PORTS.md`-correlated port mapper, NSSM-safe service controller, and auto-backed-up Registry operations.
- **L2 `crates/wincraft-mcp`**: Unified `wincraft` CLI + `wincraft-mcp` stdio JSON-RPC 2.0 server (26 tools).
- **L3 `crates/wincraft-app`**: Native `eframe`/`egui` 8-page Windows workstation calling `wincraft_core::dispatch` in-process.

## Hard Rules

- `unwrap_used`, `expect_used`, `panic`, `unimplemented`, and `todo` are **denied** at workspace level.
- CLI, MCP, and L3 GUI must never diverge: all commands go through `wincraft_core::dispatch`.
- Never bypass `confirm: true` on mutating verbs or remove `.bak` / `.reg` pre-write backups.
- Commit with `-c user.name="Sandra Schipal (<tool>)"` and max 10 files per commit.
