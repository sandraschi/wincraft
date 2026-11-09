# WinCraft — Parity & Consolidation Matrix

| Capability | Legacy Python Source | WinCraft (`v0.1.0`) Status | Notes |
|---|---|---|---|
| Logical disk free/used GB | `system-admin-mcp`, `windows-operations-mcp` | **Shipped (`win_disk_list`)** | Native Win32 `GetLogicalDrives` + `GetDiskFreeSpaceExW` FFI |
| Process table & memory sorting | `system-admin-mcp`, `sysinternals-mcp` | **Shipped (`win_proc_list`)** | Includes protected-process allowlist classification |
| Port-to-PID + Fleet Repo Owner | `sysinternals-mcp` (`tcpvcon`) | **Shipped (`win_port_map`)** | Auto-correlates against `WEBAPP_PORTS.md` |
| RAM Weeder (orphan test/engine reaper) | `system-admin-mcp` (`ram_weeder_tool`) | **Shipped (`win_weeder_scan` / `win_weeder_reap`)** | Protected allowlist + `confirm=true` gate |
| Who Locks This File? | `sysinternals-mcp` (`handle64`) | **Shipped (`win_lock_inspect`)** | Native Win32 `rstrtmgr.dll` FFI (zero external binary required) |
| Directory size (`du`) & Large files | `filesystem-mcp`, `filecraft`, `sysinternals-mcp` | **Shipped (`win_fs_du`, `win_fs_large`)** | Skips `.git`/`target`/`node_modules` by default |
| Duplicate file detection | `filesystem-mcp`, `filecraft` | **Shipped (`win_fs_dups`)** | 3-stage: size -> 4KB SHA-256 prefix -> 64KB streaming SHA-256 |
| Atomic `.bak` write & undo | `filesystem-mcp` | **Shipped (`win_fs_backup_write`, `win_fs_undo_backup`)** | Timestamped `.bak` sibling backup + instant restore |
| Archive inspect & pack | `winrar-mcp`, `windows-operations-mcp` | **Shipped (`win_fs_archive_inspect`, `win_fs_archive_create`)** | Native `.zip` in Rust; probes `Rar.exe`/`7z.exe` in `win_doctor` |
| Windows Services & NSSM safety | `windows-operations-mcp`, `system-admin-mcp` | **Shipped (`win_svc_list`, `win_svc_control`)** | Detects NSSM wrapper via `sc.exe qc` |
| Registry CRUD with `.reg` backup | `windows-operations-mcp` | **Shipped (`win_reg_read`, `win_reg_write`)** | Auto-exports `.reg` backup before mutating `HKCU` |
| Autoruns, Scheduled Tasks, Event Logs | `windows-operations-mcp`, `sysinternals-mcp` | **Shipped (`win_startup_list`, `win_task_list`, `win_event_tail`)** | Native `reg.exe`, `schtasks.exe`, `wevtutil.exe` |
| Raw NTFS MFT undelete | `system-admin-mcp` | **Out of scope for v0.1.0** | Requires raw volume elevation (`\\.\C:`); stays in `system-admin-mcp` elevated bridge until Phase 2 |
