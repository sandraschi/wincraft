# `wincraft-mcp` (L2 Dual-Mode CLI & MCP Stdio Server)

Single binary (`wincraft`) serving both human/script CLI workflows and the Model Context Protocol (MCP) JSON-RPC 2.0 `stdio` server (31 tools).

## CLI Usage

Every collection command defaults to **Cards** view and supports `--format <cards|list|json|csv|md|txt>`, `--query-filter <TEXT>`, `--sort-col <N> [--desc]`, `--page <N> --page-size <N>`, and `--export <path>`:

```powershell
wincraft --smoke
wincraft status
wincraft proc-list --page-size 10
wincraft svc-list --format list --sort-col 1 --page-size 15
wincraft disk-list --format md --export exports\disks.csv
wincraft lock-inspect --path D:\Dev\repos\wincraft\Cargo.toml
wincraft cpu-audit --page-size 10
wincraft thermal
wincraft weeder-scan
wincraft port-map
wincraft mft-search --drive C:\ --filter *.iso --min-size-mb 1000
```

## MCP Stdio Server

Run `wincraft mcp` (stdout is reserved strictly for JSON-RPC 2.0 frames; diagnostics go to stderr):

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

## Safety Flags

- `--confirm` — Required on all mutating verbs (`weeder-reap`, `fs-backup-write`, `fs-undo-backup`, `fs-archive-create`, `svc-control`, `reg-write`).
- `--read-only` / `WINCRAFT_READ_ONLY=1` — Globally blocks all mutations.

## Verification

```powershell
cargo test -p wincraft-mcp
cargo run -p wincraft-mcp --bin wincraft -- --smoke
```
