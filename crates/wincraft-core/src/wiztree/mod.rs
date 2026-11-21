//! WizTree NTFS MFT engine integration, sub-second search, disk analytics, and hierarchical cushion treemaps.

pub mod cli_driver;
pub mod csv_parser;
pub mod drives;
pub mod finder;
pub mod models;
pub mod treemap;

pub use cli_driver::{execute_wiztree_scan, scan_drive_usage, search_single_drive};
pub use csv_parser::parse_wiztree_csv;
pub use drives::{can_read_mft, is_elevated, list_fixed_drives};
pub use finder::find_wiztree;
pub use models::{
    DiskUsageSummary, DriveInfo, ExtensionUsage, FileRecord, LayoutRect, SearchResult, TreeNode,
    TreemapNode, TreemapTile,
};
pub use treemap::{
    analyze_disk_records, build_file_tree, compute_hierarchical_treemap, generate_treemap_html,
    HierarchicalTreemapLayout,
};
