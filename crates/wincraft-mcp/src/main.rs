//! `wincraft` / `wincraft-mcp`: L2 unified CLI and stdio JSON-RPC 2.0 MCP server.
//! All verbs route through `wincraft_core::dispatch`.

use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use wincraft_core::{
    dispatch, filter_sort_paginate, payload_to_tabular, render_tabular, tools_catalog, Command,
    ExecOptions, OutputFormat,
};

#[derive(Parser, Debug)]
#[command(
    name = "wincraft",
    version,
    about = "WinCraft: Pure-Rust Windows host, RAM Weeder, Restart Manager lock inspector, FileOps, and NSSM-safe SysAdmin studio"
)]
struct Cli {
    /// Run deterministic self-test and exit
    #[arg(long)]
    smoke: bool,

    /// Enforce read-only mode (refuse all mutating verbs)
    #[arg(long, global = true)]
    read_only: bool,

    /// Confine all filesystem/lock/archive paths to this root directory
    #[arg(long, global = true)]
    root: Option<PathBuf>,

    /// Output format: cards (default), list, json, csv, md, txt
    #[arg(long, global = true, default_value = "cards")]
    format: String,

    /// Emit JSON output (shorthand for --format json)
    #[arg(long, global = true)]
    json: bool,

    /// Filter output rows by case-insensitive substring
    #[arg(long, global = true, default_value = "")]
    query_filter: String,

    /// Sort column index (0-based)
    #[arg(long, global = true)]
    sort_col: Option<usize>,

    /// Sort descending instead of ascending
    #[arg(long, global = true)]
    desc: bool,

    /// Page number (1-based)
    #[arg(long, global = true, default_value_t = 1)]
    page: usize,

    /// Rows per page (0 = all rows)
    #[arg(long, global = true, default_value_t = 25)]
    page_size: usize,

    /// Export formatted output to a file (.csv, .md, .txt, or .json)
    #[arg(long, global = true)]
    export: Option<PathBuf>,

    #[command(subcommand)]
    verb: Option<Verb>,
}

