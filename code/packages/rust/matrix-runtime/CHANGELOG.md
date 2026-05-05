# Changelog

All notable changes to `matrix-runtime` are documented here.  The
format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.6.0] — 2026-05-05

### Added — MX05 Phase 3 V2 (specialisation policy)

- New `policy` module exporting:
  - **`SpecialisationPolicy`** trait — `should_specialise(observation,
    op_kind, output_dtype, backend_id) -> Option<SpecKey>`.  Pluggable
    so frameworks can supply workload-specific policies (LLM
    inference, image processing, scientific simulation).  `Send + Sync`
    for use behind a `Box<dyn>`.
  - **`DefaultPolicy`** — V1 implementation of the spec rules from
    MX05 §"Specialisation trigger".  Two configurable thresholds
    (`min_invocations` default 1000; `stability_threshold` default 0.95).

### Default policy rules

Trigger when `invocation_count >= min_invocations`, AND **at least one** of:

  1. **Constant input**: an input tensor where `observed_min ==
     observed_max` and `samples / invocation_count >=
     stability_threshold`.  Emits `RangeClass::Constant { bytes }` with
     the value little-endian-encoded in the output dtype.
  2. **Range narrowing**: F32 input with bounded observed range.
     Emits `RangeClass::FloatBits` with the IEEE-754 bit-encoded
     bounds.  Backends decide whether to act on the signal in V1
     (none yet support narrower dtypes; the spec carries the
     opportunity for Phase 2b's auto-narrow Cast insertion or for
     workload-specific policies that bias toward narrowing).

Shape stability is implicit in `Profiler::subhash` (which already
includes per-tensor shape metadata), so this policy doesn't track it
explicitly.  Cross-shape specialisation is Phase 4 work.

### Tests (12 new)

- `below_min_invocations_returns_none`
- `at_min_invocations_with_constant_input_fires`
- `constant_input_below_stability_threshold_falls_through_to_narrowing`
- `nonconstant_bounded_range_uses_narrowing_branch`
- `empty_tensor_observations_returns_none`
- `output_only_observations_dont_fire_constant_branch`
- `integer_dtype_no_narrowing`
- `u8_constant_input_fires_with_byte_value`
- `i32_constant_input_fires_with_le_bytes`
- `unbounded_range_does_not_fire_narrowing`
- `custom_thresholds_work`
- `forced_fire_helper_constructs_a_speckey`

Total tests: 63 unit + 8 integration = 71 (was 51 + 8 = 59).

### Notes

- Policy is **passive** in V2 — it returns SpecKeys but nothing
  in the dispatch loop calls it yet.  Phase 3 V3 wires
  `SpecialisationPolicy` + `Specialiser` + `SpecCache` into a
  hot-path that invokes the policy after `record_dispatch`,
  consults the cache, and asks the backend's specialiser to emit a
  kernel on cache miss.
- Constant-input encoding (`encode_constant_bytes`) clamps to the
  destination dtype's range — out-of-range observations encode to
  the nearest representable value.  An aggressive policy that wants
  to reject out-of-range observations can wrap `DefaultPolicy` and
  add the check.
- Image-filter routing on macOS unchanged: `invert` → CPU,
  `greyscale` / `sepia` → Metal across the synthetic test-image suite.

## [0.5.0] — 2026-05-05

### Added — MX05 Phase 3 V1 (SpecKey + Specialiser trait + LRU SpecCache)

- New `spec` module exporting:
  - `SpecKey` — equivalence class identifying which observed pattern
    a specialised kernel targets.  Carries `op_kind` + `dtype` +
    `ShapeClass` + `RangeClass` + `backend_id`.  `Hash + Eq` so it
    works as a HashMap key.
  - `ShapeClass` — `Static(Shape)` / `StaticRank(u8)` / `Dynamic`.
  - `RangeClass` — `FloatBits` (IEEE-754 bit-encoded so the enum can
    derive `Hash`), `Integer { min, max }`, `Constant { bytes }`,
    `Unknown`.  `RangeClass::float(min, max)` constructor handles
    the bit conversion and collapses NaN ends to `Unknown` so
    downstream HashMap lookups stay stable.
  - `Specialiser` trait — backends implement to emit specialised
    kernels for a given key.  Default `NoopSpecialiser` always
    returns `None` so an executor that hasn't opted in still works.
  - `SpecialisedKernel` — `(SpecKey, opaque handle, source_summary)`.
  - `SpecCache` — bounded LRU keyed by `SpecKey`.  Default capacity
    64 entries; capacity 0 means "never cache".  `get` touches LRU,
    `insert` evicts the least-recently-used entry when full.

### Tests (13 new)

