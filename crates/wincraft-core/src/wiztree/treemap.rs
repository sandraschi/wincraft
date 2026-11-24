use super::models::{
    DiskUsageSummary, ExtensionUsage, FileRecord, LayoutRect, TreeNode, TreemapTile,
};
use std::collections::HashMap;
use std::path::Path;

/// Folder container color by depth. Delegates to the `cushion-treemap`
/// crate so the app and the published crate share one palette.
/// Kept under the historic name for API compatibility.
pub fn get_folder_color(depth: usize) -> [u8; 3] {
    cushion_treemap::depth_color(depth)
}

/// File cushion color by extension. Delegates to the `cushion-treemap` crate.
/// Kept under the historic name for API compatibility.
pub fn get_extension_color(ext: &str, is_folder: bool) -> [u8; 3] {
    cushion_treemap::extension_color(ext, is_folder)
}

pub fn build_file_tree(scan_root: &str, records: &[FileRecord]) -> TreeNode {
    let root_norm = scan_root.trim_end_matches(['\\', '/']).to_string();
    let root_name = Path::new(&root_norm)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&root_norm)
        .to_string();

    let mut root = TreeNode {
        name: if root_name.is_empty() {
            scan_root.to_string()
        } else {
            root_name
        },
        path: scan_root.to_string(),
        size: 0,
        is_folder: true,
        extension: String::new(),
        children: Vec::new(),
    };

    let root_norm_lower = root_norm.to_lowercase();

    for r in records {
        let p_trimmed = r.path.trim_end_matches(['\\', '/']);
        if p_trimmed.eq_ignore_ascii_case(&root_norm) {
            continue;
        }

        let p_lower = p_trimmed.to_lowercase();
        let rel_str = if p_lower.starts_with(&root_norm_lower) {
            let sub = &p_trimmed[root_norm.len()..];
            sub.trim_start_matches(['\\', '/'])
        } else if let Some(pos) = p_trimmed.find(':') {
            p_trimmed[pos + 1..].trim_start_matches(['\\', '/'])
        } else {
            p_trimmed.trim_start_matches(['\\', '/'])
        };

        if rel_str.is_empty() {
            continue;
        }

        let parts: Vec<&str> = rel_str
            .split(['\\', '/'])
            .filter(|s| !s.is_empty())
            .collect();
        if parts.is_empty() {
            continue;
        }

        let mut curr = &mut root;
        let mut curr_path = root_norm.clone();

        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;
            curr_path.push('\\');
            curr_path.push_str(part);

            if is_last {
                let ext = Path::new(&r.path)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                if let Some(existing) = curr
                    .children
                    .iter_mut()
                    .find(|c| c.name.eq_ignore_ascii_case(part))
                {
                    existing.size = r.size;
                    existing.is_folder = r.is_folder;
                    existing.extension = ext;
                } else {
                    curr.children.push(TreeNode {
                        name: part.to_string(),
                        path: r.path.clone(),
                        size: r.size,
                        is_folder: r.is_folder,
                        extension: ext,
                        children: Vec::new(),
                    });
                }
            } else {
                let idx = if let Some(pos) = curr
                    .children
                    .iter()
                    .position(|c| c.name.eq_ignore_ascii_case(part))
                {
                    pos
                } else {
                    let new_node = TreeNode {
                        name: part.to_string(),
                        path: curr_path.clone(),
                        size: 0,
                        is_folder: true,
                        extension: String::new(),
                        children: Vec::new(),
                    };
                    curr.children.push(new_node);
                    curr.children.len() - 1
                };
                curr = &mut curr.children[idx];
            }
        }
    }

    compute_tree_sizes(&mut root);
    root
}

fn compute_tree_sizes(node: &mut TreeNode) -> u64 {
    if node.is_folder && !node.children.is_empty() {
        let mut sum = 0;
        for child in &mut node.children {
            sum += compute_tree_sizes(child);
        }
        if node.size == 0 || sum > node.size {
            node.size = sum;
        }
        node.children.sort_by_key(|c| std::cmp::Reverse(c.size));
        node.size
    } else {
        node.size
    }
}

