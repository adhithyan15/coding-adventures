# palette-indexed

Deterministic, zero-allocation indexed-colour pixel formats, palette fading and framebuffer blitting for Rust.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME20`).

---

## Overview

Console and arcade hardware (NES, Game Boy, GBA, VGA mode 13h) never stored RGB per pixel: they stored a small *palette index* per pixel and looked colours up at scan-out time. `palette-indexed` models that pipeline and bridges it to the universal RGBA8 [`PixelContainer`](../pixel-container) (`IMG00`):

1. **Packed formats:** `BitDepth::Bpp2` (4 colours), `Bpp4` (16), `Bpp8` (256), packed chunky MSB-first (leftmost pixel in the high bits), rows byte-aligned with an arbitrary `stride`.
2. **Palette storage:** `Palette` is a fixed 1 KiB `Copy` value (no heap) with bounds-checked entry access.
3. **Quantized fading:** `fade_color`, `fade_palette_into` (fade-to-colour) and `crossfade_palette_into` interpolate in integer arithmetic and snap to console channel precision (`ChannelBits::Bits5` for GBA/SNES, `Bits3` for Genesis, ...) with bit replication so full intensity stays 255.
4. **Colour-0 transparency:** `BlitOptions { color0_transparent, h_flip, v_flip }`; index 0 pixels leave the destination untouched.
5. **Blitting:** `blit_indexed_to_container` (whole image), `blit_scanline_to_container` (one scanline, enabling per-scanline palette/raster effects), `expand_row_to_rgba_into` (line buffer), and `IndexedCanvas::blit_indexed` (indexed to indexed sprite compositing). All clip against the destination.

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** `"capabilities": []` in `required_capabilities.json`.
- **`CAP002` (Zero Unsafe):** `#![forbid(unsafe_code)]`.
- **Zero Allocation Hot Path (`VER07`):** pack/unpack, fading and every blit write into caller-supplied buffers; verified with the counting allocator in `tests/microbenchmarks.rs`.
- **Strict Validation:** CWE-20 (indices vs bit depth, palette size vs image depth), CWE-125/787 (stride and buffer lengths checked before access, tampered `PixelContainer` lengths rejected), CWE-190 (checked size arithmetic, `i64` clipping), CWE-369 (`steps == 0` rejected), CWE-400 (`MAX_DIMENSION = 16384`, `MAX_FADE_STEPS = 1024`).
- **26-CWE Weakness Closure (`VER15`):** `assurance_report.json` records the closure/exclusion matrix.

---

## Usage

```rust
use palette_indexed::*;

let palette = Palette::from_colors(
    BitDepth::Bpp2,
    &[[0, 0, 0, 255], [255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]],
)?;

// 4x1 sprite: indices 0 1 2 3 packed into a single byte.
let mut packed = [0u8; 1];
pack_row_into(BitDepth::Bpp2, &[0, 1, 2, 3], &mut packed)?;
let sprite = IndexedImage::view(BitDepth::Bpp2, 4, 1, 1, &packed)?;

let mut screen = PixelContainer::new(16, 16);
blit_indexed_to_container(&sprite, &palette, BLIT_SPRITE, &mut screen, 4, 4)?;

// GBA-style fade to black, step 8 of 16.
let mut faded = Palette::blank(BitDepth::Bpp2);
fade_palette_into(&palette, &mut faded, [0, 0, 0, 255], 8, 16, ChannelBits::Bits5)?;
# Ok::<(), PaletteError>(())
```

## Core Types & Functions

- `BitDepth`, `PaletteError`, `Rgba8`, `Palette`, `ChannelBits`, `BlitOptions` (`BLIT_OPAQUE`, `BLIT_SPRITE`)
- `IndexedImage<'a>` (read-only view), `IndexedCanvas<'a>` (mutable framebuffer)
- `packed_row_len`, `required_buffer_len`, `pack_row_into`, `unpack_row_into`, `get_index`, `set_index`
- `quantize_channel`, `quantize_color`, `fade_color`, `fade_palette_into`, `crossfade_palette_into`
- `expand_row_to_rgba_into`, `blit_scanline_to_container`, `blit_indexed_to_container`

## Testing

```
cargo test -p palette-indexed -- --nocapture
cargo test -p palette-indexed --release -- --nocapture
```
