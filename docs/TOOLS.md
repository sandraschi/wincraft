# WinCraft — Tools Reference (26 Tools)

Every tool is available over stdio MCP (`win_<name>` or `win.<name>`) and via the `wincraft` CLI (`wincraft <kebab-verb>`).

| Tool | CLI Verb | Domain | Read-Only | Confirm Gate | Description |
|---|---|---|---|---|---|
| `win_status` | `status` | `system` | Yes | No | Host overview: disks, process count, weeder count, dead PATH count. |
| `win_doctor` | `doctor` | `system` | Yes | No | Check Restart Manager FFI, `sc.exe`, `reg.exe`, `wevtutil.exe`, `robocopy.exe`, `Rar.exe`, `7z.exe`, and Sysinternals cache. |
| `win_disk_list` | `disk-list` | `system` | Yes | No | Enumerate logical volumes (`A:`..`Z:`) via Win32 `GetDiskFreeSpaceExW`. |
| `win_env_audit` | `env-audit` | `system` | Yes | No | Audit `PATH` for dead/missing directories and duplicate entries. |
| `win_proc_list` | `proc-list` | `processes` | Yes | No | List running processes sorted by memory with protected status. |
| `win_port_map` | `port-map` | `processes` | Yes | No | Map TCP/UDP ports to PIDs and correlate with `WEBAPP_PORTS.md`. |
| `win_weeder_scan` | `weeder-scan` | `weeder` | Yes | No | Detect orphaned cargo test binaries, headless engines, and runaway workers. |
| `win_weeder_reap` | `weeder-reap` | `weeder` | **No** | **Yes** | Terminate a non-protected PID (`confirm=true`). |
| `win_lock_inspect` | `lock-inspect` | `fileops` | Yes | No | Query Win32 Restart Manager (`rstrtmgr.dll`) for processes locking a file/folder. |
| `win_fs_list` | `fs-list` | `fileops` | Yes | No | List directory contents with depth control. |
| `win_fs_du` | `fs-du` | `fileops` | Yes | No | Recursive directory disk usage + top file extensions. |
| `win_fs_large` | `fs-large` | `fileops` | Yes | No | Find largest files above byte threshold. |
| `win_fs_dups` | `fs-dups` | `fileops` | Yes | No | 3-stage streaming SHA-256 duplicate file finder. |
| `win_fs_hash` | `fs-hash` | `fileops` | Yes | No | Compute streaming FIPS 180-4 SHA-256 digest of a file. |
| `win_fs_grep` | `fs-grep` | `fileops` | Yes | No | Search text files while skipping binaries and build dirs. |
| `win_fs_backup_write` | `fs-backup-write` | `fileops` | **No** | **Yes** | Write file after creating a timestamped `.bak` backup (`confirm=true`). |
| `win_fs_undo_backup` | `fs-undo-backup` | `fileops` | **No** | **Yes** | Restore the most recent `.bak` backup for a file (`confirm=true`). |
| `win_fs_archive_inspect` | `fs-archive-inspect` | `fileops` | Yes | No | Inspect entries and compression ratios inside a `.zip` archive. |
| `win_fs_archive_create` | `fs-archive-create` | `fileops` | **No** | **Yes** | Create a deflated `.zip` archive from a file or directory (`confirm=true`). |
| `win_svc_list` | `svc-list` | `sysadmin` | Yes | No | List Windows services with state and PID. |
| `win_svc_control` | `svc-control` | `sysadmin` | **No** | **Yes** | Start, stop, or restart a service via `sc.exe` with NSSM detection (`confirm=true`). |
| `win_startup_list` | `startup-list` | `sysadmin` | Yes | No | List startup autoruns across `HKCU`/`HKLM` Run keys and Startup folder. |
| `win_task_list` | `task-list` | `sysadmin` | Yes | No | List Windows Scheduled Tasks (`schtasks`). |
| `win_reg_read` | `reg-read` | `sysadmin` | Yes | No | Read values and subkeys from a Registry key. |
| `win_reg_write` | `reg-write` | `sysadmin` | **No** | **Yes** | Write an `HKCU` Registry value after exporting a `.reg` backup (`confirm=true`). |
| `win_event_tail` | `event-tail` | `sysadmin` | Yes | No | Tail recent critical/error/warning events from Windows Event Logs. |