- `spec_key_equality_on_all_fields`
- `shape_class_static_is_hashable`
- `range_class_float_round_trip`
- `range_class_float_with_nan_collapses_to_unknown`
- `range_class_constant_is_hashable`
- `noop_specialiser_returns_none`
- `cache_insert_and_get_round_trip`
- `cache_get_miss_returns_none`
- `cache_evicts_lru_when_full`
- `cache_capacity_zero_does_not_store`
- `cache_re_insert_updates_value_and_touches_lru`
- `cache_clear_drops_everything`
- `cache_at_default_capacity_evicts_in_lru_order`

Total tests: 51 unit + 8 integration = 59 (was 38 + 8 = 46).

### Notes

- Phase 3 V1 ships the **shape** of profile-guided specialisation
  without yet plugging it into the dispatch path.  Phase 3 V2 will
  add the policy that turns a `ProfileObservation` (Phase 2a output)
  into a `SpecKey` and decides when to call the backend's
  `Specialiser`.  Phase 3 V3 will route dispatch through `SpecCache`
  on the hot path.
- `SpecCache::get` is currently O(n) on hit (a `VecDeque::retain`
  call to maintain LRU order).  Acceptable at the V1 default capacity
  of 64; will revisit in Phase 3 V2 if profiling shows it as a hot
  spot.
- Image-filter routing on macOS unchanged: `invert` → CPU,
  `greyscale` / `sepia` → Metal across the synthetic test-image
  suite.

## [0.4.0] — 2026-05-05

### Added — MX05 Phase 2a (range observation)

- `Profiler::sample_tensor(graph_subhash, op_index, slot, is_input,
  dtype, bytes)` — accumulates per-(graph, op, slot, direction) running
  statistics from raw bytes.  Bounded ≤ ~64 bytes of state regardless
  of `bytes.len()`; work scales with the number of scalars passed in.
  Supports F32, U8, I32 (the V1 dtype set).  F32 NaNs are skipped (do
  not poison min/max) but are still counted as samples so sparsity
  ratios stay honest.  Trailing partial scalars are silently
  truncated, matching what dispatchers do.
- `Profiler::tensor_observation(graph_subhash, op_index, slot,
  is_input)` — read-back accessor for tests and policy code.
- `Profiler::should_sample()` — counter-based sampling gate;
  deterministic, returns `true` once per `sample_rate` calls.  Default
  rate is 100 (≈ 1% sampling, matching the spec).
- `Profiler::set_sample_rate(rate)` — tunes the gate.  Rate `0` means
  "never sample", `1` means "always sample", anything in between gives
  a 1/N hit rate.
- `Profiler::observations()` now folds sampled `TensorObservation`s
  into the matching `ProfileObservation`'s `tensor_observations`
  vector, sorted inputs-before-outputs and by slot.
- `Profiler::reset()` now also clears tensor observations and rewinds
  the sample counter.

### Tests (10 new)

- `sample_tensor_f32_records_min_max_zeros`
- `sample_tensor_u8_records_min_max_zeros`
- `sample_tensor_i32_records_min_max_zeros`
- `sample_tensor_accumulates_across_calls`
- `sample_tensor_f32_skips_nan`
- `sample_tensor_truncates_partial_trailing_scalar`
- `observations_includes_tensor_observations`
- `should_sample_at_default_rate_yields_one_in_hundred`
- `should_sample_rate_one_always_yields_true`
- `should_sample_rate_zero_never_yields_true`
- `reset_clears_tensor_observations_and_sample_counter`
- `observations_orders_tensor_observations_by_input_then_slot`

Total tests: 38 unit + 8 integration = 46 (was 26 + 8 = 34).

### Notes

- No specialisation policy yet — Phase 2a only ships the data
  plumbing.  The future Phase 2b (auto-narrow `Cast` insertion) and
  Phase 3 (`SpecKey` / `Specialiser` trait) consume `tensor_observation`
  output to decide which graphs / ops are worth specialising.
- Image-filter routing on macOS is unchanged: `invert` → CPU,
  `greyscale` / `sepia` → Metal across the synthetic test-image suite
  (regression check).

## [0.3.0] — 2026-05-04

### Added — MX05 Phase 1 (profile sampler)

- New `profile` module exporting `Profiler`, `ProfileObservation`,
  and `TensorObservation`.  Implements **Phase 1 of spec MX05** —
  the observation infrastructure that future phases plug
  specialisation into.
- `Profiler::record_dispatch(placed: &ComputeGraph)` bumps a per-op
  invocation counter for every `PlacedOp::Compute` in the graph,
  keyed by a stable `(graph_subhash, op_index)` pair.  Counters
  survive re-plans of the same matrix-ir Graph against different
  executor topologies because the subhash deliberately ignores
  residency-specific fields.