pub fn analyze_disk_records(
    scan_path: &str,
    records: &[FileRecord],
    top_n: usize,
) -> DiskUsageSummary {
    let mut total_size: u64 = 0;
    let mut file_count: usize = 0;
    let mut ext_map: HashMap<String, (usize, u64)> = HashMap::new();
    let mut file_records: Vec<FileRecord> = Vec::new();

    for r in records {
        if !r.is_folder {
            total_size += r.size;
            file_count += 1;
            file_records.push(r.clone());

            let ext = Path::new(&r.path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("no_ext")
                .to_lowercase();

            let entry = ext_map.entry(ext).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += r.size;
        }
    }

    file_records.sort_by_key(|a| std::cmp::Reverse(a.size));
    let largest_files: Vec<FileRecord> = file_records.into_iter().take(top_n).collect();

    let mut ext_list: Vec<ExtensionUsage> = ext_map
        .into_iter()
        .map(|(ext, (count, size))| {
            let percent = if total_size > 0 {
                (size as f64 / total_size as f64) * 100.0
            } else {
                0.0
            };
            ExtensionUsage {
                extension: ext,
                count,
                total_size_bytes: size,
                percent_of_total: percent,
            }
        })
        .collect();

    ext_list.sort_by_key(|a| std::cmp::Reverse(a.total_size_bytes));
    let top_extensions: Vec<ExtensionUsage> = ext_list.into_iter().take(top_n).collect();

    // Build the hierarchical tree
    let file_tree = Some(build_file_tree(scan_path, records));

    DiskUsageSummary {
        path: scan_path.to_string(),
        total_files: file_count,
        total_size_bytes: total_size,
        largest_files,
        top_extensions,
        file_tree,
    }
}

/// File-system adapter: WinCraft's [`TreeNode`] is a [`cushion_treemap::TreemapTree`].
/// Children are kept largest-first by `compute_tree_sizes`, which the layout relies on.
impl cushion_treemap::TreemapTree for TreeNode {
    fn treemap_size(&self) -> u64 {
        self.size
    }
    fn treemap_children(&self) -> &[Self] {
        &self.children
    }
}

/// Walk an [`cushion_treemap::Tile`] `index_path` back to the owning node.
fn node_at_path<'a>(root: &'a TreeNode, path: &[usize]) -> Option<&'a TreeNode> {
    let mut node = root;
    for step in path {
        node = node.children.get(*step)?;
    }
    Some(node)
}

#[derive(Debug, Clone, Default)]
pub struct HierarchicalTreemapLayout {
    pub folder_tiles: Vec<TreemapTile>,
    pub file_tiles: Vec<TreemapTile>,
}

pub fn compute_hierarchical_treemap(
    root: &TreeNode,
    bounds: LayoutRect,
    max_depth: usize,
) -> HierarchicalTreemapLayout {
    // `LayoutRect` is re-exported from `cushion-treemap`, so bounds pass straight through.
    let tiles = cushion_treemap::hierarchical_layout_with_options(
        root,
        bounds,
        &cushion_treemap::LayoutOptions {
            max_depth,
            ..Default::default()
        },
    );
    let mut layout = HierarchicalTreemapLayout::default();
    for tile in tiles {
        let Some(node) = node_at_path(root, &tile.index_path) else {
            continue;
        };
        let parent_path = tile
            .index_path
            .split_last()
            .map_or_else(String::new, |(_, parent)| {
                node_at_path(root, parent)
                    .map(|p| p.path.clone())
                    .unwrap_or_default()
            });
        // Empty folders surface as leaf slots in the generic layout; keep
        // WinCraft's historic shape by filing them as folder tiles.
        if tile.is_container || node.is_folder {
            layout.folder_tiles.push(TreemapTile {
                name: node.name.clone(),
                path: node.path.clone(),
                size: node.size,
                is_folder: true,
                extension: String::new(),
                rect: tile.rect,
                color_rgb: get_folder_color(tile.depth),
                depth: tile.depth,
                parent_path,
            });
        } else {
            layout.file_tiles.push(TreemapTile {
                name: node.name.clone(),
                path: node.path.clone(),
                size: node.size,
                is_folder: false,
                extension: node.extension.clone(),
                rect: tile.rect,
                color_rgb: get_extension_color(&node.extension, false),
                depth: tile.depth,
                parent_path,
            });
        }
    }
    layout
}

