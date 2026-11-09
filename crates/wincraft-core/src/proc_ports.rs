//! Process table, TCP/UDP port-to-PID mapper (with fleet `WEBAPP_PORTS.md` correlation),
//! and rule-based RAM Weeder (orphan test-runner / headless engine leak detector + confirm-gated reaper).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessEntry {
    pub name: String,
    pub pid: u32,
    pub session_name: String,
    pub memory_kb: u64,
    pub memory_mb: f64,
    pub protected: bool,
}

const PROTECTED_PROCESSES: &[&str] = &[
    "system",
    "system idle process",
    "registry",
    "smss.exe",
    "csrss.exe",
    "wininit.exe",
    "services.exe",
    "lsass.exe",
    "svchost.exe",
    "fontdrvhost.exe",
    "winlogon.exe",
    "dwm.exe",
    "explorer.exe",
    "sihost.exe",
    "taskhostw.exe",
    "searchhost.exe",
    "startmenuexperiencehost.exe",
    "ctfmon.exe",
    "audiodg.exe",
    "spoolsv.exe",
    "msmpeng.exe",
    "nissrv.exe",
    "securityhealthservice.exe",
    "nssm.exe",
    "pwsh.exe",
    "powershell.exe",
    "code.exe",
    "claude.exe",
    "antigravity.exe",
    "wincraft.exe",
    "wincraft-mcp.exe",
    "wincraft-app.exe",
];

pub fn is_protected_process(name: &str, pid: u32) -> bool {
    if pid <= 4 || pid == std::process::id() {
        return true;
    }
    let lower = name.trim().to_lowercase();
    PROTECTED_PROCESSES.iter().any(|&p| p == lower)
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for ch in line.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
        } else if ch == ',' && !in_quotes {
            fields.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(ch);
        }
    }
    fields.push(current.trim().to_string());
    fields
}

fn parse_mem_kb(raw: &str) -> u64 {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse::<u64>().unwrap_or(0)
}

pub fn enumerate_processes() -> Result<Vec<ProcessEntry>, String> {
    let out = Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .output()
        .map_err(|e| format!("Failed to execute tasklist: {e}"))?;

    let text = String::from_utf8_lossy(&out.stdout);
    let mut procs = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let cols = parse_csv_line(trimmed);
        if cols.len() >= 5 {
            let name = cols[0].clone();
            let pid = cols[1].parse::<u32>().unwrap_or(0);
            let session_name = cols[2].clone();
            let memory_kb = parse_mem_kb(&cols[4]);
            let memory_mb = (memory_kb as f64 / 1024.0 * 100.0).round() / 100.0;
            let protected = is_protected_process(&name, pid);
            procs.push(ProcessEntry {
                name,
                pid,
                session_name,
                memory_kb,
                memory_mb,
                protected,
            });
        }
    }
    procs.sort_by_key(|p| std::cmp::Reverse(p.memory_kb));
    Ok(procs)
}

pub fn proc_list(filter: Option<&str>, limit: usize) -> Result<serde_json::Value, String> {
    let all = enumerate_processes()?;
    let total_memory_kb: u64 = all.iter().map(|p| p.memory_kb).sum();
    let needle = filter.map(|s| s.to_lowercase());

    let mut filtered: Vec<ProcessEntry> = all
        .into_iter()
        .filter(|p| {
            if let Some(ref n) = needle {
                p.name.to_lowercase().contains(n) || p.pid.to_string() == *n
            } else {
                true
            }
        })
        .collect();

    let cap = limit.clamp(1, 500);
    filtered.truncate(cap);

    Ok(serde_json::json!({
        "count": filtered.len(),
        "total_process_memory_mb": (total_memory_kb as f64 / 1024.0 * 100.0).round() / 100.0,
        "processes": filtered,
    }))
}

