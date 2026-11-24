//! Shared treemap primitives: bounds, tiles, and the tree trait.
//!
//! The layout functions in [`crate::layout`] work against [`TreemapTree`],
//! so any hierarchy (files, MFT records, budgets, flame graphs) can use this
//! crate without depending on filesystem types. Walk a returned [`Tile`]'s
//! `index_path` back through your own tree to recover names, sizes, and
//! payloads for hover tooltips and click-to-zoom.

use serde::{Deserialize, Serialize};

/// Pixel-space rectangle. `x`/`y` is the top-left corner.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LayoutRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl LayoutRect {
    /// A zero-area rectangle at the given origin.
    pub fn empty_at(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            w: 0.0,
            h: 0.0,
        }
    }

    /// Area in square pixels. Never negative.
    pub fn area(&self) -> f32 {
        self.w.max(0.0) * self.h.max(0.0)
    }

    /// True when the rect is too small to render or subdivide.
    pub fn is_degenerate(&self) -> bool {
        self.w <= 0.0 || self.h <= 0.0
    }

    /// True when `other` sits fully inside `self` (with epsilon slack for
    /// float rounding along shared edges).
    pub fn contains(&self, other: &LayoutRect) -> bool {
        let eps = 0.1_f32;
        other.x + eps >= self.x
            && other.y + eps >= self.y
            && other.x + other.w <= self.x + self.w + eps
            && other.y + other.h <= self.y + self.h + eps
    }
}

/// One computed rectangle in a treemap layout.
///
/// - `index_path` walks from the layout root to the node
///   (`[]` is the root itself).
/// - `is_container` marks enveloping folder boxes; `false` marks leaves.
/// - `depth` is 0 at the root and grows inward.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tile {
    pub index_path: Vec<usize>,
    pub rect: LayoutRect,
    pub depth: usize,
    pub is_container: bool,
}

/// Anything hierarchical that can be treemapped.
///
/// Returning `0` for empty folders is fine; layout skips zero-size children
/// at each level so they never steal pixels from real data.
pub trait TreemapTree {
    /// Cumulative size of this node (files sum into their folders).
    fn treemap_size(&self) -> u64;
    /// Direct children, pre-sorted largest-first for stable layouts.
    /// Empty for leaves.
    fn treemap_children(&self) -> &[Self]
    where
        Self: Sized;
}
