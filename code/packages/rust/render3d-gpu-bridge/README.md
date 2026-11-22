# render3d-gpu-bridge

Hardware 3D render-bridge uniting `gpu-runtime` (`G09`) compute shaders, backend capability descriptors, and **bit-exact deterministic CPU fallback** to `raster3d-soft` (`GAME33`).

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME44`, Era 4 of [`GAME00`](../../../specs/GAME00-historical-game-engine-roadmap.md)), building on `gpu-runtime` (`G09`), `raster3d-soft` (`GAME33`), `math3d` (`GAME30`), and `pixel-container` (`IMG00`).

---

## Overview

1. **Unified Backend Capability Descriptors:** Models `Software`, `GpuRuntimeCpu`, `GpuRuntimeMetal`, `GpuRuntimeCuda`, `Wgpu`, `Metal`, and `Vulkan` capability records via `BackendCapabilities::backend_caps_for` and `backend_caps_detect_safe` (without implicit driver probes).
2. **Validated Multi-Draw `RenderPass`:** Validates `RenderTargetSpec`, `CameraUniforms` (`math3d::Mat4`), `LightRig`, `MeshSlice` (`raster3d_soft::Vertex` + `&[u32]`), `MaterialBinding`, and `DrawCall` ranges with strict bounds (`MAX_FRAMEBUFFER_WIDTH/HEIGHT <= 4096`, `MAX_DRAW_CALLS = 4096`, `MAX_TOTAL_TRIANGLES = 1_000_000`).
3. **Multi-Draw Atomicity:** Validates the entire `RenderPass` before clearing or mutating `framebuffer`, `depth`, or `scratch` — an error on draw `#1` leaves all caller buffers byte-for-byte untouched.
4. **`gpu-runtime` Compute Shaders & Bit-Exact Parity:** Registers `BRIDGE_POSTPROCESS_SHADERS` and `BRIDGE_DEPTH_RESOLVE_SHADERS` (pure-Rust `cpu` kernels + MSL + CUDA C sources) and guarantees bit-exact 64-bit FNV-1a frame and depth digest parity across `ForceSoftware`, `AutoHardwareWithSoftwareFallback`, and `ForceGpuRuntimeCpu` (`Runtime::cpu_only()`).
5. **Zero-Allocation Steady-State Execution (`VER07`):** `execute_render_pass_into` with caller-owned `RenderBridgeScratch` executes directly onto caller-owned `PixelContainer` + `DepthBufferMut` with `0` heap allocations.

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** `"capabilities": []` in `required_capabilities.json`; default build depends on `gpu-runtime` with `default-features = false` (optional `metal` feature).
- **`CAP002` (Zero Unsafe):** `#![forbid(unsafe_code)]`.
- **Zero Steady-State Heap Allocation (`VER07`):** Verified with `CountingAllocator` in `tests/microbenchmarks.rs`.
- **Sandbox Safety (`VER08b` / `RCG001`):** Never calls `gpu_runtime::Runtime::detect()` or `Runtime::global()` implicitly; uses `Runtime::cpu_only()` in capability conformance tests.

---

## Usage

```rust
use math3d::{Mat4, Vec3};
use pixel_container::PixelContainer;
use render3d_gpu_bridge::{
    execute_render_pass_into, BackendPreference, CameraUniforms, DepthBufferMut, DrawCall,
    LightRig, MaterialBinding, MeshSlice, RenderBridgeScratch, RenderPass, RenderTargetSpec,
    ShadingMode, Vertex,
};

let verts = [
    Vertex::vertex_new(Vec3::new(-0.5, -0.5, 0.0), Vec3::unit_z(), [1.0, 0.0, 0.0], 0.0, 0.0),
    Vertex::vertex_new(Vec3::new(0.5, -0.5, 0.0), Vec3::unit_z(), [0.0, 1.0, 0.0], 1.0, 0.0),
    Vertex::vertex_new(Vec3::new(0.0, 0.5, 0.0), Vec3::unit_z(), [0.0, 0.0, 1.0], 0.5, 1.0),
];
let indices = [0u32, 1, 2];
let meshes = [MeshSlice::mesh_slice_new(&verts, &indices)?];
let materials = [MaterialBinding::material_new(ShadingMode::Gouraud)];
let draw_calls = [DrawCall::draw_call_new(0, 0, Mat4::mat4_identity(), 0, 3)?];

let pass = RenderPass {
    target: RenderTargetSpec::target_spec_new(64, 64, [0, 0, 0, 255], 1.0)?,
    camera: CameraUniforms::camera_from_combined(Mat4::mat4_identity(), Vec3::zero())?,
    light: LightRig::light_rig_default(),
    meshes: &meshes,
    materials: &materials,
    draw_calls: &draw_calls,
};

let mut fb = PixelContainer::new(64, 64);
let mut zb = vec![1.0_f64; 64 * 64];
let mut scratch = RenderBridgeScratch::bridge_scratch_with_capacity(verts.len(), 64, 64)?;
let report = execute_render_pass_into(
    &pass,
    BackendPreference::AutoHardwareWithSoftwareFallback,
    &mut fb,
    DepthBufferMut::F64(&mut zb),
    &mut scratch,
)?;
assert_ne!(report.frame_digest, 0);
# Ok::<(), render3d_gpu_bridge::BridgeError>(())
```

---

## Testing

```
cargo test -p render3d-gpu-bridge -- --nocapture
cargo test -p render3d-gpu-bridge --release -- --nocapture
cargo clippy -p render3d-gpu-bridge --all-targets -- -D warnings
```