#[derive(Subcommand, Debug, Clone)]
enum Verb {
    /// Host summary: disks, processes, RAM weeder candidates, and PATH health
    Status,
    /// Probe native Restart Manager FFI, Windows system tools, and Sysinternals cache
    Doctor,
    /// List all 26 MCP tools and their safety metadata
    Tools,
    /// List logical disk volumes (A:..Z:) via Win32 GetDiskFreeSpaceExW
    DiskList,
    /// Audit PATH for dead/missing directories and duplicates
    EnvAudit,
    /// List running Windows processes sorted by memory
    ProcList {
        #[arg(long)]
        filter: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Map TCP/UDP ports to owning PIDs and correlate with fleet WEBAPP_PORTS.md
    PortMap {
        #[arg(long)]
        port: Option<u16>,
        #[arg(long)]
        all: bool,
    },
    /// Scan for orphaned cargo test binaries, headless engines, and runaway workers
    WeederScan,
    /// Terminate a non-protected process by PID (requires --confirm)
    WeederReap {
        #[arg(long)]
        pid: u32,
        #[arg(long)]
        confirm: bool,
    },
    /// Who locks this file or directory? (Native Windows Restart Manager rstrtmgr.dll)
    LockInspect {
        #[arg(long)]
        path: PathBuf,
    },
    /// List directory contents
    FsList {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        recursive: bool,
        #[arg(long, default_value_t = 2)]
        depth: usize,
    },
    /// Recursive directory disk usage breakdown + top extensions
    FsDu {
        #[arg(long)]
        path: PathBuf,
        #[arg(long, default_value_t = true)]
        skip_build_dirs: bool,
    },
    /// Find largest files exceeding threshold
    FsLarge {
        #[arg(long)]
        path: PathBuf,
        #[arg(long, default_value_t = 1_048_576)]
        min_size: u64,
        #[arg(long, default_value_t = 25)]
        limit: usize,
        #[arg(long, default_value_t = true)]
        skip_build_dirs: bool,
    },
    /// 3-stage streaming duplicate file detector
    FsDups {
        #[arg(long)]
        path: PathBuf,
        #[arg(long, default_value_t = true)]
        skip_build_dirs: bool,
    },
    /// Compute streaming FIPS 180-4 SHA-256 of a file
    FsHash {
        #[arg(long)]
        path: PathBuf,
    },
    /// Search text files for a query string
    FsGrep {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        query: String,
        #[arg(long)]
        ext: Option<String>,
        #[arg(long, default_value_t = 100)]
        max_matches: usize,
    },
    /// Write file with mandatory timestamped .bak backup
    FsBackupWrite {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        content: String,
        #[arg(long)]
        confirm: bool,
    },
    /// Restore most recent .bak backup for a file
    FsUndoBackup {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        confirm: bool,
    },
    /// Inspect entries inside a .zip archive
    FsArchiveInspect {
        #[arg(long)]
        path: PathBuf,
    },
    /// Create a .zip archive from a file or directory
    FsArchiveCreate {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        dest: PathBuf,
        #[arg(long)]
        confirm: bool,
    },
    /// List Windows services
    SvcList {
        #[arg(long)]
        filter: Option<String>,
        #[arg(long)]
        state: Option<String>,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Start, stop, or restart a Windows service via sc.exe (NSSM-safe)
    SvcControl {
        #[arg(long)]
        name: String,
        #[arg(long)]
        action: String,
        #[arg(long)]
        confirm: bool,
    },
    /// List Windows startup autorun entries
    StartupList,
    /// List Windows Scheduled Tasks
    TaskList {
        #[arg(long)]
        filter: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Read a Windows Registry key
    RegRead {
        #[arg(long)]
        key: String,
        #[arg(long)]
        value: Option<String>,
    },
    /// Write an HKCU Windows Registry value with automatic .reg backup
    RegWrite {
        #[arg(long)]
        key: String,
        #[arg(long)]
        value: String,
        #[arg(long, default_value = "REG_SZ")]
        reg_type: String,
        #[arg(long)]
        data: String,
        #[arg(long)]
        confirm: bool,
    },
    /// Tail Windows Event Log warnings and errors
    EventTail {
        #[arg(long, default_value = "System")]
        channel: String,
        #[arg(long, default_value_t = 10)]
        count: usize,
    },
    /// Start stdio JSON-RPC 2.0 MCP server
    Mcp,
}

fn verb_to_command(v: Verb) -> Option<Command> {
    match v {
        Verb::Status => Some(Command::Status),
        Verb::Doctor => Some(Command::Doctor),
        Verb::Tools => Some(Command::Tools),
        Verb::DiskList => Some(Command::DiskList),
        Verb::EnvAudit => Some(Command::EnvAudit),
        Verb::ProcList { filter, limit } => Some(Command::ProcList { filter, limit }),
        Verb::PortMap { port, all } => Some(Command::PortMap {
            port,
            listening_only: !all,
        }),
        Verb::WeederScan => Some(Command::WeederScan),
        Verb::WeederReap { pid, confirm } => Some(Command::WeederReap { pid, confirm }),
        Verb::LockInspect { path } => Some(Command::LockInspect { path }),
        Verb::FsList {
            path,
            recursive,
            depth,
        } => Some(Command::FsList {
            path,
            recursive,
            depth,
        }),
        Verb::FsDu {
            path,
            skip_build_dirs,
        } => Some(Command::FsDu {
            path,
            skip_build_dirs,
        }),
        Verb::FsLarge {
            path,
            min_size,
            limit,
            skip_build_dirs,
        } => Some(Command::FsLarge {
            path,
            min_size_bytes: min_size,
            limit,
            skip_build_dirs,
        }),
        Verb::FsDups {
            path,
            skip_build_dirs,
        } => Some(Command::FsDups {
            path,
            skip_build_dirs,
        }),
        Verb::FsHash { path } => Some(Command::FsHash { path }),
        Verb::FsGrep {
            path,
            query,
            ext,
            max_matches,
        } => Some(Command::FsGrep {
            path,
            query,
            ext,
            max_matches,
        }),
        Verb::FsBackupWrite {
            path,
            content,
            confirm,
        } => Some(Command::FsBackupWrite {
            path,
            content,
            confirm,
        }),
        Verb::FsUndoBackup { path, confirm } => Some(Command::FsUndoBackup { path, confirm }),
        Verb::FsArchiveInspect { path } => Some(Command::FsArchiveInspect { path }),
        Verb::FsArchiveCreate {
            source,
            dest,
            confirm,
        } => Some(Command::FsArchiveCreate {
            source,
            dest_zip: dest,
            confirm,
        }),
        Verb::SvcList {
            filter,
            state,
            limit,
        } => Some(Command::SvcList {
            filter,
            state,
            limit,
        }),
        Verb::SvcControl {
            name,
            action,
            confirm,
        } => Some(Command::SvcControl {
            name,
            action,
            confirm,
        }),
        Verb::StartupList => Some(Command::StartupList),
        Verb::TaskList { filter, limit } => Some(Command::TaskList { filter, limit }),
        Verb::RegRead { key, value } => Some(Command::RegRead { key, value }),
        Verb::RegWrite {
            key,
            value,
            reg_type,
            data,
            confirm,
        } => Some(Command::RegWrite {
            key,
            value,
            reg_type,
            data,
            confirm,
        }),
        Verb::EventTail { channel, count } => Some(Command::EventTail { channel, count }),
        Verb::Mcp => None,
    }
}

pub fn parse_mcp_tool_call(tool_name: &str, args: &Value) -> Result<Command, String> {
    let norm = tool_name.replace('.', "_");
    let str_arg = |k: &str| args.get(k).and_then(|v| v.as_str()).map(|s| s.to_string());
    let bool_arg = |k: &str, def: bool| args.get(k).and_then(|v| v.as_bool()).unwrap_or(def);
    let u64_arg = |k: &str, def: u64| args.get(k).and_then(|v| v.as_u64()).unwrap_or(def);

    match norm.as_str() {
        "win_status" | "status" => Ok(Command::Status),
        "win_doctor" | "doctor" => Ok(Command::Doctor),
        "win_tools" | "tools" => Ok(Command::Tools),
        "win_disk_list" | "disk_list" => Ok(Command::DiskList),
        "win_env_audit" | "env_audit" => Ok(Command::EnvAudit),
        "win_proc_list" | "proc_list" => Ok(Command::ProcList {
            filter: str_arg("filter"),
            limit: u64_arg("limit", 50) as usize,
        }),
        "win_port_map" | "port_map" => Ok(Command::PortMap {
            port: args.get("port").and_then(|v| v.as_u64()).map(|n| n as u16),
            listening_only: bool_arg("listening_only", true),
        }),
        "win_weeder_scan" | "weeder_scan" => Ok(Command::WeederScan),
        "win_weeder_reap" | "weeder_reap" => {
            let pid = args
                .get("pid")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| "Missing required integer argument 'pid'".to_string())?
                as u32;
            Ok(Command::WeederReap {
                pid,
                confirm: bool_arg("confirm", false),
            })
        }
        "win_lock_inspect" | "lock_inspect" => {
            let path = str_arg("path").ok_or_else(|| "Missing 'path'".to_string())?;
            Ok(Command::LockInspect {
                path: PathBuf::from(path),
            })
        }
        "win_fs_list" | "fs_list" => {
            let path = str_arg("path").ok_or_else(|| "Missing 'path'".to_string())?;
            Ok(Command::FsList {
                path: PathBuf::from(path),
                recursive: bool_arg("recursive", false),
                depth: u64_arg("depth", 2) as usize,
            })
        }
        "win_fs_du" | "fs_du" => {
            let path = str_arg("path").ok_or_else(|| "Missing 'path'".to_string())?;
            Ok(Command::FsDu {
                path: PathBuf::from(path),
                skip_build_dirs: bool_arg("skip_build_dirs", true),
            })
        }
        "win_fs_large" | "fs_large" => {
            let path = str_arg("path").ok_or_else(|| "Missing 'path'".to_string())?;
            Ok(Command::FsLarge {
                path: PathBuf::from(path),
                min_size_bytes: u64_arg("min_size_bytes", 1_048_576),
                limit: u64_arg("limit", 25) as usize,
                skip_build_dirs: bool_arg("skip_build_dirs", true),
            })
        }
        "win_fs_dups" | "fs_dups" => {
            let path = str_arg("path").ok_or_else(|| "Missing 'path'".to_string())?;
            Ok(Command::FsDups {
                path: PathBuf::from(path),
                skip_build_dirs: bool_arg("skip_build_dirs", true),
            })
        }
        "win_fs_hash" | "fs_hash" => {
            let path = str_arg("path").ok_or_else(|| "Missing 'path'".to_string())?;
            Ok(Command::FsHash {
                path: PathBuf::from(path),
            })
        }
        "win_fs_grep" | "fs_grep" => {
            let path = str_arg("path").ok_or_else(|| "Missing 'path'".to_string())?;
            let query = str_arg("query").ok_or_else(|| "Missing 'query'".to_string())?;
            Ok(Command::FsGrep {
                path: PathBuf::from(path),
                query,
                ext: str_arg("ext"),
                max_matches: u64_arg("max_matches", 100) as usize,
            })
        }
        "win_fs_backup_write" | "fs_backup_write" => {
            let path = str_arg("path").ok_or_else(|| "Missing 'path'".to_string())?;
            let content = str_arg("content").ok_or_else(|| "Missing 'content'".to_string())?;
            Ok(Command::FsBackupWrite {
                path: PathBuf::from(path),
                content,
                confirm: bool_arg("confirm", false),
            })
        }
        "win_fs_undo_backup" | "fs_undo_backup" => {
            let path = str_arg("path").ok_or_else(|| "Missing 'path'".to_string())?;
            Ok(Command::FsUndoBackup {
                path: PathBuf::from(path),
                confirm: bool_arg("confirm", false),
            })
        }
        "win_fs_archive_inspect" | "fs_archive_inspect" => {
            let path = str_arg("path").ok_or_else(|| "Missing 'path'".to_string())?;
            Ok(Command::FsArchiveInspect {
                path: PathBuf::from(path),
            })
        }
        "win_fs_archive_create" | "fs_archive_create" => {
            let source = str_arg("source").ok_or_else(|| "Missing 'source'".to_string())?;
            let dest_zip = str_arg("dest_zip").ok_or_else(|| "Missing 'dest_zip'".to_string())?;
            Ok(Command::FsArchiveCreate {
                source: PathBuf::from(source),
                dest_zip: PathBuf::from(dest_zip),
                confirm: bool_arg("confirm", false),
            })
        }
        "win_svc_list" | "svc_list" => Ok(Command::SvcList {
            filter: str_arg("filter"),
            state: str_arg("state"),
            limit: u64_arg("limit", 100) as usize,
        }),
        "win_svc_control" | "svc_control" => {
            let name = str_arg("name").ok_or_else(|| "Missing 'name'".to_string())?;
            let action = str_arg("action").ok_or_else(|| "Missing 'action'".to_string())?;
            Ok(Command::SvcControl {
                name,
                action,
                confirm: bool_arg("confirm", false),
            })
        }
        "win_startup_list" | "startup_list" => Ok(Command::StartupList),
        "win_task_list" | "task_list" => Ok(Command::TaskList {
            filter: str_arg("filter"),
            limit: u64_arg("limit", 50) as usize,
        }),
        "win_reg_read" | "reg_read" => {
            let key = str_arg("key").ok_or_else(|| "Missing 'key'".to_string())?;
            Ok(Command::RegRead {
                key,
                value: str_arg("value"),
            })
        }
        "win_reg_write" | "reg_write" => {
            let key = str_arg("key").ok_or_else(|| "Missing 'key'".to_string())?;
            let value = str_arg("value").ok_or_else(|| "Missing 'value'".to_string())?;
            let data = str_arg("data").ok_or_else(|| "Missing 'data'".to_string())?;
            Ok(Command::RegWrite {
                key,
                value,
                reg_type: str_arg("reg_type").unwrap_or_else(|| "REG_SZ".to_string()),
                data,
                confirm: bool_arg("confirm", false),
            })
        }
        "win_event_tail" | "event_tail" => Ok(Command::EventTail {
            channel: str_arg("channel").unwrap_or_else(|| "System".to_string()),
            count: u64_arg("count", 10) as usize,
        }),
        other => Err(format!("Unknown WinCraft MCP tool: {other}")),
    }
}

fn build_mcp_tools_schema() -> Value {
    let list: Vec<Value> = tools_catalog()
        .into_iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "annotations": {
                    "readOnlyHint": t.read_only,
                    "destructiveHint": t.confirm_gated,
                    "domain": t.domain,
                },
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Target file or directory path" },
                        "filter": { "type": "string", "description": "Optional name/substring filter" },
                        "query": { "type": "string", "description": "Search query for win_fs_grep" },
                        "pid": { "type": "integer", "description": "Process ID for win_weeder_reap" },
                        "port": { "type": "integer", "description": "Port filter for win_port_map" },
                        "name": { "type": "string", "description": "Service name for win_svc_control" },
                        "action": { "type": "string", "description": "start | stop | restart" },
                        "key": { "type": "string", "description": "Registry key path" },
                        "value": { "type": "string", "description": "Registry value name" },
                        "data": { "type": "string", "description": "Registry value data" },
                        "channel": { "type": "string", "description": "Event log channel (System or Application)" },
                        "confirm": { "type": "boolean", "description": "Required true for mutating verbs" }
                    }
                }
            })
        })
        .collect();
    json!({ "tools": list })
}

