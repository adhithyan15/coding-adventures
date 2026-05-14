# Changelog

All notable changes to `matrix-cpu` are documented here.

## [0.7.0] — 2026-05-14

### Added — MX05 Phase 5 (kernel eviction)

- `SpecialisedTable::evict(handle) -> bool` — drops a kernel by
  handle.
- `CpuExecutor::evict_specialised(handle) -> bool` — public API
  exposed to the deoptimisation path in image-gpu-core.  When a
  previously-folded constant changes at runtime, the cached
  closure is wrong; this drops it so subsequent
  `DispatchSpecialised` requests fall through to the generic path.

## [0.6.0] — 2026-05-13

### Added — MX05 Phase 4.10 (MatMul closure with folded matrix)

`build_specialised_kernel` now produces a closure for
`Op::MatMul(0x15)` f32 with a folded **RHS** matrix.  Mirrors
matrix-metal's emitter (v0.10.0):

  - 2×2 (16 bytes) and 4×4 (64 bytes) constant matrices only
  - `folded_slot = Some(1)` only (RHS folded)
  - Variable LHS shape `[m, dim]`; closure derives `m` from input
    buffer byte length

Closure logic: for each output element `C[r, c]`, computes
`sum_k A[r, k] * B[k, c]` using the captured constant matrix.

#### Tests

All 33 existing tests still pass.  The new MatMul closure is
exercised by image-gpu-core's
`cpu_matmul_folded_rhs_2x2_produces_correct_output` integration
test, which asserts the result of
`A = [[1, 2], [3, 4]] × B = [[5, 6], [7, 8]] = [[19, 22], [43, 50]]`
through the full
install → DispatchSpecialised → DownloadBuffer path.

## [0.5.0] — 2026-05-13

### Added — MX05 Phase 4.9 (CpuSpecialiser closure builder)

Closes the gap that's been open since Phase 4.3: matrix-cpu now
emits **Rust closures** (`Box<SpecialisedKernelFn>`) for the same
SpecKey shapes the matrix-metal MSL emitter handles.  This lets
image-gpu-core's auto-installer install on the CPU executor too,
not just metal.

#### New public function

`matrix_cpu::build_specialised_kernel(key: &SpecKey, _handle: u64)
-> Option<Box<SpecialisedKernelFn>>`

Mirrors `matrix_metal::emit_specialised_kernel` — instead of an
MSL string, it returns a closure that operates on the
`BufferStore` directly.  Coverage matches matrix-metal v0.9.0:

  - **Commutative binary** (Add 0x07, Mul 0x09, Max 0x0B, Min 0x0C)
    with folded constant — `out[i] = a[i] OP K`
  - **Non-commutative binary** (Sub 0x08, Div 0x0A, Pow 0x0D) with
    `folded_slot = Some(0)` or `Some(1)` — picks `K OP a[i]` or
    `a[i] OP K`
  - **Unary with folded input** (Neg 0x00, Abs 0x01, Sqrt 0x02,
    Exp 0x03, Log 0x04, Tanh 0x05, Recip 0x06) — `f(K)` is
    precomputed at build time; the closure becomes a memset

f32 only in V1.

### Changed — `dispatch::run` always reallocates constant buffers

Previously matrix-cpu's `dispatch::run` had:
```rust
if !buffers.contains(c.residency.buffer) {
    buffers.alloc(c.residency.buffer, c.bytes.len());
}
buffers.write(c.residency.buffer, 0, &c.bytes)?;
```

The `if !contains` guard was brittle once image-gpu-core promoted
`CpuExecutor` to a long-lived (thread-local) singleton: a stale
buffer with the wrong size could shadow a fresh constant's
allocation, causing `write past end` errors.  The guard is gone;
we now always call `buffers.alloc(...)` (which replaces, per its
docstring).  Reallocation on host is cheap and matches matrix-metal's
behaviour.

### Tests

All 33 existing tests still pass.  The new closure builder is
exercised by image-gpu-core's
`cpu_auto_installer_registers_kernel_after_threshold` integration
test (v0.12.0).

## [0.4.1] — 2026-05-13

### Changed — MX05 Phase 4.6 (handle hash includes `folded_slot`)

`CpuSpecialiser`'s FNV-1a handle hash now feeds the new
`SpecKey::folded_slot` field that matrix-profile v0.2 introduced.
Two `SpecKey`s differing only in `folded_slot` (e.g. LHS-folded
vs RHS-folded `Op::Sub`) now produce **distinct** 64-bit handles,
preventing kernel collisions in the executor's `SpecialisedTable`.

Encoding: `None` → discriminator byte `0xFF`; `Some(s)` →
discriminator byte `0x00` followed by `s`.

Test helper `key()` updated to set `folded_slot: None`.  All 33
existing tests still pass.

## [0.4.0] — 2026-05-12

