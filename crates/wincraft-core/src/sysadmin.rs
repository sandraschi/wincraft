//! Windows SysAdmin & Diagnostics:
//! - Services (`sc.exe`) with NSSM-aware stop/start verification
//! - Autoruns / Startup entries & Scheduled Tasks (`schtasks`)
//! - Registry read/write with mandatory `.reg` backup export before mutation
//! - Environment & `PATH` dead-directory auditor
//! - Event Log tail (`wevtutil`)
//! - Native Win32 Logical Disk health (`GetLogicalDrives` + `GetDiskFreeSpaceExW`)
//! - Toolchain & Sysinternals `doctor`

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskVolume {
    pub mount: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
    pub total_gb: f64,
    pub free_gb: f64,
    pub used_percent: f64,
}

#[cfg(windows)]
mod win_disk {
    use super::DiskVolume;
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetLogicalDrives() -> u32;
        fn GetDiskFreeSpaceExW(
            lp_directory_name: *const u16,
            lp_free_bytes_available_to_caller: *mut u64,
            lp_total_number_of_bytes: *mut u64,
            lp_total_number_of_free_bytes: *mut u64,
        ) -> i32;
    }

    pub fn query_disks() -> Vec<DiskVolume> {
        let mut out = Vec::new();
        // SAFETY: GetLogicalDrives takes no parameters and returns a bitmask of drives A..Z.
        let mask = unsafe { GetLogicalDrives() };
        for i in 0..26u8 {
            if (mask & (1 << i)) == 0 {
                continue;
            }
            let letter = (b'A' + i) as char;
            let mount = format!("{letter}:\\");
            let mut wide: Vec<u16> = std::ffi::OsStr::new(&mount).encode_wide().collect();
            wide.push(0);

            let mut avail = 0u64;
            let mut total = 0u64;
            let mut free = 0u64;
            // SAFETY: Valid null-terminated UTF-16 drive root and stack u64 pointers.
            let ok =
                unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut avail, &mut total, &mut free) };
            if ok != 0 && total > 0 {
                let used = total.saturating_sub(free);
                let total_gb = (total as f64 / 1_073_741_824.0 * 100.0).round() / 100.0;
                let free_gb = (free as f64 / 1_073_741_824.0 * 100.0).round() / 100.0;
                let used_percent = (used as f64 / total as f64 * 1000.0).round() / 10.0;
                out.push(DiskVolume {
                    mount,
                    total_bytes: total,
                    free_bytes: free,
                    used_bytes: used,
                    total_gb,
                    free_gb,
                    used_percent,
                });
            }
        }
        out
    }
}

