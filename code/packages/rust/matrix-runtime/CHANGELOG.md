# Changelog

All notable changes to `matrix-runtime` are documented here.  The
format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
