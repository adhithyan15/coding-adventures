# sprite-oam

Deterministic, zero-allocation 128-sprite hardware OAM (Object Attribute Memory) manager, priority sorter, scanline sprite-limit evaluator and metasprite animation sequencer for Rust.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME22`).

---

## Overview

NES, Game Boy and GBA style consoles do not keep a scene graph: they keep a small fixed table of sprite attribute slots that the video hardware walks every scanline. `sprite-oam` models exactly that table, so game logic written against it behaves like the hardware: the same draw order, the same per-line sprite limits, the same affine sampling.

```rust
use sprite_oam::{OamEntry, OamTable, SpriteShape, SpriteSize};

let mut oam = OamTable::blank_table();
oam.set_entry(0, OamEntry {
    x: 24, y: 40, shape: SpriteShape::Square, size: SpriteSize::Size1,
    priority: 1, tile: 12, hidden: false, ..OamEntry::blank_entry()
}).unwrap();

let mut order = [0u8; 128];
let n = oam.draw_order_into(&mut order).unwrap(); // back-to-front slots
assert_eq!(&order[..n], &[0]);
```

---

## Features

1. **`OamTable` / `OamEntry`:** 128 slots with x/y, `SpriteShape` x `SpriteSize` (12 hardware sizes from 8x8 to 64x64), H/V flip, palette bank (0..=15), priority (0..=3), affine slot (0..32), 10-bit tile index and hidden / affine / double-size flags.
2. **Affine groups:** 32 `AffineParams` (8.8 fixed point `PA/PB/PC/PD`) convertible to and from `affine2d::Affine2D` (`G2D01`) with range-checked quantization. `texel_at` samples a sprite texture through flips or its matrix, including double-size boxes.
3. **Priority sorting:** `draw_order_into` yields visible slots back-to-front by priority, then OAM index (lower slot in front on ties), via a four-bucket pass with no allocation.
4. **Scanline limits:** `scanline_sprites_into` evaluates one line in ascending OAM order with an NES/GBA style per-line `limit`, reporting `dropped` sprites; `scanline_stats` summarises overflow over a whole frame.
5. **Metasprites and animation:** `MetaFrame` groups up to 16 hardware sprites with a duration; `Animation` binds up to 64 frames to `PlayMode::{OneShot, Loop, PingPong}`; the `Copy` `Animator` ticks deterministically (advancing N ticks equals N single ticks) and `OamTable::stamp_frame` writes a frame into OAM with optional H/V mirroring.
6. **Lowering:** `lower_to_quads_into` writes `paint-batch-tiles` (`P2D09`) `SpriteQuad`s without allocating, resolving palette banks through a `palette-indexed` (`GAME20`) `Palette`; `lower_to_paint_scene` produces a `paint-instructions` (`P2D00`) scene. Quad ids and z-priorities are chosen so the batch painter reproduces hardware order (`quad_id_for_slot`, `slot_from_quad_id`).

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** `"capabilities": []` in `required_capabilities.json`.
- **`CAP002` (Zero Unsafe):** `#![forbid(unsafe_code)]`.
- **Zero Allocation Hot Path (`VER07`):** sort, scanline evaluation, texel sampling, animation ticks, stamping and quad lowering use caller buffers or fixed storage; verified with the counting allocator in `tests/microbenchmarks.rs`. Only `lower_to_paint_scene` allocates (its result is heap-backed).
- **Strict Validation:** CWE-20 (slot, affine slot, priority, bank, tile, coordinate and flag checks), CWE-125/787 (checked slot ranges, atomic `stamp_frame`, buffer-length checks before any write), CWE-190 (`i32`/`i64` math, checked adds, `AffineNotRepresentable`), CWE-369 (no caller-controlled divisors), CWE-400 (`OAM_SLOTS = 128`, `MAX_PARTS = 16`, `MAX_FRAMES = 64`, `MAX_SCREEN_HEIGHT = 4096`, `MAX_TICK_DELTA = 65535`).
- **26-CWE Weakness Closure (`VER15`):** `assurance_report.json` records the closure/exclusion matrix.

---

## Testing

```
cargo test -p sprite-oam -- --nocapture
cargo test -p sprite-oam --release -- --nocapture
```