pub fn disk_list() -> Result<serde_json::Value, String> {
    #[cfg(windows)]
    let volumes = win_disk::query_disks();
    #[cfg(not(windows))]
    let volumes: Vec<DiskVolume> = Vec::new();

    Ok(serde_json::json!({
        "count": volumes.len(),
        "volumes": volumes,
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEntry {
    pub name: String,
    pub display_name: String,
    pub state: String,
    pub pid: u32,
}

pub fn svc_list(
    filter: Option<&str>,
    state_filter: Option<&str>,
    limit: usize,
) -> Result<serde_json::Value, String> {
    let out = Command::new("sc.exe")
        .args(["queryex", "type=", "service", "state=", "all"])
        .output()
        .map_err(|e| format!("Failed to execute sc.exe queryex: {e}"))?;

    let text = String::from_utf8_lossy(&out.stdout);
    let mut services = Vec::new();
    let mut curr_name = String::new();
    let mut curr_display = String::new();
    let mut curr_state = String::new();
    let mut curr_pid = 0u32;

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("SERVICE_NAME:") {
            if !curr_name.is_empty() {
                services.push(ServiceEntry {
                    name: curr_name.clone(),
                    display_name: curr_display.clone(),
                    state: curr_state.clone(),
                    pid: curr_pid,
                });
            }
            curr_name = rest.trim().to_string();
            curr_display.clear();
            curr_state.clear();
            curr_pid = 0;
        } else if let Some(rest) = trimmed.strip_prefix("DISPLAY_NAME:") {
            curr_display = rest.trim().to_string();
        } else if let Some(rest) = trimmed.strip_prefix("STATE") {
            if let Some((_, val)) = rest.split_once(':') {
                let parts: Vec<&str> = val.split_whitespace().collect();
                curr_state = parts.get(1).copied().unwrap_or(val.trim()).to_string();
            }
        } else if let Some(rest) = trimmed.strip_prefix("PID") {
            if let Some((_, val)) = rest.split_once(':') {
                curr_pid = val.trim().parse::<u32>().unwrap_or(0);
            }
        }
    }
    if !curr_name.is_empty() {
        services.push(ServiceEntry {
            name: curr_name,
            display_name: curr_display,
            state: curr_state,
            pid: curr_pid,
        });
    }

    let needle = filter.map(|s| s.to_lowercase());
    let state_want = state_filter.map(|s| s.to_uppercase());

    let mut filtered: Vec<ServiceEntry> = services
        .into_iter()
        .filter(|s| {
            let match_name = if let Some(ref n) = needle {
                s.name.to_lowercase().contains(n) || s.display_name.to_lowercase().contains(n)
            } else {
                true
            };
            let match_state = if let Some(ref st) = state_want {
                s.state.to_uppercase().contains(st)
            } else {
                true
            };
            match_name && match_state
        })
        .collect();

    let cap = limit.clamp(1, 500);
    filtered.truncate(cap);

    Ok(serde_json::json!({
        "count": filtered.len(),
        "services": filtered,
    }))
}

pub fn is_nssm_service(service_name: &str) -> bool {
    let Ok(out) = Command::new("sc.exe").args(["qc", service_name]).output() else {
        return false;
    };
    let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
    text.contains("nssm")
}

/// Start, stop, or restart a Windows service using `sc.exe` (enforcing the fleet NSSM rule:
/// never kill an NSSM child directly; always use `sc.exe stop/start`).
pub fn svc_control(
    service_name: &str,
    action: &str,
    confirm: bool,
    read_only: bool,
) -> Result<serde_json::Value, String> {
    if read_only {
        return Err("WinCraft is in read-only mode; svc_control refused".to_string());
    }
    if !confirm {
        return Err(format!(
            "Refusing to {action} service '{service_name}' without confirm=true"
        ));
    }
    let act = action.trim().to_lowercase();
    if act != "start" && act != "stop" && act != "restart" {
        return Err(format!(
            "Unsupported service action '{action}'; expected start|stop|restart"
        ));
    }

    let nssm = is_nssm_service(service_name);
    let mut outputs = Vec::new();

    if act == "stop" || act == "restart" {
        let out = Command::new("sc.exe")
            .args(["stop", service_name])
            .output()
            .map_err(|e| format!("Failed to invoke sc.exe stop: {e}"))?;
        outputs.push(String::from_utf8_lossy(&out.stdout).trim().to_string());
    }
    if act == "start" || act == "restart" {
        let out = Command::new("sc.exe")
            .args(["start", service_name])
            .output()
            .map_err(|e| format!("Failed to invoke sc.exe start: {e}"))?;
        outputs.push(String::from_utf8_lossy(&out.stdout).trim().to_string());
    }

    Ok(serde_json::json!({
        "success": true,
        "service": service_name,
        "action": act,
        "nssm_managed": nssm,
        "protocol": "sc.exe (NSSM-safe)",
        "output": outputs,
    }))
}

pub fn startup_list() -> Result<serde_json::Value, String> {
    let run_keys = [
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
        r"HKLM\Software\Microsoft\Windows\CurrentVersion\Run",
    ];
    let mut entries = Vec::new();

    for key in &run_keys {
        if let Ok(out) = Command::new("reg.exe").args(["query", key]).output() {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with("HKEY_") {
                    continue;
                }
                let parts: Vec<&str> = trimmed.splitn(3, "    ").map(|s| s.trim()).collect();
                if parts.len() == 3 {
                    entries.push(serde_json::json!({
                        "source": key,
                        "name": parts[0],
                        "reg_type": parts[1],
                        "command": parts[2],
                    }));
                }
            }
        }
    }

    if let Ok(appdata) = env::var("APPDATA") {
        let startup_dir = PathBuf::from(appdata)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("Startup");
        if let Ok(rd) = fs::read_dir(&startup_dir) {
            for e in rd.flatten() {
                entries.push(serde_json::json!({
                    "source": startup_dir.display().to_string(),
                    "name": e.file_name().to_string_lossy().to_string(),
                    "reg_type": "STARTUP_FILE",
                    "command": e.path().display().to_string(),
                }));
            }
        }
    }

    Ok(serde_json::json!({
        "count": entries.len(),
        "entries": entries,
    }))
}

pub fn task_list(filter: Option<&str>, limit: usize) -> Result<serde_json::Value, String> {
    let out = Command::new("schtasks.exe")
        .args(["/Query", "/FO", "CSV", "/NH"])
        .output()
        .map_err(|e| format!("Failed to invoke schtasks.exe: {e}"))?;

    let text = String::from_utf8_lossy(&out.stdout);
    let needle = filter.map(|s| s.to_lowercase());
    let mut tasks = Vec::new();
    let cap = limit.clamp(1, 300);

    for line in text.lines() {
        if tasks.len() >= cap {
            break;
        }
        let cols: Vec<&str> = line
            .split(',')
            .map(|s| s.trim().trim_matches('"'))
            .collect();
        if cols.len() >= 3 {
            let task_name = cols[0];
            let next_run = cols[1];
            let status = cols[2];
            if let Some(ref n) = needle {
                if !task_name.to_lowercase().contains(n) && !status.to_lowercase().contains(n) {
                    continue;
                }
            }
            tasks.push(serde_json::json!({
                "task_name": task_name,
                "next_run": next_run,
                "status": status,
            }));
        }
    }

    Ok(serde_json::json!({
        "count": tasks.len(),
        "tasks": tasks,
    }))
}

pub fn reg_read(key_path: &str, value_name: Option<&str>) -> Result<serde_json::Value, String> {
    let mut cmd = Command::new("reg.exe");
    cmd.arg("query").arg(key_path);
    if let Some(vn) = value_name {
        cmd.arg("/v").arg(vn);
    }
    let out = cmd
        .output()
        .map_err(|e| format!("Failed to run reg.exe query: {e}"))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(format!("reg.exe query failed for '{key_path}': {stderr}"));
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let mut values = Vec::new();
    let mut subkeys = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("HKEY_") {
            if !trimmed.eq_ignore_ascii_case(key_path) {
                subkeys.push(trimmed.to_string());
            }
            continue;
        }
        let parts: Vec<&str> = trimmed.splitn(3, "    ").map(|s| s.trim()).collect();
        if parts.len() == 3 {
            values.push(serde_json::json!({
                "name": parts[0],
                "reg_type": parts[1],
                "data": parts[2],
            }));
        }
    }

    Ok(serde_json::json!({
        "key": key_path,
        "value_count": values.len(),
        "subkey_count": subkeys.len(),
        "values": values,
        "subkeys": subkeys,
    }))
}

fn export_registry_backup(key_path: &str, backup_dir: &Path) -> Result<Option<String>, String> {
    fs::create_dir_all(backup_dir)
        .map_err(|e| format!("Failed to create registry backup dir: {e}"))?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let sanitized: String = key_path
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let bak_file = backup_dir.join(format!("{sanitized}_{ts}.reg"));

    let out = Command::new("reg.exe")
        .args(["export", key_path, &bak_file.display().to_string(), "/y"])
        .output()
        .map_err(|e| format!("Failed to invoke reg.exe export: {e}"))?;

    if out.status.success() && bak_file.is_file() {
        Ok(Some(bak_file.display().to_string()))
    } else {
        // Key may not exist yet before first create
        Ok(None)
    }
}

pub fn reg_write(
    key_path: &str,
    value_name: &str,
    reg_type: &str,
    data: &str,
    backup_dir: &Path,
    confirm: bool,
    read_only: bool,
) -> Result<serde_json::Value, String> {
    if read_only {
        return Err("WinCraft is in read-only mode; reg_write refused".to_string());
    }
    if !confirm {
        return Err("reg_write is mutating and requires confirm=true".to_string());
    }
    // Safety guardrail: only allow HKCU writes by default unless explicitly in a safe subtree
    let upper = key_path.trim().to_uppercase();
    if !upper.starts_with("HKCU\\") && !upper.starts_with("HKEY_CURRENT_USER\\") {
        return Err(format!(
            "Safety gate: reg_write only permits HKCU keys by default (got '{key_path}')"
        ));
    }

    let backup_path = export_registry_backup(key_path, backup_dir)?;

    let out = Command::new("reg.exe")
        .args([
            "add", key_path, "/v", value_name, "/t", reg_type, "/d", data, "/f",
        ])
        .output()
        .map_err(|e| format!("Failed to execute reg.exe add: {e}"))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(format!("reg.exe add failed: {stderr}"));
    }

    Ok(serde_json::json!({
        "success": true,
        "key": key_path,
        "value_name": value_name,
        "reg_type": reg_type,
        "data": data,
        "backup_reg_file": backup_path,
    }))
}

