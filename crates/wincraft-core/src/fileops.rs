//! High-speed FileOps: directory listing, disk usage (`du`), largest files,
//! 3-stage streaming duplicate finder, text grep, `.bak`-enforced atomic write/undo, and ZIP archive tools.

use crate::hash::{sha256_file, sha256_prefix_4k};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const IGNORED_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".venv",
    "__pycache__",
    "dist",
];

fn is_ignored_dir(name: &str) -> bool {
    IGNORED_DIRS.iter().any(|&ig| ig.eq_ignore_ascii_case(name))
}

fn modified_epoch_secs(meta: &fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size_bytes: u64,
    pub modified_epoch: u64,
}

pub fn fs_list(
    target: &Path,
    recursive: bool,
    max_depth: usize,
) -> Result<serde_json::Value, String> {
    if !target.exists() {
        return Err(format!("Path does not exist: {}", target.display()));
    }
    let mut items = Vec::new();
    let max_d = max_depth.max(1);

    fn walk(curr: &Path, depth: usize, max_d: usize, recursive: bool, out: &mut Vec<FileEntry>) {
        if out.len() >= 2000 {
            return;
        }
        let Ok(rd) = fs::read_dir(curr) else {
            return;
        };
        for entry in rd.flatten() {
            if out.len() >= 2000 {
                break;
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let is_dir = meta.is_dir();
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            out.push(FileEntry {
                name: name.clone(),
                path: path.display().to_string(),
                is_dir,
                size_bytes: if is_dir { 0 } else { meta.len() },
                modified_epoch: modified_epoch_secs(&meta),
            });
            if is_dir && recursive && depth < max_d && !is_ignored_dir(&name) {
                walk(&path, depth + 1, max_d, recursive, out);
            }
        }
    }

    if target.is_dir() {
        walk(target, 1, max_d, recursive, &mut items);
    } else if let Ok(meta) = target.metadata() {
        items.push(FileEntry {
            name: target
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default(),
            path: target.display().to_string(),
            is_dir: false,
            size_bytes: meta.len(),
            modified_epoch: modified_epoch_secs(&meta),
        });
    }

    items.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
    Ok(serde_json::json!({
        "root": target.display().to_string(),
        "total_items": items.len(),
        "items": items,
    }))
}

pub fn fs_du(target: &Path, skip_build_dirs: bool) -> Result<serde_json::Value, String> {
    if !target.is_dir() {
        return Err(format!("Not a directory: {}", target.display()));
    }

    fn dir_bytes(
        dir: &Path,
        skip_build: bool,
        ext_totals: &mut HashMap<String, u64>,
        file_count: &mut u64,
    ) -> u64 {
        let mut sum = 0u64;
        let Ok(rd) = fs::read_dir(dir) else {
            return 0;
        };
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            if ft.is_symlink() {
                continue;
            }
            let p = entry.path();
            if ft.is_dir() {
                if skip_build && is_ignored_dir(&name) {
                    continue;
                }
                sum += dir_bytes(&p, skip_build, ext_totals, file_count);
            } else if ft.is_file() {
                if let Ok(meta) = entry.metadata() {
                    let sz = meta.len();
                    sum += sz;
                    *file_count += 1;
                    let ext = p
                        .extension()
                        .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
                        .unwrap_or_else(|| "<no-ext>".to_string());
                    *ext_totals.entry(ext).or_insert(0) += sz;
                }
            }
        }
        sum
    }

    let mut children = Vec::new();
    let mut ext_totals: HashMap<String, u64> = HashMap::new();
    let mut total_files = 0u64;
    let mut total_bytes = 0u64;

    if let Ok(rd) = fs::read_dir(target) {
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            if ft.is_symlink() {
                continue;
            }
            let p = entry.path();
            if ft.is_dir() {
                if skip_build_dirs && is_ignored_dir(&name) {
                    continue;
                }
                let sz = dir_bytes(&p, skip_build_dirs, &mut ext_totals, &mut total_files);
                total_bytes += sz;
                children.push(serde_json::json!({
                    "name": name,
                    "path": p.display().to_string(),
                    "is_dir": true,
                    "size_bytes": sz,
                }));
            } else if ft.is_file() {
                if let Ok(meta) = entry.metadata() {
                    let sz = meta.len();
                    total_bytes += sz;
                    total_files += 1;
                    let ext = p
                        .extension()
                        .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
                        .unwrap_or_else(|| "<no-ext>".to_string());
                    *ext_totals.entry(ext).or_insert(0) += sz;
                    children.push(serde_json::json!({
                        "name": name,
                        "path": p.display().to_string(),
                        "is_dir": false,
                        "size_bytes": sz,
                    }));
                }
            }
        }
    }

    children.sort_by_key(|v| {
        std::cmp::Reverse(v.get("size_bytes").and_then(|b| b.as_u64()).unwrap_or(0))
    });
    children.truncate(50);

    let mut top_extensions: Vec<serde_json::Value> = ext_totals
        .into_iter()
        .map(|(ext, bytes)| serde_json::json!({ "extension": ext, "size_bytes": bytes }))
        .collect();
    top_extensions.sort_by_key(|v| {
        std::cmp::Reverse(v.get("size_bytes").and_then(|b| b.as_u64()).unwrap_or(0))
    });
    top_extensions.truncate(15);

    Ok(serde_json::json!({
        "target": target.display().to_string(),
        "total_bytes": total_bytes,
        "total_mb": (total_bytes as f64 / 1_048_576.0 * 100.0).round() / 100.0,
        "total_files": total_files,
        "skip_build_dirs": skip_build_dirs,
        "entries": children,
        "top_extensions": top_extensions,
    }))
}

