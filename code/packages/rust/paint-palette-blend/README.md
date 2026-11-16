# paint-palette-blend

Pure, deterministic, zero-allocation indexed-color scanline blits, hardware blend modes, and retained-mode scene diffing for Rust.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`P2D10`).

---

## Overview

Console and arcade video hardware (NES/SNES PPU, Game Boy, Neo Geo, Sega System 16) never stored full-color pixels: tiles held small palette *indices* and the video chip resolved them through a palette while compositing scanlines with fixed-function blend logic. `paint-palette-blend` reproduces that pipeline for `PaintVM` (`P2D01`):

1. **Indexed scanline and image blits** — expand 8-bit palette indices (palette of at most 256 RGBA colors) into an RGBA8 destination with color-key transparency, horizontal/vertical flipping, edge clipping, and explicit row strides. Works on raw buffers or directly on a `PixelContainer` (`IMG00`).
2. **Hardware blend modes** — `Normal`, `Add`, `Multiply`, `Screen`, `Alpha`, in exact integer arithmetic (rounded `/255`), bit-identical on every platform.
3. **Zero-allocation scene diffing** — `diff_paint_scene_into` compares two `PaintScene` graphs by stable `PaintBase.id` and writes `PatchOp` records (`Added`, `Modified`, `Removed`) into a caller-supplied slice for `PaintVM.patch()`.

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** `required_capabilities.json` declares `"capabilities": []`. No filesystem, network, process, environment, time, stdio, random, or thread access.
- **`CAP002` (Safe Rust):** `#![forbid(unsafe_code)]`.
- **Zero Allocation Hot Path (`VER07`):** blits, blends, and diffs perform no heap allocation, verified with `CountingAllocator` in `tests/microbenchmarks.rs` (`max_alloc_bytes: Some(0)`).
- **No Partial Writes (`CWE-787`):** every palette index in the visible region and every buffer length is validated *before* the first destination byte is written; a rejected call leaves the destination untouched.
- **Strict Input Validation (`CWE-20`, `CWE-125`, `CWE-369`):** dimensions must be `1..=MAX_DIMENSION` (8192), strides must cover a row, palettes must be `1..=256` entries, alpha conversion rejects NaN/Infinity/out-of-range, blend codes are checked.
- **Integer Overflow Defense (`CWE-190`):** all offset/length arithmetic uses `checked_*` or widened `i64` clipping math; `i32::MIN`/`i32::MAX` blit origins and `usize::MAX` strides are rejected or clipped safely.
- **Resource Exhaustion Defense (`CWE-400`):** scenes are capped at `MAX_SCENE_INSTRUCTIONS = 4096`.
- **26-CWE Weakness Closure (`VER15`):** complete closure or exclusion matrix in `assurance_report.json`.

---

## Blend Semantics

With effective alpha `ea = src.a * alpha / 255`:

| Mode | Result per channel |
|------|--------------------|
| `Normal` | `src` (alpha parameter ignored) |
| `Add` | `lerp(dst, min(255, src + dst), ea)` |
| `Multiply` | `lerp(dst, src * dst / 255, ea)` |
| `Screen` | `lerp(dst, 255 - (255-src)(255-dst)/255, ea)` |
| `Alpha` | `lerp(dst, src, ea)` (source-over, exact for opaque destinations) |

`ea == 0` returns `dst` unchanged. Result alpha is `ea + dst.a * (255 - ea) / 255`.

---

## Core Types & Functions

### Data Structures

- `Rgba8` (`[u8; 4]`), `BlendMode`, `BlitParams`, `OPAQUE_BLIT`
- `IndexedImage` / `RgbaTarget`: strided views over index and RGBA8 buffers
- `PatchKind`, `PatchOp`, `PatchSummary`
- `PaletteBlendError`: exhaustive error enum

### Functions

- `blend_pixel(mode, src, dst, alpha) -> Rgba8`
- `blit_indexed_scanline(dst, indices, palette, params) -> Result<usize, PaletteBlendError>`
- `blit_indexed_image(target, x, y, src, palette, params) -> Result<usize, PaletteBlendError>`
- `blit_indexed_to_pixel_container(container, x, y, src, palette, params)`
- `blend_solid_span` / `blend_rgba_scanline`
- `blend_mode_from_code`, `blend_mode_from_paint`, `alpha_from_unit_f32`
- `diff_paint_scene_into(prev, next, out_ops) -> Result<PatchSummary, PaletteBlendError>`
- `instruction_id(instr) -> Option<&str>`
