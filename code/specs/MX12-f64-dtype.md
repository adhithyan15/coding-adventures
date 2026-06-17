# MX12 — An `f64` dtype for the matrix-execution substrate

## Status

Active. Adds **`DType::F64`** (IEEE-754 binary64) to the shared matrix-execution
stack — [`matrix-ir`](MX01-matrix-ir.md) → `matrix-cpu` → `matrix-runtime` →
`array-runtime` — so that **`f64`-native languages lower onto the shared
substrate at full precision** instead of being silently downcast to `f32`.

## §1 Why

The matrix substrate ([MX00](MX00-matrix-execution-overview.md)) was built `f32`-
first: `matrix-ir`'s only float dtype is `F32`, the `matrix-cpu` executor's
kernels read/write 4-byte floats, and `array-runtime`'s `execute` path converts
`f64 → f32 → f64` at the boundary (see `array-runtime/src/exec.rs`, which notes
"A future `f64` dtype in `matrix-ir` removes this"). That was the right V1 scope
for the MATLAB numeric lane, where `single`-precision was acceptable for the
GPU-by-cost payoff.

But the substrate is meant to be **shared** across the whole historical-math
program ([HML00](HML00-historical-math-languages-roadmap.md)), and most of those
languages are **`f64`-native**:

- **R / S** compute every numeric in `f64`; a stats language that rounded
  `m %*% n` to `f32` would be wrong, so today R's matrices use hand-written
  `f64` CPU loops in `s-runtime` *instead of* the shared substrate — exactly the
  duplication this program tries to avoid.
- **MATLAB**'s default numeric class is `double` (`f64`); routing `A*B` through
  the `f32` path is a silent precision downgrade.

The fix is to make the substrate *genuinely* dtype-general: give it an `f64`
dtype so both lanes lower onto the same GPU-aware execution path **and** keep
exact double precision. The IR and executor are already **dtype-agnostic by
architecture** — dtype is a per-tensor property threaded through the SSA graph —
so this is a strictly *additive* change (new enum variant, new kernel arms), not
a redesign.

## §2 The rollout (one item = one PR)

- **MXF-1 — `matrix-ir` `F64` + `matrix-cpu` kernels** *(shipped)*. The dtype
  itself plus a working CPU executor, so an `f64` graph **builds, validates, and
  executes exactly** on CPU end-to-end.
- **MXF-2 — `matrix-runtime` cost model** *(this PR)*. A `gflops_f64` throughput on
  `BackendProfile` and an `F64` arm in the cost model, so the planner places
  `f64` ops on the cheapest backend that can actually run them — a backend with
  no `f64` kernel advertises `gflops_f64 = 0`, which the cost model turns into the
  ∞-cost sentinel, keeping `f64` on the CPU.
- **MXF-3 — `array-runtime` `f64` path.** Lower `Array`s to an `F64` graph and
  add an 8-byte codec, so `execute` keeps the exact `f64` answer (no `f32`
  round-trip). The reference `ops` path and the executed path now agree to full
  precision.
- **MXF-4 — R adopts the substrate.** Route `s-runtime`'s `%*%` (and the
  elementwise/transpose/reduction matrix ops where it wins) through
  `array_runtime::execute` at `f64`, so R gets cost-based CPU/GPU dispatch with
  no precision loss — replacing the hand-written loops. (MATLAB switching its
  `double` ops to `F64` is a natural follow-on.)

## §3 MXF-1 — what this PR delivers

### `matrix-ir`
- `DType::F64` variant; `size_bytes() == 8`; a new `wire_tag` (`0x05`, leaving the
  reserved `0x03`/`0x04` for F16/I64) and its `from_wire_tag` round-trip.
- The float-only validation checks (`Sqrt`/`Exp`/`Log`/`Tanh`/`Recip`/`Div`/`Pow`
  require a float input) accept `F64` as well as `F32`, via an `is_float(dtype)`
  predicate rather than a bare `== F32`.
- No builder-API change: `input`/`constant` already take an explicit dtype, and
  every op propagates the input dtype, so `F64` flows through unchanged.

### `matrix-cpu`
- `f64` byte codecs (`read_f64_vec`/`write_f64_vec`, 8-byte little-endian) and
  the `f64` kernel variants `unary_f64` / `binary_f64` / `matmul_f64` /
  `reduce_f64`, mirroring the existing `f32` ones.
- An `F64` arm in every per-dtype dispatch `match` (unary, binary, matmul,
  reduce), and the `f64 ↔ {f32, i32, u8}` cases in `cast`.
- The specialiser's V1 `f32`-only fast path is left as-is (`F64` takes the
  general kernel path); this is a performance detail, not a correctness one, and
  is called out as deferred.

### Tests
- The CPU integration tests are parameterized over `{F32, F64}` for the core ops
  (add/mul/matmul/reduce), asserting `f64` results are **bit-exact** where `f32`
  would round (e.g. a sum that is not representable in `f32`), proving the new
  path is genuinely double-precision.

## §4 MXF-2 — what this PR delivers

### `executor-protocol`
- A `gflops_f64: u32` field on `BackendProfile`, sitting next to `gflops_f32`,
  with matching `encode_profile`/`decode_profile` wire I/O (one extra `u32`,
  inserted symmetrically so the round-trip stays balanced). Every constructor of
  the profile — the CPU/CUDA/Metal executor defaults, the runtime/registry/test
  stubs — sets it: CPUs to their `f32` rate, GPUs (no `f64` kernel in V1) to `0`.

### `matrix-runtime`
- `compute_cost`'s per-dtype rate selection now reads `profile.gflops_f64` for
  `DType::F64`, replacing the MXF-1 placeholder that reused `gflops_f32`. The
  existing `gflops == 0 → u64::MAX / 2` branch then makes an `f64` op on a
  backend with no `f64` throughput cost *infinity*, so the planner never ships
  `f64` to a GPU that can't run it.

### Tests
- Unit: a 4096³ `f64` matmul costs the ∞ sentinel on the GPU profile but a
  finite amount on the CPU; halving only `gflops_f64` doubles the `f64` cost,
  proving the F64 arm reads its own field rather than the `f32` rate.
- Integration: an `f64` matmul planned against a CPU + a *faster* GPU (that
  advertises `f64` capability but `gflops_f64 = 0`) lands on the CPU — the
  decision falls through to cost, not the capability filter.

## §5 Out of scope (later items / future)

- The GPU (CUDA/Metal) executors gaining `f64` kernels — MXF-1/-3 keep `f64` on
  the CPU executor; the planner (MXF-2) simply won't place `f64` on a GPU that
  doesn't advertise `f64` support. (CPU-fallback parity is the contract.)
- `F16`/`I64` (the other reserved wire tags).
- The specialiser fast path for `f64`.

## §6 References

Internal: [`MX00`](MX00-matrix-execution-overview.md),
[`MX01`](MX01-matrix-ir.md), [`MX03`](MX03-executor-protocol.md),
[`MX04`](MX04-compute-runtime.md), [`MA00`](MA00-array-runtime.md),
[`HML00`](HML00-historical-math-languages-roadmap.md), `R00` (R's matrices).
