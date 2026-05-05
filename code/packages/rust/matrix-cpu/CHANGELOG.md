# Changelog

All notable changes to `matrix-cpu` are documented here.

## [0.3.0] — 2026-05-05

### Added — MX05 Phase 4 (first real backend Specialiser)

- New `specialiser` module exporting `CpuSpecialiser` and a
  convenience function `specialiser() -> Box<dyn Specialiser>`.  This
  is the **first real backend `Specialiser` implementation** in the
  workspace (previously every test and demo used `NoopSpecialiser`,
  which always returned `None`).
- `CpuSpecialiser::specialise(key)` emits a `SpecialisedKernel` for
  any `SpecKey` it sees, with a deterministic 64-bit handle (FNV-1a
  over a stable byte serialisation of every public field of `SpecKey`).
  Two calls with the same key produce identical handles; distinct
  keys produce distinct handles with extremely high probability.
- New direct dependency on `matrix-profile` so we can `impl Specialiser`
  without going through `matrix-runtime`'s re-export.

### Phase 4 minimum-viable scope

The kernel handle is **opaque to the runtime** — the dispatch path
doesn't yet consume it (that needs an `executor-protocol` extension
to add something like `ExecutorRequest::DispatchSpecialised`,
which is V2 work).  But emitting the handle proves the wiring is
live: under a `SpecRouter` configured with this specialiser plus
a low policy threshold, hot graphs visibly populate the
`SpecCache`.

The integration test `router_with_cpu_specialiser_populates_cache_when_policy_fires`
is the first place in the codebase where `cache.len()` rises above
zero — the spec MX05 promise that "Phase 4 will see spec_cache_len
rise" has finally cashed in.

### Tests (7 new, all in `specialiser::tests`)

- `specialise_emits_kernel_for_any_key`
- `handles_are_deterministic_for_same_key`
- `handles_differ_for_distinct_keys`
- `handle_is_sensitive_to_shape_class`
- `handle_is_sensitive_to_constant_bytes`
- `specialiser_function_returns_box_dyn`
- `router_with_cpu_specialiser_populates_cache_when_policy_fires`
  (end-to-end integration with `SpecRouter`, `DefaultPolicy(1, 0.95)`,
  and a hot synthetic `ProfileObservation`).

Total tests: 26 unit + 17 integration = 43 (was 19 + 17 = 36).

## [0.2.0] — 2026-05-05

### Added — opt-in cost-model calibration

- New `calibrate` module exporting `calibrate() -> BackendProfile`.
  Runs a brief throughput measurement (~10 ms per dtype, ~30 ms total)
  on F32 / U8 / I32 elementwise add and returns a `BackendProfile`
  with calibrated `gflops_*` fields.  Other fields are inherited from
  the hardcoded defaults in `profile()`.  Result is cached via
  `OnceLock` so repeat calls are ~10 ns.
- Calibration is **opt-in**: `profile()` continues to return the
  hardcoded defaults so CI stays deterministic and existing call
  sites (image-gpu-core, instagram-filters) keep working unchanged.
  Programs that want accurate routing on heterogeneous hardware call
  `matrix_cpu::calibrate()` at startup and use the result in place of
  `profile()`.
- `clamp_gflops()` floor-protects against ridiculously low
  measurements (< 1 GFLOPS suggests the system was thrashing during
  calibration; we fall back to the default in that case) and caps at
  `u32::MAX` to fit the `BackendProfile` field width.

### What we measure / don't measure

Measured: F32, U8, I32 elementwise-add throughput.  The planner only
needs ordinal correctness for routing decisions, not per-cycle
accuracy, so a coarse measurement is sufficient.

Not measured (V1): memory bandwidth (`host_to_device_bw` etc.).
Inherits the heuristic 100 bytes/ns default which is close enough for
the cost model to make the right shape of decision on host-resident
buffers.  V2 of calibration could add a memcpy benchmark.

### Tests (6 new)

- `calibrate_returns_sane_profile` — values within plausible range.
- `calibrate_is_idempotent` — caching works (subsequent calls give
  exactly the same numbers).
- `calibrate_inherits_non_throughput_fields_from_profile` — only the
  three `gflops_*` fields differ from the default.
