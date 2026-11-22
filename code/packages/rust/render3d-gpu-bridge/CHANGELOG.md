# Changelog

All notable changes to `render3d-gpu-bridge` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-21

### Added
- Initial implementation of `GAME44: render3d-gpu-bridge` hardware 3D render-bridge over `gpu-runtime` (`G09`) with bit-exact deterministic CPU fallback to `raster3d-soft` (`GAME33`).
- Backend capability descriptors (`Software`, `GpuRuntimeCpu`, `GpuRuntimeMetal`, `GpuRuntimeCuda`, `Wgpu`, `Metal`, `Vulkan`), `BackendPreference` selection policy (`AutoHardwareWithSoftwareFallback`, `ForceSoftware`, `ForceGpuRuntimeCpu`, `RequireHardware`), and `RenderExecutionReport` with canonical FNV-1a frame and depth digests.
- Validated multi-draw `RenderPass`, `RenderTargetSpec`, `CameraUniforms`, `MeshSlice`, `MaterialBinding`, and `DrawCall` types with strict CWE-20/125/190/369/400 bounds and upfront multi-draw atomicity.
- Registered `BRIDGE_POSTPROCESS_SHADERS` and `BRIDGE_DEPTH_RESOLVE_SHADERS` (`gpu_runtime::Shaders` bundles with pure-Rust CPU kernels, MSL, and CUDA C sources) and `resolve_depth_to_rgba_with_runtime`.
- Zero-allocation steady-state `execute_render_pass_into` using caller-owned `RenderBridgeScratch` (`VER07` `max_alloc_bytes = 0`).
- Conformance suite (`tests/conformance.rs`), hostile-input atomic rollback & 2,000-case fuzz suite (`tests/hostile_inputs.rs`), zero-alloc microbenchmarks (`tests/microbenchmarks.rs`), runtime capability gate (`tests/runtime_capability.rs`), and formal verification artifacts (`code/specs/render3d-gpu-bridge.warrant`, `assurance-profile.json`, `assurance_report.json`, `effect_certificate.json`).
