# Changelog — matrix-cuda

## 0.1.0 — 2026-05-13

### Added — MX06 Phase 1 (crate skeleton + stub)

Initial release.  Lands the crate placeholder so Phases 2–7 of MX06
each ship as small, isolated PRs against a stable surface.

- `CudaExecutor` struct with `new()` constructor that probes for
  CUDA via `cuda-compute`'s `CudaDevice::new(0)`.  On hosts without
  an NVIDIA driver / device, returns `Err(...)` and upstream
  registration silently skips the executor.
- `profile()` returns a `BackendProfile` with `kind: "cuda"`,
  `supported_ops: 0` (Phase 1 is dormant — Phase 5 flips the V1 op
  bits on), and placeholder cost-model coefficients representative
  of a mid-range Ampere card over PCIe gen 4.  Real calibration
  lands in Phase 7.
- `handle(req)` implements the full `ExecutorRequest` surface:
  - `Register` echoes our currently-stored `ExecutorId`.
  - `Heartbeat` replies `Alive { profile }`.
  - `Shutdown` is a graceful no-op.
  - Every other variant returns
    `ErrorCode::NOT_IMPLEMENTED` with a pointer to the spec
    (`code/specs/MX06-cuda-executor.md`) so future readers know
    which phase fills it in.
- `set_our_id(id)` so `matrix-runtime::register` can hand back the
  assigned `ExecutorId`.
- **MX05 specialisation surface** (`install_specialised`,
  `install_specialised_from_emitted`, `specialised_count`,
  `evict_specialised`) as contract-preserving no-ops.  Lets MX05's
  auto-installer hook into us in MX06 Phase 6 without changing call
  sites.
- `EmittedKernelPlaceholder` — the surface
  `install_specialised_from_emitted` accepts.  Replaced in Phase 4
  by the real type from `cuda_emitter`.
- Free helpers: `local_transport()` (wraps the executor in a
  `LocalTransport`) and `register(runtime)` (registers under the
  name `"cuda"`).

### Tests

12 unit tests — they assert:

- `profile()` advertises `kind = "cuda"` and the documented
  placeholder coefficients.
- `supported_ops_bitset()` returns `0` (sentinel that Phase 5 has
  not been accidentally merged in).
- `new()` returns either `Ok` (CUDA-bearing developer box) or `Err`
  with a message tagged with the crate name — no panics, no hangs.
- The MX05 surface (install / count / evict) all return the
  documented placeholder values.
- `handle(req)` routes every variant to the correct stub branch
  (Register → Registered, Heartbeat → Alive, Dispatch* / CancelJob
  → Error with NOT_IMPLEMENTED, buffer ops → Error with
  NOT_IMPLEMENTED).

### Dependencies

- `matrix-ir`, `compute-ir`, `matrix-runtime`, `matrix-profile`,
  `executor-protocol` — the standard executor stack.
- `cuda-compute` — runtime-loaded libcuda wrapper (zero link-time
  NVIDIA dependency).

No `unsafe` blocks added in this phase.

### Why this is its own PR

Phase 1 is the placeholder.  Splitting it from Phases 2–7 means each
later PR is a focused, isolated change against a known surface, and
the planner / above-layer code can start referencing
`matrix_cuda::profile()` immediately without waiting for real
dispatch to land.

### Migration

No behaviour change for existing users — `matrix-cuda` is not yet
registered by `image-gpu-core` (that wiring is Phase 6) and the
planner does not see it.

Adding `matrix-cuda` as a workspace member exposes one new package
in `cargo build --workspace` output; nothing else changes.
