//! T2 floor integration tests: public L1 API only, hermetic tempdirs,
//! no admin required, no network. Pushes workspace past 50 tests.
//!
//! Note: workspace lints deny `unwrap_used`/`expect_used` (including tests),
//! so setup uses `let Ok(x) = ... else { return; }` and value extraction
//! uses `assert!(res.is_ok())` + `unwrap_or_default`-style fallbacks that
//! cannot panic. A returned-early test mirrors the existing lib tests.

use std::fs;
use std::path::PathBuf;
use wincraft_core::dispatch;
use wincraft_core::export::{
    filter_sort_paginate, format_cards_txt, format_csv, format_list_txt, format_markdown,
};
use wincraft_core::{payload_to_tabular, tools_catalog, Command, ExecOptions, OutputFormat};

fn ok_value<E: std::fmt::Debug>(
    res: Result<serde_json::Value, E>,
    what: &str,
) -> serde_json::Value {
    assert!(res.is_ok(), "{what} failed: {:?}", res.as_ref().err());
    res.unwrap_or(serde_json::Value::Null)
}

// ---------- sysadmin (read-only) ----------

#[test]
fn disk_list_reports_volumes() {
    let v = ok_value(wincraft_core::sysadmin::disk_list(), "disk_list");
    assert!(v.get("count").and_then(|c| c.as_u64()).unwrap_or(0) >= 1);
}

#[test]
fn env_audit_reports_path_health() {
    let v = ok_value(wincraft_core::sysadmin::env_audit(), "env_audit");
    assert!(v.get("dead_directory_count").is_some());
}

#[test]
fn doctor_reports_tool_matrix() {
    let v = ok_value(wincraft_core::sysadmin::doctor(), "doctor");
    assert!(v.get("tools").is_some());
}

#[test]
fn startup_list_shape() {
    let v = ok_value(wincraft_core::sysadmin::startup_list(), "startup_list");
    assert!(v.get("entries").and_then(|e| e.as_array()).is_some());
}

#[test]
fn task_list_respects_limit() {
    let v = ok_value(wincraft_core::sysadmin::task_list(None, 5), "task_list");
    let n = v
        .get("tasks")
        .and_then(|t| t.as_array())
        .map(|a| a.len())
        .unwrap_or(99);
    assert!(n <= 5);
}

#[test]
fn event_tail_application_channel() {
    let v = ok_value(
        wincraft_core::sysadmin::event_tail("Application", 1),
        "event_tail",
    );
    assert!(v.get("events").is_some());
}

#[test]
fn reg_read_hkcu_environment() {
    let v = ok_value(
        wincraft_core::sysadmin::reg_read(r"HKCU\Environment", None),
        "reg_read",
    );
    assert!(v.get("values").is_some());
}

#[test]
fn reg_write_hklm_refused_by_safety_gate() {
    let opts = ExecOptions::default();
    let res = dispatch(
        &Command::RegWrite {
            key: r"HKLM\Software\WinCraftTest".to_string(),
            value: "V".to_string(),
            reg_type: "REG_SZ".to_string(),
            data: "x".to_string(),
            confirm: true,
        },
        &opts,
    );
    assert!(res.is_err());
}

// ---------- processes / ports / weeder ----------

#[test]
fn enumerate_processes_nonempty() {
    let res = wincraft_core::proc_ports::enumerate_processes();
    assert!(res.is_ok(), "enumerate_processes failed");
    assert!(!res.unwrap_or_default().is_empty());
}

#[test]
fn proc_list_limit_and_filter() {
    let v = ok_value(wincraft_core::proc_ports::proc_list(None, 3), "proc_list");
    let n = v
        .get("processes")
        .and_then(|p| p.as_array())
        .map(|a| a.len())
        .unwrap_or(99);
    assert!(n <= 3);
}

#[test]
fn weeder_scan_shape() {
    let v = ok_value(wincraft_core::proc_ports::weeder_scan(), "weeder_scan");
    assert!(v.get("candidate_count").is_some());
}

#[test]
fn weeder_reap_needs_confirm() {
    let res = wincraft_core::proc_ports::weeder_reap(999999, false, false);
    assert!(res.is_err());
}

#[test]
fn weeder_reap_refuses_protected_pid_4() {
    let res = wincraft_core::proc_ports::weeder_reap(4, true, false);
    assert!(res.is_err());
}

#[test]
fn protected_process_predicate() {
    assert!(wincraft_core::proc_ports::is_protected_process(
        "svchost.exe",
        123
    ));
    assert!(wincraft_core::proc_ports::is_protected_process(
        "csrss.exe",
        999991
    ));
    assert!(!wincraft_core::proc_ports::is_protected_process(
        "my-test-worker-xyz.exe",
        999991
    ));
}

// ---------- locks ----------

#[test]
fn lock_inspect_temp_file_ok() {
    let Ok(tmp) = tempfile::tempdir() else {
        return;
    };
    let f = tmp.path().join("lockme.txt");
    assert!(fs::write(&f, b"hello").is_ok());
    let res = wincraft_core::locks::inspect_locks(&f);
    assert!(res.is_ok(), "inspect_locks failed");
    if let Ok(rep) = res {
        assert_eq!(rep.locking_count, 0);
    }
}

