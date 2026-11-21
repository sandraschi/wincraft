# WinCraft — AGENTS.md

## Architecture

Pure-Rust ArtCraft 3-tier workspace:
- **L1 `crates/wincraft-core`**: Single `dispatch(cmd, opts)` bus + `export` engine (`OutputFormat::Cards` default, `TabularData`, `filter_sort_paginate`, and `CSV`/`MD`/`TXT`/`JSON` formatters). Native Win32 `rstrtmgr.dll` Restart Manager file-lock inspector, `kernel32.dll` logical volume inspector, 3-stage streaming SHA-256 duplicate finder, RAM Weeder, `WEBAPP_PORTS.md`-correlated port mapper, NSSM-safe service controller, and auto-backed-up Registry operations.
- **L2 `crates/wincraft-mcp`**: Unified `wincraft` CLI (defaulting to Cards view with `--format <cards|list|json|csv|md|txt>`, `--query-filter`, `--sort-col`, `--desc`, `--page`, `--page-size`, `--export <path>`) + `wincraft-mcp` stdio JSON-RPC 2.0 server (28 tools).
- **L3 `crates/wincraft-app`**: Native `eframe`/`egui` 9-page Windows workstation calling `wincraft_core::dispatch` in-process, with Cards (default) / List / JSON view switching, live filtering, sortable columns, pagination, and one-click CSV/MD/TXT/JSON export.

## Hard Rules

- `unwrap_used`, `expect_used`, `panic`, `unimplemented`, and `todo` are **denied** at workspace level.
- CLI, MCP, and L3 GUI must never diverge: all commands go through `wincraft_core::dispatch`.
- Never bypass `confirm: true` on mutating verbs or remove `.bak` / `.reg` pre-write backups.
- Commit with `-c user.name="Sandra Schipal (<tool>)"` and max 10 files per commit.


## Documentation preservation and hooks

On 2026-11-20 a README "standardization" sweep across the fleet deleted project descriptions,
status sections and user warnings in nine repos. Full rules:
`mcp-central-docs/standards/CONTENT_PRESERVATION_STANDARD.md`.

- Applying any standard or template restructures; it never deletes. Move content (within the file
  or to `docs/` with a link), do not drop it. Never remove what the project is, what it is based on,
  its status (what works, what does not, what is not built) or warnings to users.
- Remove content only when Sandra explicitly asks for that removal, in this repo.
- Never add claims you have not verified (release downloads, versions, test counts, features).
- Only standards with `status: active` may be applied across repos. A change across several repos is
  done on one repo first and approved by Sandra; its commits name the tool and carry a `Sweep-Id:` line.
- `.githooks/commit-msg` blocks a commit that deletes a Markdown file, removes 20% of its lines
  (15 or more) or removes 3 or more headings. A removal Sandra asked for passes with
  `Docs-Removal-Approved: <who and why>` in the message. Enable once per clone:
  `git config core.hooksPath .githooks`. Never use `--no-verify`; if a hook blocks you, stop and ask.
