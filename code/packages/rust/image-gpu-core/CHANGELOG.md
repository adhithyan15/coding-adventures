# Changelog — image-gpu-core

## 0.5.0 — 2026-05-05

### Added — MX05 Phase 4 visibility (CpuSpecialiser wired)

- The process-wide `SpecRouter` now uses **`matrix_cpu::specialiser()`**
  instead of `NoopSpecialiser`, hooking up the first real backend
  `Specialiser` (landed in `matrix-cpu` v0.3.0).
- A small custom `HotPolicy` replaces `DefaultPolicy` while the
  per-tensor sampling pipeline matures: it fires the Specialiser
  on raw invocation count alone (threshold 100), without requiring
  the constant-input or narrow-range observations that
  `DefaultPolicy` checks.  This is enough to demonstrate the cache
  rising above zero in CLI demos and tests.
- `HOTNESS_THRESHOLD` is `100` — much lower than spec MX05's 1000
  default — because Phase 4's specialisation is still
  observation-only (the dispatch path doesn't yet consume the
  kernel handle; that's Phase 4.1 + an executor-protocol extension).
  The threshold will return to 1000 once specialised dispatch
  actually saves cycles.

### Tests

- New `cpu_specialiser_populates_cache_after_hotness_threshold` test
  drives `gpu_invert` 150 times and asserts that `spec_cache_len()`
  rises.  This is the first place in `image-gpu-core`'s test suite
  where the SpecCache is observably non-empty after a real dispatch.
- The earlier `dispatch_drives_spec_router_pipeline` test no longer
  asserts `cache_len == 0` (which used to be the NoopSpecialiser
  invariant) — it only checks that invocation counters climb.

Total tests: 27 unit + 1 doc = 28 (was 26 + 1 = 27).

### Regression check

`instagram-filters` routing on macOS unchanged — the dispatch path
itself doesn't consume the specialised kernel handle yet, so output
bytes and the `last_executor()` value are identical to V0.4.0:

```
  invert        → cpu     (small graph, planner picks CPU)
  greyscale     → metal   (sRGB + matmul, ships to GPU)
  sepia         → metal   (matmul-heavy, ships to GPU)
```

The only new observable is `spec_cache_len()` — call it after a few
hundred filter invocations and watch the number rise.

## 0.4.0 — 2026-05-05

### Added — MX05 Phase 3 V4 wiring

- Each call to `pipeline::run_graph_with_constant_inputs` now drives
  the MX05 specialisation pipeline end-to-end:
    1. `Profiler::record_dispatch` bumps per-(graph, op) invocation
       counters.
    2. `SpecRouter::route` is consulted for every Compute op, with the
       op's wire tag, output dtype, and target executor id.
    3. The router's return is **discarded in V1** — `NoopSpecialiser`
       declines every key.  The wiring is foundation for Phase 4
       when a real specialiser arrives.
- New public observation hooks:
    - `image_gpu_core::profiler_observations()` — snapshot the
      accumulated `ProfileObservation` set.  Useful for telemetry,
      tests, and future phase 4 caller-side logic.
    - `image_gpu_core::spec_cache_len()` — how many specialised
      kernels are cached process-wide.  Always `0` while
      `NoopSpecialiser` is installed.
- Per-process `Profiler` and `SpecRouter` singletons via `OnceLock`
  so the routing pipeline is set up once and amortised across all
  filter invocations.

### Tests (1 new)

- `dispatch_drives_spec_router_pipeline` — gpu_invert produces a
  visible bump in `profiler_observations`'s aggregate invocation
  count.  Cache stays empty (NoopSpecialiser).

### Notes

- No behavioural change in the dispatch path itself.  Routing,
  output bytes, and the `last_executor()` value are unchanged.
- Phase 4 will install a backend-specific specialiser (e.g. an
  MSL emitter that constant-folds bias values for an LLM bias-add
  pattern).  When that lands, `spec_cache_len()` will start rising
  and `route()` will start returning `Some(SpecialisedKernel)`s
  that the dispatch path will consume — once `executor-protocol`
  grows a way for backends to dispatch via a SpecKey-keyed kernel
  handle.

## 0.3.0 — 2026-05-04

### Added

- **Optional `matrix-metal` backend** behind a default-on `metal-backend`
  feature.  With the feature enabled, `image-gpu-core` registers both
  `matrix-cpu` and `matrix-metal` in the runtime and lets the planner
  pick per graph based on its cost model.  On non-Apple platforms the
  feature is a no-op (matrix-metal's `local_transport()` returns Err
  and we transparently fall back to CPU-only dispatch).
- `pipeline::last_executor()` (re-exported as `image_gpu_core::last_executor`)
  reports the executor that handled the most recent dispatch on this
  thread (`"cpu"`, `"metal"`, or `None`).  CLI demos use this to surface
  which backend ran without changing the public per-op signatures.

### Changed

- `pipeline::run_graph_with_constant_inputs` now plans against the
  full multi-executor registry when `metal-backend` is enabled, and
  inspects the resulting `ComputeGraph` for single-executor placement
  before dispatching.  See `pipeline.rs` for the V1 single-executor
  dispatch design notes.

### Limitations

- V1 only supports **single-executor placements**: if the planner
  splits a graph across CPU and Metal (with `Transfer` ops between),
  we re-plan on a CPU-only registry and run on CPU.  The matrix
  execution layer's runtime crate doesn't yet ship a multi-executor
  coordinator that can drive cross-executor dispatch end-to-end —
  that's V2 work.  In practice the image-filter graphs in this crate
  are short single-op chains that the planner places homogeneously,
  so the mixed-placement fallback rarely triggers.

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
