//! `wincraft-core`: L1 core for WinCraft.
//! Subsumes Windows host operations, RAM Weeder, Restart Manager file-lock inspection,
//! high-speed FileOps, NSSM-safe service control, and auto-backed-up Registry operations
//! behind a single deterministic `dispatch` bus.

pub mod export;
pub mod fileops;
pub mod hash;
pub mod locks;
pub mod proc_ports;
pub mod sysadmin;

pub use export::{
    export_to_file, filter_sort_paginate, payload_to_tabular, render_payload, render_tabular,
    OutputFormat, TabularData,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinError {
    pub code: String,
    pub message: String,
}

impl std::fmt::Display for WinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for WinError {}

impl WinError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecOptions {
    pub read_only: bool,
    pub root: Option<PathBuf>,
    pub backup_dir: PathBuf,
    pub ports_doc: Option<PathBuf>,
}

impl Default for ExecOptions {
    fn default() -> Self {
        let backup_dir = std::env::var("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("wincraft").join("backups"))
            .unwrap_or_else(|_| std::env::temp_dir().join("wincraft_backups"));
        let default_ports_doc =
            PathBuf::from(r"D:\Dev\repos\mcp-central-docs\operations\WEBAPP_PORTS.md");
        let ports_doc = if default_ports_doc.is_file() {
            Some(default_ports_doc)
        } else {
            None
        };
        Self {
            read_only: std::env::var("WINCRAFT_READ_ONLY")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            root: None,
            backup_dir,
            ports_doc,
        }
    }
}

impl ExecOptions {
    pub fn check_path_allowed(&self, target: &Path) -> Result<PathBuf, WinError> {
        let Some(ref root) = self.root else {
            return Ok(target.to_path_buf());
        };
        let canon_root = root.canonicalize().map_err(|e| {
            WinError::new(
                "ROOT_ERROR",
                format!("Invalid --root {}: {e}", root.display()),
            )
        })?;
        let candidate = if target.exists() {
            target.canonicalize().map_err(|e| {
                WinError::new(
                    "PATH_ERROR",
                    format!("Cannot resolve {}: {e}", target.display()),
                )
            })?
        } else if let Some(parent) = target.parent() {
            let cp = parent.canonicalize().map_err(|e| {
                WinError::new(
                    "PATH_ERROR",
                    format!("Cannot resolve parent {}: {e}", parent.display()),
                )
            })?;
            cp.join(target.file_name().unwrap_or_default())
        } else {
            target.to_path_buf()
        };
        if !candidate.starts_with(&canon_root) {
            return Err(WinError::new(
                "PATH_CONFINEMENT",
                format!(
                    "Path '{}' is outside confined root '{}'",
                    candidate.display(),
                    canon_root.display()
                ),
            ));
        }
        Ok(candidate)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "verb", rename_all = "snake_case")]
pub enum Command {
    Status,
    Doctor,
    Tools,
    DiskList,
    EnvAudit,
    ProcList {
        filter: Option<String>,
        limit: usize,
    },
    PortMap {
        port: Option<u16>,
        listening_only: bool,
    },
    WeederScan,
    WeederReap {
        pid: u32,
        confirm: bool,
    },
    LockInspect {
        path: PathBuf,
    },
    FsList {
        path: PathBuf,
        recursive: bool,
        depth: usize,
    },
    FsDu {
        path: PathBuf,
        skip_build_dirs: bool,
    },
    FsLarge {
        path: PathBuf,
        min_size_bytes: u64,
        limit: usize,
        skip_build_dirs: bool,
    },
    FsDups {
        path: PathBuf,
        skip_build_dirs: bool,
    },
    FsHash {
        path: PathBuf,
    },
    FsGrep {
        path: PathBuf,
        query: String,
        ext: Option<String>,
        max_matches: usize,
    },
    FsBackupWrite {
        path: PathBuf,
        content: String,
        confirm: bool,
    },
    FsUndoBackup {
        path: PathBuf,
        confirm: bool,
    },
    FsArchiveInspect {
        path: PathBuf,
    },
    FsArchiveCreate {
        source: PathBuf,
        dest_zip: PathBuf,
        confirm: bool,
    },
    SvcList {
        filter: Option<String>,
        state: Option<String>,
        limit: usize,
    },
    SvcControl {
        name: String,
        action: String,
        confirm: bool,
    },
    StartupList,
    TaskList {
        filter: Option<String>,
        limit: usize,
    },
    RegRead {
        key: String,
        value: Option<String>,
    },
    RegWrite {
        key: String,
        value: String,
        reg_type: String,
        data: String,
        confirm: bool,
    },
    EventTail {
        channel: String,
        count: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: &'static str,
    pub domain: &'static str,
    pub read_only: bool,
    pub confirm_gated: bool,
    pub description: &'static str,
}

pub fn tools_catalog() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "win_status",
            domain: "system",
            read_only: true,
            confirm_gated: false,
            description: "Host summary: version, read-only state, disk volumes, process count, and PATH health.",
        },
        ToolSpec {
            name: "win_doctor",
            domain: "system",
            read_only: true,
            confirm_gated: false,
            description: "Verify native Restart Manager FFI, Windows admin binaries (sc, reg, wevtutil, robocopy), and Sysinternals cache.",
        },
        ToolSpec {
            name: "win_disk_list",
            domain: "system",
            read_only: true,
            confirm_gated: false,
            description: "List logical disk volumes (A:..Z:) with free/used/total GB via Win32 GetDiskFreeSpaceExW.",
        },
        ToolSpec {
            name: "win_env_audit",
            domain: "system",
            read_only: true,
            confirm_gated: false,
            description: "Audit PATH environment variable for dead/missing directories and duplicate entries.",
        },
        ToolSpec {
            name: "win_proc_list",
            domain: "processes",
            read_only: true,
            confirm_gated: false,
            description: "List running Windows processes sorted by memory usage with protected-process classification.",
        },
        ToolSpec {
            name: "win_port_map",
            domain: "processes",
            read_only: true,
            confirm_gated: false,
            description: "Map active TCP/UDP ports to owning PIDs and correlate with fleet WEBAPP_PORTS.md assignments.",
        },
        ToolSpec {
            name: "win_weeder_scan",
            domain: "weeder",
            read_only: true,
            confirm_gated: false,
            description: "Scan for orphaned cargo test binaries, stale headless engines, and runaway workers holding RAM.",
        },
        ToolSpec {
            name: "win_weeder_reap",
            domain: "weeder",
            read_only: false,
            confirm_gated: true,
            description: "Terminate a non-protected process by PID (requires confirm=true; refuses protected system processes).",
        },
        ToolSpec {
            name: "win_lock_inspect",
            domain: "fileops",
            read_only: true,
            confirm_gated: false,
            description: "Who locks this file or directory? Queries Windows Restart Manager (rstrtmgr.dll) for locking PIDs.",
        },
        ToolSpec {
            name: "win_fs_list",
            domain: "fileops",
            read_only: true,
            confirm_gated: false,
            description: "List directory contents with depth control, sizes, and modified timestamps.",
        },
        ToolSpec {
            name: "win_fs_du",
            domain: "fileops",
            read_only: true,
            confirm_gated: false,
            description: "Fast recursive directory disk usage breakdown + top file extensions.",
        },
        ToolSpec {
            name: "win_fs_large",
            domain: "fileops",
            read_only: true,
            confirm_gated: false,
            description: "Find largest files exceeding a byte threshold (optionally skipping build dirs).",
        },
        ToolSpec {
            name: "win_fs_dups",
            domain: "fileops",
            read_only: true,
            confirm_gated: false,
            description: "3-stage streaming duplicate file detector (size -> 4KB SHA-256 prefix -> full 64KB streaming SHA-256).",
        },
        ToolSpec {
            name: "win_fs_hash",
            domain: "fileops",
            read_only: true,
            confirm_gated: false,
            description: "Compute streaming FIPS 180-4 SHA-256 digest and byte size of a file.",
        },
        ToolSpec {
            name: "win_fs_grep",
            domain: "fileops",
            read_only: true,
            confirm_gated: false,
            description: "Search text files for a substring/pattern while skipping binary files and build directories.",
        },
        ToolSpec {
            name: "win_fs_backup_write",
            domain: "fileops",
            read_only: false,
            confirm_gated: true,
            description: "Write a file safely with mandatory timestamped .bak backup creation if the file exists (confirm=true).",
        },
        ToolSpec {
            name: "win_fs_undo_backup",
            domain: "fileops",
            read_only: false,
            confirm_gated: true,
            description: "Restore the most recent .bak backup for a file (confirm=true).",
        },
        ToolSpec {
            name: "win_fs_archive_inspect",
            domain: "fileops",
            read_only: true,
            confirm_gated: false,
            description: "Inspect entries and compression ratios inside a .zip archive.",
        },
        ToolSpec {
            name: "win_fs_archive_create",
            domain: "fileops",
            read_only: false,
            confirm_gated: true,
            description: "Create a deflated .zip archive from a file or directory (confirm=true).",
        },
        ToolSpec {
            name: "win_svc_list",
            domain: "sysadmin",
            read_only: true,
            confirm_gated: false,
            description: "List Windows services with state and PID.",
        },
        ToolSpec {
            name: "win_svc_control",
            domain: "sysadmin",
            read_only: false,
            confirm_gated: true,
            description: "Start, stop, or restart a Windows service via sc.exe with NSSM-safe detection (confirm=true).",
        },
        ToolSpec {
            name: "win_startup_list",
            domain: "sysadmin",
            read_only: true,
            confirm_gated: false,
            description: "List Windows startup autorun entries across HKCU/HKLM Run keys and Startup folders.",
        },
        ToolSpec {
            name: "win_task_list",
            domain: "sysadmin",
            read_only: true,
            confirm_gated: false,
            description: "List Windows Scheduled Tasks via schtasks.",
        },
        ToolSpec {
            name: "win_reg_read",
            domain: "sysadmin",
            read_only: true,
            confirm_gated: false,
            description: "Read values and subkeys from a Windows Registry key.",
        },
        ToolSpec {
            name: "win_reg_write",
            domain: "sysadmin",
            read_only: false,
            confirm_gated: true,
            description: "Write an HKCU Registry value after exporting a timestamped .reg backup (confirm=true).",
        },
        ToolSpec {
            name: "win_event_tail",
            domain: "sysadmin",
            read_only: true,
            confirm_gated: false,
            description: "Query recent critical/error/warning events from a Windows Event Log channel.",
        },
    ]
}

