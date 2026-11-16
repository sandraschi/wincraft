//! CPU Hogger & Compute Audit + Host Thermal Diagnostics:
//! - Identifies all processes consuming significant CPU time or compute resources.
//! - Categorizes processes into Precleared System, Precleared Fleet/Dev, Unauthorized Hogs,
//!   Suspicious Paths (running from %TEMP% / Downloads / masquerading), and Known Miner Signatures.
//! - Reports host processor architecture, load percentage, clock speeds, and thermal status.
//!
//! Subsumes the rogue miner detector and unauthorized background worker auditor
//! behind WinCraft's single deterministic `dispatch` bus.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditStatus {
    PreclearedSystem,
    PreclearedDev,
    UnauthorizedHog,
    SuspiciousPath,
    SuspiciousMiner,
    NormalBackground,
}

impl AuditStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PreclearedSystem => "precleared_system",
            Self::PreclearedDev => "precleared_dev",
            Self::UnauthorizedHog => "unauthorized_hog",
            Self::SuspiciousPath => "suspicious_path",
            Self::SuspiciousMiner => "suspicious_miner",
            Self::NormalBackground => "normal_background",
        }
    }

    pub fn badge(self) -> &'static str {
        match self {
            Self::PreclearedSystem => "🛡 SYSTEM",
            Self::PreclearedDev => "✅ AUTHORIZED DEV",
            Self::UnauthorizedHog => "🚨 UNAUTHORIZED HOG",
            Self::SuspiciousPath => "⚠️ SUSPICIOUS PATH",
            Self::SuspiciousMiner => "🛑 SUSPICIOUS MINER",
            Self::NormalBackground => "⚪ BACKGROUND",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessAuditRecord {
    pub pid: u32,
    pub name: String,
    pub path: String,
    pub cpu_seconds: f64,
    pub cpu_percent: f64,
    pub memory_mb: f64,
    pub status: AuditStatus,
    pub status_badge: String,
    pub reason: String,
    pub protected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalStatusReport {
    pub processor_name: String,
    pub load_percentage: u32,
    pub cores: u32,
    pub logical_processors: u32,
    pub max_clock_mhz: u32,
    pub current_clock_mhz: u32,
    pub throttle_detected: bool,
    pub status: String,
    pub high_load_count: usize,
}

const PRECLEARED_SYSTEM_NAMES: &[&str] = &[
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
    "taskmgr.exe",
    "shellexperiencehost.exe",
    "searchindexer.exe",
    "memory compression",
    "wmiprvse.exe",
    "secure system",
];

const PRECLEARED_DEV_AND_TOOLS: &[&str] = &[
    // Fleet & AI tools
    "antigravity.exe",
    "claude.exe",
    "code.exe",
    "nssm.exe",
    "wincraft.exe",
    "wincraft-mcp.exe",
    "wincraft-app.exe",
    "pwsh.exe",
    "powershell.exe",
    "cmd.exe",
    "windowsterminal.exe",
    // Compilers & Runtimes
    "cargo.exe",
    "rustc.exe",
    "rust-analyzer.exe",
    "language_server.exe",
    "git.exe",
    "node.exe",
    "python.exe",
    "python3.exe",
    "uv.exe",
    "just.exe",
    // Containers & Dev infra
    "docker.exe",
    "dockerd.exe",
    "com.docker.backend.exe",
    "podman.exe",
    "podman desktop.exe",
    "ollama.exe",
    "ollama_app.exe",
    "msedgewebview2.exe",
    "wsl.exe",
    "vmmem.exe",
    "vmmemwsl.exe",
    "wslhost.exe",
    "wslservice.exe",
    "wslrelay.exe",
    "vmware-tray.exe",
    "vmware-vmx.exe",
    // Productivity & Media
    "lm studio.exe",
    "discord.exe",
    "spotify.exe",
    "slack.exe",
    "notion.exe",
    "chrome.exe",
    "msedge.exe",
    "firefox.exe",
    "amplibraryagent.exe",
    "appledetectors.exe",
    "appledevices.exe",
    "appledevicesagent.exe",
    "applemusic.exe",
    "appletv.exe",
    "wizfile64.exe",
    "wizfile.exe",
    "wizmouse.exe",
    "wispr flow.exe",
    "wispr flow helper.exe",
    "nvidia broadcast.exe",
    "nvcontainer.exe",
];

const KNOWN_MINER_SIGNATURES: &[&str] = &[
    "xmrig",
    "minerd",
    "ethminer",
    "nbminer",
    "phoenixminer",
    "t-rex",
    "gminer",
    "ccminer",
    "cpuminer",
    "cryptonight",
    "stratum",
    "nanopool",
    "nicehash",
    "kawpow",
    "wildrig",
    "srbminer",
    "teamredminer",
    "lolminer",
];

fn normalize_name(raw: &str) -> String {
    let mut s = raw.trim().to_lowercase();
    // Pseudo-processes (no image file) keep their bare names
    let pseudo = [
        "system",
        "system idle process",
        "registry",
        "memory compression",
        "secure system",
        "idle",
    ];
    if !s.ends_with(".exe") && !pseudo.contains(&s.as_str()) {
        s.push_str(".exe");
    }
    s
}

/// Load custom authorized process names from optional user config file
fn load_custom_authorized(config_path: Option<&Path>) -> HashSet<String> {
    let mut set = HashSet::new();

    let candidates = if let Some(p) = config_path {
        vec![p.to_path_buf()]
    } else {
        let mut list = Vec::new();
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            list.push(
                Path::new(&local_app_data)
                    .join("wincraft")
                    .join("authorized_processes.json"),
            );
        }
        list.push(Path::new("authorized_processes.json").to_path_buf());
        list
    };

    for path in candidates {
        if path.is_file() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(parsed) = serde_json::from_str::<Value>(&content) {
                    if let Some(arr) = parsed.as_array() {
                        for item in arr {
                            if let Some(s) = item.as_str() {
                                set.insert(normalize_name(s));
                            }
                        }
                    } else if let Some(arr) = parsed.get("authorized").and_then(|v| v.as_array()) {
                        for item in arr {
                            if let Some(s) = item.as_str() {
                                set.insert(normalize_name(s));
                            }
                        }
                    }
                }
            }
        }
    }

    set
}