#[test]
fn lock_inspect_missing_path_err() {
    let res = wincraft_core::locks::inspect_locks(&PathBuf::from(
        r"C:\definitely\not\here\wincraft_nope.txt",
    ));
    assert!(res.is_err());
}

// ---------- hash ----------

#[test]
fn sha256_empty_matches_fips_vector() {
    let Ok(tmp) = tempfile::tempdir() else {
        return;
    };
    let f = tmp.path().join("empty.bin");
    assert!(fs::write(&f, b"").is_ok());
    let res = wincraft_core::hash::sha256_file(&f);
    assert!(res.is_ok(), "sha256_file failed");
    let (digest, size) = res.unwrap_or_default();
    assert_eq!(size, 0);
    assert_eq!(
        digest,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn sha256_prefix_matches_full_for_small_file() {
    let Ok(tmp) = tempfile::tempdir() else {
        return;
    };
    let f = tmp.path().join("small.bin");
    assert!(fs::write(&f, b"wincraft-prefix-check").is_ok());
    let prefix = wincraft_core::hash::sha256_prefix_4k(&f);
    assert!(prefix.is_ok(), "prefix_4k failed");
    let full = wincraft_core::hash::sha256_file(&f);
    assert!(full.is_ok(), "sha256_file failed");
    assert_eq!(prefix.unwrap_or_default(), full.unwrap_or_default().0);
}

// ---------- fileops via dispatch (tempdir) ----------

fn temp_tree() -> Option<tempfile::TempDir> {
    let tmp = tempfile::tempdir().ok()?;
    if fs::write(tmp.path().join("a.txt"), b"hello wincraft").is_err() {
        return None;
    }
    if fs::write(tmp.path().join("b.log"), b"nothing here").is_err() {
        return None;
    }
    Some(tmp)
}

#[test]
fn fs_list_du_large_grep_dups_shapes() {
    let Some(tmp) = temp_tree() else {
        return;
    };
    let root = tmp.path().to_path_buf();
    let opts = ExecOptions::default();
    let list = ok_value(
        dispatch(
            &Command::FsList {
                path: root.clone(),
                recursive: false,
                depth: 1,
            },
            &opts,
        ),
        "FsList",
    );
    assert!(
        list.get("total_items")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            >= 2
    );
    let du = ok_value(
        dispatch(
            &Command::FsDu {
                path: root.clone(),
                skip_build_dirs: true,
            },
            &opts,
        ),
        "FsDu",
    );
    assert!(du.get("total_files").and_then(|v| v.as_u64()).unwrap_or(0) >= 2);
    let large = ok_value(
        dispatch(
            &Command::FsLarge {
                path: root.clone(),
                min_size_bytes: 1,
                limit: 10,
                skip_build_dirs: true,
            },
            &opts,
        ),
        "FsLarge",
    );
    assert!(large.get("count").and_then(|v| v.as_u64()).unwrap_or(0) >= 2);
    let grep = ok_value(
        dispatch(
            &Command::FsGrep {
                path: root.clone(),
                query: "wincraft".to_string(),
                ext: None,
                max_matches: 10,
            },
            &opts,
        ),
        "FsGrep",
    );
    assert!(
        grep.get("match_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            >= 1
    );
    let dups = ok_value(
        dispatch(
            &Command::FsDups {
                path: root,
                skip_build_dirs: true,
            },
            &opts,
        ),
        "FsDups",
    );
    assert!(dups.get("duplicate_groups").is_some());
}

#[test]
fn fs_backup_write_lf_then_undo_roundtrip() {
    let Ok(tmp) = tempfile::tempdir() else {
        return;
    };
    let file = tmp.path().join("cfg.txt");
    assert!(fs::write(&file, "v=1\n").is_ok());
    let opts = ExecOptions::default();
    let written = ok_value(
        dispatch(
            &Command::FsBackupWrite {
                path: file.clone(),
                content: "v=2\r\nline2\rline3\n".to_string(),
                confirm: true,
            },
            &opts,
        ),
        "FsBackupWrite",
    );
    assert!(written
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false));
    let raw = fs::read(&file).unwrap_or_default();
    assert!(!raw.contains(&b'\r'));
    let undone = ok_value(
        dispatch(
            &Command::FsUndoBackup {
                path: file.clone(),
                confirm: true,
            },
            &opts,
        ),
        "FsUndoBackup",
    );
    assert!(undone
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false));
    assert_eq!(fs::read_to_string(&file).unwrap_or_default(), "v=1\n");
}

#[test]
fn fs_backup_write_refuses_without_confirm_and_readonly() {
    let Ok(tmp) = tempfile::tempdir() else {
        return;
    };
    let file = tmp.path().join("x.txt");
    let opts = ExecOptions::default();
    assert!(dispatch(
        &Command::FsBackupWrite {
            path: file.clone(),
            content: "hi".into(),
            confirm: false
        },
        &opts,
    )
    .is_err());
    let ro = ExecOptions {
        read_only: true,
        ..Default::default()
    };
    assert!(dispatch(
        &Command::FsBackupWrite {
            path: file,
            content: "hi".into(),
            confirm: true
        },
        &ro,
    )
    .is_err());
}

#[test]
fn fs_archive_create_and_inspect_roundtrip() {
    let Ok(tmp) = tempfile::tempdir() else {
        return;
    };
    let src = tmp.path().join("data");
    assert!(fs::create_dir_all(&src).is_ok());
    assert!(fs::write(src.join("a.txt"), b"zip me").is_ok());
    let zip = tmp.path().join("out.zip");
    let opts = ExecOptions::default();
    let created = ok_value(
        dispatch(
            &Command::FsArchiveCreate {
                source: src,
                dest_zip: zip.clone(),
                confirm: true,
            },
            &opts,
        ),
        "FsArchiveCreate",
    );
    assert!(created
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false));
    let inspected = ok_value(
        dispatch(&Command::FsArchiveInspect { path: zip }, &opts),
        "FsArchiveInspect",
    );
    assert!(
        inspected
            .get("entry_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            >= 1
    );
}

// ---------- export engine ----------

#[test]
fn export_filter_sort_paginate_and_formats() {
    let payload = serde_json::json!({
        "processes": [
            {"pid": 2, "name": "beta.exe", "memory_mb": 9.0, "memory_kb": 1, "session_name": "C", "protected": false},
            {"pid": 1, "name": "alpha.exe", "memory_mb": 3.0, "memory_kb": 1, "session_name": "C", "protected": false}
        ]
    });
    let tab = payload_to_tabular(&payload);
    let (sorted, total, _) = filter_sort_paginate(tab, Some("alpha"), None, true, 1, 10);
    assert_eq!(total, 1);
    assert_eq!(
        sorted
            .rows
            .first()
            .and_then(|r| r.get(1))
            .map(String::as_str),
        Some("alpha.exe")
    );
    let tab2 = payload_to_tabular(&payload);
    let csv = format_csv(&tab2);
    assert!(csv.contains("PID"));
    let md = format_markdown(&tab2);
    assert!(md.contains('|'));
    let txt = format_list_txt(&tab2);
    assert!(txt.contains("alpha.exe"));
    let cards = format_cards_txt(&tab2);
    assert!(cards.contains("Card #1"));
}

#[test]
fn output_format_parse_and_extension_matrix() {
    assert_eq!(OutputFormat::parse_str("list"), OutputFormat::List);
    assert_eq!(OutputFormat::parse_str("JSON"), OutputFormat::Json);
    assert_eq!(OutputFormat::parse_str("csv"), OutputFormat::Csv);
    assert_eq!(OutputFormat::parse_str("md"), OutputFormat::Md);
    assert_eq!(OutputFormat::parse_str("nope"), OutputFormat::Cards);
    assert_eq!(OutputFormat::Json.extension(), "json");
}

// ---------- dispatch bus ----------

#[test]
fn tools_catalog_has_28_confirm_gated_mutations() {
    let tools = tools_catalog();
    assert_eq!(tools.len(), 28);
    for t in &tools {
        if !t.read_only {
            assert!(
                t.confirm_gated,
                "mutating tool {} must be confirm-gated",
                t.name
            );
        }
    }
}

#[test]
fn dispatch_status_and_tools_ok() {
    let opts = ExecOptions::default();
    assert!(dispatch(&Command::Status, &opts).is_ok());
    assert!(dispatch(&Command::Tools, &opts).is_ok());
}

#[test]
fn dispatch_path_confinement_blocks_escape() {
    let Ok(tmp) = tempfile::tempdir() else {
        return;
    };
    let opts = ExecOptions {
        root: Some(tmp.path().to_path_buf()),
        ..Default::default()
    };
    let res = dispatch(
        &Command::FsList {
            path: PathBuf::from(r"C:\Windows\System32"),
            recursive: false,
            depth: 1,
        },
        &opts,
    );
    assert!(res.is_err());
}

#[test]
fn dispatch_cpu_audit_and_thermal_ok() {
    let opts = ExecOptions::default();
    let audit = ok_value(
        dispatch(
            &Command::CpuAudit {
                filter: None,
                min_cpu_seconds: None,
                min_cpu_percent: None,
            },
            &opts,
        ),
        "CpuAudit",
    );
    assert!(audit.get("audit_entries").is_some());
    let therm = ok_value(dispatch(&Command::ThermalStatus, &opts), "ThermalStatus");
    assert!(therm.get("thermal_report").is_some());
}

#[test]
fn dispatch_svc_and_ports_shapes() {
    let opts = ExecOptions::default();
    let svc = ok_value(
        dispatch(
            &Command::SvcList {
                filter: None,
                state: None,
                limit: 5,
            },
            &opts,
        ),
        "SvcList",
    );
    assert!(svc.get("services").is_some());
    let ports = ok_value(
        dispatch(
            &Command::PortMap {
                port: None,
                listening_only: true,
            },
            &opts,
        ),
        "PortMap",
    );
    assert!(ports.get("bindings").is_some());
}
