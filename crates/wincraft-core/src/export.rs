//! Structured dataset formatting, sorting, filtering, pagination, and export (`CSV`, `MD`, `TXT`, `Cards`, `JSON`).
//! Shared by L1 core, L2 CLI/MCP, and L3 `eframe`/`egui` desktop studio.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Cards,
    List,
    Json,
    Csv,
    Md,
    Txt,
}

impl OutputFormat {
    pub fn parse_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "list" | "table" => Self::List,
            "json" => Self::Json,
            "csv" => Self::Csv,
            "md" | "markdown" => Self::Md,
            "txt" | "text" => Self::Txt,
            _ => Self::Cards,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Cards | Self::List | Self::Txt => "txt",
            Self::Json => "json",
            Self::Csv => "csv",
            Self::Md => "md",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cards => "cards",
            Self::List => "list",
            Self::Json => "json",
            Self::Csv => "csv",
            Self::Md => "md",
            Self::Txt => "txt",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabularData {
    pub title: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

fn val_str(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

/// Extract normalized tabular columns & rows from any WinCraft command JSON payload.
pub fn payload_to_tabular(payload: &Value) -> TabularData {
    if let Some(arr) = payload.get("processes").and_then(|v| v.as_array()) {
        let headers = vec![
            "PID".to_string(),
            "Name".to_string(),
            "Memory (MB)".to_string(),
            "Memory (KB)".to_string(),
            "Session".to_string(),
            "Protected".to_string(),
        ];
        let rows = arr
            .iter()
            .map(|item| {
                vec![
                    val_str(item, "pid"),
                    val_str(item, "name"),
                    val_str(item, "memory_mb"),
                    val_str(item, "memory_kb"),
                    val_str(item, "session_name"),
                    val_str(item, "protected"),
                ]
            })
            .collect();
        return TabularData {
            title: "Windows Processes".to_string(),
            headers,
            rows,
        };
    }

    if let Some(arr) = payload.get("services").and_then(|v| v.as_array()) {
        let headers = vec![
            "Service Name".to_string(),
            "Display Name".to_string(),
            "State".to_string(),
            "PID".to_string(),
        ];
        let rows = arr
            .iter()
            .map(|item| {
                vec![
                    val_str(item, "name"),
                    val_str(item, "display_name"),
                    val_str(item, "state"),
                    val_str(item, "pid"),
                ]
            })
            .collect();
        return TabularData {
            title: "Windows Services".to_string(),
            headers,
            rows,
        };
    }

    if let Some(arr) = payload.get("bindings").and_then(|v| v.as_array()) {
        let headers = vec![
            "Port".to_string(),
            "Proto".to_string(),
            "Local Address".to_string(),
            "State".to_string(),
            "PID".to_string(),
            "Process".to_string(),
            "Fleet Owner".to_string(),
        ];
        let rows = arr
            .iter()
            .map(|item| {
                vec![
                    val_str(item, "port"),
                    val_str(item, "proto"),
                    val_str(item, "local_addr"),
                    val_str(item, "state"),
                    val_str(item, "pid"),
                    val_str(item, "process_name"),
                    val_str(item, "fleet_owner"),
                ]
            })
            .collect();
        return TabularData {
            title: "Active TCP/UDP Port Bindings".to_string(),
            headers,
            rows,
        };
    }

    if let Some(arr) = payload.get("volumes").and_then(|v| v.as_array()) {
        let headers = vec![
            "Mount".to_string(),
            "Used %".to_string(),
            "Free (GB)".to_string(),
            "Total (GB)".to_string(),
        ];
        let rows = arr
            .iter()
            .map(|item| {
                vec![
                    val_str(item, "mount"),
                    val_str(item, "used_percent"),
                    val_str(item, "free_gb"),
                    val_str(item, "total_gb"),
                ]
            })
            .collect();
        return TabularData {
            title: "Logical Disk Volumes".to_string(),
            headers,
            rows,
        };
    }

    if let Some(arr) = payload.get("candidates").and_then(|v| v.as_array()) {
        let headers = vec![
            "PID".to_string(),
            "Name".to_string(),
            "Memory (MB)".to_string(),
            "Category".to_string(),
            "Reason".to_string(),
        ];
        let rows = arr
            .iter()
            .map(|item| {
                vec![
                    val_str(item, "pid"),
                    val_str(item, "name"),
                    val_str(item, "memory_mb"),
                    val_str(item, "category"),
                    val_str(item, "reason"),
                ]
            })
            .collect();
        return TabularData {
            title: "RAM Weeder Candidates".to_string(),
            headers,
            rows,
        };
    }

    if let Some(arr) = payload.get("audit_entries").and_then(|v| v.as_array()) {
        let headers = vec![
            "PID".to_string(),
            "Name".to_string(),
            "Status".to_string(),
            "CPU (s)".to_string(),
            "CPU %".to_string(),
            "Memory (MB)".to_string(),
            "Reason".to_string(),
            "Path".to_string(),
        ];
        let rows = arr
            .iter()
            .map(|item| {
                vec![
                    val_str(item, "pid"),
                    val_str(item, "name"),
                    val_str(item, "status_badge"),
                    val_str(item, "cpu_seconds"),
                    val_str(item, "cpu_percent"),
                    val_str(item, "memory_mb"),
                    val_str(item, "reason"),
                    val_str(item, "path"),
                ]
            })
            .collect();
        return TabularData {
            title: "CPU Hogger & Compute Audit".to_string(),
            headers,
            rows,
        };
    }

    if let Some(_obj) = payload.get("thermal_report").and_then(|v| v.as_object()) {
        let report = &payload["thermal_report"];
        let headers = vec!["Metric".to_string(), "Value".to_string()];
        let rows = vec![
            vec!["Processor".to_string(), val_str(report, "processor_name")],
            vec!["Status".to_string(), val_str(report, "status")],
            vec!["Load %".to_string(), val_str(report, "load_percentage")],
            vec!["Physical Cores".to_string(), val_str(report, "cores")],
            vec![
                "Logical Processors".to_string(),
                val_str(report, "logical_processors"),
            ],
            vec![
                "Max Clock (MHz)".to_string(),
                val_str(report, "max_clock_mhz"),
            ],
            vec![
                "Current Clock (MHz)".to_string(),
                val_str(report, "current_clock_mhz"),
            ],
            vec![
                "Throttle Detected".to_string(),
                val_str(report, "throttle_detected"),
            ],
        ];
        return TabularData {
            title: "Host Processor & Thermal Status".to_string(),
            headers,
            rows,
        };
    }

    if let Some(arr) = payload.get("duplicates").and_then(|v| v.as_array()) {
        let headers = vec![
            "SHA-256".to_string(),
            "Size (Bytes)".to_string(),
            "Reclaimable (Bytes)".to_string(),
            "Paths".to_string(),
        ];
        let rows = arr
            .iter()
            .map(|item| {
                let paths = item
                    .get("paths")
                    .and_then(|p| p.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str())
                            .collect::<Vec<_>>()
                            .join(" ; ")
                    })
                    .unwrap_or_default();
                vec![
                    val_str(item, "sha256"),
                    val_str(item, "size_bytes"),
                    val_str(item, "reclaimable_bytes"),
                    paths,
                ]
            })
            .collect();
        return TabularData {
            title: "Duplicate File Groups".to_string(),
            headers,
            rows,
        };
    }

    if let Some(arr) = payload
        .get("items")
        .or_else(|| payload.get("files"))
        .or_else(|| payload.get("entries"))
        .and_then(|v| v.as_array())
    {
        if let Some(first) = arr.first() {
            if first.get("command").is_some() && first.get("source").is_some() {
                let headers = vec![
                    "Name".to_string(),
                    "Type".to_string(),
                    "Command".to_string(),
                    "Source".to_string(),
                ];
                let rows = arr
                    .iter()
                    .map(|item| {
                        vec![
                            val_str(item, "name"),
                            val_str(item, "reg_type"),
                            val_str(item, "command"),
                            val_str(item, "source"),
                        ]
                    })
                    .collect();
                return TabularData {
                    title: "Windows Startup Autoruns".to_string(),
                    headers,
                    rows,
                };
            }
        }
        let headers = vec![
            "Name".to_string(),
            "Type".to_string(),
            "Size (Bytes)".to_string(),
            "Path".to_string(),
        ];
        let rows = arr
            .iter()
            .map(|item| {
                let is_dir = item
                    .get("is_dir")
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false);
                vec![
                    val_str(item, "name"),
                    if is_dir {
                        "DIR".to_string()
                    } else {
                        "FILE".to_string()
                    },
                    val_str(item, "size_bytes"),
                    val_str(item, "path"),
                ]
            })
            .collect();
        return TabularData {
            title: "FileOps Entries".to_string(),
            headers,
            rows,
        };
    }