#[derive(Deserialize)]
struct RawProcMetric {
    pid: u32,
    name: String,
    path: String,
    cpu_sec: f64,
    cpu_pct: f64,
    mem_mb: f64,
}

fn fetch_process_metrics_via_powershell() -> Result<Vec<RawProcMetric>, String> {
    let script = r#"
$perfMap = @{}
try {
    Get-CimInstance Win32_PerfFormattedData_PerfProc_Process -ErrorAction SilentlyContinue | Where-Object { $_.Name -ne "_Total" -and $_.Name -ne "Idle" } | ForEach-Object {
        # IDProcess is UInt32 while Get-Process Id is Int32; normalize keys to [int]
        $perfMap[[int]$_.IDProcess] = $_.PercentProcessorTime
    }
} catch {}

$procs = Get-Process | ForEach-Object {
    $key = [int]$_.Id
    $pct = if ($perfMap.ContainsKey($key)) { $perfMap[$key] } else { 0 }
    $p = ""
    try { $p = $_.Path } catch {}
    [PSCustomObject]@{
        pid = $_.Id
        name = $_.ProcessName
        path = if ($p) { $p } else { "" }
        cpu_sec = [math]::Round($_.CPU, 2)
        cpu_pct = $pct
        mem_mb = [math]::Round($_.WorkingSet64 / 1MB, 1)
    }
}
$filtered = $procs | Where-Object { $_.cpu_sec -gt 0 -or $_.cpu_pct -gt 0 }
$filtered | ConvertTo-Json -Compress
"#;

    let out = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
        .map_err(|e| format!("Failed to spawn powershell for CPU metrics: {e}"))?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("PowerShell CPU metrics query failed: {err}"));
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    // PowerShell ConvertTo-Json emits a single object if 1 result, array if multiple
    if trimmed.starts_with('{') {
        let single: RawProcMetric = serde_json::from_str(trimmed)
            .map_err(|e| format!("Failed to parse single process metric: {e}"))?;
        Ok(vec![single])
    } else {
        let list: Vec<RawProcMetric> = serde_json::from_str(trimmed)
            .map_err(|e| format!("Failed to parse process metrics array: {e}"))?;
        Ok(list)
    }
}

