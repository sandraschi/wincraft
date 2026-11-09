# Changelog

## [0.1.0] - 2026-11-09

### Added
- Initial Pure-Rust ArtCraft 3-tier release (`wincraft-core`, `wincraft-mcp`, `wincraft-app`) subsuming Windows host, FileOps, RAM Weeder, and Sysinternals/SysAdmin operations.
- **Native Win32 Restart Manager (`rstrtmgr.dll` FFI)**: `win_lock_inspect` identifies exact locking PIDs and application names for any file or folder without third-party drivers.
- **3-Stage Streaming Duplicate Detector**: `win_fs_dups` groups by file size, filters by 4 KB SHA-256 prefix, and streams 64 KB blocks for full FIPS 180-4 SHA-256 verification.
- **RAM Weeder**: `win_weeder_scan` and confirm-gated `win_weeder_reap` with a protected Windows/host process allowlist.
- **Fleet Port Mapper**: `win_port_map` correlates `netstat -ano` TCP/UDP PIDs against `mcp-central-docs/operations/WEBAPP_PORTS.md`.
- **Write-Safety Guards**: `win_fs_backup_write` / `win_fs_undo_backup` (`.bak` snapshots), `win_reg_write` (`.reg` backup export + `HKCU` confinement), and `win_svc_control` (NSSM-aware `sc.exe stop/start`).
- **8-Page Native Desktop Studio (`wincraft-app`)**: Host & Disks, Processes & Ports, RAM Weeder, FileOps, Lock Inspector, Services & NSSM, Startup & Registry, Doctor & Tools.
