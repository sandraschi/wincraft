# cushion-treemap

UI-agnostic **squarified hierarchical cushion-treemap** layout for Rust.
Computes rectangles; your renderer draws them.

Extracted from [WinCraft](https://github.com/sandraschi/wincraft)'s WizTree-style
disk treemap and generalized over any tree.

## Why another treemap crate?

| Crate | Flat leaves | Enveloping containers | Renderer-free |
|---|---|---|---|
| `treemap` | yes | no | yes |
| `streemap` | yes | partial | yes |
| `tui-treemap` | yes | no | no (ratatui) |
| **`cushion-treemap`** | yes | **yes** | yes |

If you want WizTree-style nesting - folder boxes that visibly *contain*
their files, hover tooltips with filename + size, click-to-zoom into folders -
this is the crate.

## Use

```rust
use cushion_treemap::{LayoutRect, TreemapTree, hierarchical_layout};

struct Node { name: String, size: u64, children: Vec<Node> }

impl TreemapTree for Node {
    fn treemap_size(&self) -> u64 { self.size }
    fn treemap_children(&self) -> &[Self] { &self.children }
}

let bounds = LayoutRect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 };
let tiles = hierarchical_layout(&root, bounds, 5);
// each Tile has index_path + rect + depth + is_container:
// walk index_path back through your tree for hover/zoom.
```

- `squarify_layout(&sizes, bounds)` - one flat level (Bruls et al. squarified).
- `hierarchical_layout(&root, bounds, max_depth)` - enveloping containers + leaves.
- `hierarchical_layout_with_options` - custom padding / header / depth tuning.
- `color::{depth_color, extension_color}` - optional WizTree-style palette.

## Rendering cushions (van Wijk style)

Fill the tile with its base color, lighten top/left edges, darken
bottom/right, add a soft top-left highlight. Depth darkens folder containers.
WinCraft's egui page is the reference implementation.

## Status

- `0.1.0`: layout + colors + tests, no I/O, no renderer deps (only `serde`).
- Not yet on crates.io - publish with `cargo publish -p cushion-treemap --dry-run` first.

## License

MIT OR Apache-2.0