    if let Some(arr) = payload.get("matches").and_then(|v| v.as_array()) {
        let headers = vec![
            "File".to_string(),
            "Line #".to_string(),
            "Line Preview".to_string(),
        ];
        let rows = arr
            .iter()
            .map(|item| {
                vec![
                    val_str(item, "file"),
                    val_str(item, "line_number"),
                    val_str(item, "line"),
                ]
            })
            .collect();
        return TabularData {
            title: "Grep Matches".to_string(),
            headers,
            rows,
        };
    }

    if let Some(arr) = payload.get("tasks").and_then(|v| v.as_array()) {
        let headers = vec![
            "Task Name".to_string(),
            "Next Run".to_string(),
            "Status".to_string(),
        ];
        let rows = arr
            .iter()
            .map(|item| {
                vec![
                    val_str(item, "task_name"),
                    val_str(item, "next_run"),
                    val_str(item, "status"),
                ]
            })
            .collect();
        return TabularData {
            title: "Scheduled Tasks".to_string(),
            headers,
            rows,
        };
    }

    if let Some(arr) = payload.get("values").and_then(|v| v.as_array()) {
        let headers = vec![
            "Value Name".to_string(),
            "Type".to_string(),
            "Data".to_string(),
        ];
        let rows = arr
            .iter()
            .map(|item| {
                vec![
                    val_str(item, "name"),
                    val_str(item, "reg_type"),
                    val_str(item, "data"),
                ]
            })
            .collect();
        return TabularData {
            title: format!("Registry Key: {}", val_str(payload, "key")),
            headers,
            rows,
        };
    }