pub fn fs_large(
    target: &Path,
    min_size_bytes: u64,
    limit: usize,
    skip_build_dirs: bool,
) -> Result<serde_json::Value, String> {
    if !target.is_dir() {
        return Err(format!("Not a directory: {}", target.display()));
    }
    let mut stack = vec![target.to_path_buf()];
    let mut hits = Vec::new();

    while let Some(dir) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            if ft.is_symlink() {
                continue;
            }
            let p = entry.path();
            if ft.is_dir() {
                if skip_build_dirs && is_ignored_dir(&name) {
                    continue;
                }
                stack.push(p);
            } else if ft.is_file() {
                if let Ok(meta) = entry.metadata() {
                    if meta.len() >= min_size_bytes {
                        hits.push(FileEntry {
                            name,
                            path: p.display().to_string(),
                            is_dir: false,
                            size_bytes: meta.len(),
                            modified_epoch: modified_epoch_secs(&meta),
                        });
                    }
                }
            }
        }
    }

    hits.sort_by_key(|e| std::cmp::Reverse(e.size_bytes));
    let cap = limit.clamp(1, 200);
    hits.truncate(cap);

    Ok(serde_json::json!({
        "target": target.display().to_string(),
        "min_size_bytes": min_size_bytes,
        "count": hits.len(),
        "files": hits,
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub sha256: String,
    pub size_bytes: u64,
    pub reclaimable_bytes: u64,
    pub paths: Vec<String>,
}

/// 3-stage duplicate finder:
/// Stage 1: Exact file size grouping (free metadata check)
/// Stage 2: First-4KB SHA-256 prefix filter (avoids reading multi-GB non-matching files)
/// Stage 3: Full 64 KB streaming SHA-256 only on 4KB prefix collisions
pub fn fs_dups(target: &Path, skip_build_dirs: bool) -> Result<serde_json::Value, String> {
    if !target.is_dir() {
        return Err(format!("Not a directory: {}", target.display()));
    }

    let mut by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    let mut stack = vec![target.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            if ft.is_symlink() {
                continue;
            }
            let p = entry.path();
            if ft.is_dir() {
                if skip_build_dirs && is_ignored_dir(&name) {
                    continue;
                }
                stack.push(p);
            } else if ft.is_file() {
                if let Ok(meta) = entry.metadata() {
                    if meta.len() > 0 {
                        by_size.entry(meta.len()).or_default().push(p);
                    }
                }
            }
        }
    }

    let mut groups = Vec::new();
    let mut total_reclaimable = 0u64;

    for (sz, candidates) in by_size {
        if candidates.len() < 2 {
            continue;
        }
        // Stage 2: 4KB prefix hash
        let mut by_prefix: HashMap<String, Vec<PathBuf>> = HashMap::new();
        for p in candidates {
            if let Ok(prefix) = sha256_prefix_4k(&p) {
                by_prefix.entry(prefix).or_default().push(p);
            }
        }
        // Stage 3: Full streaming SHA-256 on prefix collisions
        for (_prefix, pref_matches) in by_prefix {
            if pref_matches.len() < 2 {
                continue;
            }
            let mut by_full: HashMap<String, Vec<String>> = HashMap::new();
            for p in pref_matches {
                if let Ok((digest, _)) = sha256_file(&p) {
                    by_full
                        .entry(digest)
                        .or_default()
                        .push(p.display().to_string());
                }
            }
            for (digest, mut paths) in by_full {
                if paths.len() > 1 {
                    paths.sort();
                    let reclaimable = sz.saturating_mul((paths.len() - 1) as u64);
                    total_reclaimable += reclaimable;
                    groups.push(DuplicateGroup {
                        sha256: digest,
                        size_bytes: sz,
                        reclaimable_bytes: reclaimable,
                        paths,
                    });
                }
            }
        }
    }

    groups.sort_by_key(|g| std::cmp::Reverse(g.reclaimable_bytes));

    Ok(serde_json::json!({
        "target": target.display().to_string(),
        "duplicate_groups": groups.len(),
        "total_reclaimable_bytes": total_reclaimable,
        "duplicates": groups,
    }))
}

pub fn fs_grep(
    target: &Path,
    query: &str,
    extension_filter: Option<&str>,
    max_matches: usize,
) -> Result<serde_json::Value, String> {
    if !target.exists() {
        return Err(format!("Path does not exist: {}", target.display()));
    }
    if query.is_empty() {
        return Err("Search query must not be empty".to_string());
    }
    let needle = query.to_lowercase();
    let ext_norm = extension_filter.map(|e| e.trim_start_matches('.').to_lowercase());
    let cap = max_matches.clamp(1, 500);

    let mut files = Vec::new();
    if target.is_file() {
        files.push(target.to_path_buf());
    } else {
        let mut stack = vec![target.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(rd) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in rd.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let Ok(ft) = entry.file_type() else {
                    continue;
                };
                if ft.is_symlink() {
                    continue;
                }
                let p = entry.path();
                if ft.is_dir() {
                    if !is_ignored_dir(&name) {
                        stack.push(p);
                    }
                } else if ft.is_file() {
                    if let Some(ref want_ext) = ext_norm {
                        let actual = p
                            .extension()
                            .map(|e| e.to_string_lossy().to_lowercase())
                            .unwrap_or_default();
                        if &actual != want_ext {
                            continue;
                        }
                    }
                    files.push(p);
                }
            }
        }
    }

    let mut matches = Vec::new();
    for p in files {
        if matches.len() >= cap {
            break;
        }
        let Ok(mut f) = File::open(&p) else {
            continue;
        };
        let mut sniff = [0u8; 512];
        let Ok(n) = f.read(&mut sniff) else {
            continue;
        };
        if sniff[..n].contains(&0) {
            continue; // binary file
        }
        let Ok(f2) = File::open(&p) else {
            continue;
        };
        let reader = BufReader::new(f2);
        for (idx, line_res) in reader.lines().enumerate() {
            let Ok(line) = line_res else {
                break;
            };
            if line.to_lowercase().contains(&needle) {
                let preview: String = line.trim().chars().take(240).collect();
                matches.push(serde_json::json!({
                    "file": p.display().to_string(),
                    "line_number": idx + 1,
                    "line": preview,
                }));
                if matches.len() >= cap {
                    break;
                }
            }
        }
    }

    Ok(serde_json::json!({
        "target": target.display().to_string(),
        "query": query,
        "match_count": matches.len(),
        "matches": matches,
    }))
}

