use super::csv_parser::parse_wiztree_csv;
use super::finder::find_wiztree;
use super::models::FileRecord;
use std::process::Command;

pub fn execute_wiztree_scan(
    drive: &str,
    filter: Option<&str>,
    export_files: bool,
    export_folders: bool,
    sort_by_size: bool,
    max_records: usize,
) -> Result<Vec<FileRecord>, String> {
    let wiztree_exe = find_wiztree()?;

    let mut drive_norm = drive.trim().to_string();
    if (drive_norm.len() == 2 && drive_norm.ends_with(':'))
        || (!drive_norm.ends_with('\\') && !drive_norm.contains('/'))
    {
        drive_norm.push('\\');
    }

    let random_id: u128 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_path =
        std::env::temp_dir().join(format!("wiztree_{}_{}.csv", std::process::id(), random_id));

    let mut cmd = Command::new(&wiztree_exe);
    cmd.arg(&drive_norm);

    if let Some(pattern) = filter {
        if !pattern.trim().is_empty() {
            cmd.arg(format!("/filter={pattern}"));
        }
    }

    cmd.arg(format!("/export={}", temp_path.display()));

    // If process is already elevated (Administrator), do not pass /admin=0,
    // allowing WizTree to use direct NTFS MFT access on all drives.
    // If non-elevated, pass /admin=0 to suppress interactive UAC prompts.
    if !super::drives::is_elevated() {
        cmd.arg("/admin=0");
    }

    let folders_flag = if export_folders {
        "/exportfolders=1"
    } else {
        "/exportfolders=0"
    };
    let files_flag = if export_files {
        "/exportfiles=1"
    } else {
        "/exportfiles=0"
    };
    cmd.arg(folders_flag);
    cmd.arg(files_flag);

    if sort_by_size {
        cmd.arg("/sortby=1");
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to execute WizTree: {e}"))?;

    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(60);

    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    if temp_path.exists() {
                        let _ = std::fs::remove_file(&temp_path);
                    }
                    return Err(format!(
                        "WizTree scan timed out on {drive_norm} after 60s. Direct NTFS MFT reading on this volume requires Administrator privileges. Run app as Administrator."
                    ));
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => return Err(format!("Error waiting for WizTree: {e}")),
        }
    };

    if !status.success() {
        return Err(format!("WizTree exited with non-zero status: {status}"));
    }

    if !temp_path.exists() {
        return Ok(Vec::new());
    }

    let records = parse_wiztree_csv(&temp_path, max_records)?;
    let _ = std::fs::remove_file(temp_path);

    Ok(records)
}

pub fn search_single_drive(
    drive: &str,
    pattern: &str,
    max_results: usize,
) -> Result<Vec<FileRecord>, String> {
    execute_wiztree_scan(
        drive,
        Some(pattern),
        true,  // export files
        false, // export folders
        false, // sort by size
        max_results,
    )
}

pub fn scan_drive_usage(drive: &str, max_records: usize) -> Result<Vec<FileRecord>, String> {
    execute_wiztree_scan(
        drive,
        None,
        true,  // export files
        true,  // export folders
        false, // sort by size (we sort in Rust in-memory, avoiding Delphi GUI sorting overhead)
        max_records,
    )
}
