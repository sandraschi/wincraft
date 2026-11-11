# WinCraft — Tiered Execution Plan

**Parent Standard:** [`RUST_ARTCRAFT_STANDARD.md`](../../mcp-central-docs/standards/RUST_ARTCRAFT_STANDARD.md)  
**Menu Bar Standard:** [`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md)  
**Architecture Model:** 4-Tier ArtCraft Incremental Delivery  
**Status:** Active  

---

## Architecture Flow

```
Tier 1: Core Engine ──> Tier 2: MCP / CLI ──> Tier 3: Desktop UI ──> Tier 4: Menu Bar, Help & Packaging
(wincraft-core)        (wincraft-mcp)        (wincraft-app)        (Hardened Workstation)
```

---

## Tier Specifications

### Tier 1 — Core Host Diagnostics & Win32 Engine (`wincraft-core`)
- [x] L1 `Command::dispatch` universal system execution bus.
- [x] Windows process, port, and RAM inspection engine (`Toolhelp32`, `IPHLPAPI`).
- [x] Win32 Restart Manager file-lock inspection (`rstrtmgr.dll`).
- [x] NSSM-safe service lifecycle control via `sc.exe`.
- [x] HKCU registry operations with pre-mutation `.reg` backups.
- [x] `#![forbid(unsafe_code)]` or strictly audited Win32 FFI containment.

### Tier 2 — Headless Driver & FastMCP (`wincraft-mcp`)
- [x] Unified CLI `wincraft` with `--json` output.
- [x] FastMCP stdio server communicating over JSON-RPC 2.0 (17 tools).
- [x] Zero network port footprint.
- [x] Headless `--smoke` verification without window initialization.

### Tier 3 — Desktop Workstation UI (`wincraft-app`)
- [x] Immediate-mode `egui` 0.37 / `wgpu` workstation.
- [x] Live process monitor with CPU/RAM metrics and port matrix.
- [x] File lock unlocker HUD and NSSM service control board.

### Tier 4 — Craft Menu Bar Standard, Help Suite & Hardened Verification
- [ ] Universal 8-menu bar (`Session`/`File`, `Edit`, `View`, `Insert`, `Commands`, `Window`, `Help`) with `switch_on_hover`.
- [ ] Mandatory 5-dialog Help suite (`Command Palette...`, `Keyboard Shortcuts...`, `Project Files & Docs...`, `Licenses & Notices...`, `About WinCraft`).
- [ ] Fleet UI/UX Pattern 2 adherence: viewport-centric navigation, top button row mapped to View toggles, panel visibility managed via `View` / `Window`.
- [ ] Tofu Guard verification (100% font glyph coverage, zero tofu placeholder boxes).
- [ ] Headless `--smoke` parity validation.
- [ ] Standalone release binary packaging and NSIS installer.
