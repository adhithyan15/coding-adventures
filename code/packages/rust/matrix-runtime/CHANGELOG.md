# Changelog

All notable changes to `matrix-runtime` are documented here.  The
format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
