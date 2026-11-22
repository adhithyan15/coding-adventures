# raycast-dda

Deterministic, zero-allocation (steady-state) 2.5D orthogonal grid DDA raycaster, perspective floor/ceiling plane caster, and Z-buffered billboard sprite renderer for Rust.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME31`), building atop `math3d` (`GAME30`), `trig` (`PHY00`), and `pixel-container` (`IMG00`).

---

## Overview

1. **2D Orthogonal Grid DDA (`cast_dda_ray`):** Fisheye-free perpendicular distance projection, `EastWest` vs `NorthSouth` wall side detection and shading, and horizontal wall texture `U/V` sampling.
2. **Floor & Ceiling Plane Casting (`PlaneSurface`):** Supports both solid fills and perspective-correct scanline textured floor and ceiling planes.
3. **Z-Buffered Depth-Sorted Billboard Sprites (`Sprite`, `SpriteScratch`):** Column Z-buffer occlusion against walls, deterministic back-to-front in-place sorting via `slice::sort_unstable_by`, and alpha-tested/blended rendering into caller-provided `PixelContainer` + `&mut [f64]` Z-buffer slices.
4. **Zero-Allocation Steady-State Frame Rendering:** `render_frame_into` performs zero heap allocations when supplied with pre-sized `PixelContainer`, Z-buffer slice, and `SpriteScratch`.

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** `"capabilities": []` in `required_capabilities.json`.
- **`CAP002` (Zero Unsafe):** `#![forbid(unsafe_code)]`.
- **Zero-Allocation Hot Path (`VER07`):** Verified with `CountingAllocator` in `tests/microbenchmarks.rs`.
- **Security & Robustness Controls:**
  - `CWE-20`: Rejects `NaN`/`+/-Inf` and out-of-range coordinates (`MAX_COORD = 1e6`).
  - `CWE-369`: Guards near-zero ray components, degenerate camera bases, and clamps `perp_wall_dist >= MIN_PERP_DIST`.
  - `CWE-400` / `CWE-835`: Caps map dimensions (`MAX_MAP_DIM = 1024`), framebuffer (`MAX_FRAME_WIDTH/HEIGHT = 4096`), textures (`MAX_TEXTURE_DIM = 2048`), sprite counts (`MAX_SPRITES = 4096`), and caps DDA traversal at `width + height` steps per ray so open maps always terminate.
  - `CWE-125` / `CWE-787` / `CWE-190`: Validates buffer lengths upfront with checked arithmetic and performs checked indexing on all grid, texture, Z-buffer, scratch, and framebuffer accesses.
- **26-CWE Weakness Closure (`VER15`):** `assurance_report.json` records the closure/exclusion matrix.

---

## Usage

```rust
use pixel_container::PixelContainer;
use raycast_dda::{
    render_frame_into, Camera2D, GridMap, RenderConfig, Sprite, SpriteScratch, TextureRgba,
};

let cells = vec![
    1, 1, 1, 1,
    1, 0, 0, 1,
    1, 0, 0, 1,
    1, 1, 1, 1,
];
let grid = GridMap::grid_new(4, 4, cells)?;
let cam = Camera2D::camera_from_yaw_fov(1.5, 1.5, 0.0, trig::PI / 3.0)?;
let textures = [TextureRgba::texture_checkerboard(
    16, 16, 4, [200, 80, 60, 255], [140, 50, 40, 255],
)?];
let sprites = [Sprite::sprite_new(2.5, 2.0, 0)?];
let config = RenderConfig::render_config_default();

let mut scratch = SpriteScratch::scratch_with_capacity(8)?;
let mut zbuffer = vec![0.0; 160];
let mut framebuffer = PixelContainer::new(160, 120);

let stats = render_frame_into(
    &grid, &cam, &textures, &sprites, &config, &mut scratch, &mut zbuffer, &mut framebuffer,
)?;
assert_eq!(stats.columns_cast, 160);
# Ok::<(), raycast_dda::RaycastError>(())
```

---

## Testing

```
cargo test -p raycast-dda -- --nocapture
cargo test -p raycast-dda --release -- --nocapture
cargo clippy -p raycast-dda --all-targets -- -D warnings
```
