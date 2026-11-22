# terrain-lod

Deterministic, zero-allocation heightfield synthesis, runway placement, and restricted quadtree terrain Level-of-Detail (LOD) mesh emission for Rust (`GAME42`).

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME00` §2 Era 4 & §8), serving 6-DOF flight simulation (`aero-flight-model`, `render3d-gpu-bridge`, `game-flight-sim`).

---

## Overview

1. **Caller-Owned `(2^n + 1)^2` Heightfields (`Heightfield`, `HeightfieldMut`):**
   Zero hidden allocations; grids borrow caller-provided `&[f64]` / `&mut [f64]` buffers (`1 <= level <= MAX_LEVELS`, `3 <= side <= 1025`).
2. **Deterministic Terrain Synthesis (`rng::Pcg32`):**
   - `synthesize_diamond_square`: classic midpoint displacement in a fixed row-major traversal.
   - `synthesize_fbm`: multi-octave value-noise fractional Brownian motion using a seeded 256-entry lattice table and pure polynomial smoothstep interpolation (zero `std` `libm` transcendental calls).
3. **Bilinear Queries & Central-Difference Normals:**
   `height_at(x, z)`, `normal_at(x, z)`, `vertex_position(ix, iz)`, `vertex_normal(ix, iz)`, and `emit_vertices_into`.
4. **Rotated Runway Placement (`place_runway`):**
   Flattens a rotated rectangle inside the validated grid domain to target elevation `h` with linear shoulder blending, atomically refusing out-of-bounds or degenerate runways (`OutOfBounds` / `InvalidRunway`).
5. **Restricted Quadtree LOD & Watertight Stitching (`refine_into`, `emit_mesh_into`):**
   - Top-down BFS refinement driven by per-node object-space error divided by AABB-to-camera distance, with optional `math3d::Frustum` culling.
   - Iterative neighbor-level restriction (`|depth_a - depth_b| <= 1`) and center-fan perimeter stitching so adjacent LOD levels share every interior edge with zero T-junction cracks.
   - Reuses caller-owned `TerrainScratch` and writes `u32` triangle indices into a caller-provided slice, atomically returning `Err(CapacityExceeded)` rather than allocating.
   - `mesh_digest_fnv1a` provides deterministic 64-bit FNV-1a verification digests.

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** `"capabilities": []` in `required_capabilities.json`.
- **`CAP002` (Zero Unsafe):** `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`.
- **Zero-Allocation Hot Path (`VER07`):** `height_at`, `normal_at`, `place_runway`, `refine_into`, `emit_mesh_into`, and `mesh_digest_fnv1a` perform zero heap allocations (`CountingAllocator` verified in `tests/microbenchmarks.rs`).
- **CWE Hardening (`VER15`):**
  - `CWE-20`: non-finite `NaN`/`+/-Inf` rejection and scale-relative thresholds (`cell_size * 1e-3`, `cell_size * 1e-4`).
  - `CWE-190`: `checked_mul` grid sizing and `u32` index limits (`MAX_LEVELS = 10`).
  - `CWE-125` / `CWE-787`: checked grid, index, and atomic output capacity guards.
  - `CWE-400` / `CWE-835`: explicit caps (`MAX_OCTAVES = 12`, `MAX_QUADTREE_NODES = 65536`, `MAX_OUTPUT_TRIANGLES = 262144`, `MAX_RESTRICT_PASSES = 12`).

---

## Usage

```rust
use math3d::Vec3;
use terrain_lod::{
    emit_mesh_into, mesh_digest_fnv1a, refine_into, Heightfield, HeightfieldMut,
    HeightfieldSpec, LodParams, RunwaySpec, TerrainScratch,
};

let spec = HeightfieldSpec::spec_new(5, 10.0, -160.0, -160.0)?;
let mut heights = vec![0.0; spec.spec_vertex_count()?];

{
    let mut hfm = HeightfieldMut::from_heights_mut(spec, &mut heights)?;
    hfm.synthesize_diamond_square(0x4242_1337, 100.0, 45.0, 0.55)?;
    let runway = RunwaySpec::runway_new(0.0, 0.0, 0.4, 120.0, 30.0, 20.0, 102.0)?;
    hfm.place_runway(&runway)?;
}

let hf = Heightfield::from_heights(spec, &heights)?;
let mut scratch = TerrainScratch::scratch_with_capacity(4096)?;
let params = LodParams::lod_params_new(
    Vec3::new(0.0, 80.0, 0.0),
    0.02,
    1.0,
    5,
    4096,
    65_536,
    None,
)?;
let stats = refine_into(&hf, &params, &mut scratch)?;
let mut indices = vec![0u32; stats.index_count];
let written = emit_mesh_into(&hf, &scratch, &mut indices, 65_536)?;
let digest = mesh_digest_fnv1a(&hf, &indices[..written])?;
assert!(digest != 0);
# Ok::<(), terrain_lod::TerrainError>(())
```

---

## Testing

```
cargo test -p terrain-lod -- --nocapture
cargo test -p terrain-lod --release -- --nocapture
cargo clippy -p terrain-lod --all-targets -- -D warnings
```
