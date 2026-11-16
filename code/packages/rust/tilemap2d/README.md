# tilemap2d

Deterministic, zero-allocation, multi-layer scrolling tilemap engine for Rust: parallax, viewport culling, tile-attribute collision, `paint-batch-tiles` lowering and indexed scanline rendering.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME21`).

---

## Overview

NES, Game Boy, SNES and GBA hardware drew backgrounds as grids of small tiles, each cell carrying a tile id, flip bits and a palette bank, scrolled per layer and composited scanline by scanline. `tilemap2d` models that pipeline on top of the repo's existing graphics crates:

1. **Layers** (`TileLayer`): a borrowed, validated `cols x rows` grid of `TileCell { tile_id, flags, palette_bank, attr }` with Q8 fixed-point parallax (`parallax_x_q8`, `PARALLAX_ONE = 256`), a static scroll offset, per-axis `WrapMode::{Wrap, Clamp, Void}`, H/V tile flipping, visibility, colour-0 transparency, blend mode and alpha.
2. **Culling** (`cull_layer`, `layer_scroll`): the visible tile rectangle (`VisibleRect`) for a `Viewport` and `Camera`, in checked `i64` arithmetic. Wrapping axes may extend past the map (resolved modulo its size); clamped / void axes are clipped to it.
3. **Collision** (`attr_at_point`, `aabb_attrs`, `aabb_first_hit`, `aabb_blocked`, ...): point and AABB queries over `ATTR_SOLID`, `ATTR_ONE_WAY`, `ATTR_WATER`, `ATTR_HAZARD` and `ATTR_TRIGGER` bits. One-way platforms block only downward motion that started above the tile. Query area is capped at `MAX_QUERY_TILES`.
4. **Lowering** (`lower_layer_cells_into`, `lower_layers_to_paint_scene`): visible tiles become `paint-batch-tiles` (`P2D09`) `TileBatchCell`s (caller buffer, zero allocation) or a `paint-instructions` (`P2D00`) `PaintScene` with stable `layer<N>/tile:<col>:<row>` ids and the sub-tile scroll offset applied.
5. **Indexed scanline rendering** (`render_layer_scanline`, `render_layers_to_container`): tiles live packed in a `Tileset` (`palette-indexed`, `GAME20`), one tile row is unpacked at a time (flips applied) and expanded through `Palette` banks with the blend modes of `paint-palette-blend` (`P2D10`). Calling the scanline renderer per row with a different `Camera` gives raster (HBlank) scroll effects.

Tile id `0` (`EMPTY_TILE`) is the blank tile: never drawn, never lowered (it keeps its collision attributes).

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** `"capabilities": []` in `required_capabilities.json`.
- **`CAP002` (Zero Unsafe):** `#![forbid(unsafe_code)]`.
- **Zero Allocation Hot Path (`VER07`):** scroll, cull, collision, cell lowering and scanline / frame rendering never touch the heap (verified with the counting allocator in `tests/microbenchmarks.rs`). Only `lower_layers_to_paint_scene` builds an owned scene.
- **Strict Validation:** CWE-20 (cell flag / attribute bits, tile ids, palette banks, bit depths), CWE-125/787 (buffer and container lengths checked before access; validation passes run before the first write so errors leave the target untouched), CWE-190 (checked `i64`/`usize` arithmetic, scroll extremes), CWE-369 (all divisors validated non-zero), CWE-400 (`MAX_MAP_DIM = 4096`, `MAX_TILE_SIZE = 256`, `MAX_VIEWPORT = 8192`, `MAX_LAYERS = 8`, `MAX_QUERY_TILES = 65536`, `MAX_PALETTE_BANKS = 256`).
- **26-CWE Weakness Closure (`VER15`):** `assurance_report.json` records the closure/exclusion matrix.

---

## Usage

```rust
use tilemap2d::*;
use palette_indexed::pack_row_into;

// One 4x4 Bpp4 tile (id 1) filled with colour 3; tile 0 is blank.
let mut tile_data = [0u8; 2 * 4 * 2];
for y in 0..4 {
    pack_row_into(BitDepth::Bpp4, &[3, 3, 3, 3], &mut tile_data[8 + y * 2..8 + y * 2 + 2])?;
}
let tileset = Tileset::attach_tiles(BitDepth::Bpp4, &tile_data, 4, 4, 2)?;

let mut bank = Palette::blank(BitDepth::Bpp4);
bank.set_entry(3, [255, 0, 0, 255])?;

let cells = [TileCell::tile(1).with_attr(ATTR_SOLID), TileCell::tile(0)];
let mut layer = TileLayer::attach(&cells, 2, 1, 4, 4)?;
layer.parallax_x_q8 = PARALLAX_ONE / 2;

let mut screen = PixelContainer::new(8, 4);
render_layers_to_container(&[layer], &tileset, &[bank], Camera { x: 0, y: 0 }, None, &mut screen)?;

assert!(point_has_attr(&layer, 1, 1, ATTR_SOLID));
# Ok::<(), TilemapError>(())
```

## Core Types & Functions

- `TileCell`, `TileLayer<'a>`, `WrapMode`, `Camera`, `Viewport`, `PixelPos`, `VisibleRect`, `Tileset<'a>`, `TilemapError`
- `layer_scroll`, `screen_to_layer`, `cull_layer`, `cell_at_tile`, `validate_viewport`
- `Aabb`, `MotionHint`, `TileHit`, `attr_at_tile`, `attr_at_point`, `point_has_attr`, `aabb_attrs`, `aabb_has_attr`, `aabb_first_hit`, `aabb_blocked`
- `count_visible_cells`, `lower_layer_cells_into`, `lower_layers_to_paint_scene`
- `render_layer_scanline`, `render_layers_to_container`

## Testing

```
cargo test -p tilemap2d -- --nocapture
cargo test -p tilemap2d --release -- --nocapture
```
