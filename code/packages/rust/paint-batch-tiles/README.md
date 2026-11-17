# paint-batch-tiles

Pure, deterministic, zero-allocation 2D sprite and tile batch lowering and retained-mode diffing for Rust.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`P2D09`).

---

## Overview

In 2D game loops, sprite rasterizers, and console tilemap systems (arcade machines, NES/SNES PPU, Game Boy, Flash display lists), rendering thousands of moving objects per frame with separate draw calls causes massive CPU and driver overhead.

`paint-batch-tiles` bridges game simulation state with the universal intermediate representation `paint-instructions` (`P2D00` [`PaintScene`]) and native rendering virtual machines (`PaintVM` `P2D01`):

1. **Hardware-Inspired Batch Primitives:** Flat, `Copy` structs (`SpriteQuad`, `TileBatchCell`) encoding destination bounds, priority layers, mirroring flags, palette indices, and tints.
2. **Deterministic Layer Lowering:** Sorts visible sprites by `(z_priority, id)` and tile cells by row-major `(grid_row, grid_col)`, emitting canonical `PaintInstruction::Rect` nodes with stable IDs (`sprite:<id>`, `tile:<col>:<row>`).
3. **Zero-Allocation Retained-Mode Diffing (`VER07`):** Computes per-sprite delta records (`SpriteDeltaOp`: `Added`, `Modified`, `Removed`, `Unchanged`) between consecutive frames with **zero heap allocation** (`max_alloc_bytes: Some(0)`).
4. **Retained-Mode Scene Graph Diffing:** Diffs two `PaintScene` trees by stable `PaintBase.id` to produce minimal `ScenePatch` records for `PaintVM.patch()`.

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** Declares `"capabilities": []` in `required_capabilities.json`. Contains zero filesystem (`fs`), network (`net`), process (`proc`), environment variable (`env`), system time (`time`), standard I/O (`stdin`/`stdout`), system random (`rand`), or multithreading (`thread`) access.
- **`CAP002` (Zero Unsafe / Safe Rust):** Compiled with `#![forbid(unsafe_code)]`. Prohibits raw pointers, manual memory allocation, and FFI reachability.
- **Zero Allocation Hot Path (`VER07`):** `diff_sprite_batches_into` operates directly into a caller-supplied slice with zero heap allocations, verified by `CountingAllocator` in microbenchmarks.
- **Strict Input Validation (`CWE-20`):** Validates finite positive dimensions, rejects NaN/Infinity coordinates, and rejects duplicate sprite/tile/scene IDs.
- **Resource Exhaustion Defense (`CWE-400`):** Enforces a strict upper bound of `MAX_BATCH_SPRITES = 4096` on all batch and tile operations.
- **26-CWE Weakness Closure (`VER15`):** Complete formal closure or exclusion matrix for all 26 canonical CWE classes.

---

## Core Types & Functions

### Data Structures

- `SpriteQuad`: `Copy` representation of a 2D quad (id, x, y, width, height, z_priority, h_flip, v_flip, color_rgb, visible).
- `TileBatchCell`: `Copy` representation of a tilemap grid cell (grid_col, grid_row, tile_id, palette_id, h_flip, v_flip, color_rgb).
- `DeltaKind`: `Added`, `Modified`, `Removed`, `Unchanged`.
- `SpriteDeltaOp`: `Copy` delta record (kind, id, old_index, new_index).
- `ScenePatch`: Retained-mode delta summary (added_ids, modified_ids, removed_ids, unchanged_count).
- `PaintBatchError`: Exhaustive error enum with descriptive error messages.

### Functions

- `diff_sprite_batches_into(prev: &[SpriteQuad], next: &[SpriteQuad], out_ops: &mut [SpriteDeltaOp]) -> Result<usize, PaintBatchError>`
- `diff_paint_scenes(prev: &PaintScene, next: &PaintScene) -> Result<ScenePatch, PaintBatchError>`
- `lower_sprites_to_paint_scene(width: f64, height: f64, background: &str, sprites: &[SpriteQuad]) -> Result<PaintScene, PaintBatchError>`
- `lower_tile_batch_to_paint_scene(scene_width: f64, scene_height: f64, tile_width: f64, tile_height: f64, background: &str, cells: &[TileBatchCell]) -> Result<PaintScene, PaintBatchError>`
