# Installing & Running WinCraft

## Build from Source

```powershell
cd D:\Dev\repos\wincraft
cargo build --release
.\target\release\wincraft.exe --smoke
```

## Claude Desktop / Cursor / Antigravity MCP Configuration

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

Pass `"args": ["--read-only", "mcp"]` to run in strict read-only mode, or `"args": ["--root", "D:\\Dev\\repos", "mcp"]` to confine file operations to a specific directory tree.