- `clamp_gflops_floors_implausibly_low_at_default`
- `clamp_gflops_caps_at_u32_max`
- `clamp_gflops_passes_through_normal_values`

Total tests: 19 unit + 17 integration = 36 (was 13 + 17 = 30).

### Notes

- On the author's M-series Mac the calibrated F32 number (~10 GFLOPS
  for a single-thread scalar elementwise add) is **lower** than the
  default's 40 GFLOPS.  That's expected — the default was set
  optimistically — and the absolute values matter less than the
  relative gap to the registered specialised backends.  Programs
  using calibration on this hardware will see Metal preferred for
  more graphs than under the defaults.
- Image-filter routing in instagram-filters is unchanged because
  image-gpu-core still uses `profile()`, not `calibrate()`.  Switching
  it over is a separate opt-in (and arguably the wrong default, since
  it'd make the routing depend on the CPU's mood at startup).

## [0.1.1] — 2026-05-04

### Fixed

- **`profile().supported_ops` bitmask now includes `Op::Const` (tag 0x1B
  = bit 27).**  The original mask `0x07FF_FFFF` set only bits 0..=26 and
  silently dropped `Op::Const`, even though `dispatch.rs` had
  always implemented the `Op::Const` runtime handler.  The mismatch
  caused the planner's capability filter to force every `Op::Const`
  onto a non-CPU backend whenever one was registered, which in turn
  prevented uniform-CPU placement from ever winning the cost-model
  comparison and made `image-gpu-core`'s "embedded-as-constants"
  graphs route work to Metal even at sizes where CPU was clearly
  cheaper.  Fix: change the mask to `0x0FFF_FFFF` so all 28 V1 ops are
  advertised correctly.

## [0.1.0] — 2026-05-04

Initial release.  First executor crate of the matrix execution layer.

### Added

- `CpuExecutor` — owns buffer store + kernel cache; processes the full
  set of `ExecutorRequest` variants from `executor-protocol`.
- `BufferStore` — HashMap-backed `BufferId → Vec<u8>` map.  Bounds-checked
  reads and writes with `checked_add` for offset+len.
- Per-op evaluators (`src/eval.rs`):
  - Elementwise unary (Neg, Abs, Sqrt, Exp, Log, Tanh, Recip) — 27 ops
    × 3 dtypes
  - Elementwise binary (Add, Sub, Mul, Div, Max, Min, Pow)
  - MatMul on F32/U8/I32 with row-major layout
  - Reductions (Sum, Max, Mean) along arbitrary axes with keep_dims
  - Shape ops (Reshape, Transpose with permutation, Broadcast)
  - Comparisons (Equal, Less, Greater) producing U8 output
  - Where (per-element predicate selection)
  - Cast across F32 ↔ U8 ↔ I32 (saturating clamps for out-of-range)
- `dispatch::run()` — walks `ComputeGraph.ops` in order, executes each
  Compute op, copies bytes for Transfer ops, allocates/frees buffers.
- `profile()` — default `BackendProfile` for CPU executors.
- `register()` — convenience that registers CPU with a `Runtime`.
- `local_transport()` — wraps a fresh CpuExecutor in a LocalTransport.

### Test coverage: 27 tests passing

- 13 unit tests (buffer store, eval helpers, dtype conversion)
- 14 integration tests covering:
  - Direct request/response (alloc/upload/download, heartbeat,
    shutdown, cancel)
  - Single-op dispatch (Add, MatMul, ReduceSum, Where, Less)
  - Multi-input dispatch with constants
  - Local transport pipeline (alloc → upload → dispatch → download)
  - Per-dtype unary smoke tests

### Constraints

- Zero external dependencies.  Only matrix-ir, compute-ir,
  executor-protocol, matrix-runtime (path-only).
- Single-threaded execution; mutex-guarded internal state for thread
  safety of `Arc<CpuExecutor>`.
- IEEE-754 float semantics; wrapping integer arithmetic; saturating
  clamps for cross-dtype Cast.

### Out of scope (V1, deferred to V2)

- Multi-threaded / SIMD evaluation
- Real per-op timing measurements
- Async-aware cancel
- Cooperative streams / overlap with transfers
