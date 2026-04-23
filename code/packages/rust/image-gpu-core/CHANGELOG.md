# Changelog — image-gpu-core

## 0.1.0 — 2026-04-23

Initial release.

### Added

- `gpu_invert` — invert RGB channels; alpha unchanged.  Direct sRGB u8
  operation (no colorspace conversion needed).
- `gpu_colour_matrix` — apply a 3×3 colour matrix in linear light.  Uniforms:
  9 × `f32` in row-major order (36 bytes).
- `gpu_greyscale` — convert to greyscale using specified `LuminanceWeights`
  (Rec.709, BT.601, or Average).  Uniforms: 3 × `f32` (12 bytes).
- `gpu_gamma` — power-law gamma in linear light.  Uniforms: 1 × `f32` (4 bytes).
- `gpu_brightness` — additive brightness shift in sRGB u8, clamped to
  \[0, 255\].  Uniforms: 1 × `i32` (4 bytes).
- `LuminanceWeights` — enum for greyscale luminance weight sets.
- MSL compute shaders: `shaders/metal/{invert,colour_matrix,greyscale,gamma,brightness}.metal`
- CUDA C kernels: `shaders/cuda/{invert,colour_matrix,greyscale,gamma,brightness}.cu`
- CPU fallback Rust functions (CPU path, identical logic to GPU shaders).
- Thread dispatch model: one GPU thread per RGBA pixel via
  `Runtime::run_pixels()`.
- sRGB encode/decode implemented identically in Rust, MSL, and CUDA C to
  within ±1 LSB rounding.
- Feature flag `metal` (default: on): propagates to `gpu-runtime/metal`.
- Unit tests use `Runtime::cpu_only()` — no GPU required; pass on any
  platform.  GPU tests can be run with `-- --ignored` on a real GPU machine.
- 16 unit tests covering all operations: edge cases (clamping, identity,
  double-invert), colorspace round-trips, uniform-encoding correctness.
