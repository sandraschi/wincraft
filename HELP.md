# WinCraft Help & Operator Guide

**Parent Hub:** [RUST_ARTCRAFT_STANDARD.md](../../mcp-central-docs/standards/rust-artcraft/RUST_ARTCRAFT_STANDARD.md)  
**Standard Reference:** [CRAFT_MENU_BAR_STANDARD.md](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md)

---

## 1. Get Started (5-Step Tour)

1. **Launch the Workstation**: Run `cargo run -p wincraft-app` or start `wincraft-app.exe` from your Start Menu.
2. **Review Host Metrics**: On Workspace 1 (**Host & Disks**), check volume free space, memory usage, and storage health.
3. **Elevate for MFT Access**: If running as a standard user, click **Relaunch as Admin** in the top bar to restart via UAC and unlock raw NTFS direct-read access.
4. **Scan Storage with Treemaps**: Navigate to Workspace 10 (**MFT Cushion Treemap**), select a fixed drive, and click **Scan MFT Drive** for instant sub-second cushion treemap visualization. Click any tile to drill down into folders.
5. **Clean Orphaned Test Processes**: Navigate to Workspace 4 (**RAM Weeder**), inspect zombie dev processes, arm the confirmation gate (`Alt+A` or `Edit -> Arm Confirm Gate`), and safely reclaim RAM.

---

## 2. Functions & Engine Capabilities

- **WizTree NTFS MFT Direct Engine**: Bypasses the Windows Win32 API to parse NTFS `$MFT` directly in sub-seconds, generating space metrics, extension distributions, and hierarchical enveloping cushion treemaps.
- **RAM Working Set Weeder**: Scans running processes for abandoned engines, orphan `cargo test` runners, and memory leaks. Employs a protected system allowlist (`csrss.exe`, `lsass.exe`, `svchost.exe`, `explorer.exe`, `nssm.exe`, `pwsh.exe`) that can never be terminated.
- **Restart Manager File-Lock Inspector**: FFI bridge into native `rstrtmgr.dll` (`RmStartSession`, `RmRegisterResources`, `RmGetList`) to pinpoint which processes hold open handles on any file or directory.
- **High-Speed FileOps**: 3-stage streaming SHA-256 duplicate detection (quick size pre-filter -> 4KB prefix hash -> full cryptographic SHA-256), fast regex grep, and mandatory `.bak` backups before modifying files.
- **Services & SysAdmin Studio**: NSSM-safe Windows service management (`sc.exe stop/start`), scheduled task inspection, and HKCU registry editor with automated `.reg` snapshot exports.

---

## 3. Usage (GUI, CLI & MCP)

### Workstation (GUI)
- Native pure-Rust immediate-mode desktop application (`egui 0.37` + `eframe`).
- Zero webview, zero Node/Bun runtime, zero open ports.
- Cards default view switchable to List/Table or raw JSON.
- Global `Ctrl+Wheel` UI zoom (0.6x to 2.5x) and high-contrast Slate-950 dark theme.

### Command-Line Interface (CLI)
Run any of the 31 verbs directly with structured formatting:
```powershell
# Status overview in default Cards format
cargo run -p wincraft-mcp --bin wincraft -- status

# Process list in tabular format
cargo run -p wincraft-mcp --bin wincraft -- proc-list --format list --page-size 20

# Scan disk usage and export to CSV
cargo run -p wincraft-mcp --bin wincraft -- disk-list --export exports\disks.csv

# Find which processes lock a file
cargo run -p wincraft-mcp --bin wincraft -- lock-inspect --path D:\Dev\repos\wincraft\Cargo.toml

# Fast MFT search for files larger than 1 GB
cargo run -p wincraft-mcp --bin wincraft -- mft-search --drive C:\ --filter *.iso --min-size-mb 1000
```

### Model Context Protocol (MCP) Server
WinCraft exposes 31 native tools over `stdio` for agent integration:
```powershell
cargo run -p wincraft-mcp --bin wincraft -- mcp
```
Configure in Claude Desktop (`claude_desktop_config.json`) or Antigravity:
```json
{
  "mcpServers": {
    "wincraft": {
      "command": "D:\\Dev\\repos\\wincraft\\target\\release\\wincraft.exe",
      "args": ["mcp"]
    }
  }
}
```

---

## 4. Tips & Tricks

- **Command Palette (`Ctrl+K`)**: Instantly execute any menu command or tool by fuzzy-searching in the Command Palette.
- **Interactive D3 Treemap**: Click **Open Interactive D3 in Browser** on Page 10 to inspect an interactive zoomable D3 sunburst/treemap in your default web browser.
- **Card Action Pills**: On Page 1 (**Host & Disks**), click the quick action pills on any drive card to jump directly to FileOps browse, large files, or duplicate scans for that volume.
- **Reset Zoom**: Press `Ctrl+1` or click the zoom percentage badge in the top bar to instantly reset UI zoom to 100%.

---

## 5. Common Pitfalls & Safety Invariants

1. **Confirmation Gate Required**: All mutating actions (`weeder_reap`, `fs_backup_write`, `fs_undo_backup`, `fs_archive_create`, `svc_control`, `reg_write`) fail immediately unless explicitly armed via `Arm Confirm Gate` or `--confirm`.
2. **Read-Only Guard**: Toggling `Read-Only Guard` (`WINCRAFT_READ_ONLY=1`) blocks all write actions fleet-wide across CLI, GUI, and MCP.
3. **Protected Process Immunity**: System-critical processes (`csrss`, `lsass`, `svchost`, `explorer`, `nssm`, `pwsh`, `code`, `claude`) are protected by hardcoded safety predicates and cannot be killed.
4. **NSSM Service Restart Rule**: NSSM-managed services are restarted strictly via `sc.exe stop` and `sc.exe start`. Never terminate child service processes directly.
5. **Mandatory Backups**: File writes automatically create timestamped `.bak` files; registry modifications export a `.reg` file to `%LOCALAPPDATA%\wincraft\backups\` prior to writing.

---

## 6. Planned Extensions

- **Volume Shadow Copy (VSS) Browser**: Snapshot traversal for point-in-time file recovery.
- **Network Adapter & Packet Inspection**: Real-time throughput metrics correlated with process PIDs.
- **Fleet Worktree Health Monitor**: Direct integration with DevCraft worktrees.

---

## 7. Ecosystem & Other Crafts

WinCraft coordinates with the wider Storytold Craft ecosystem:

| Craft | Relationship to WinCraft |
|---|---|
| **DevCraft** | Fleet repo management, worktree checkpoints, and service port coordination. |
| **Containercraft** | Docker and Podman container lifecycle management on Windows hosts. |
| **Deskcraft** | Virtual desktop and multi-monitor workspace layout automation. |
| **Taskcraft** | Background task orchestration and job scheduling. |
