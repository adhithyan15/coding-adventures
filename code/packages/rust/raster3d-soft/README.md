# raster3d-soft

Deterministic, zero-allocation (steady-state) software 3D triangle rasterizer for Rust.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME33`, Era 3 of [`GAME00`](../../../specs/GAME00-historical-game-engine-roadmap.md)), building on `math3d` (`GAME30`) and `pixel-container` (`IMG00`).

---

## Overview

1. **Full Software 3D Pipeline:** MVP (`Mat4`) vertex transformation, 3x3 normal transformation, homogeneous 4D Sutherland-Hodgman frustum and near-plane (`w >= MIN_CLIP_W`) clipping, perspective divide, viewport mapping, and backface/frontface culling (`CounterClockwise` / `Clockwise`).
2. **Perspective-Correct Half-Space Rasterization:** Top-left tie-breaking edge-function triangle rasterizer with perspective-correct `1/w` barycentric interpolation of depth, RGB color, U/V texture coordinates, and vertex normals.
3. **Shading & Texturing Modes:** `Flat`, `Gouraud`, `Phong`, `TexturedNearest`, and `TexturedBilinear` (`Repeat` / `Clamp` wrapping), driven by `LightRig` (ambient + directional lighting) and `f32`/`f64` Z-buffer depth testing.
4. **Zero-Allocation Steady-State Rendering (`VER07`):** `draw_triangle_into` uses stack-only clipping buffers and `draw_indexed_mesh_into` reuses a pre-allocated `RasterScratch` buffer to write directly into caller-provided `PixelContainer` + Z-buffer slices.
5. **Canonical Digest:** `frame_digest_fnv1a` computes a deterministic 64-bit FNV-1a digest over framebuffer dimensions and RGBA8 pixels.

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** `"capabilities": []` in `required_capabilities.json`.
- **`CAP002` (Zero Unsafe):** `#![forbid(unsafe_code)]`.
- **Zero Steady-State Heap Allocation (`VER07`):** Verified with `CountingAllocator` in `tests/microbenchmarks.rs`.
- **Security Guards:** Finite input validation (`CWE-20`), bounded dimensions/mesh limits (`CWE-400`), near-zero `w` / `1/w` / screen-area guards (`CWE-369`), `f64` bounding-box clamping prior to integer conversion (`CWE-190`), and checked vertex-index, texture, and framebuffer indexing (`CWE-125`/`CWE-787`).

---

## Usage

```rust
use math3d::{Mat4, Vec3};
use pixel_container::PixelContainer;
use raster3d_soft::{
    clear_depth_f64, clear_framebuffer, draw_indexed_mesh_into, frame_digest_fnv1a,
    DepthBufferMut, RasterConfig, RasterScratch, Vertex,
};

let mut fb = PixelContainer::new(128, 128);
let mut zb = vec![1.0_f64; 128 * 128];
clear_framebuffer(&mut fb, [0, 0, 0, 255])?;
clear_depth_f64(&mut zb, 1.0)?;

let view = Mat4::look_at(Vec3::new(0.0, 0.0, 3.0), Vec3::zero(), Vec3::unit_y())?;
let proj = Mat4::perspective(std::f64::consts::FRAC_PI_3, 1.0, 0.1, 100.0)?;
let cfg = RasterConfig::raster_config_new(proj.mul(&view)?)?;

let verts = [
    Vertex::vertex_new(Vec3::new(-0.5, -0.5, 0.0), Vec3::unit_z(), [1.0, 0.0, 0.0], 0.0, 0.0),
    Vertex::vertex_new(Vec3::new(0.5, -0.5, 0.0), Vec3::unit_z(), [0.0, 1.0, 0.0], 1.0, 0.0),
    Vertex::vertex_new(Vec3::new(0.0, 0.5, 0.0), Vec3::unit_z(), [0.0, 0.0, 1.0], 0.5, 1.0),
];
let mut scratch = RasterScratch::scratch_with_capacity(verts.len())?;
draw_indexed_mesh_into(
    &mut fb,
    DepthBufferMut::F64(&mut zb),
    &cfg,
    None,
    &verts,
    &[0, 1, 2],
    &mut scratch,
)?;
let _digest = frame_digest_fnv1a(&fb);
# Ok::<(), raster3d_soft::RasterError>(())
```

---

## Testing

```
cargo test -p raster3d-soft -- --nocapture
cargo test -p raster3d-soft --release -- --nocapture
cargo clippy -p raster3d-soft --all-targets -- -D warnings
```
