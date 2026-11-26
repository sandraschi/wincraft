# WinCraft 0.3.0 - first tagged release

No prior tags existed; v0.3.0 consolidates the v0.1.0 core, the v0.2.0
CPU audit and thermals, and everything after (T1/T2 floors, menu bar
standard, MFT storage tools, cushion treemap, light theme, LF writes,
legal files).

## Highlights

- 31 MCP tools on stdio (`wincraft mcp`), including new MFT storage
  tools: `win_mft_search` (sub-second NTFS wildcard search),
  `win_disk_usage` (space and extension summary), `win_treemap`
  (self-contained D3 cushion HTML).
- L3 `wincraft-app`: 10-page egui workstation, Craft menu bar
  (`File` through `Help`), Command Palette (`Ctrl+K`), Page 10
  hierarchical cushion treemap with breadcrumb drill-down and
  Relaunch as Admin.
- Publishable `cushion-treemap` crate (squarified layout) backing
  the treemap in `wincraft-core`.
- CPU audit (`win_cpu_audit`) and thermal diagnostics
  (`win_thermal_status`) with dedicated L3 page and one-click reap.
- Safety invariants unchanged: `confirm: true` on mutating verbs,
  `--read-only` switch, mandatory `.bak`/`.reg` backups, protected
  process allowlist, NSSM-safe service control.

## Verify

```powershell
cargo run -p wincraft-mcp --bin wincraft -- status
cargo run -p wincraft-mcp --bin wincraft -- cpu-audit --page-size 10
cargo run -p wincraft-app
```

Artifacts: `wincraft.exe`, `wincraft-app.exe`,
`wincraft-0.3.0-setup.exe` (per-user NSIS installer),
`wincraft-0.3.0-windows-x64-portable.zip` (launcher extra),
`wincraft-v0.3.0.mcpb`, `SHA256SUMS.txt`.
