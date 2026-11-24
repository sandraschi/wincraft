use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub allocated: u64,
    pub modified: String,
    pub attributes: u32,
    pub is_folder: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveInfo {
    pub root: String,
    pub is_fixed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub success: bool,
    pub pattern: String,
    pub search_path: String,
    pub count: usize,
    pub results: Vec<FileRecord>,
    pub drives_searched: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionUsage {
    pub extension: String,
    pub count: usize,
    pub total_size_bytes: u64,
    pub percent_of_total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsageSummary {
    pub path: String,
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub largest_files: Vec<FileRecord>,
    pub top_extensions: Vec<ExtensionUsage>,
    pub file_tree: Option<TreeNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeNode {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_folder: bool,
    pub extension: String,
    pub children: Vec<TreeNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreemapNode {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub children: Vec<TreemapNode>,
}

pub use cushion_treemap::LayoutRect;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreemapTile {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_folder: bool,
    pub extension: String,
    pub rect: LayoutRect,
    pub color_rgb: [u8; 3],
    pub depth: usize,
    pub parent_path: String,
}