fn handle_rpc_request(req: &Value, opts: &ExecOptions) -> Option<Value> {
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = req.get("id").cloned();

    // JSON-RPC notifications have no id and expect no response
    id.as_ref()?;
    let req_id = id.unwrap_or(Value::Null);

    match method {
        "initialize" => Some(json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {},
                    "prompts": {},
                    "resources": {}
                },
                "serverInfo": {
                    "name": "wincraft-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        })),
        "ping" => Some(json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {}
        })),
        "tools/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": build_mcp_tools_schema()
        })),
        "prompts/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "prompts": [
                    {
                        "name": "windows_triage",
                        "description": "Full Windows host health, port conflict, and RAM weeder audit."
                    }
                ]
            }
        })),
        "resources/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "resources": [
                    {
                        "uri": "wincraft://status",
                        "name": "WinCraft Host Status",
                        "mimeType": "application/json"
                    }
                ]
            }
        })),
        "tools/call" => {
            let params = req.get("params").cloned().unwrap_or_else(|| json!({}));
            let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));

            match parse_mcp_tool_call(tool_name, &args) {
                Ok(cmd) => match dispatch(&cmd, opts) {
                    Ok(payload) => {
                        let text = serde_json::to_string_pretty(&payload)
                            .unwrap_or_else(|_| "{}".to_string());
                        Some(json!({
                            "jsonrpc": "2.0",
                            "id": req_id,
                            "result": {
                                "content": [{ "type": "text", "text": text }],
                                "isError": false
                            }
                        }))
                    }
                    Err(err) => {
                        let text =
                            serde_json::to_string_pretty(&err).unwrap_or_else(|_| err.to_string());
                        Some(json!({
                            "jsonrpc": "2.0",
                            "id": req_id,
                            "result": {
                                "content": [{ "type": "text", "text": text }],
                                "isError": true
                            }
                        }))
                    }
                },
                Err(parse_err) => Some(json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [{ "type": "text", "text": parse_err }],
                        "isError": true
                    }
                })),
            }
        }
        _ => Some(json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "error": {
                "code": -32601,
                "message": format!("Method not found: {method}")
            }
        })),
    }
}