- `Profiler::observations()` and
  `Profiler::invocation_count(subhash, op_index)` expose the
  counters; `Profiler::reset()` clears them between benchmark runs.
- `TensorObservation` is reserved for Phase 2 (range / sparsity
  sampling); Phase 1 always returns it empty.

### Tests

- 8 new tests in `profile::tests::*` covering empty-state behaviour,
  counter monotonicity past the spec-mandated 1000-invocation
  threshold, that non-Compute ops don't bump counters, that
  `last_executor` updates correctly, that subhashes are
  deterministic and that distinct graphs produce distinct subhashes,
  and that observation count matches the number of distinct Compute
  ops.

### Notes

- Phase 1 ships as a module inside `matrix-runtime` rather than its
  own `matrix-profile` crate (per the eventual layout in spec
  MX05).  We'll promote it to a separate crate when Phase 2 lands
  real observation logic with its own dependency surface.

## [0.2.0] — 2026-05-04

### Added

- **Pass 2b: single-executor preference** in the planner (spec MX04
  §"Single-executor preference (V1.1)").  Greedy cost minimisation
  in pass 2 makes per-op decisions and never amortises the up-front
  host→device transfer.  Pass 2b scores each healthy executor as a
  *uniform* placement candidate for the whole graph and replaces the
  greedy placement if a uniform alternative is strictly cheaper
  end-to-end.  When pass 2b fires, it also reassigns every constant
  source-tensor's residency to the chosen uniform executor so
  downstream consumers see an honestly-uniform `ComputeGraph`.

### Tests

- 4 new tests in `planner::tests::*` covering the pass 2b paths:
  `long_elementwise_chain_ships_to_gpu_uniformly`,
  `single_tiny_op_stays_on_cpu`,
  `capability_hole_disables_uniform_gpu`,
  `uniform_replaces_only_when_strictly_cheaper`.
- All 17 existing tests continue to pass without modification.

## [0.1.0] — 2026-05-04

Initial release.  Implements spec MX04 V1.

### Renamed from `compute-runtime`

The spec MX04 originally named this crate `compute-runtime`, but that
name was already taken by the G05 GPU-runtime simulator (Vulkan-inspired
device-discovery / command-recording crate).  Renamed to `matrix-runtime`
to disambiguate.  Specs MX00–MX04 updated.

### Added

- **`Runtime`** — the public API surface.  Owns a registry, exposes
  `plan()` (lower MatrixIR → ComputeIR), `register()`, `set_healthy()`,
  `update_profile()`, `executors()`.
- **`Registry`** — `Vec`-backed list of `RegisteredExecutor` records,
  indexed by `ExecutorId`.  CPU pre-registered at `ExecutorId(0)` via
  `Registry::with_cpu()`.
- **`RegisteredExecutor`** — `id`, `kind`, `profile`, `healthy` plus
  `supports_op()` / `supports_dtype()` capability bitset accessors.
- **`plan()`** — the four-pass planner:
  1. Capability filter
  2. Greedy cost minimisation
  3. Transfer insertion
  4. Lifetime annotation (`Alloc` before first use, `Free` after last use)
- **Cost model** — `estimate_flops()`, `estimate_matmul_flops()`,
  `transfer_cost_ns()`, `compute_cost()`.  Coarse but ordinally correct.
- **`PlanError`** — `InvalidGraph`, `NoCapableExecutor`, `EmptyRegistry`,
  `UndefinedTensor`.
- **`RuntimeError`** — wraps `PlanError` plus future execution errors.

### Behaviours that fall out of the cost model

- **Small ops stay on CPU** — transfer cost exceeds GPU speedup.
- **Large ops ship to GPU** — GPU speedup exceeds transfer cost.
- **Capability fallback** — ops with unsupported dtypes fall back
  to CPU automatically.
- **Health-aware placement** — unhealthy executors are filtered.
- **Monotonic threshold** — slower PCIe makes the GPU threshold higher.

All without any special-case logic.

### Test coverage: 21 tests passing

- 8 integration tests (CPU-only no-transfers, small-add-stays-on-CPU,
  large-matmul-ships-to-GPU, unsupported-dtype-falls-back-to-CPU,
  unhealthy-executor-skipped, empty-runtime-errors,
  cost-model-threshold-monotonic, plan-function-with-registry)
- 13 unit tests across cost / registry / runtime / planner modules

### Constraints

- Zero external dependencies.  Only `matrix-ir`, `compute-ir`, and
  `executor-protocol` (path-only).
- Pure planning logic.  No I/O — execution is delegated to executor
  crates via the `Transport` from `executor-protocol`.

### Out of scope (V1, deferred to V2)

- End-to-end `run()` — depends on the first executor crate.
- Graph caching.
- Cost-model auto-tuning via microbenchmarks.
- Speculative execution.
- Within-graph parallelism.
