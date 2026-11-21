# bsp-portal25d

Deterministic, zero-allocation (steady-state) 2D sector Binary Space Partitioning (BSP) tree builder, front-to-back portal traversal with 1D column occlusion clipping, dynamic visibility lists, and 2.5D Doom/Duke3D-style sector renderer for Rust.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME32`), building atop `math3d` (`GAME30`), `trig` (`PHY00`), and `pixel-container` (`IMG00`).

---

## Overview

1. **Iterative 2D Sector BSP Builder (`BspTree::bsp_build`):** Partitions arbitrary 2D sector linedefs into convex subsectors using an explicit work stack (`CWE-674` recursion-free), splitting straddling segments while preserving texture `u` ranges and bounding boxes.
2. **Front-to-Back Portal Traversal & 1D Column Occlusion (`render_frame_into`):** Traverses the BSP tree near-to-far using an iterative stack in `RenderScratch`, maintaining per-column `[y_top[x], y_bottom[x]]` open spans. Solid 1-sided walls close columns immediately; 2-sided portals render upper/lower step walls and narrow the portal window for deeper sectors, exiting early once all columns close.
3. **2.5D Sector & Texture Pipeline:** Supports per-sector variable floor/ceiling heights, sector light levels (`0..=255`), solid or tiled RGBA8 textures (`TextureRgba`), and dynamic visibility lists (`scratch_visible_sectors`, `scratch_visible_subsectors`).
4. **`math3d` Interop & Deterministic Digests:** Converts seamlessly between `Vec2`/`Aabb2` and `math3d::Vec3`/`Aabb3`/`Ray3`/`Mat4`, and provides canonical `frame_digest_fnv1a` and `scene_digest_fnv1a` digests.

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** `"capabilities": []` in `required_capabilities.json`.
- **`CAP002` (Zero Unsafe):** `#![forbid(unsafe_code)]`.
- **Zero-Allocation Hot Path (`VER07`):** Verified with `CountingAllocator` in `tests/microbenchmarks.rs`.
- **Security & Robustness Controls:**
  - `CWE-20`: Rejects `NaN`/`+/-Inf` and out-of-range coordinates/heights/FOV (`MAX_COORD = 1e6`).
  - `CWE-369`: Guards zero-length linedefs, degenerate camera bases, near-zero projection denominators, and clamps `perp_dist >= MIN_PERP_DIST`.
  - `CWE-400` / `CWE-674`: Caps sectors (`MAX_SECTORS = 4096`), linedefs (`MAX_LINEDEFS = 16384`), BSP depth (`MAX_BSP_DEPTH = 64`), nodes/segs (`65536`), framebuffer (`4096x4096`), and textures (`2048x2048`), using iterative stacks instead of recursion.
  - `CWE-125` / `CWE-787` / `CWE-190`: Culls offscreen spans in `f64`, clamps screen-Y bounds before integer conversion, validates all indices upfront, and uses checked indexing throughout.
- **26-CWE Weakness Closure (`VER15`):** `assurance_report.json` records the closure/exclusion matrix.

---

## Usage

```rust
use bsp_portal25d::{
    render_frame_into, BspTree, Camera25D, Linedef, Map25D, RenderConfig, RenderScratch, Sector,
    SurfaceStyle, Vec2,
};
use pixel_container::PixelContainer;

let s0 = Sector::sector_new(0.0, 4.0, [40, 80, 40, 255], [40, 40, 90, 255], 255)?;
let w = SurfaceStyle::surface_solid([180, 60, 60, 255]);
let linedefs = vec![
    Linedef::linedef_solid(Vec2::vec2_new(0.0, 0.0), Vec2::vec2_new(0.0, 10.0), 0, w)?,
    Linedef::linedef_solid(Vec2::vec2_new(0.0, 10.0), Vec2::vec2_new(10.0, 10.0), 0, w)?,
    Linedef::linedef_solid(Vec2::vec2_new(10.0, 10.0), Vec2::vec2_new(10.0, 0.0), 0, w)?,
    Linedef::linedef_solid(Vec2::vec2_new(10.0, 0.0), Vec2::vec2_new(0.0, 0.0), 0, w)?,
];
let map = Map25D::map_new(vec![s0], linedefs)?;
let bsp = BspTree::bsp_build(&map)?;
let cam = Camera25D::camera_from_yaw_fov(5.0, 5.0, 2.0, 0.0, trig::PI / 3.0)?;

let mut fb = PixelContainer::new(160, 120);
let mut scratch = RenderScratch::scratch_for_frame_and_map(&fb, &map, &bsp)?;
let stats = render_frame_into(
    &map,
    &bsp,
    &cam,
    &[],
    &RenderConfig::render_config_default(),
    &mut scratch,
    &mut fb,
)?;
assert_eq!(stats.columns_drawn, 160);
# Ok::<(), bsp_portal25d::BspError>(())
```

---

## Testing

```
cargo test -p bsp-portal25d -- --nocapture
cargo test -p bsp-portal25d --release -- --nocapture
cargo clippy -p bsp-portal25d --all-targets -- -D warnings
```