    if let Some(arr) = payload.get("tools").and_then(|v| v.as_array()) {
        let headers = vec![
            "Tool".to_string(),
            "Domain".to_string(),
            "Read-Only".to_string(),
            "Confirm Gate".to_string(),
            "Description".to_string(),
        ];
        let rows = arr
            .iter()
            .map(|item| {
                vec![
                    val_str(item, "name"),
                    val_str(item, "domain"),
                    val_str(item, "read_only"),
                    val_str(item, "confirm_gated"),
                    val_str(item, "description"),
                ]
            })
            .collect();
        return TabularData {
            title: "WinCraft Tools Catalog".to_string(),
            headers,
            rows,
        };
    }

    // Fallback key-value summary table
    let mut rows = Vec::new();
    if let Some(obj) = payload.as_object() {
        for (k, v) in obj {
            let val_s = match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            rows.push(vec![k.clone(), val_s]);
        }
    }
    TabularData {
        title: "WinCraft Output".to_string(),
        headers: vec!["Field".to_string(), "Value".to_string()],
        rows,
    }
}

/// Filter, sort, and paginate a `TabularData` dataset.
pub fn filter_sort_paginate(
    mut data: TabularData,
    filter_query: Option<&str>,
    sort_col: Option<usize>,
    ascending: bool,
    page: usize,
    page_size: usize,
) -> (TabularData, usize, usize) {
    if let Some(q) = filter_query {
        let needle = q.trim().to_lowercase();
        if !needle.is_empty() {
            data.rows
                .retain(|row| row.iter().any(|cell| cell.to_lowercase().contains(&needle)));
        }
    }

    if let Some(col) = sort_col {
        if col < data.headers.len() {
            data.rows.sort_by(|a, b| {
                let sa = a.get(col).map(|s| s.as_str()).unwrap_or("");
                let sb = b.get(col).map(|s| s.as_str()).unwrap_or("");
                // Numeric-aware comparison when both parse as f64
                let cmp = match (sa.parse::<f64>(), sb.parse::<f64>()) {
                    (Ok(na), Ok(nb)) => na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal),
                    _ => sa.to_lowercase().cmp(&sb.to_lowercase()),
                };
                if ascending {
                    cmp
                } else {
                    cmp.reverse()
                }
            });
        }
    }

    let total_rows = data.rows.len();
    let per_page = page_size.max(1);
    let total_pages = total_rows.div_ceil(per_page).max(1);
    let clamped_page = page.clamp(1, total_pages);
    let start = (clamped_page - 1) * per_page;
    let end = (start + per_page).min(total_rows);
    data.rows = if start < total_rows {
        data.rows[start..end].to_vec()
    } else {
        Vec::new()
    };

    (data, total_rows, total_pages)
}

