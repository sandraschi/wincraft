# `wincraft-core` (L1 Domain Engine)

Pure-Rust, UI-agnostic Windows host diagnostics, RAM Weeder, Restart Manager file-lock inspector, streaming FileOps, NSSM-safe SysAdmin, and NTFS MFT storage engine.

Both `wincraft-mcp` (L2 CLI + MCP stdio) and `wincraft-app` (L3 egui desktop) execute all 31 verbs through `wincraft_core::dispatch(cmd, opts)` so CLI, MCP, and GUI never diverge.

## Modules

| Module | Purpose |
|---|---|
| `lib.rs` | Unified `dispatch(cmd, opts)` bus, `tools_catalog()`, confirmation & read-only safety gates. |
| `cpu_audit.rs` | CPU audit, per-core load, and thermal sensor telemetry (`win_cpu_audit`, `win_thermal_status`). |
| `proc_ports.rs` | Process list, RAM Working Set Weeder with protected system allowlist, and `WEBAPP_PORTS.md`-correlated port mapper. |
| `locks.rs` | Native Win32 Restart Manager FFI (`rstrtmgr.dll`) file-lock inspector (`win_lock_inspect`). |
| `fileops.rs` & `hash.rs` | High-speed directory listing, disk usage, large-file scan, 3-stage streaming SHA-256 duplicate finder, regex grep, mandatory `.bak` backup writes, and archive inspect/create. |
| `sysadmin.rs` | NSSM-safe Windows service controller (`sc.exe stop/start`), startup items, scheduled tasks, HKCU registry read/write with auto `.reg` snapshots, and Windows event log tail. |
| `wiztree/` | Vendored WizTree NTFS `$MFT` engine (`cli_driver`, `csv_parser`, `drives`, `finder`, `models`, `treemap`) for sub-second search and hierarchical cushion treemap data. |
| `export.rs` | `OutputFormat::Cards` (default), `List`, `JSON`, `CSV`, `MD`, `TXT`, live filtering, numeric-aware sorting, and pagination. |

## Safety Invariants

- Workspace denies `unwrap_used`, `expect_used`, `panic`, `unimplemented`, and `todo`.
- Mutating verbs require `confirm: true`; `WINCRAFT_READ_ONLY=1` blocks all mutations globally.
- `weeder_reap` refuses to terminate protected host processes (`csrss.exe`, `lsass.exe`, `svchost.exe`, `explorer.exe`, `nssm.exe`, `pwsh.exe`, `code.exe`, `claude.exe`).

## Verification

```powershell
cargo test -p wincraft-core
cargo clippy -p wincraft-core --all-targets -- -D warnings
```