pub fn parse_webapp_ports_doc(doc_path: &Path) -> HashMap<u16, String> {
    let mut map = HashMap::new();
    let Ok(content) = fs::read_to_string(doc_path) else {
        return map;
    };
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        let cols: Vec<&str> = trimmed.split('|').map(|s| s.trim()).collect();
        if cols.len() < 3 {
            continue;
        }
        let mut repo_label: Option<String> = None;
        for c in &cols {
            let clean = c.trim_matches('`').trim_matches('*').trim();
            if (clean.ends_with("-mcp") || clean.ends_with("craft") || clean.contains('-'))
                && !clean.chars().all(|ch| ch.is_ascii_digit() || ch == '-')
                && clean.len() >= 3
            {
                repo_label = Some(clean.to_string());
                break;
            }
        }
        if let Some(repo) = repo_label {
            for c in &cols {
                for token in c.split(|ch: char| !ch.is_ascii_digit()) {
                    if token.len() == 4 || token.len() == 5 {
                        if let Ok(port) = token.parse::<u16>() {
                            if (3000..=65000).contains(&port) {
                                map.entry(port).or_insert_with(|| repo.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    map
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortBinding {
    pub proto: String,
    pub local_addr: String,
    pub port: u16,
    pub state: String,
    pub pid: u32,
    pub process_name: String,
    pub fleet_owner: Option<String>,
}

pub fn port_map(
    port_filter: Option<u16>,
    listening_only: bool,
    ports_doc: Option<&Path>,
) -> Result<serde_json::Value, String> {
    let procs = enumerate_processes().unwrap_or_default();
    let pid_to_name: HashMap<u32, String> = procs.into_iter().map(|p| (p.pid, p.name)).collect();

    let fleet_ports = ports_doc.map(parse_webapp_ports_doc).unwrap_or_default();

    let out = Command::new("netstat")
        .args(["-ano"])
        .output()
        .map_err(|e| format!("Failed to run netstat -ano: {e}"))?;

    let text = String::from_utf8_lossy(&out.stdout);
    let mut bindings = Vec::new();

    for line in text.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() < 4 {
            continue;
        }
        let proto = tokens[0].to_uppercase();
        if proto != "TCP" && proto != "UDP" {
            continue;
        }
        let local = tokens[1];
        let Some(colon_idx) = local.rfind(':') else {
            continue;
        };
        let Ok(port) = local[colon_idx + 1..].parse::<u16>() else {
            continue;
        };
        if let Some(want_port) = port_filter {
            if port != want_port {
                continue;
            }
        }

        let (state, pid_str) = if proto == "TCP" && tokens.len() >= 5 {
            (tokens[3].to_string(), tokens[4])
        } else {
            ("STATELESS".to_string(), tokens[tokens.len() - 1])
        };

        if listening_only && proto == "TCP" && state != "LISTENING" {
            continue;
        }

        let pid = pid_str.parse::<u32>().unwrap_or(0);
        let process_name = pid_to_name
            .get(&pid)
            .cloned()
            .unwrap_or_else(|| "<unknown>".to_string());
        let fleet_owner = fleet_ports.get(&port).cloned();

        bindings.push(PortBinding {
            proto,
            local_addr: local.to_string(),
            port,
            state,
            pid,
            process_name,
            fleet_owner,
        });
    }

    bindings.sort_by_key(|b| (b.port, b.pid));
    bindings.dedup_by(|a, b| {
        a.proto == b.proto && a.port == b.port && a.pid == b.pid && a.state == b.state
    });

    Ok(serde_json::json!({
        "count": bindings.len(),
        "listening_only": listening_only,
        "fleet_ports_loaded": fleet_ports.len(),
        "bindings": bindings,
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeederCandidate {
    pub pid: u32,
    pub name: String,
    pub memory_mb: f64,
    pub category: String,
    pub reason: String,
}

pub fn classify_weeder_candidate(proc_entry: &ProcessEntry) -> Option<WeederCandidate> {
    if proc_entry.protected {
        return None;
    }
    let lower = proc_entry.name.to_lowercase();

    // Category 1: Orphaned Rust cargo test binaries (typically named `<crate>-<16hex>.exe`)
    let stem = lower.strip_suffix(".exe").unwrap_or(&lower);
    if let Some((prefix, hash_part)) = stem.rsplit_once('-') {
        if !prefix.is_empty()
            && hash_part.len() == 16
            && hash_part.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Some(WeederCandidate {
                pid: proc_entry.pid,
                name: proc_entry.name.clone(),
                memory_mb: proc_entry.memory_mb,
                category: "orphan_cargo_test_binary".to_string(),
                reason: "Matches cargo test executable pattern `<crate>-<16hex>.exe`".to_string(),
            });
        }
    }

    // Category 2: Headless game / 3D / browser automation engines holding large RAM
    if (lower.starts_with("godot")
        || lower.starts_with("blender")
        || lower.starts_with("chromedriver")
        || lower.starts_with("msedgedriver")
        || lower.starts_with("geckodriver"))
        && proc_entry.memory_mb >= 50.0
    {
        return Some(WeederCandidate {
            pid: proc_entry.pid,
            name: proc_entry.name.clone(),
            memory_mb: proc_entry.memory_mb,
            category: "headless_engine_worker".to_string(),
            reason: format!(
                "Engine/driver worker consuming {:.1} MB RAM",
                proc_entry.memory_mb
            ),
        });
    }

    // Category 3: Heavy runaway python/node workers (>1.5 GB RAM)
    if (lower == "python.exe" || lower == "node.exe" || lower == "bun.exe")
        && proc_entry.memory_mb >= 1536.0
    {
        return Some(WeederCandidate {
            pid: proc_entry.pid,
            name: proc_entry.name.clone(),
            memory_mb: proc_entry.memory_mb,
            category: "heavy_runtime_worker".to_string(),
            reason: format!(
                "Runtime worker exceeding 1.5 GB ({:.1} MB)",
                proc_entry.memory_mb
            ),
        });
    }

    None
}

pub fn weeder_scan() -> Result<serde_json::Value, String> {
    let procs = enumerate_processes()?;
    let mut candidates = Vec::new();
    let mut reclaimable_mb = 0.0f64;

    for p in &procs {
        if let Some(cand) = classify_weeder_candidate(p) {
            reclaimable_mb += cand.memory_mb;
            candidates.push(cand);
        }
    }

    Ok(serde_json::json!({
        "scanned_processes": procs.len(),
        "candidate_count": candidates.len(),
        "reclaimable_mb": (reclaimable_mb * 100.0).round() / 100.0,
        "candidates": candidates,
    }))
}

pub fn weeder_reap(pid: u32, confirm: bool, read_only: bool) -> Result<serde_json::Value, String> {
    if read_only {
        return Err("WinCraft is in read-only mode; weeder_reap refused".to_string());
    }
    if !confirm {
        return Err(format!(
            "Refusing to terminate PID {pid} without confirm=true"
        ));
    }
    if pid <= 4 || pid == std::process::id() {
        return Err(format!(
            "PID {pid} is a protected system/self process; refusing termination"
        ));
    }

    let procs = enumerate_processes()?;
    let target = procs
        .iter()
        .find(|p| p.pid == pid)
        .ok_or_else(|| format!("PID {pid} not found in running process table"))?;

    if target.protected || is_protected_process(&target.name, target.pid) {
        return Err(format!(
            "Process '{}' (PID {}) is in the protected system allowlist and cannot be reaped",
            target.name, target.pid
        ));
    }

    let out = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .output()
        .map_err(|e| format!("Failed to invoke taskkill: {e}"))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(format!("taskkill failed for PID {pid}: {stderr}"));
    }

    Ok(serde_json::json!({
        "success": true,
        "reaped_pid": pid,
        "process_name": target.name,
        "reclaimed_mb": target.memory_mb,
    }))
}
