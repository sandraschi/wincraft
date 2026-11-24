//! Squarified hierarchical layout (Bruls et al., 2000) with enveloping
//! containers in the WizTree / van Wijk cushion style.
//!
//! Two entry points:
//!
//! - [`squarify_layout`] lays out one flat row of siblings.
//! - [`hierarchical_layout`] (or [`hierarchical_layout_with_options`]) recurses
//!   into children, emitting one container [`Tile`](crate::model::Tile) per
//!   folder plus one leaf tile per childless node. Parent containers strictly
//!   envelop their children, so hover and click-to-zoom just work.

use crate::model::{LayoutRect, Tile, TreemapTree};

/// Tuning knobs for [`hierarchical_layout_with_options`].
#[derive(Debug, Clone)]
pub struct LayoutOptions {
    /// How many levels below the root to recurse (root is depth 0).
    pub max_depth: usize,
    /// Inset of children inside their container, in pixels.
    pub container_pad: f32,
    /// Reserved header strip at the top of a container for its label.
    /// Applied only when the container is large enough to show one.
    pub header_h: f32,
    /// Containers smaller than this stop recursing (avoids sub-pixel noise).
    pub min_container_px: f32,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            max_depth: 5,
            container_pad: 3.0,
            header_h: 14.0,
            min_container_px: 12.0,
        }
    }
}

#[derive(Clone, Copy)]
struct SquarifyItem {
    index: usize,
    area: f64,
}

fn worst_aspect_ratio(row: &[SquarifyItem], side_len: f64) -> f64 {
    if row.is_empty() || side_len <= 0.0 {
        return f64::MAX;
    }
    let mut row_area = 0.0;
    let mut min_area = f64::MAX;
    let mut max_area = 0.0;
    for item in row {
        row_area += item.area;
        if item.area < min_area {
            min_area = item.area;
        }
        if item.area > max_area {
            max_area = item.area;
        }
    }
    if row_area <= 0.0 || min_area <= 0.0 {
        return f64::MAX;
    }
    let side_sq = side_len * side_len;
    let row_area_sq = row_area * row_area;
    let r1 = (side_sq * max_area) / row_area_sq;
    let r2 = row_area_sq / (side_sq * min_area);
    r1.max(r2)
}

/// Lay out one flat list of sibling sizes into `bounds`.
///
/// Areas are proportional to `sizes`. Degenerate bounds or an all-zero total
/// yield zero-area rects at the bounds origin instead of failing.
pub fn squarify_layout(sizes: &[u64], bounds: LayoutRect) -> Vec<LayoutRect> {
    let empty: Vec<LayoutRect> = sizes
        .iter()
        .map(|_| LayoutRect::empty_at(bounds.x, bounds.y))
        .collect();
    if sizes.is_empty() || bounds.is_degenerate() {
        return empty;
    }
    let total: u64 = sizes.iter().sum();
    if total == 0 {
        return empty;
    }

    let total_area = f64::from(bounds.w) * f64::from(bounds.h);
    let items: Vec<SquarifyItem> = sizes
        .iter()
        .enumerate()
        .map(|(index, size)| SquarifyItem {
            index,
            area: (*size as f64 / total as f64) * total_area,
        })
        .collect();

    let mut remaining = bounds;
    let mut rects = vec![
        LayoutRect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0
        };
        sizes.len()
    ];
    let mut current_row: Vec<SquarifyItem> = Vec::new();

    for item in items {
        if current_row.is_empty() {
            current_row.push(item);
        } else {
            let mut candidate = current_row.clone();
            candidate.push(item);
            let side_len = f64::from(remaining.w.min(remaining.h));
            let cur_worst = worst_aspect_ratio(&current_row, side_len);
            let cand_worst = worst_aspect_ratio(&candidate, side_len);
            if cand_worst <= cur_worst {
                current_row.push(item);
            } else {
                layout_row_to_rects(&current_row, &mut remaining, &mut rects);
                current_row = vec![item];
            }
        }
    }

    if !current_row.is_empty() {
        layout_row_to_rects(&current_row, &mut remaining, &mut rects);
    }

    rects
}

