# WinCraft — Safety Architecture

1. **Confirm Gate (`confirm: true`):** All 6 mutating tools (`win_weeder_reap`, `win_fs_backup_write`, `win_fs_undo_backup`, `win_fs_archive_create`, `win_svc_control`, `win_reg_write`) refuse execution unless `confirm: true` (or `--confirm` on the CLI) is explicitly passed.
2. **Global Read-Only Mode (`--read-only` / `WINCRAFT_READ_ONLY=1`):** Overrides `confirm: true` and blocks all mutating operations across CLI, MCP, and L3 GUI.
3. **Path Confinement (`--root <dir>`):** When configured, `ExecOptions::check_path_allowed` canonicalizes every path and rejects any access outside `--root`.
4. **Protected Process Allowlist:** `win_weeder_reap` refuses to terminate system processes (`System`, `csrss.exe`, `lsass.exe`, `svchost.exe`, `explorer.exe`, `dwm.exe`), service wrappers (`nssm.exe`), and developer hosts (`pwsh.exe`, `code.exe`, `claude.exe`, `antigravity.exe`, `wincraft.exe`).
5. **Mandatory Pre-Write Backups:**
   - `win_fs_backup_write` copies any existing file to `<filename>.<epoch_ms>.bak` before writing.
   - `win_reg_write` exports the target `HKCU` registry key to `%LOCALAPPDATA%\wincraft\backups\<key>_<epoch_ms>.reg` via `reg.exe export` before writing.
6. **NSSM Service Protocol:** `win_svc_control` checks `sc.exe qc <service>` for `nssm` and executes `sc.exe stop` / `sc.exe start` rather than killing child processes directly.
