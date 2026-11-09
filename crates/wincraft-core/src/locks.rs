//! Native Windows File-Lock Inspector via Restart Manager (`rstrtmgr.dll`) + directory scan.
//! Answers "Who locks this file or directory?" without requiring third-party kernel drivers.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockingProcess {
    pub pid: u32,
    pub app_name: String,
    pub service_short_name: String,
    pub app_type: String,
    pub locked_resource: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockReport {
    pub target: String,
    pub is_dir: bool,
    pub scanned_resources: usize,
    pub locking_count: usize,
    pub engine: String,
    pub processes: Vec<LockingProcess>,
}

#[cfg(windows)]
mod win_rm {
    use super::LockingProcess;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    const CCH_RM_MAX_APP_NAME: usize = 255;
    const CCH_RM_MAX_SVC_NAME: usize = 63;
    const ERROR_SUCCESS: u32 = 0;
    const ERROR_MORE_DATA: u32 = 234;

    #[repr(C)]
    #[derive(Copy, Clone, Default)]
    struct FileTime {
        dw_low_date_time: u32,
        dw_high_date_time: u32,
    }

    #[repr(C)]
    #[derive(Copy, Clone, Default)]
    struct RmUniqueProcess {
        dw_process_id: u32,
        process_start_time: FileTime,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct RmProcessInfo {
        process: RmUniqueProcess,
        str_app_name: [u16; CCH_RM_MAX_APP_NAME + 1],
        str_service_short_name: [u16; CCH_RM_MAX_SVC_NAME + 1],
        application_type: u32,
        app_status: u32,
        ts_session_id: u32,
        b_restartable: i32,
    }

    impl Default for RmProcessInfo {
        fn default() -> Self {
            Self {
                process: RmUniqueProcess::default(),
                str_app_name: [0u16; CCH_RM_MAX_APP_NAME + 1],
                str_service_short_name: [0u16; CCH_RM_MAX_SVC_NAME + 1],
                application_type: 0,
                app_status: 0,
                ts_session_id: 0,
                b_restartable: 0,
            }
        }
    }

    #[link(name = "rstrtmgr")]
    extern "system" {
        fn RmStartSession(
            p_session_handle: *mut u32,
            dw_session_flags: u32,
            str_session_key: *mut u16,
        ) -> u32;
        fn RmRegisterResources(
            dw_session_handle: u32,
            n_files: u32,
            rgs_file_names: *const *const u16,
            n_applications: u32,
            rg_applications: *const RmUniqueProcess,
            n_services: u32,
            rgs_service_names: *const *const u16,
        ) -> u32;
        fn RmGetList(
            dw_session_handle: u32,
            pn_proc_info_needed: *mut u32,
            pn_proc_info: *mut u32,
            rg_affected_apps: *mut RmProcessInfo,
            lpdw_reboot_reasons: *mut u32,
        ) -> u32;
        fn RmEndSession(dw_session_handle: u32) -> u32;
    }

    fn utf16_to_string(buf: &[u16]) -> String {
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..len])
    }

    fn app_type_label(code: u32) -> &'static str {
        match code {
            1 => "MainWindow",
            2 => "OtherWindow",
            3 => "Service",
            4 => "Explorer",
            5 => "Console",
            1000 => "Critical",
            _ => "Unknown",
        }
    }

    pub fn query_restart_manager(
        paths: &[std::path::PathBuf],
    ) -> Result<Vec<LockingProcess>, String> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }

        let wide_paths: Vec<(String, Vec<u16>)> = paths
            .iter()
            .map(|p| {
                let s = p.display().to_string();
                let mut w: Vec<u16> = p.as_os_str().encode_wide().collect();
                w.push(0);
                (s, w)
            })
            .collect();

        let mut results = Vec::new();
        // Query in chunks of up to 32 files per Restart Manager session
        for chunk in wide_paths.chunks(32) {
            let mut session: u32 = 0;
            let mut session_key = [0u16; 64];
            // SAFETY: Standard Win32 Restart Manager API call with valid stack buffers.
            let rc = unsafe { RmStartSession(&mut session, 0, session_key.as_mut_ptr()) };
            if rc != ERROR_SUCCESS {
                return Err(format!("RmStartSession failed with Win32 code {rc}"));
            }

            let ptrs: Vec<*const u16> = chunk.iter().map(|(_, w)| w.as_ptr()).collect();
            let reg_rc = unsafe {
                RmRegisterResources(
                    session,
                    ptrs.len() as u32,
                    ptrs.as_ptr(),
                    0,
                    ptr::null(),
                    0,
                    ptr::null(),
                )
            };

            if reg_rc == ERROR_SUCCESS {
                let mut needed: u32 = 0;
                let mut count: u32 = 0;
                let mut reasons: u32 = 0;
                let list_rc = unsafe {
                    RmGetList(
                        session,
                        &mut needed,
                        &mut count,
                        ptr::null_mut(),
                        &mut reasons,
                    )
                };
                if list_rc == ERROR_MORE_DATA && needed > 0 {
                    let mut infos = vec![RmProcessInfo::default(); needed as usize];
                    count = needed;
                    let list_rc2 = unsafe {
                        RmGetList(
                            session,
                            &mut needed,
                            &mut count,
                            infos.as_mut_ptr(),
                            &mut reasons,
                        )
                    };
                    if list_rc2 == ERROR_SUCCESS {
                        let resource_label = if chunk.len() == 1 {
                            chunk[0].0.clone()
                        } else {
                            format!("{} (+{} files)", chunk[0].0, chunk.len() - 1)
                        };
                        for info in infos.iter().take(count as usize) {
                            results.push(LockingProcess {
                                pid: info.process.dw_process_id,
                                app_name: utf16_to_string(&info.str_app_name),
                                service_short_name: utf16_to_string(&info.str_service_short_name),
                                app_type: app_type_label(info.application_type).to_string(),
                                locked_resource: resource_label.clone(),
                            });
                        }
                    }
                }
            }

            unsafe {
                let _ = RmEndSession(session);
            }
        }

        // Deduplicate by PID
        results.sort_by_key(|p| p.pid);
        results.dedup_by_key(|p| p.pid);
        Ok(results)
    }
}

fn collect_candidate_files(target: &Path, max_files: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if target.is_file() {
        out.push(target.to_path_buf());
        return out;
    }
    let mut stack = vec![target.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if out.len() >= max_files {
            break;
        }
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if out.len() >= max_files {
                break;
            }
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.is_file() {
                out.push(p);
            }
        }
    }
    out
}

pub fn inspect_locks(target: &Path) -> Result<LockReport, String> {
    if !target.exists() {
        return Err(format!("Path does not exist: {}", target.display()));
    }
    let canonical = target
        .canonicalize()
        .unwrap_or_else(|_| target.to_path_buf());
    let is_dir = canonical.is_dir();
    let files = collect_candidate_files(&canonical, 128);
    let scanned_resources = files.len();

    #[cfg(windows)]
    let processes = win_rm::query_restart_manager(&files)?;

    #[cfg(not(windows))]
    let processes = Vec::new();

    Ok(LockReport {
        target: canonical.display().to_string(),
        is_dir,
        scanned_resources,
        locking_count: processes.len(),
        engine: "windows_restart_manager_rstrtmgr".to_string(),
        processes,
    })
}