fn layout_row_to_rects(row: &[SquarifyItem], remaining: &mut LayoutRect, rects: &mut [LayoutRect]) {
    if row.is_empty() || remaining.is_degenerate() {
        return;
    }
    let row_area: f64 = row.iter().map(|i| i.area).sum();
    if row_area <= 0.0 {
        return;
    }

    if remaining.w >= remaining.h {
        // Vertical strip of width W.
        let strip_w = ((row_area / f64::from(remaining.h)) as f32).min(remaining.w);
        let mut cur_y = remaining.y;
        for (i, item) in row.iter().enumerate() {
            let item_h = if i == row.len() - 1 {
                (remaining.y + remaining.h - cur_y).max(0.0)
            } else {
                ((item.area / row_area) * f64::from(remaining.h)) as f32
            };
            rects[item.index] = LayoutRect {
                x: remaining.x,
                y: cur_y,
                w: strip_w,
                h: item_h,
            };
            cur_y += item_h;
        }
        remaining.x += strip_w;
        remaining.w = (remaining.w - strip_w).max(0.0);
    } else {
        // Horizontal strip of height H.
        let strip_h = ((row_area / f64::from(remaining.w)) as f32).min(remaining.h);
        let mut cur_x = remaining.x;
        for (i, item) in row.iter().enumerate() {
            let item_w = if i == row.len() - 1 {
                (remaining.x + remaining.w - cur_x).max(0.0)
            } else {
                ((item.area / row_area) * f64::from(remaining.w)) as f32
            };
            rects[item.index] = LayoutRect {
                x: cur_x,
                y: remaining.y,
                w: item_w,
                h: strip_h,
            };
            cur_x += item_w;
        }
        remaining.y += strip_h;
        remaining.h = (remaining.h - strip_h).max(0.0);
    }
}

/// Lay out `root` and its descendants with default [`LayoutOptions`].
pub fn hierarchical_layout<T: TreemapTree>(
    root: &T,
    bounds: LayoutRect,
    max_depth: usize,
) -> Vec<Tile> {
    hierarchical_layout_with_options(
        root,
        bounds,
        &LayoutOptions {
            max_depth,
            ..LayoutOptions::default()
        },
    )
}

/// Lay out `root` and its descendants with explicit options.
///
/// Emits container tiles for folders at depth > 0 and leaf tiles for
/// childless nodes. Children render inside their parent's padded inner rect,
/// so every child rect is contained in its parent rect. The root itself gets
/// no tile when it has children (it *is* the canvas); a childless root yields
/// a single leaf tile covering `bounds`.
pub fn hierarchical_layout_with_options<T: TreemapTree>(
    root: &T,
    bounds: LayoutRect,
    opts: &LayoutOptions,
) -> Vec<Tile> {
    let mut tiles = Vec::new();
    if bounds.is_degenerate() {
        return tiles;
    }
    let mut path = Vec::new();
    layout_node_recursive(root, bounds, 0, opts, &mut path, &mut tiles);
    tiles
}