### Added — MX05 Phase 4.1 (specialised dispatch lands on CPU)

- New `SpecialisedTable` (internal module `specialised_table`) — a
  per-`CpuExecutor` table of installed specialised kernel closures,
  keyed by the opaque `u64` handle that `CpuSpecialiser` emits.
  Wraps a `HashMap<u64, Box<SpecialisedKernelFn>>` with installation
  semantics (re-install replaces, ready for Phase 5 deoptimisation).
- New `SpecialisedKernelFn` type alias —
  `dyn Fn(&mut BufferStore, &[BufferId], &[BufferId]) -> Result<Vec<OpTiming>, String> + Send + Sync`.
  Bounded by `Fn` (not `FnMut`) so a specialised kernel is statically
  pure with respect to its captured environment; all per-call mutation
  goes through the `&mut BufferStore` argument.
- New `CpuExecutor::install_specialised(handle, kernel)` API.  Locks
  the executor's internal `Mutex<State>` and inserts the closure
  under `handle`.  Re-installing a previously-installed handle
  replaces the closure.
- New `CpuExecutor::specialised_count()` accessor — number of
  installed specialised kernels.  Test/metric only; matches the
  shape of `BufferStore::len`.
- **`ExecutorRequest::DispatchSpecialised` handler is now live.**  On
  handle hit: invokes the installed closure with the request's
  `inputs`/`outputs` buffer ids and `&mut BufferStore`, returns
  `DispatchDone { job_id, timings }` on success, or
  `Error { code: RUNTIME_ERROR, .. }` if the closure errs.  On handle
  miss: returns `Error { code: NOT_IMPLEMENTED, .. }` so the runtime
  can fall back to the generic `Dispatch` path with the original
  `ComputeGraph`.

### Security hardening

- **Panic-safe specialised dispatch.**  The closure invocation is
  wrapped in `std::panic::catch_unwind(AssertUnwindSafe(...))` so a
  panicking kernel (e.g. one that indexes attacker-supplied empty
  `inputs[0]` and triggers an out-of-bounds panic) becomes a clean
  `Error { code: RUNTIME_ERROR, message: "specialised kernel 0x…
  panicked: …", .. }` response instead of unwinding through the
  mutex guard and out of `handle()`.  This honours the existing
  contract documented on `CpuExecutor::handle()`: "a single bad
  request cannot DoS the executor for all subsequent clients".
  Caught during the pre-push security review.

### What this unlocks

Up to v0.3.0, the Phase 4 pipeline emitted handles and populated
`SpecCache` but no executor *did* anything with them — every
dispatch still walked the generic `ComputeGraph` op-by-op.  Phase 4.1
closes that loop on the CPU side: `DispatchSpecialised` now returns
`DispatchDone` for installed handles.  matrix-metal's MSL emitter
(Phase 4.2) will follow the same plumbing pattern but compile to
`MetalComputePipelineState` cached by handle.

### Tests (12 new, all passing)

In `specialised_table::tests`:

- `install_then_lookup_finds_kernel`
- `lookup_of_missing_handle_returns_none`
- `install_overwrites_prior_kernel`
- `kernel_can_read_inputs_and_write_outputs`
- `kernel_error_propagates`
- `specialised_table_is_send_sync`
- `debug_impl_shows_handles_not_pointers`

In `tests/integration.rs` (§6 MX05 Phase 4.1):

- `dispatch_specialised_returns_not_implemented_when_handle_unknown`
- `dispatch_specialised_returns_dispatch_done_after_install`
- `dispatch_specialised_kernel_error_becomes_runtime_error`
- `install_specialised_overwrites_prior_kernel`
- `dispatch_specialised_kernel_can_call_real_eval` — installs a
  real f32 add closure, fires DispatchSpecialised, downloads the
  result, asserts numerical correctness.  This is the test that
  proves the full round-trip works.
- `dispatch_specialised_kernel_panic_becomes_runtime_error_not_unwind`
  — security regression for the `catch_unwind` hardening above.
  Asserts that a panicking kernel surfaces as `RUNTIME_ERROR` and
  that the executor still answers Heartbeat normally afterwards.

Total test count: 33 unit + 23 integration (was 26 + 17).

### Notes

- The Phase 4.1 spec comment in `specialiser.rs` ("matrix-cpu can
  store these in a per-handle `Vec<Box<dyn Fn>>`") is now realised —
  except as a `HashMap<u64, ...>` since handles are sparse FNV-1a
  hashes, not contiguous indices.
- This release does **not** yet wire `SpecRouter` cache hits to
  `install_specialised` automatically — that's a matrix-runtime
  concern (next phase: runtime observes a cache hit, looks up the
  `SpecialisedKernel`, installs a closure into the target executor,
  routes the next invocation to `DispatchSpecialised`).  The
  plumbing on the *executor* side, which is the harder of the two,
  lands here.

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