pub fn generate_treemap_html(title: &str, records: &[FileRecord]) -> String {
    let summary = analyze_disk_records(title, records, 250);
    let items_json = serde_json::to_string(&summary.file_tree).unwrap_or_else(|_| "{}".to_string());
    let exts_json =
        serde_json::to_string(&summary.top_extensions).unwrap_or_else(|_| "[]".to_string());

    let total_gb = (summary.total_size_bytes as f64) / 1_073_741_824.0;

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>WizTree Hierarchical Cushion Treemap - {title}</title>
<script src="https://d3js.org/d3.v7.min.js"></script>
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{ background: #0b0f19; color: #f1f5f9; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; padding: 16px; overflow: hidden; height: 100vh; display: flex; flex-direction: column; }}
  header {{ display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid #1e293b; padding-bottom: 12px; margin-bottom: 12px; flex-shrink: 0; }}
  h1 {{ font-size: 18px; font-weight: 700; color: #38bdf8; display: flex; align-items: center; gap: 8px; }}
  .badge {{ background: #1e293b; border: 1px solid #334155; padding: 4px 10px; border-radius: 6px; font-size: 12px; color: #94a3b8; }}
  .badge strong {{ color: #f8fafc; }}
  #breadcrumb {{ font-size: 12px; color: #94a3b8; margin-bottom: 8px; flex-shrink: 0; display: flex; gap: 6px; align-items: center; }}
  #chart-container {{ flex: 1; position: relative; border-radius: 8px; overflow: hidden; background: #020617; border: 1px solid #1e293b; }}
  #chart {{ width: 100%; height: 100%; position: relative; }}
  .node-folder {{
    position: absolute;
    box-sizing: border-box;
    border: 1px solid rgba(71, 85, 105, 0.7);
    background: rgba(15, 23, 42, 0.4);
    border-radius: 4px;
    pointer-events: none;
  }}
  .folder-tag {{
    position: absolute;
    top: 2px;
    left: 4px;
    font-size: 10px;
    font-weight: 600;
    color: #94a3b8;
    pointer-events: none;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }}
  .node-file {{
    position: absolute;
    box-sizing: border-box;
    overflow: hidden;
    cursor: pointer;
    border-radius: 3px;
    border: 1px solid rgba(0, 0, 0, 0.6);
    box-shadow: inset 1px 1px 0 rgba(255, 255, 255, 0.25), inset -1px -1px 0 rgba(0, 0, 0, 0.4);
    transition: outline 0.1s ease, filter 0.1s ease;
  }}
  .node-file:hover {{
    outline: 2px solid #facc15;
    outline-offset: -1px;
    z-index: 50;
    filter: brightness(1.2);
  }}
  .node-label {{
    position: absolute;
    top: 4px;
    left: 6px;
    right: 6px;
    font-size: 11px;
    font-weight: 600;
    color: #ffffff;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.9), 0 0 4px rgba(0, 0, 0, 0.7);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    pointer-events: none;
  }}
  .node-size {{
    position: absolute;
    bottom: 4px;
    right: 6px;
    font-size: 10px;
    color: rgba(255, 255, 255, 0.85);
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.9);
    pointer-events: none;
  }}
  .tooltip {{
    position: absolute;
    background: rgba(15, 23, 42, 0.95);
    backdrop-filter: blur(8px);
    border: 1px solid #38bdf8;
    color: #f8fafc;
    padding: 10px 14px;
    border-radius: 8px;
    font-size: 12px;
    pointer-events: none;
    display: none;
    z-index: 1000;
    box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.6), 0 8px 10px -6px rgba(0, 0, 0, 0.6);
    max-width: 450px;
    word-break: break-all;
  }}
  #inspector {{
    flex-shrink: 0;
    margin-top: 10px;
    padding: 8px 14px;
    background: #0f172a;
    border: 1px solid #1e293b;
    border-radius: 6px;
    font-size: 12px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: #94a3b8;
  }}
  #inspector strong {{ color: #f8fafc; }}
</style>
</head>
<body>
<header>
  <div>
    <h1>🗺️ WizTree Hierarchical Cushion Treemap &bull; {title}</h1>
    <div style="font-size: 11px; color: #64748b; margin-top: 2px;">Enveloping Directory Containers &bull; True Nested Cushions</div>
  </div>
  <div style="display: flex; gap: 8px;">
    <span class="badge">Items: <strong>{files_count}</strong></span>
    <span class="badge">Scanned: <strong>{total_gb:.2} GB</strong></span>
  </div>
</header>
<div id="chart-container">
  <div id="chart"></div>
</div>
<div id="inspector">
  <div id="inspector-text">Hover or click a cushion tile to inspect file details.</div>
  <div id="legend" style="display: flex; gap: 8px; font-size: 11px;"></div>
</div>
<div id="tooltip" class="tooltip"></div>
<script>
  const treeData = {items_json};
  const exts = {exts_json};
  const tooltip = document.getElementById('tooltip');
  const chart = document.getElementById('chart');
  const inspectorText = document.getElementById('inspector-text');
  const legend = document.getElementById('legend');

  const extColors = {{
    mp4: '#a855f7', mkv: '#a855f7', avi: '#a855f7', mov: '#a855f7',
    mp3: '#c084fc', wav: '#c084fc', flac: '#c084fc',
    zip: '#f97316', rar: '#f97316', '7z': '#f97316', tar: '#f97316', gz: '#f97316',
    iso: '#ef4444', img: '#ef4444', vhd: '#ef4444',
    exe: '#22c55e', msi: '#22c55e', dll: '#10b981', sys: '#10b981',
    pdf: '#e11d48', doc: '#38bdf8', docx: '#38bdf8', txt: '#38bdf8',
    xls: '#0d9488', xlsx: '#0d9488', csv: '#0d9488',
    safetensors: '#ec4899', ckpt: '#ec4899', onnx: '#ec4899', gguf: '#ec4899',
    png: '#06b6d4', jpg: '#06b6d4', jpeg: '#06b6d4', webp: '#06b6d4',
    rs: '#6366f1', py: '#6366f1', js: '#6366f1', ts: '#6366f1'
  }};

  function getColor(ext) {{
    const clean = (ext || '').toLowerCase();
    if (extColors[clean]) return extColors[clean];
    let hash = 0;
    for (let i = 0; i < clean.length; i++) hash = clean.charCodeAt(i) + ((hash << 5) - hash);
    const hues = ['#38bdf8', '#a855f7', '#22c55e', '#f59e0b', '#f43f5e', '#06b6d4', '#14b8a6', '#6366f1'];
    return hues[Math.abs(hash) % hues.length];
  }}

  if (exts && exts.length > 0) {{
    exts.slice(0, 5).forEach(e => {{
      const span = document.createElement('span');
      span.innerHTML = `<span style="display:inline-block;width:8px;height:8px;border-radius:2px;background:${{getColor(e.extension)}};margin-right:4px;"></span>.${{e.extension}} (${{e.percent_of_total.toFixed(1)}}%)`;
      legend.appendChild(span);
    }});
  }}

  if (!treeData || !treeData.children || treeData.children.length === 0) {{
    chart.innerHTML = '<div style="display:flex;height:100%;align-items:center;justify-content:center;color:#64748b;">No files to display</div>';
  }} else {{
    const width = chart.clientWidth;
    const height = chart.clientHeight;

    const root = d3.hierarchy(treeData)
      .sum(d => d.children && d.children.length > 0 ? 0 : d.size)
      .sort((a, b) => b.value - a.value);

    // True hierarchical padding so folder boxes visibly envelope children!
    d3.treemap()
      .size([width, height])
      .paddingTop(14)
      .paddingInner(2)
      .paddingOuter(3)
      .round(true)(root);

    // Render folder enveloping containers first
    root.descendants().filter(d => d.depth > 0 && d.children).forEach(folder => {{
      const fDiv = document.createElement('div');
      fDiv.className = 'node-folder';
      fDiv.style.left = folder.x0 + 'px';
      fDiv.style.top = folder.y0 + 'px';
      const w = folder.x1 - folder.x0;
      const h = folder.y1 - folder.y0;
      fDiv.style.width = w + 'px';
      fDiv.style.height = h + 'px';

      if (w > 60 && h > 28) {{
        const tag = document.createElement('div');
        tag.className = 'folder-tag';
        tag.textContent = '📁 ' + folder.data.name;
        fDiv.appendChild(tag);
      }}
      chart.appendChild(fDiv);
    }});

    // Render leaf file cushions inside
    root.leaves().forEach(leaf => {{
      const div = document.createElement('div');
      div.className = 'node-file';
      div.style.left = leaf.x0 + 'px';
      div.style.top = leaf.y0 + 'px';
      const w = leaf.x1 - leaf.x0;
      const h = leaf.y1 - leaf.y0;
      div.style.width = w + 'px';
      div.style.height = h + 'px';
      
      const ext = leaf.data.path ? leaf.data.path.split('.').pop() : 'other';
      const bg = getColor(ext);
      div.style.background = `linear-gradient(135deg, ${{bg}}cc 0%, ${{bg}} 100%)`;

      if (w > 55 && h > 24) {{
        const label = document.createElement('div');
        label.className = 'node-label';
        label.textContent = leaf.data.name;
        div.appendChild(label);
      }}

      if (w > 75 && h > 38) {{
        const sizeTag = document.createElement('div');
        sizeTag.className = 'node-size';
        const mb = leaf.value / 1048576;
        sizeTag.textContent = mb >= 1024 ? (mb / 1024).toFixed(2) + ' GB' : mb.toFixed(1) + ' MB';
        div.appendChild(sizeTag);
      }}

      div.addEventListener('mouseenter', (e) => {{
        tooltip.style.display = 'block';
        const mb = leaf.value / 1048576;
        const sizeStr = mb >= 1024 ? (mb / 1024).toFixed(2) + ' GB' : mb.toFixed(2) + ' MB';
        tooltip.innerHTML = `<strong>${{leaf.data.name}}</strong><br><span style="color:#94a3b8;">Path:</span> ${{leaf.data.path}}<br><span style="color:#94a3b8;">Size:</span> <span style="color:#38bdf8;font-weight:600;">${{sizeStr}}</span> (${{leaf.value.toLocaleString()}} bytes)`;
        inspectorText.innerHTML = `Selected: <strong>${{leaf.data.name}}</strong> &bull; ${{sizeStr}} &bull; <span style="color:#64748b;">${{leaf.data.path}}</span>`;
      }});

      div.addEventListener('mousemove', (e) => {{
        const tipW = tooltip.offsetWidth;
        const tipH = tooltip.offsetHeight;
        let left = e.pageX + 14;
        let top = e.pageY + 14;
        if (left + tipW > window.innerWidth) left = e.pageX - tipW - 14;
        if (top + tipH > window.innerHeight) top = e.pageY - tipH - 14;
        tooltip.style.left = left + 'px';
        tooltip.style.top = top + 'px';
      }});

      div.addEventListener('mouseleave', () => {{
        tooltip.style.display = 'none';
      }});

      chart.appendChild(div);
    }});
  }}
</script>
</body>
</html>"#,
        title = title,
        files_count = summary.total_files,
        total_gb = total_gb,
        items_json = items_json,
        exts_json = exts_json,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hierarchical_enveloping_treemap() {
        // Construct a directory tree:
        // C:\
        //   FolderA\ (size 700)
        //     file1.mp4 (size 500)
        //     file2.zip (size 200)
        //   FolderB\ (size 300)
        //     file3.pdf (size 300)
        let records = vec![
            FileRecord {
                path: "C:\\FolderA".into(),
                name: "FolderA".into(),
                size: 700,
                allocated: 700,
                modified: "2026-11-21".into(),
                attributes: 16,
                is_folder: true,
            },
            FileRecord {
                path: "C:\\FolderA\\file1.mp4".into(),
                name: "file1.mp4".into(),
                size: 500,
                allocated: 500,
                modified: "2026-11-21".into(),
                attributes: 32,
                is_folder: false,
            },
            FileRecord {
                path: "C:\\FolderA\\file2.zip".into(),
                name: "file2.zip".into(),
                size: 200,
                allocated: 200,
                modified: "2026-11-21".into(),
                attributes: 32,
                is_folder: false,
            },
            FileRecord {
                path: "C:\\FolderB".into(),
                name: "FolderB".into(),
                size: 300,
                allocated: 300,
                modified: "2026-11-21".into(),
                attributes: 16,
                is_folder: true,
            },
            FileRecord {
                path: "C:\\FolderB\\file3.pdf".into(),
                name: "file3.pdf".into(),
                size: 300,
                allocated: 300,
                modified: "2026-11-21".into(),
                attributes: 32,
                is_folder: false,
            },
        ];

        let tree = build_file_tree("C:\\", &records);
        assert_eq!(tree.children.len(), 2);
        assert_eq!(tree.children[0].name, "FolderA");
        assert_eq!(tree.children[0].children.len(), 2);
        assert_eq!(tree.children[1].name, "FolderB");
        assert_eq!(tree.children[1].children.len(), 1);

        let bounds = LayoutRect {
            x: 0.0,
            y: 0.0,
            w: 1000.0,
            h: 1000.0,
        };

        let layout = compute_hierarchical_treemap(&tree, bounds, 4);
        assert_eq!(layout.folder_tiles.len(), 2); // FolderA and FolderB
        assert_eq!(layout.file_tiles.len(), 3); // file1, file2, file3

        // CRITICAL CHECK: Verify that child files are strictly inside FolderA's envelope!
        assert!(layout.folder_tiles.iter().any(|f| f.name == "FolderA"));
        assert!(layout.file_tiles.iter().any(|f| f.name == "file1.mp4"));
        assert!(layout.file_tiles.iter().any(|f| f.name == "file2.zip"));
        assert!(layout.folder_tiles.iter().any(|f| f.name == "FolderB"));
        assert!(layout.file_tiles.iter().any(|f| f.name == "file3.pdf"));

        for folder_a in layout.folder_tiles.iter().filter(|f| f.name == "FolderA") {
            for file1 in layout.file_tiles.iter().filter(|f| f.name == "file1.mp4") {
                assert!(file1.rect.x >= folder_a.rect.x);
                assert!(file1.rect.y >= folder_a.rect.y);
                assert!(file1.rect.x + file1.rect.w <= folder_a.rect.x + folder_a.rect.w + 0.1);
                assert!(file1.rect.y + file1.rect.h <= folder_a.rect.y + folder_a.rect.h + 0.1);
            }
            for file2 in layout.file_tiles.iter().filter(|f| f.name == "file2.zip") {
                assert!(file2.rect.x >= folder_a.rect.x);
                assert!(file2.rect.y >= folder_a.rect.y);
                assert!(file2.rect.x + file2.rect.w <= folder_a.rect.x + folder_a.rect.w + 0.1);
                assert!(file2.rect.y + file2.rect.h <= folder_a.rect.y + folder_a.rect.h + 0.1);
            }
        }

        // file3 must be INSIDE folder_b
        for folder_b in layout.folder_tiles.iter().filter(|f| f.name == "FolderB") {
            for file3 in layout.file_tiles.iter().filter(|f| f.name == "file3.pdf") {
                assert!(file3.rect.x >= folder_b.rect.x);
                assert!(file3.rect.y >= folder_b.rect.y);
                assert!(file3.rect.x + file3.rect.w <= folder_b.rect.x + folder_b.rect.w + 0.1);
                assert!(file3.rect.y + file3.rect.h <= folder_b.rect.y + folder_b.rect.h + 0.1);
            }
        }
    }
}
