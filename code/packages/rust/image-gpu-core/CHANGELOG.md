# Changelog — image-gpu-core

## 0.2.0 — 2026-05-04

Major migration: backend swapped from `gpu-runtime` (per-backend hand-written
shaders for Metal / CUDA / CPU) to the **matrix execution layer**
(`matrix-ir` → `matrix-runtime` planner → `matrix-cpu` executor).
**Public API of v0.1 is preserved** — all five existing functions accept
and return the same `PixelContainer`s.

### Migration details

- Each operation now builds a `matrix_ir::Graph` describing its
  computation, runs it through the matrix-execution-layer planner, and
  dispatches via `matrix_cpu::local_transport()`.
- sRGB ↔ linear conversion stays in Rust (the piecewise transfer
  function is awkward to express in MatrixIR's V1 op set; could be
  added in V2 via `Where(Less(...), ...)`).
- v0.1's per-op shader bundles (MSL + CUDA C + Rust fallback) are gone.
- v0.1's dependency on `gpu-runtime`, `metal-compute`, `cuda-compute`
  is removed.  New deps: `matrix-ir`, `compute-ir`, `matrix-runtime`,
  `matrix-cpu`, `executor-protocol`.

### Added

- `gpu_sepia` — classic Microsoft sepia tone (3×3 colour matrix).
- `gpu_contrast(scale)` — adjust contrast around mid-grey 128.
- `gpu_posterize(levels)` — reduce to N distinct values per channel.

These three new ops complete the filter set needed for the upcoming
Instagram-style filter CLI.

### Changed

- `GpuError` simplified.  v0.1 had several variants tied to specific
  GPU backend errors; v0.2 has just `Other(String)` since the matrix
  execution layer's failure surface is much smaller.

### Bug fixes (in matrix-cpu, included in this PR)

- `Op::Const` handler in `matrix-cpu` was a stub that didn't actually
  materialise the constant's bytes into the output tensor's buffer.
  All graphs that used `GraphBuilder::constant()` produced zero-filled
  results.  Now Const correctly copies bytes from
  `graph.constants[i].bytes` into the op's output buffer.

### Tests

20 unit tests + 1 doctest pass.  Numerical results match v0.1 within
±1 LSB for tests that allow tolerance; tests that asserted exact
byte equality (`invert_rgb`, `invert_preserves_alpha`,
`invert_double_is_identity`) still pass exactly.

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