fn escape_csv(cell: &str) -> String {
    if cell.contains(',') || cell.contains('"') || cell.contains('\n') || cell.contains('\r') {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell.to_string()
    }
}

pub fn format_csv(data: &TabularData) -> String {
    let mut out = String::new();
    let header_line: Vec<String> = data.headers.iter().map(|h| escape_csv(h)).collect();
    out.push_str(&header_line.join(","));
    out.push('\n');
    for row in &data.rows {
        let cells: Vec<String> = row.iter().map(|c| escape_csv(c)).collect();
        out.push_str(&cells.join(","));
        out.push('\n');
    }
    out
}

pub fn format_markdown(data: &TabularData) -> String {
    let mut out = format!("# {}\n\n", data.title);
    if data.headers.is_empty() {
        return out;
    }
    out.push_str("| ");
    out.push_str(&data.headers.join(" | "));
    out.push_str(" |\n| ");
    let sep: Vec<&str> = data.headers.iter().map(|_| "---").collect();
    out.push_str(&sep.join(" | "));
    out.push_str(" |\n");
    for row in &data.rows {
        let escaped: Vec<String> = row
            .iter()
            .map(|c| c.replace('|', "\\|").replace('\n', " "))
            .collect();
        out.push_str("| ");
        out.push_str(&escaped.join(" | "));
        out.push_str(" |\n");
    }
    out
}

pub fn format_list_txt(data: &TabularData) -> String {
    let mut out = format!("=== {} ({} rows) ===\n", data.title, data.rows.len());
    if data.headers.is_empty() {
        return out;
    }
    let mut widths: Vec<usize> = data.headers.iter().map(|h| h.len()).collect();
    for row in &data.rows {
        for (i, cell) in row.iter().enumerate() {
            if let Some(w) = widths.get_mut(i) {
                *w = (*w).max(cell.len()).min(48);
            }
        }
    }
    for (i, h) in data.headers.iter().enumerate() {
        let w = widths.get(i).copied().unwrap_or(10);
        out.push_str(&format!("{h:<w$}  "));
    }
    out.push('\n');
    for w in &widths {
        out.push_str(&"-".repeat(*w));
        out.push_str("  ");
    }
    out.push('\n');
    for row in &data.rows {
        for (i, cell) in row.iter().enumerate() {
            let w = widths.get(i).copied().unwrap_or(10);
            let clipped: String = cell.chars().take(w).collect();
            out.push_str(&format!("{clipped:<w$}  "));
        }
        out.push('\n');
    }
    out
}

