//! `cushion-treemap`: UI-agnostic squarified hierarchical treemap layout.
//!
//! Computes rectangles; your renderer draws them (egui, D3, TUI, whatever).
//! Extracted from WinCraft's WizTree-style disk treemap, generalized over any
//! [`TreemapTree`] hierarchy.
//!
//! # Quick start
//!
//! ```rust
//! use cushion_treemap::{LayoutRect, TreemapTree, hierarchical_layout};
//!
//! struct Node {
//!     name: String,
//!     size: u64,
//!     children: Vec<Node>,
//! }
//!
//! impl TreemapTree for Node {
//!     fn treemap_size(&self) -> u64 {
//!         self.size
//!     }
//!     fn treemap_children(&self) -> &[Self] {
//!         &self.children
//!     }
//! }
//!
//! let root = Node {
//!     name: "drive".to_string(),
//!     size: 300,
//!     children: vec![
//!         Node { name: "a.mp4".to_string(), size: 200, children: vec![] },
//!         Node { name: "b.zip".to_string(), size: 100, children: vec![] },
//!     ],
//! };
//! let bounds = LayoutRect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 };
//! let tiles = hierarchical_layout(&root, bounds, 5);
//! assert_eq!(tiles.len(), 2);
//! ```
//!
//! # Why another treemap crate?
//!
//! - **Hierarchical, not flat.** Existing crates (`treemap`, `streemap`)
//!   lay out one level of leaves. This crate emits *enveloping container*
//!   tiles for folders plus leaf tiles, so parents visibly contain children.
//! - **Renderer-agnostic.** No egui/D3/TUI dependency. Tiles carry
//!   `index_path`s; walk them back through your tree for hover tooltips
//!   (`filename + size`) and click-to-zoom.
//! - **Batteries, not mandates.** [`color`] ships the WizTree-style palette
//!   (video purple, archive orange, exe green, ...) as an optional helper.
//!
//! # Rendering cushions
//!
//! A tile rect plus its `depth` is enough for the classic van Wijk look:
//! fill the base color, lighten the top/left edges, darken bottom/right,
//! and add a soft top-left highlight. See WinCraft's egui page or the
//! `color` module for the reference palette.

pub mod color;
pub mod layout;
pub mod model;

pub use color::{depth_color, extension_color};
pub use layout::{
    hierarchical_layout, hierarchical_layout_with_options, squarify_layout, LayoutOptions,
};
pub use model::{LayoutRect, Tile, TreemapTree};