pub fn classify_process(
    name: &str,
    path: &str,
    cpu_sec: f64,
    cpu_pct: f64,
    min_cpu_seconds: f64,
    min_cpu_percent: f64,
    custom_authorized: &HashSet<String>,
) -> (AuditStatus, String) {
    let lower_name = normalize_name(name);
    let lower_path = path.trim().to_lowercase();

    // 1. Miner signature check (name or path)
    for &sig in KNOWN_MINER_SIGNATURES {
        if lower_name.contains(sig) || lower_path.contains(sig) {
            return (
                AuditStatus::SuspiciousMiner,
                format!("Matches known crypto miner signature '{sig}'"),
            );
        }
    }

    // 2. Suspicious path / masquerading check
    if !lower_path.is_empty() {
        // Masquerading: system name outside Windows\System32, Windows\SysWOW64, Windows\SystemApps, or C:\Windows\explorer.exe
        let is_system_name = PRECLEARED_SYSTEM_NAMES
            .iter()
            .any(|&sys| lower_name == sys || lower_name == format!("{sys}.exe"));
        if is_system_name {
            let valid_system_path = lower_path.contains(r"\windows\system32\")
                || lower_path.contains(r"\windows\syswow64\")
                || lower_path.contains(r"\windows\systemapps\")
                || (lower_name == "explorer.exe" && lower_path.ends_with(r"\windows\explorer.exe"));
            if !valid_system_path {
                return (
                    AuditStatus::SuspiciousPath,
                    format!("Masquerading: system binary '{name}' executing from '{path}'"),
                );
            }
        }

        // Suspicious temporary / download directories
        if (lower_path.contains(r"\appdata\local\temp\")
            || lower_path.contains(r"\temp\")
            || lower_path.contains(r"\downloads\"))
            && (cpu_sec > 10.0 || cpu_pct >= min_cpu_percent)
        {
            return (
                AuditStatus::SuspiciousPath,
                format!("Compute task executing from volatile directory: '{path}'"),
            );
        }
    }

    // 3. Precleared system check
    if PRECLEARED_SYSTEM_NAMES.iter().any(|&sys| lower_name == sys) {
        return (
            AuditStatus::PreclearedSystem,
            "Protected Windows core system component".to_string(),
        );
    }

    // 4. Precleared dev / tools check
    if PRECLEARED_DEV_AND_TOOLS
        .iter()
        .any(|&tool| lower_name == tool)
        || custom_authorized.contains(&lower_name)
    {
        return (
            AuditStatus::PreclearedDev,
            "Precleared developer environment or desktop tool".to_string(),
        );
    }

    // 5. Unauthorized CPU hog check
    let is_hog = cpu_pct >= min_cpu_percent || cpu_sec >= min_cpu_seconds;
    if is_hog {
        let reason = if cpu_pct >= min_cpu_percent && cpu_sec >= min_cpu_seconds {
            format!(
                "High sustained load: {cpu_pct}% CPU instantaneous + {cpu_sec:.1}s cumulative (unauthorized)"
            )
        } else if cpu_pct >= min_cpu_percent {
            format!("High instantaneous compute: {cpu_pct}% CPU (unauthorized)")
        } else {
            format!("High cumulative compute time: {cpu_sec:.1}s (unauthorized)")
        };
        return (AuditStatus::UnauthorizedHog, reason);
    }

    (
        AuditStatus::NormalBackground,
        "Nominal background process".to_string(),
    )
}

/// Run full CPU Hogger & Compute Audit
pub fn cpu_audit(
    filter: Option<&str>,
    min_cpu_seconds: Option<f64>,
    min_cpu_percent: Option<f64>,
    custom_config: Option<&Path>,
) -> Result<Value, String> {
    let min_sec = min_cpu_seconds.unwrap_or(30.0);
    let min_pct = min_cpu_percent.unwrap_or(5.0);
    let custom_set = load_custom_authorized(custom_config);

    let raw_procs = fetch_process_metrics_via_powershell()?;
    let needle = filter.map(|s| s.to_lowercase());

    let mut audited = Vec::new();
    let mut total_unauthorized_hogs = 0usize;
    let mut total_suspicious_paths = 0usize;
    let mut total_miners = 0usize;
    let mut total_precleared = 0usize;

    for p in raw_procs {
        if let Some(ref n) = needle {
            let lower_name = p.name.to_lowercase();
            let lower_path = p.path.to_lowercase();
            if !lower_name.contains(n) && !lower_path.contains(n) && p.pid.to_string() != *n {
                continue;
            }
        }

        let (status, reason) = classify_process(
            &p.name,
            &p.path,
            p.cpu_sec,
            p.cpu_pct,
            min_sec,
            min_pct,
            &custom_set,
        );

        match status {
            AuditStatus::UnauthorizedHog => total_unauthorized_hogs += 1,
            AuditStatus::SuspiciousPath => total_suspicious_paths += 1,
            AuditStatus::SuspiciousMiner => total_miners += 1,
            AuditStatus::PreclearedSystem | AuditStatus::PreclearedDev => total_precleared += 1,
            AuditStatus::NormalBackground => {}
        }

        let protected = crate::proc_ports::is_protected_process(&p.name, p.pid);

        audited.push(ProcessAuditRecord {
            pid: p.pid,
            name: p.name,
            path: p.path,
            cpu_seconds: p.cpu_sec,
            cpu_percent: p.cpu_pct,
            memory_mb: p.mem_mb,
            status,
            status_badge: status.badge().to_string(),
            reason,
            protected,
        });
    }

    // Sort: Miners and Suspicious first, then Unauthorized Hogs, then by CPU percent desc, then CPU seconds desc
    audited.sort_by(|a, b| {
        let rank = |status: AuditStatus| match status {
            AuditStatus::SuspiciousMiner => 0,
            AuditStatus::SuspiciousPath => 1,
            AuditStatus::UnauthorizedHog => 2,
            AuditStatus::NormalBackground => 3,
            AuditStatus::PreclearedDev => 4,
            AuditStatus::PreclearedSystem => 5,
        };
        let r_a = rank(a.status);
        let r_b = rank(b.status);
        if r_a != r_b {
            r_a.cmp(&r_b)
        } else if (b.cpu_percent - a.cpu_percent).abs() > 0.1 {
            b.cpu_percent
                .partial_cmp(&a.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
        } else {
            b.cpu_seconds
                .partial_cmp(&a.cpu_seconds)
                .unwrap_or(std::cmp::Ordering::Equal)
        }
    });

    let total_scanned = audited.len();

    Ok(json!({
        "status": "ok",
        "craft": "wincraft",
        "total_scanned": total_scanned,
        "total_unauthorized_hogs": total_unauthorized_hogs,
        "total_suspicious_paths": total_suspicious_paths,
        "total_suspicious_miners": total_miners,
        "total_precleared": total_precleared,
        "min_cpu_seconds_threshold": min_sec,
        "min_cpu_percent_threshold": min_pct,
        "audit_entries": audited,
    }))
}

/// Query host processor metrics and thermal throttling state
pub fn thermal_status() -> Result<Value, String> {
    let script = r#"
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1 Name, LoadPercentage, NumberOfCores, NumberOfLogicalProcessors, MaxClockSpeed, CurrentClockSpeed
$cpu | ConvertTo-Json -Compress
"#;

    let out = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
        .map_err(|e| format!("Failed to spawn powershell for thermal status: {e}"))?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("PowerShell thermal status query failed: {err}"));
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("No processor metrics returned by Windows CIM".to_string());
    }

    let val: Value = serde_json::from_str(trimmed)
        .map_err(|e| format!("Failed to parse processor metrics JSON: {e}"))?;

    let name = val
        .get("Name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown Processor")
        .trim()
        .to_string();
    let load = val
        .get("LoadPercentage")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let cores = val
        .get("NumberOfCores")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as u32;
    let logical = val
        .get("NumberOfLogicalProcessors")
        .and_then(|v| v.as_u64())
        .unwrap_or(cores as u64) as u32;
    let max_clock = val
        .get("MaxClockSpeed")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let cur_clock = val
        .get("CurrentClockSpeed")
        .and_then(|v| v.as_u64())
        .unwrap_or(max_clock as u64) as u32;

    // Thermal throttling heuristic: Under high load (> 50%), if clock is throttled below 70% of max
    let throttle_detected = load > 50 && max_clock > 0 && cur_clock < (max_clock * 70 / 100);

    let status = if throttle_detected {
        "THROTTLED_HIGH_HEAT"
    } else if load >= 85 {
        "CRITICAL_HIGH_LOAD"
    } else if load >= 60 {
        "ELEVATED_LOAD"
    } else {
        "NOMINAL"
    };

    let report = ThermalStatusReport {
        processor_name: name,
        load_percentage: load,
        cores,
        logical_processors: logical,
        max_clock_mhz: max_clock,
        current_clock_mhz: cur_clock,
        throttle_detected,
        status: status.to_string(),
        high_load_count: 0,
    };

    Ok(json!({
        "status": "ok",
        "thermal_report": report,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_detects_known_miner_signatures() {
        let custom = HashSet::new();
        let (status, reason) = classify_process(
            "xmrig.exe",
            r"C:\opt\xmrig.exe",
            100.0,
            50.0,
            30.0,
            5.0,
            &custom,
        );
        assert_eq!(status, AuditStatus::SuspiciousMiner);
        assert!(reason.contains("xmrig"));
    }

    #[test]
    fn classification_detects_masquerading_system_binary() {
        let custom = HashSet::new();
        let (status, reason) = classify_process(
            "svchost.exe",
            r"C:\Users\User\AppData\Local\svchost.exe",
            10.0,
            2.0,
            30.0,
            5.0,
            &custom,
        );
        assert_eq!(status, AuditStatus::SuspiciousPath);
        assert!(reason.contains("Masquerading"));
    }

    #[test]
    fn classification_preclears_authorized_developer_tools() {
        let custom = HashSet::new();
        let (status, _) = classify_process(
            "antigravity.exe",
            r"C:\Users\sandr\AppData\Local\Programs\Antigravity\antigravity.exe",
            500.0,
            12.0,
            30.0,
            5.0,
            &custom,
        );
        assert_eq!(status, AuditStatus::PreclearedDev);
    }

    #[test]
    fn classification_flags_unauthorized_cpu_hogs() {
        let custom = HashSet::new();
        let (status, reason) = classify_process(
            "rogue_compute_engine.exe",
            r"C:\Program Files\Rogue\worker.exe",
            120.0,
            15.0,
            30.0,
            5.0,
            &custom,
        );
        assert_eq!(status, AuditStatus::UnauthorizedHog);
        assert!(reason.contains("unauthorized"));
    }
}