pub fn format_cards_txt(data: &TabularData) -> String {
    let mut out = format!(
        "=== {} [Cards View — {} items] ===\n\n",
        data.title,
        data.rows.len()
    );
    for (idx, row) in data.rows.iter().enumerate() {
        let primary = row.first().map(|s| s.as_str()).unwrap_or("-");
        let secondary = row.get(1).map(|s| s.as_str()).unwrap_or("");
        out.push_str(&format!(
            "┌─ Card #{:<4} {} · {}\n",
            idx + 1,
            primary,
            secondary
        ));
        for (col_idx, header) in data.headers.iter().enumerate() {
            let val = row.get(col_idx).map(|s| s.as_str()).unwrap_or("");
            out.push_str(&format!("│  {:<18}: {}\n", header, val));
        }
        out.push_str("└──────────────────────────────────────────────\n");
    }
    out
}

pub fn render_payload(payload: &Value, fmt: OutputFormat) -> String {
    if fmt == OutputFormat::Json {
        return serde_json::to_string_pretty(payload).unwrap_or_else(|_| "{}".to_string());
    }
    let tab = payload_to_tabular(payload);
    render_tabular(&tab, payload, fmt)
}

pub fn render_tabular(tab: &TabularData, raw_payload: &Value, fmt: OutputFormat) -> String {
    match fmt {
        OutputFormat::Cards => format_cards_txt(tab),
        OutputFormat::List | OutputFormat::Txt => format_list_txt(tab),
        OutputFormat::Csv => format_csv(tab),
        OutputFormat::Md => format_markdown(tab),
        OutputFormat::Json => {
            serde_json::to_string_pretty(raw_payload).unwrap_or_else(|_| "{}".to_string())
        }
    }
}

pub fn export_to_file(
    tab: &TabularData,
    raw_payload: &Value,
    fmt: OutputFormat,
    dest: &Path,
) -> Result<String, String> {
    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create export folder: {e}"))?;
        }
    }
    let content = render_tabular(tab, raw_payload, fmt);
    fs::write(dest, content.as_bytes()).map_err(|e| format!("Failed to write export file: {e}"))?;
    Ok(dest.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tabular_extraction_and_csv_md_cards_formatting_work() {
        let sample = json!({
            "count": 2,
            "processes": [
                {
                    "pid": 100,
                    "name": "alpha.exe",
                    "memory_mb": 42.5,
                    "memory_kb": 43520,
                    "session_name": "Console",
                    "protected": false
                },
                {
                    "pid": 200,
                    "name": "beta.exe",
                    "memory_mb": 128.0,
                    "memory_kb": 131072,
                    "session_name": "Services",
                    "protected": true
                }
            ]
        });

        let tab = payload_to_tabular(&sample);
        assert_eq!(tab.headers.len(), 6);
        assert_eq!(tab.rows.len(), 2);

        // Sort by Memory (MB) column (index 2) descending
        let (sorted, total, pages) = filter_sort_paginate(tab, None, Some(2), false, 1, 10);
        assert_eq!(total, 2);
        assert_eq!(pages, 1);
        assert_eq!(sorted.rows[0][1], "beta.exe");

        let csv = format_csv(&sorted);
        assert!(csv.contains("PID,Name,Memory (MB)"));
        assert!(csv.contains("200,beta.exe,128"));

        let md = format_markdown(&sorted);
        assert!(md.contains("| PID | Name | Memory (MB) |"));
        assert!(md.contains("| 200 | beta.exe | 128.0 |"));

        let cards = format_cards_txt(&sorted);
        assert!(cards.contains("Card #1"));
        assert!(cards.contains("beta.exe"));
    }
}
