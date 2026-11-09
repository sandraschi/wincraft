# WinCraft

Pure-Rust Windows host, RAM Weeder, Restart Manager file-lock inspector, high-speed FileOps, and NSSM-safe SysAdmin studio (`v0.1.0`).

Subsumes `windows-operations-mcp`, `system-admin-mcp` (RAM Weeder), `sysinternals-mcp`, `filesystem-mcp`, `winrar-mcp`, and the retired Python `filecraft` scaffold into one zero-tax ArtCraft 3-tier workspace (`wincraft-core`, `wincraft-mcp`, `wincraft-app`).

## Quick Start

```powershell
cargo run -p wincraft-mcp --bin wincraft -- --smoke
cargo run -p wincraft-mcp --bin wincraft -- status
cargo run -p wincraft-mcp --bin wincraft -- proc-list --page-size 10
cargo run -p wincraft-mcp --bin wincraft -- svc-list --format list --sort-col 1 --page-size 15
cargo run -p wincraft-mcp --bin wincraft -- disk-list --format md --export exports\disks.csv
cargo run -p wincraft-mcp --bin wincraft -- lock-inspect --path D:\Dev\repos\wincraft\Cargo.toml
cargo run -p wincraft-mcp --bin wincraft -- weeder-scan
cargo run -p wincraft-mcp --bin wincraft -- port-map
cargo run -p wincraft-app
```

## Output Modes, Pagination, Sorting, Filtering & Export

Every collection output in both the L3 `wincraft-app` desktop UI and the L2 `wincraft` CLI (`processes`, `services`, `ports`, `disks`, `weeder` candidates, `fileops` entries/duplicates/grep matches, `locks`, `startup`, `tasks`, `registry`, and `tools`) supports:

- **View Switcher (Default: `Cards`):** Switch between **🗂 Cards (Default)**, **☰ List / Table**, and **`{ } JSON`** (via UI toolbar or `--format <cards|list|json|csv|md|txt>` / `--json`).
- **Live Filtering:** Case-insensitive substring filter across all columns (UI `Filter:` box or `--query-filter <TEXT>`).
- **Column Sorting:** Numeric-aware ascending/descending sort on any column (clickable column headers/pills in UI or `--sort-col <INDEX> [--desc]`).
- **Pagination:** Configurable page size (`10`, `25`, `50`, `100` in UI; `--page <N> --page-size <N>` in CLI) with `<< First`, `< Prev`, `Page X / Y`, `Next >`, `Last >>`.
- **Export (`CSV`, `MD`, `TXT`, `JSON`):** One-click export buttons in `wincraft-app` (`exports/<dataset>_<timestamp>.<ext>`) and `--export <path>` on any CLI verb.

## 26 MCP Tools (`wincraft mcp` on stdio)

| Domain | Tools |
|---|---|
| **System & Diagnostics** | `win_status`, `win_doctor`, `win_disk_list`, `win_env_audit` |
| **Processes & Ports** | `win_proc_list`, `win_port_map` (correlated with `WEBAPP_PORTS.md`) |
| **RAM Weeder** | `win_weeder_scan`, `win_weeder_reap` (`confirm=true`, protected system allowlist) |
| **File-Lock Inspector** | `win_lock_inspect` (native Win32 Restart Manager `rstrtmgr.dll` FFI) |
| **High-Speed FileOps** | `win_fs_list`, `win_fs_du`, `win_fs_large`, `win_fs_dups` (3-stage streaming SHA-256), `win_fs_hash`, `win_fs_grep`, `win_fs_backup_write` (mandatory `.bak`), `win_fs_undo_backup`, `win_fs_archive_inspect`, `win_fs_archive_create` |
| **SysAdmin & Registry** | `win_svc_list`, `win_svc_control` (NSSM-safe `sc.exe`), `win_startup_list`, `win_task_list`, `win_reg_read`, `win_reg_write` (auto `.reg` backup), `win_event_tail` |

## Safety Invariants

1. **Confirmation Gate:** Every mutating verb (`weeder_reap`, `fs_backup_write`, `fs_undo_backup`, `fs_archive_create`, `svc_control`, `reg_write`) requires `confirm: true` (or `--confirm`).
2. **Read-Only Switch:** `--read-only` or `WINCRAFT_READ_ONLY=1` blocks all mutations across CLI, MCP, and GUI.
3. **Backup-Before-Write:** `fs_backup_write` creates a timestamped `.bak` copy before modifying an existing file; `reg_write` exports a timestamped `.reg` snapshot to `%LOCALAPPDATA%\wincraft\backups\` before writing to `HKCU`.
4. **Protected Process Allowlist:** `weeder_reap` refuses to terminate system/host processes (`csrss.exe`, `lsass.exe`, `svchost.exe`, `explorer.exe`, `nssm.exe`, `pwsh.exe`, `code.exe`, `claude.exe`).
5. **NSSM Service Rule:** `svc_control` inspects `sc.exe qc` for NSSM wrappers and always uses `sc.exe stop`/`sc.exe start` rather than killing child processes.

## Lineage — the Storytold craft-app pattern

Storytold ArtCraft ([storytold/artcraft](https://github.com/storytold/artcraft) — "the IDE for artists") was the starting point. The 16 craft apps in the Storytold stable each ship one binary that is both the human CLI and the agent MCP server (stdio), headless-first, with deterministic JSON output and an in-process native GUI. That modus operandi and harness is what this repo implements; the code here is independent, the pattern is shared.