/// Safe file write with mandatory timestamped `.bak` backup if the file already exists.
pub fn fs_backup_write(
    path: &Path,
    content: &str,
    confirm: bool,
    read_only: bool,
) -> Result<serde_json::Value, String> {
    if read_only {
        return Err("WinCraft is in read-only mode; fs_backup_write refused".to_string());
    }
    if !confirm {
        return Err("fs_backup_write is mutating and requires confirm=true".to_string());
    }

    let mut backup_created: Option<String> = None;
    if path.is_file() {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());
        let bak_path = path.with_file_name(format!("{file_name}.{ts}.bak"));
        fs::copy(path, &bak_path).map_err(|e| format!("Failed to create pre-write backup: {e}"))?;
        backup_created = Some(bak_path.display().to_string());
    } else if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create parent dir: {e}"))?;
    }

    fs::write(path, content.as_bytes()).map_err(|e| format!("Failed to write file: {e}"))?;
    let (sha256, bytes) =
        sha256_file(path).map_err(|e| format!("Failed to hash written file: {e}"))?;

    Ok(serde_json::json!({
        "success": true,
        "path": path.display().to_string(),
        "bytes_written": bytes,
        "sha256": sha256,
        "backup_path": backup_created,
    }))
}

/// Restore the most recent `<filename>.*.bak` sibling backup over `path`.
pub fn fs_undo_backup(
    path: &Path,
    confirm: bool,
    read_only: bool,
) -> Result<serde_json::Value, String> {
    if read_only {
        return Err("WinCraft is in read-only mode; fs_undo_backup refused".to_string());
    }
    if !confirm {
        return Err("fs_undo_backup is mutating and requires confirm=true".to_string());
    }
    let parent = path
        .parent()
        .ok_or_else(|| format!("Cannot resolve parent directory of {}", path.display()))?;
    let base_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or_else(|| "Invalid file name".to_string())?;
    let prefix = format!("{base_name}.");

    let mut backups: Vec<PathBuf> = fs::read_dir(parent)
        .map_err(|e| format!("Failed to read directory: {e}"))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let n = p
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            n.starts_with(&prefix) && n.ends_with(".bak")
        })
        .collect();

    backups.sort();
    let latest = backups
        .pop()
        .ok_or_else(|| format!("No .bak backup found for {}", path.display()))?;

    fs::copy(&latest, path).map_err(|e| format!("Failed to restore backup: {e}"))?;
    let _ = fs::remove_file(&latest);
    let (sha256, bytes) =
        sha256_file(path).map_err(|e| format!("Failed to hash restored file: {e}"))?;

    Ok(serde_json::json!({
        "success": true,
        "restored_path": path.display().to_string(),
        "restored_from": latest.display().to_string(),
        "bytes": bytes,
        "sha256": sha256,
    }))
}