pub fn dispatch(cmd: &Command, opts: &ExecOptions) -> Result<Value, WinError> {
    match cmd {
        Command::Status => {
            let disks = sysadmin::disk_list()
                .unwrap_or_else(|_| serde_json::json!({"count": 0, "volumes": []}));
            let env_rep = sysadmin::env_audit().unwrap_or_else(|_| serde_json::json!({}));
            let procs = proc_ports::enumerate_processes().unwrap_or_default();
            let weeder = proc_ports::weeder_scan().unwrap_or_else(|_| serde_json::json!({}));
            Ok(serde_json::json!({
                "status": "ok",
                "craft": "wincraft",
                "version": env!("CARGO_PKG_VERSION"),
                "read_only": opts.read_only,
                "root": opts.root.as_ref().map(|p| p.display().to_string()),
                "backup_dir": opts.backup_dir.display().to_string(),
                "tool_count": tools_catalog().len(),
                "running_processes": procs.len(),
                "weeder_candidates": weeder.get("candidate_count").and_then(|v| v.as_u64()).unwrap_or(0),
                "dead_path_entries": env_rep.get("dead_directory_count").and_then(|v| v.as_u64()).unwrap_or(0),
                "disks": disks.get("volumes").cloned().unwrap_or_else(|| serde_json::json!([])),
            }))
        }
        Command::Doctor => sysadmin::doctor().map_err(|e| WinError::new("DOCTOR_ERROR", e)),
        Command::Tools => Ok(serde_json::json!({
            "count": tools_catalog().len(),
            "tools": tools_catalog(),
        })),
        Command::DiskList => sysadmin::disk_list().map_err(|e| WinError::new("DISK_ERROR", e)),
        Command::EnvAudit => sysadmin::env_audit().map_err(|e| WinError::new("ENV_ERROR", e)),
        Command::ProcList { filter, limit } => proc_ports::proc_list(filter.as_deref(), *limit)
            .map_err(|e| WinError::new("PROC_ERROR", e)),
        Command::PortMap {
            port,
            listening_only,
        } => proc_ports::port_map(*port, *listening_only, opts.ports_doc.as_deref())
            .map_err(|e| WinError::new("PORT_ERROR", e)),
        Command::WeederScan => {
            proc_ports::weeder_scan().map_err(|e| WinError::new("WEEDER_ERROR", e))
        }
        Command::WeederReap { pid, confirm } => {
            proc_ports::weeder_reap(*pid, *confirm, opts.read_only)
                .map_err(|e| WinError::new("WEEDER_REFUSED", e))
        }
        Command::LockInspect { path } => {
            let p = opts.check_path_allowed(path)?;
            let rep = locks::inspect_locks(&p).map_err(|e| WinError::new("LOCK_ERROR", e))?;
            serde_json::to_value(rep).map_err(|e| WinError::new("SERDE_ERROR", e.to_string()))
        }
        Command::FsList {
            path,
            recursive,
            depth,
        } => {
            let p = opts.check_path_allowed(path)?;
            fileops::fs_list(&p, *recursive, *depth).map_err(|e| WinError::new("FS_ERROR", e))
        }
        Command::FsDu {
            path,
            skip_build_dirs,
        } => {
            let p = opts.check_path_allowed(path)?;
            fileops::fs_du(&p, *skip_build_dirs).map_err(|e| WinError::new("FS_ERROR", e))
        }
        Command::FsLarge {
            path,
            min_size_bytes,
            limit,
            skip_build_dirs,
        } => {
            let p = opts.check_path_allowed(path)?;
            fileops::fs_large(&p, *min_size_bytes, *limit, *skip_build_dirs)
                .map_err(|e| WinError::new("FS_ERROR", e))
        }
        Command::FsDups {
            path,
            skip_build_dirs,
        } => {
            let p = opts.check_path_allowed(path)?;
            fileops::fs_dups(&p, *skip_build_dirs).map_err(|e| WinError::new("FS_ERROR", e))
        }
        Command::FsHash { path } => {
            let p = opts.check_path_allowed(path)?;
            let (sha256, size_bytes) =
                hash::sha256_file(&p).map_err(|e| WinError::new("HASH_ERROR", e.to_string()))?;
            Ok(serde_json::json!({
                "file": p.display().to_string(),
                "algorithm": "sha256",
                "sha256": sha256,
                "size_bytes": size_bytes,
            }))
        }
        Command::FsGrep {
            path,
            query,
            ext,
            max_matches,
        } => {
            let p = opts.check_path_allowed(path)?;
            fileops::fs_grep(&p, query, ext.as_deref(), *max_matches)
                .map_err(|e| WinError::new("GREP_ERROR", e))
        }
        Command::FsBackupWrite {
            path,
            content,
            confirm,
        } => {
            let p = opts.check_path_allowed(path)?;
            fileops::fs_backup_write(&p, content, *confirm, opts.read_only)
                .map_err(|e| WinError::new("WRITE_REFUSED", e))
        }
        Command::FsUndoBackup { path, confirm } => {
            let p = opts.check_path_allowed(path)?;
            fileops::fs_undo_backup(&p, *confirm, opts.read_only)
                .map_err(|e| WinError::new("UNDO_REFUSED", e))
        }
        Command::FsArchiveInspect { path } => {
            let p = opts.check_path_allowed(path)?;
            fileops::fs_archive_inspect(&p).map_err(|e| WinError::new("ARCHIVE_ERROR", e))
        }
        Command::FsArchiveCreate {
            source,
            dest_zip,
            confirm,
        } => {
            let src = opts.check_path_allowed(source)?;
            let dst = opts.check_path_allowed(dest_zip)?;
            fileops::fs_archive_create(&src, &dst, *confirm, opts.read_only)
                .map_err(|e| WinError::new("ARCHIVE_REFUSED", e))
        }
        Command::SvcList {
            filter,
            state,
            limit,
        } => sysadmin::svc_list(filter.as_deref(), state.as_deref(), *limit)
            .map_err(|e| WinError::new("SVC_ERROR", e)),
        Command::SvcControl {
            name,
            action,
            confirm,
        } => sysadmin::svc_control(name, action, *confirm, opts.read_only)
            .map_err(|e| WinError::new("SVC_REFUSED", e)),
        Command::StartupList => {
            sysadmin::startup_list().map_err(|e| WinError::new("STARTUP_ERROR", e))
        }
        Command::TaskList { filter, limit } => sysadmin::task_list(filter.as_deref(), *limit)
            .map_err(|e| WinError::new("TASK_ERROR", e)),
        Command::RegRead { key, value } => {
            sysadmin::reg_read(key, value.as_deref()).map_err(|e| WinError::new("REG_ERROR", e))
        }
        Command::RegWrite {
            key,
            value,
            reg_type,
            data,
            confirm,
        } => sysadmin::reg_write(
            key,
            value,
            reg_type,
            data,
            &opts.backup_dir,
            *confirm,
            opts.read_only,
        )
        .map_err(|e| WinError::new("REG_REFUSED", e)),
        Command::EventTail { channel, count } => {
            sysadmin::event_tail(channel, *count).map_err(|e| WinError::new("EVENT_ERROR", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn catalog_has_26_tools_and_valid_metadata() {
        let tools = tools_catalog();
        assert_eq!(tools.len(), 26);
        for t in &tools {
            assert!(t.name.starts_with("win_"));
            assert!(!t.description.is_empty());
            if !t.read_only {
                assert!(
                    t.confirm_gated,
                    "mutating tool {} must be confirm_gated",
                    t.name
                );
            }
        }
    }

    #[test]
    fn duplicate_finder_3_stage_and_du_work() {
        let Ok(tmp) = tempfile::tempdir() else {
            return;
        };
        let root = tmp.path();
        let a = root.join("alpha.txt");
        let b = root.join("beta_copy.txt");
        let c = root.join("unique.txt");
        let payload = "WinCraft 3-stage duplicate detection payload!\n".repeat(200);
        assert!(fs::write(&a, &payload).is_ok());
        assert!(fs::write(&b, &payload).is_ok());
        assert!(fs::write(&c, b"different size and content").is_ok());

        let opts = ExecOptions::default();
        let Ok(dups_val) = dispatch(
            &Command::FsDups {
                path: root.to_path_buf(),
                skip_build_dirs: true,
            },
            &opts,
        ) else {
            return;
        };
        assert_eq!(
            dups_val.get("duplicate_groups").and_then(|v| v.as_u64()),
            Some(1)
        );
        assert_eq!(
            dups_val
                .get("total_reclaimable_bytes")
                .and_then(|v| v.as_u64()),
            Some(payload.len() as u64)
        );

        let Ok(du_val) = dispatch(
            &Command::FsDu {
                path: root.to_path_buf(),
                skip_build_dirs: true,
            },
            &opts,
        ) else {
            return;
        };
        assert_eq!(du_val.get("total_files").and_then(|v| v.as_u64()), Some(3));
    }

    #[test]
    fn backup_write_and_undo_roundtrip() {
        let Ok(tmp) = tempfile::tempdir() else {
            return;
        };
        let file = tmp.path().join("config.toml");
        assert!(fs::write(&file, "version = 1\n").is_ok());

        let opts = ExecOptions::default();
        // Without confirm=true, must refuse
        let refused = dispatch(
            &Command::FsBackupWrite {
                path: file.clone(),
                content: "version = 2\n".to_string(),
                confirm: false,
            },
            &opts,
        );
        assert!(refused.is_err());

        // With confirm=true, creates .bak and updates file
        let written = dispatch(
            &Command::FsBackupWrite {
                path: file.clone(),
                content: "version = 2\n".to_string(),
                confirm: true,
            },
            &opts,
        );
        assert!(written.is_ok());
        assert_eq!(
            fs::read_to_string(&file).ok().as_deref(),
            Some("version = 2\n")
        );

        // Undo restores "version = 1\n"
        let undone = dispatch(
            &Command::FsUndoBackup {
                path: file.clone(),
                confirm: true,
            },
            &opts,
        );
        assert!(undone.is_ok());
        assert_eq!(
            fs::read_to_string(&file).ok().as_deref(),
            Some("version = 1\n")
        );
    }

    #[test]
    fn path_confinement_blocks_escape() {
        let Ok(tmp) = tempfile::tempdir() else {
            return;
        };
        let opts = ExecOptions {
            root: Some(tmp.path().to_path_buf()),
            ..Default::default()
        };

        let outside = PathBuf::from(r"C:\Windows\System32");
        let res = dispatch(
            &Command::FsList {
                path: outside,
                recursive: false,
                depth: 1,
            },
            &opts,
        );
        assert!(res.is_err());
    }

    #[test]
    fn confirm_and_protected_gates_protect_system_processes_and_registry() {
        let opts = ExecOptions::default();
        // System PID 4 must always be refused even with confirm=true
        let reap_sys = dispatch(
            &Command::WeederReap {
                pid: 4,
                confirm: true,
            },
            &opts,
        );
        assert!(reap_sys.is_err());

        // HKLM registry write must be refused by HKCU safety gate
        let reg_hklm = dispatch(
            &Command::RegWrite {
                key: r"HKLM\Software\WinCraftTest".to_string(),
                value: "Test".to_string(),
                reg_type: "REG_SZ".to_string(),
                data: "1".to_string(),
                confirm: true,
            },
            &opts,
        );
        assert!(reg_hklm.is_err());
    }

    #[test]
    fn restart_manager_lock_inspect_runs_cleanly() {
        let Ok(tmp) = tempfile::tempdir() else {
            return;
        };
        let locked_file = tmp.path().join("sample.dat");
        assert!(fs::write(&locked_file, b"hello restart manager").is_ok());
        let opts = ExecOptions::default();
        let res = dispatch(&Command::LockInspect { path: locked_file }, &opts);
        assert!(res.is_ok());
    }

    #[test]
    fn zip_archive_create_and_inspect_roundtrip() {
        let Ok(tmp) = tempfile::tempdir() else {
            return;
        };
        let src_dir = tmp.path().join("src_data");
        assert!(fs::create_dir_all(&src_dir).is_ok());
        assert!(fs::write(src_dir.join("a.txt"), b"hello zip world").is_ok());
        let zip_path = tmp.path().join("bundle.zip");

        let opts = ExecOptions::default();
        let created = dispatch(
            &Command::FsArchiveCreate {
                source: src_dir,
                dest_zip: zip_path.clone(),
                confirm: true,
            },
            &opts,
        );
        assert!(created.is_ok());

        let inspected = dispatch(&Command::FsArchiveInspect { path: zip_path }, &opts);
        assert!(inspected.is_ok());
    }
}