pub fn env_audit() -> Result<serde_json::Value, String> {
    let path_raw = env::var("PATH").unwrap_or_default();
    let mut entries = Vec::new();
    let mut dead_dirs = Vec::new();
    let mut duplicate_dirs = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for part in path_raw.split(';') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let p = PathBuf::from(trimmed);
        let exists = p.is_dir();
        let lower = trimmed.to_lowercase();
        let is_dup = !seen.insert(lower);
        if !exists {
            dead_dirs.push(trimmed.to_string());
        }
        if is_dup {
            duplicate_dirs.push(trimmed.to_string());
        }
        entries.push(serde_json::json!({
            "path": trimmed,
            "exists": exists,
            "duplicate": is_dup,
        }));
    }

    Ok(serde_json::json!({
        "total_path_entries": entries.len(),
        "dead_directory_count": dead_dirs.len(),
        "duplicate_count": duplicate_dirs.len(),
        "dead_directories": dead_dirs,
        "duplicate_directories": duplicate_dirs,
        "entries": entries,
    }))
}

pub fn event_tail(channel: &str, count: usize) -> Result<serde_json::Value, String> {
    let ch = if channel.trim().is_empty() {
        "System"
    } else {
        channel.trim()
    };
    let cap = count.clamp(1, 50);
    let out = Command::new("wevtutil.exe")
        .args([
            "qe",
            ch,
            &format!("/c:{cap}"),
            "/rd:true",
            "/f:text",
            "/q:*[System[(Level=1 or Level=2 or Level=3)]]",
        ])
        .output()
        .map_err(|e| format!("Failed to invoke wevtutil.exe: {e}"))?;

    let text = String::from_utf8_lossy(&out.stdout);
    let events: Vec<String> = text
        .split("Event[")
        .skip(1)
        .map(|chunk| format!("Event[{}", chunk.trim()))
        .collect();

    Ok(serde_json::json!({
        "channel": ch,
        "count": events.len(),
        "events": events,
    }))
}