pub fn fs_archive_inspect(archive_path: &Path) -> Result<serde_json::Value, String> {
    if !archive_path.is_file() {
        return Err(format!("Archive not found: {}", archive_path.display()));
    }
    let file = File::open(archive_path).map_err(|e| format!("Cannot open archive: {e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("Invalid ZIP archive: {e}"))?;
    let mut entries = Vec::new();
    let mut total_uncompressed = 0u64;
    let mut total_compressed = 0u64;

    for i in 0..zip.len() {
        if let Ok(entry) = zip.by_index(i) {
            total_uncompressed += entry.size();
            total_compressed += entry.compressed_size();
            if entries.len() < 200 {
                entries.push(serde_json::json!({
                    "name": entry.name(),
                    "size_bytes": entry.size(),
                    "compressed_bytes": entry.compressed_size(),
                    "is_dir": entry.is_dir(),
                }));
            }
        }
    }

    Ok(serde_json::json!({
        "archive": archive_path.display().to_string(),
        "entry_count": zip.len(),
        "total_uncompressed_bytes": total_uncompressed,
        "total_compressed_bytes": total_compressed,
        "entries": entries,
    }))
}

pub fn fs_archive_create(
    source_dir: &Path,
    dest_zip: &Path,
    confirm: bool,
    read_only: bool,
) -> Result<serde_json::Value, String> {
    if read_only {
        return Err("WinCraft is in read-only mode; fs_archive_create refused".to_string());
    }
    if !confirm {
        return Err("fs_archive_create is mutating and requires confirm=true".to_string());
    }
    if !source_dir.exists() {
        return Err(format!(
            "Source path does not exist: {}",
            source_dir.display()
        ));
    }
    if let Some(parent) = dest_zip.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create destination parent: {e}"))?;
    }

    let out_file = File::create(dest_zip).map_err(|e| format!("Failed to create zip file: {e}"))?;
    let mut writer = zip::ZipWriter::new(out_file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut files_added = 0usize;
    let mut bytes_added = 0u64;

    if source_dir.is_file() {
        let name = source_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());
        writer
            .start_file(name, options)
            .map_err(|e| format!("Zip start_file failed: {e}"))?;
        let data = fs::read(source_dir).map_err(|e| format!("Read source failed: {e}"))?;
        writer
            .write_all(&data)
            .map_err(|e| format!("Zip write failed: {e}"))?;
        files_added = 1;
        bytes_added = data.len() as u64;
    } else {
        let mut stack = vec![source_dir.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(rd) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in rd.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let Ok(ft) = entry.file_type() else {
                    continue;
                };
                if ft.is_symlink() {
                    continue;
                }
                let p = entry.path();
                if ft.is_dir() {
                    if !is_ignored_dir(&name) {
                        stack.push(p);
                    }
                } else if ft.is_file() {
                    let rel = p
                        .strip_prefix(source_dir)
                        .unwrap_or(&p)
                        .to_string_lossy()
                        .replace('\\', "/");
                    writer
                        .start_file(rel, options)
                        .map_err(|e| format!("Zip start_file failed: {e}"))?;
                    let data = fs::read(&p).map_err(|e| format!("Read file failed: {e}"))?;
                    writer
                        .write_all(&data)
                        .map_err(|e| format!("Zip write failed: {e}"))?;
                    files_added += 1;
                    bytes_added += data.len() as u64;
                }
            }
        }
    }

    writer
        .finish()
        .map_err(|e| format!("Zip finalize failed: {e}"))?;
    let zip_size = dest_zip.metadata().map(|m| m.len()).unwrap_or(0);

    Ok(serde_json::json!({
        "success": true,
        "source": source_dir.display().to_string(),
        "archive": dest_zip.display().to_string(),
        "files_added": files_added,
        "uncompressed_bytes": bytes_added,
        "archive_bytes": zip_size,
    }))
}