fn run_mcp_stdio(opts: &ExecOptions) -> i32 {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let Ok(raw) = line else {
            break;
        };
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(req) = serde_json::from_str::<Value>(trimmed) else {
            let err_resp = json!({
                "jsonrpc": "2.0",
                "id": Value::Null,
                "error": { "code": -32700, "message": "Parse error" }
            });
            if let Ok(s) = serde_json::to_string(&err_resp) {
                let _ = writeln!(stdout, "{s}");
                let _ = stdout.flush();
            }
            continue;
        };
        if let Some(resp) = handle_rpc_request(&req, opts) {
            if let Ok(s) = serde_json::to_string(&resp) {
                let _ = writeln!(stdout, "{s}");
                let _ = stdout.flush();
            }
        }
    }
    0
}

fn run_smoke(opts: &ExecOptions) -> i32 {
    let Ok(status) = dispatch(&Command::Status, opts) else {
        eprintln!("smoke: FAILED status dispatch");
        return 1;
    };
    let Ok(doc) = dispatch(&Command::Doctor, opts) else {
        eprintln!("smoke: FAILED doctor dispatch");
        return 1;
    };
    let tool_count = status
        .get("tool_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let procs = status
        .get("running_processes")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let disks = status
        .get("disks")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let rm_ok = doc
        .pointer("/tools/restart_manager_rstrtmgr/available")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    println!(
        "smoke: ok (wincraft {}, {} tools, {} disks, {} procs, restart_manager={})",
        env!("CARGO_PKG_VERSION"),
        tool_count,
        disks,
        procs,
        rm_ok
    );
    0
}

fn emit_and_maybe_export(val: &Value, cli: &Cli) {
    let fmt = if cli.json {
        OutputFormat::Json
    } else {
        OutputFormat::parse_str(&cli.format)
    };

    let tab = payload_to_tabular(val);
    let ascending = !cli.desc;
    let effective_page_size = if cli.page_size == 0 {
        tab.rows.len().max(1)
    } else {
        cli.page_size
    };
    let (paged_tab, total_rows, total_pages) = filter_sort_paginate(
        tab.clone(),
        Some(cli.query_filter.as_str()),
        cli.sort_col,
        ascending,
        cli.page,
        effective_page_size,
    );

    if let Some(ref export_path) = cli.export {
        let ext = export_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let export_fmt = match ext.as_str() {
            "csv" => OutputFormat::Csv,
            "md" | "markdown" => OutputFormat::Md,
            "txt" => OutputFormat::Txt,
            "json" => OutputFormat::Json,
            _ => fmt,
        };
        let total_cap = tab.rows.len().max(1);
        let (all_filtered, _, _) = filter_sort_paginate(
            tab,
            Some(cli.query_filter.as_str()),
            cli.sort_col,
            ascending,
            1,
            total_cap,
        );
        let export_body = render_tabular(&all_filtered, val, export_fmt);
        if let Some(parent) = export_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::write(export_path, export_body) {
            Ok(()) => eprintln!(
                "[exported {} rows ({}) -> {}]",
                all_filtered.rows.len(),
                export_fmt.as_str(),
                export_path.display()
            ),
            Err(e) => eprintln!("[export failed -> {}: {e}]", export_path.display()),
        }
    }

    let rendered = render_tabular(&paged_tab, val, fmt);
    println!("{rendered}");
    if fmt != OutputFormat::Json && fmt != OutputFormat::Csv {
        println!(
            "--- Page {}/{} (showing {} of {} filtered rows; format={}) ---",
            cli.page.min(total_pages),
            total_pages,
            paged_tab.rows.len(),
            total_rows,
            fmt.as_str()
        );
    }
}

fn main() {
    let cli = Cli::parse();
    let mut opts = ExecOptions::default();
    if cli.read_only {
        opts.read_only = true;
    }
    if let Some(ref root) = cli.root {
        opts.root = Some(root.clone());
    }

    if cli.smoke {
        std::process::exit(run_smoke(&opts));
    }

    let Some(ref verb) = cli.verb else {
        // Default to MCP stdio when invoked as `wincraft-mcp` without args, or status otherwise
        let arg0 = std::env::args().next().unwrap_or_default().to_lowercase();
        if arg0.contains("wincraft-mcp") {
            std::process::exit(run_mcp_stdio(&opts));
        }
        match dispatch(&Command::Status, &opts) {
            Ok(v) => {
                emit_and_maybe_export(&v, &cli);
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    };

    let Some(cmd) = verb_to_command(verb.clone()) else {
        std::process::exit(run_mcp_stdio(&opts));
    };

    match dispatch(&cmd, &opts) {
        Ok(val) => {
            emit_and_maybe_export(&val, &cli);
        }
        Err(err) => {
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&err).unwrap_or_else(|_| err.to_string())
            );
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_initialize_and_tools_list_roundtrip() {
        let opts = ExecOptions::default();
        let init_req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });
        let resp = handle_rpc_request(&init_req, &opts);
        assert!(resp.is_some());

        let list_req = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        });
        let list_resp = handle_rpc_request(&list_req, &opts).unwrap_or_default();
        let count = list_resp
            .pointer("/result/tools")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        assert_eq!(count, 26);
    }

    #[test]
    fn mcp_dotted_and_underscored_tool_aliases_resolve() {
        let cmd1 = parse_mcp_tool_call("win_status", &json!({}));
        let cmd2 = parse_mcp_tool_call("win.status", &json!({}));
        assert!(cmd1.is_ok());
        assert!(cmd2.is_ok());
    }
}
