# Changelog — matrix-metal

## 0.1.0 — 2026-05-04

Initial release.  First specialised executor for the matrix execution
layer.

### Added

- `MetalExecutor` — implements the executor-protocol contract on
  Apple Metal.  Mutex-guarded internal state with poison recovery.
- V1 op support: F32 elementwise unary (Neg, Abs, Sqrt, Exp, Log,
  Tanh, Recip), F32 elementwise binary (Add, Sub, Mul, Div, Max, Min,
  Pow), F32 MatMul (rank-2), Const.
- `BufferStore<MetalBuffer>` — bounds-checked HashMap keyed by
  `BufferId`.  Mirrors `matrix-cpu::BufferStore` API.
- MSL kernel library (`src/kernels.rs`) compiled once at executor
  startup.  Pipelines cached by entry-point name.
- `local_transport()` and `register()` helpers.
- `profile()` advertises capability bitset (16 ops × F32 only) and
  cost model defaults sized for Apple Silicon (5 TFLOPS f32, 50 GB/s
  unified memory, 5 µs launch overhead).
- Up-front graph validation (16 MiB per-tensor cap, byte_size overflow
  check) — same hardening as `matrix-cpu`.
- Non-Apple platforms compile a stub that always returns
  `DEVICE_LOST` from `handle()` and `Err` from `MetalExecutor::new()`.

### Tests

5 integration tests pass on real Metal hardware:
- `neg_f32_on_gpu` — elementwise unary
- `add_f32_on_gpu` — elementwise binary
- `matmul_2x2_on_gpu` — `[[1,2],[3,4]] × [[5,6],[7,8]] = [[19,22],[43,50]]`
- `local_transport_heartbeat` — protocol round-trip via LocalTransport
- `dispatch_rejects_oversized_tensor` — validation guard

### Constraints

- Zero external Cargo dependencies.  Only path deps to `matrix-ir`,
  `compute-ir`, `executor-protocol`, `matrix-runtime`, `metal-compute`.
- F32 only in V1 — every other dtype falls back to `matrix-cpu` via
  the planner's capability filter.
