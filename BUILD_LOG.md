# WinCraft BUILD_LOG

Running record of release builds: failures, regressions, and fixes.
Per fleet NSIS Build Gate, updated after every build.

## v0.3.0 - 2026-11-26 (first tagged release)

- Pipeline: `scripts/release.ps1 -Publish`. All phases green, no
  failures, no regressions.
- Preflight: version 0.3.0, tree clean, on `main`, no stray branches,
  tag free, `CHANGELOG_LATEST.md` present, `makensis` found.
- Gates: `cargo fmt --all --check` clean; `cargo clippy --workspace
  --all-targets --all-features -- -D warnings` 0 warnings;
  `cargo test --workspace` 66 passed / 0 failed
  (8 cushion-treemap + 8 app + 18 core + 29 T2 integration + 2 mcp
  + 1 doctest).
- Release build: `wincraft-app.exe` 16.4 MB, `wincraft.exe` 2.9 MB.
  Smoke green: app, cli, MCP initialize.
- Artifacts (`dist/`): `wincraft-app.exe`, `wincraft.exe`,
  `wincraft-0.3.0-setup.exe` (5.1 MB, NSIS per-user),
  `wincraft-0.3.0-windows-x64-portable.zip` (launcher extra),
  `wincraft-v0.3.0.mcpb` (1.1 MB), `SHA256SUMS.txt`.
- Publish: tag `v0.3.0` pushed, GitHub release created (full release,
  not pre-release), 6 assets uploaded.
  https://github.com/sandraschi/wincraft/releases/tag/v0.3.0
- Pre-release prep (same day, commit `bc8398b`): workspace 0.1.0 to
  0.3.0 (no prior tags existed), `mcpb/manifest.json` regenerated
  from the live server (28 to 31 tools: `win_mft_search`,
  `win_disk_usage`, `win_treemap`), `CHANGELOG_LATEST.md` written.
- Environment note (fleet-wide, not wincraft-specific):
  `mcp-central-docs/scripts/gen-craft-mcpb-manifests.py` fails under
  opencode's shell env because a pwsh7 `PSModulePath` leaks into the
  `powershell.exe` (5.1) child, breaking `Import-PowerShellDataFile`
  auto-load (`load_config` gets empty stdout, then `json.loads('')`
  raises `Expecting value: line 1 column 1`). Workaround: set a 5.1
  `PSModulePath` before launching it:
  `C:\Users\sandr\OneDrive\Dokumente\WindowsPowerShell\Modules;C:\Program Files\WindowsPowerShell\Modules;C:\WINDOWS\system32\WindowsPowerShell\v1.0\Modules`.
  Candidate for `troubleshooting/BUGS_DEPOT.md`.