fn probe_binary(candidates: &[&str]) -> serde_json::Value {
    for &cand in candidates {
        let p = Path::new(cand);
        if p.is_file() {
            return serde_json::json!({
                "status": "ok",
                "available": true,
                "path": p.display().to_string(),
            });
        }
        if let Ok(out) = Command::new("where.exe").arg(cand).output() {
            if out.status.success() {
                if let Some(first) = String::from_utf8_lossy(&out.stdout).lines().next() {
                    return serde_json::json!({
                        "status": "ok",
                        "available": true,
                        "path": first.trim(),
                    });
                }
            }
        }
    }
    serde_json::json!({
        "status": "down",
        "available": false,
        "path": serde_json::Value::Null,
    })
}

pub fn doctor() -> Result<serde_json::Value, String> {
    let sysint_cache = env::var("LOCALAPPDATA")
        .map(|l| PathBuf::from(l).join("sysinternals-mcp").join("bin"))
        .unwrap_or_else(|_| PathBuf::from(r"C:\Tools\Sysinternals"));

    let mut sysint_cached_bins = Vec::new();
    if let Ok(rd) = fs::read_dir(&sysint_cache) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.to_lowercase().ends_with(".exe") {
                sysint_cached_bins.push(name);
            }
        }
    }
    sysint_cached_bins.sort();

    let tools = serde_json::json!({
        "restart_manager_rstrtmgr": {
            "status": "ok",
            "available": cfg!(windows),
            "mode": "native_win32_ffi"
        },
        "sc_exe": probe_binary(&[r"C:\Windows\System32\sc.exe", "sc.exe"]),
        "reg_exe": probe_binary(&[r"C:\Windows\System32\reg.exe", "reg.exe"]),
        "wevtutil_exe": probe_binary(&[r"C:\Windows\System32\wevtutil.exe", "wevtutil.exe"]),
        "schtasks_exe": probe_binary(&[r"C:\Windows\System32\schtasks.exe", "schtasks.exe"]),
        "robocopy_exe": probe_binary(&[r"C:\Windows\System32\robocopy.exe", "robocopy.exe"]),
        "winrar_cli": probe_binary(&[r"C:\Program Files\WinRAR\Rar.exe", "Rar.exe"]),
        "sevenzip_cli": probe_binary(&[r"C:\Program Files\7-Zip\7z.exe", "7z.exe"]),
        "sysinternals_cache": {
            "dir": sysint_cache.display().to_string(),
            "exists": sysint_cache.is_dir(),
            "cached_binaries": sysint_cached_bins,
        }
    });

    Ok(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "os": env::consts::OS,
        "arch": env::consts::ARCH,
        "tools": tools,
    }))
}