fn layout_node_recursive<T: TreemapTree>(
    node: &T,
    rect: LayoutRect,
    depth: usize,
    opts: &LayoutOptions,
    path: &mut Vec<usize>,
    tiles: &mut Vec<Tile>,
) {
    if rect.is_degenerate() {
        return;
    }
    let children = node.treemap_children();

    if children.is_empty() {
        // Leaf: files, single-value roots, anything childless.
        if depth > 0 || node.treemap_size() > 0 {
            tiles.push(Tile {
                index_path: path.clone(),
                rect,
                depth,
                is_container: false,
            });
        } else {
            // Childless zero-size root: still cover the canvas so renderers
            // show *something* instead of an empty screen.
            tiles.push(Tile {
                index_path: path.clone(),
                rect,
                depth,
                is_container: false,
            });
        }
        return;
    }

    if depth > 0 {
        tiles.push(Tile {
            index_path: path.clone(),
            rect,
            depth,
            is_container: true,
        });
    }

    if depth >= opts.max_depth || rect.w < opts.min_container_px || rect.h < opts.min_container_px {
        return;
    }

    let pad = if depth == 0 { 2.0 } else { opts.container_pad };
    let header_h = if depth > 0 && rect.w > 65.0 && rect.h > 36.0 {
        opts.header_h
    } else {
        0.0
    };
    let inner = LayoutRect {
        x: rect.x + pad,
        y: rect.y + pad + header_h,
        w: (rect.w - 2.0 * pad).max(0.0),
        h: (rect.h - 2.0 * pad - header_h).max(0.0),
    };
    if inner.w < 4.0 || inner.h < 4.0 {
        return;
    }

    // Only positive-size children earn pixels.
    let live: Vec<usize> = children
        .iter()
        .enumerate()
        .filter(|(_, c)| c.treemap_size() > 0)
        .map(|(i, _)| i)
        .collect();
    if live.is_empty() {
        return;
    }
    let sizes: Vec<u64> = live.iter().map(|i| children[*i].treemap_size()).collect();
    let total: u64 = sizes.iter().sum();
    if total == 0 {
        return;
    }

    let rects = squarify_layout(&sizes, inner);
    for (slot, child_idx) in live.iter().enumerate() {
        path.push(*child_idx);
        layout_node_recursive(
            &children[*child_idx],
            rects[slot],
            depth + 1,
            opts,
            path,
            tiles,
        );
        path.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TreemapTree;

    #[derive(Debug)]
    struct TestNode {
        size: u64,
        children: Vec<TestNode>,
    }

    impl TreemapTree for TestNode {
        fn treemap_size(&self) -> u64 {
            self.size
        }
        fn treemap_children(&self) -> &[Self] {
            &self.children
        }
    }

    fn file(size: u64) -> TestNode {
        TestNode {
            size,
            children: Vec::new(),
        }
    }

    fn folder(size: u64, children: Vec<TestNode>) -> TestNode {
        TestNode { size, children }
    }

    fn bounds() -> LayoutRect {
        LayoutRect {
            x: 0.0,
            y: 0.0,
            w: 1000.0,
            h: 1000.0,
        }
    }

    #[test]
    fn squarify_conserves_area() {
        let rects = squarify_layout(&[500, 300, 200], bounds());
        assert_eq!(rects.len(), 3);
        let total: f32 = rects.iter().map(|r| r.area()).sum();
        let canvas = bounds().area();
        assert!((total - canvas).abs() < canvas * 0.01);
    }

    #[test]
    fn squarify_handles_empty_and_zero() {
        assert!(squarify_layout(&[], bounds()).is_empty());
        let rects = squarify_layout(&[0, 0], bounds());
        assert_eq!(rects.len(), 2);
        assert!(rects.iter().all(|r| r.area() == 0.0));
        let deg = LayoutRect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 50.0,
        };
        let rects = squarify_layout(&[10, 20], deg);
        assert!(rects.iter().all(|r| r.area() == 0.0));
    }

    #[test]
    fn hierarchical_containers_envelop_children() {
        let root = folder(
            1000,
            vec![
                folder(700, vec![file(500), file(200)]),
                folder(300, vec![file(300)]),
            ],
        );
        let tiles = hierarchical_layout(&root, bounds(), 5);
        let containers: Vec<&Tile> = tiles.iter().filter(|t| t.is_container).collect();
        let leaves: Vec<&Tile> = tiles.iter().filter(|t| !t.is_container).collect();
        assert_eq!(containers.len(), 2);
        assert_eq!(leaves.len(), 3);

        for leaf in leaves {
            // Parent path is the leaf path minus its last segment.
            let parent_len = leaf.index_path.len().saturating_sub(1);
            let parent_path = &leaf.index_path[..parent_len];
            let mut found = false;
            for c in &containers {
                if c.index_path == parent_path {
                    assert!(c.rect.contains(&leaf.rect));
                    found = true;
                }
            }
            assert!(found);
        }
    }

    #[test]
    fn max_depth_stops_recursion() {
        let root = folder(100, vec![folder(100, vec![folder(100, vec![file(100)])])]);
        let shallow = hierarchical_layout(&root, bounds(), 1);
        let deep = hierarchical_layout(&root, bounds(), 5);
        assert!(shallow.len() < deep.len());
        assert!(shallow.iter().all(|t| t.depth <= 1));
    }

    #[test]
    fn childless_root_yields_single_leaf() {
        let root = file(42);
        let tiles = hierarchical_layout(&root, bounds(), 5);
        assert_eq!(tiles.len(), 1);
        assert!(!tiles[0].is_container);
        assert_eq!(tiles[0].rect, bounds());
    }
}
