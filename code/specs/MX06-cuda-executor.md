# MX06 — CUDA Executor (`matrix-cuda`)

## Status

Draft.  V1 spec.  Adds a second GPU executor next to `matrix-metal`
([MX03](MX03-executor-protocol.md)) so the matrix execution layer
runs on NVIDIA GPUs.  Reuses MX01–MX05 wholesale — no IR, protocol,
planner, or specialisation-runtime changes.

## Why this layer exists

MX01–MX04 ship the IR, planner, protocol, runtime, and the first
two executors: `matrix-cpu` (everywhere) and `matrix-metal` (Apple
GPUs).  MX05 adds the tiered specialisation runtime.

Three categories of user are now blocked on CUDA:

1. **Linux ML practitioners** — most of the world's training and
   serving fleets run NVIDIA GPUs on Linux.  Right now they get the
   CPU executor and watch their dispatches sit at a fraction of what
   the hardware can do.
2. **Windows + NVIDIA gamers / hobbyists** running this stack for
   image / audio side projects — Metal doesn't exist, so they're
   also CPU-bound.
3. **Mixed-fleet servers** running both NVIDIA and AMD cards — even
   if the AMD side stays CPU for now, the NVIDIA side should fly.

MX06 plugs that hole.  The narrow-waist design from MX00 means we
get this **without touching anything above the executor surface**.

## What this layer does NOT change

| Layer        | Status under MX06                                          |
| ------------ | ---------------------------------------------------------- |
| MX01 IR      | Unchanged.  `matrix-cuda` consumes the same `matrix_ir::Op`. |
| MX02 ComputeIR | Unchanged.  Same `KernelSource` discriminant per op.     |
| MX03 protocol | Unchanged.  `Dispatch` and `DispatchSpecialised` are wire-stable. |
| MX04 planner | Unchanged.  Cost model just gains a new `ExecutorId`.       |
| MX05 spec    | Unchanged.  `matrix-cuda` plugs into the same `Specialiser` interface as `matrix-metal`. |

Every above-layer test should pass with `matrix-cuda` registered
without modification.  That's the test for whether the abstraction
held.

## Reading order

To understand MX06 in full, read:

1. **MX00** — narrow-waist architecture
2. **MX03** — executor protocol (wire format, capability declaration)
3. **MX04** — runtime (how the planner picks executors)
4. **MX05** — tiered specialisation runtime (how MX06 plugs into the
   sampler / cache / install / dispatch loop)
5. **`matrix-metal` README** — the closest sibling and the template
   MX06 follows almost verbatim

## Architecture

`matrix-cuda` mirrors `matrix-metal`'s crate layout, swapping Metal
for CUDA at every device-touching layer:

```
┌──────────────────────────────────────────────────────────────────┐
│ matrix-cuda                                                       │
│  ┌────────────┐  ┌────────────┐  ┌──────────────┐  ┌──────────┐  │
│  │  BufferStore│  │   kernels  │  │  cuda_emitter │  │ specialised_table │
│  │ (cuMemAlloc │  │ (handwritten │  │ (CUDA C → PTX │  │  (HashMap     │
│  │  + cuMemcpy)│  │  PTX/cubin   │  │  via NVRTC)   │  │  handle →    │
│  │              │  │  per op)     │  │               │  │   kernel)    │
│  └────────────┘  └────────────┘  └──────────────┘  └──────────┘  │
└──────────────────────────────────────────────────────────────────┘
            │              │              │             │
            ▼              ▼              ▼             ▼
┌──────────────────────────────────────────────────────────────────┐
│  executor-protocol (MX03) — `Executor` impl + capability decl     │
└──────────────────────────────────────────────────────────────────┘
            │
            ▼
┌──────────────────────────────────────────────────────────────────┐
│  matrix-runtime planner (MX04) sees a third `ExecutorId = 2`      │
│  and routes per cost model.                                       │
└──────────────────────────────────────────────────────────────────┘
```

### Module breakdown

- **`buffers.rs`** — wraps CUDA driver-API `cuMemAlloc` /
  `cuMemcpyHtoD` / `cuMemcpyDtoH`.  Pool of `CUdeviceptr` keyed by
  size class to amortise allocation cost across dispatches.  Mirrors
  `matrix-metal::buffers::BufferStore`.
- **`kernels.rs`** — handwritten PTX (or CUDA C compiled with NVRTC
  at first call) for each supported MatrixIR op.  Cached per
  (`Op`, dtype, rank) tuple.
- **`cuda_emitter.rs`** — pure code generator from `SpecKey` to a
  string of CUDA C.  No device required to call this — same trick
  `msl_emitter` uses so non-NVIDIA CI runners can test the
  generator.
- **`specialised_table.rs`** — `HashMap<u64 handle, Box<closure>>`
  of installed specialised kernels.  Same shape as
  `matrix-metal::specialised_table`.
- **`dispatch.rs`** — implements the `Executor` trait from
  executor-protocol.  Routes `Dispatch` / `DispatchSpecialised` to
  the kernel cache, allocating / freeing buffers around the call.
- **`lib.rs`** — re-exports + `CudaExecutor::evict_specialised(handle)`
  for MX05 Phase 5 deopt hooks.

### V1 op coverage

V1 supports the same surface as `matrix-metal` V1:

- F32 elementwise unary: `Neg`, `Abs`, `Sqrt`, `Exp`, `Log`, `Tanh`,
  `Recip`
- F32 elementwise binary: `Add`, `Sub`, `Mul`, `Div`, `Max`, `Min`,
  `Pow`
- F32 `MatMul` (rank-2)
- `Const` (host → device upload to a fresh `CUdeviceptr`)

Everything else (integer dtypes, casts, reductions, shape ops,
comparisons, `Where`) is V2 work, identical to `matrix-metal`'s
phased plan.  The planner's capability filter falls back to
`matrix-cpu` automatically for unsupported ops.

## Platform support

Only built and *runtime-tested* on Linux + Windows + WSL2 hosts with
an NVIDIA driver and the CUDA toolkit installed.  On non-NVIDIA hosts
(macOS, NVIDIA-less Linux, CI without a GPU runner) the crate
compiles to a **stub** — no-op constructors, `evict_specialised`
returns `false`, capability declaration reports an empty op set so
the planner skips it.

This mirrors `matrix-metal`'s `#[cfg(target_vendor = "apple")]` split
and keeps the workspace `cargo build` green on every CI runner.

### Feature flag

`matrix-cuda` exposes a `cuda` Cargo feature, enabled by default
**only on platforms where CUDA is plausibly available**:

```toml
[features]
default = ["cuda"]
cuda = []
```

When the feature is off (e.g. on macOS or in a `--no-default-features`
build), the crate is a stub.  When on but the driver is missing at
runtime, the `CudaExecutor::new()` constructor returns
`Err(CudaError::DriverNotFound)` and the planner registration code
in `matrix-runtime` silently drops the executor — same fallback path
that already exists for `matrix-metal` failures on Apple machines
without a GPU.

## How the planner sees it

`matrix-runtime::register_default_executors()` becomes:

```rust
pub fn register_default_executors(rt: &mut Runtime) {
    rt.register(matrix_cpu::cpu_backend());
    #[cfg(target_vendor = "apple")]
    if let Some(metal) = matrix_metal::metal_backend() {
        rt.register(metal);
    }
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    if let Some(cuda) = matrix_cuda::cuda_backend() {
        rt.register(cuda);
    }
}
```

Three things to note:

1. **Order matters** — CPU first means it always wins ties and small
   ops stay there.
2. **Each backend's `*_backend()` is fallible** — returns `Option`
   so missing drivers / unsupported hardware just silently drop the
   executor.
3. **No user-facing config** — same principle as MX05: the user
   never opts in.

## How MX05 plugs in

`matrix-profile::SpecKey.backend_id` is `0` for CPU, `1` for Metal,
**`2` for CUDA**.  The auto-installer in `image-gpu-core` (Phase 4.3)
gains one more match arm:

```rust
match spec.key.backend_id {
    0 => matrix_cpu::build_specialised_kernel(spec)
              .and_then(|k| install_on(cpu_executor, k)),
    1 => matrix_metal::emit_specialised_kernel(spec)
              .and_then(|k| install_on(metal_executor, k)),
    2 => matrix_cuda::emit_specialised_kernel(spec)
              .and_then(|k| install_on(cuda_executor, k)),
    _ => false,
}
```

Phase 5 deopt similarly gets a third evict target:

```rust
cuda_executor.evict_specialised(handle);
```

That's it.  MX05's `SpecRouter`, `SpecCache`, `DefaultPolicy`,
`Profiler`, etc. don't know CUDA exists.

## Phases

Mirrors `matrix-metal`'s phased roll-out so each chunk can ship as
its own PR.

### Phase 1 — crate skeleton + stub

- New crate `code/packages/rust/matrix-cuda/` with `Cargo.toml`,
  `BUILD`, `BUILD_windows`, `README.md`, `CHANGELOG.md`.
- Empty `lib.rs` with `#[cfg]` stub constructors.
- Registered in `matrix-runtime`'s default executor list (behind a
  feature flag so CI without CUDA still builds).
- Test: workspace `cargo build --workspace` stays green on every
  platform.

### Phase 2 — buffer store

- `BufferStore` with size-classed pool.
- Cover `cuMemAlloc` / `cuMemFree` / `cuMemcpyHtoD` / `cuMemcpyDtoH`.
- Tests: round-trip a host buffer through device memory.

### Phase 3 — kernels.rs (generic kernels)

- Hand-write or NVRTC-compile PTX for the V1 op set.
- One kernel per (op, dtype, rank) tuple.  Cache the compiled
  `CUmodule` so repeat dispatches don't pay NVRTC cost.
- Tests: each op against a CPU oracle (already exists for
  `matrix-metal`).

### Phase 4 — `cuda_emitter.rs` (specialised kernels)

- Pure code-gen, callable on every platform.
- Takes a `SpecKey`, returns CUDA C source string.
- Mirrors `msl_emitter`'s constant-folding and range-narrowing
  logic.
- Tests: snapshot the emitted string for fixed `SpecKey`s.

### Phase 5 — specialised_table + Executor impl

- `specialised_table.rs` keyed by handle.
- `dispatch.rs` implements `Executor`.
- Capability declaration covers V1 op set.
- Tests: dispatch round-trip on a real CUDA device (gated on
  `MATRIX_CUDA_TESTS=1` env var so CI without a GPU skips).

### Phase 6 — MX05 hooks

- Add `backend_id = 2` arm to `image-gpu-core::try_auto_install_specialised`.
- Add `cuda_executor.evict_specialised(handle)` call in
  `image-gpu-core::scan_and_deoptimise`.
- Tests: end-to-end specialisation install + deopt on CUDA.

### Phase 7 — planner integration

- Cost-model coefficients for CUDA: transfer cost, compute speedup,
  per-op overhead.  Tunable per device generation; V1 ships a
  reasonable default for compute capability 7.0+ (Volta and newer).
- Tests: planner routes big matmuls to CUDA when registered, falls
  back to CPU on tiny ops, same shape as the existing Metal tests.

## Backend abstraction

V1 of `matrix-cuda` talks to the CUDA driver API directly via FFI to
`libcuda.so` / `nvcuda.dll`.  We do **not** depend on `cudarc` /
`rust-cuda` / `cust` because:

1. The op set is tiny — we only need `cuMemAlloc`, `cuMemcpyHtoD`,
   `cuMemcpyDtoH`, `cuMemFree`, `cuModuleLoadData`,
   `cuModuleGetFunction`, `cuLaunchKernel`, `cuStreamCreate`,
   `cuStreamSynchronize`, plus a handful of context-management
   calls.  ~15 FFI declarations total.
2. Pulling in a wrapper crate ties our minimum CUDA version and our
   error-handling style to theirs.  We already shipped a clean
   minimal wrapper for Metal; we'll do the same for CUDA.
3. Less supply-chain surface for the cryptographic-free hot path.

If a third-party wrapper later proves a clear win (e.g. tensor-core
intrinsics that are awkward to write by hand), the abstraction is
internal to the `kernels.rs` / `cuda_emitter.rs` modules — easy to
swap.

## Cost model coefficients

Phase 7 picks defaults along the same axes as Metal:

- `transfer_in_per_byte` ≈ host→device bandwidth (PCIe gen 4: ~25
  GB/s) — typically ~10× Metal's unified memory cost.
- `transfer_out_per_byte` — same.
- `compute_speedup_factor` — per-op multiplier vs CPU.  MatMul at
  large rank: 30–100×.  Elementwise: 5–20×.
- `fixed_per_dispatch_overhead` — kernel-launch latency, ~5–10
  microseconds on modern hardware.

These are tunable via `Runtime::set_executor_cost_coeffs()` from
MX04 — same API the planner already exposes for Metal.

Crossover from CPU to CUDA happens around the same tensor-size
thresholds Metal sees, give or take half an order of magnitude,
because PCIe transfer cost dominates more than Apple unified memory
does.

## Testing strategy

Three rings, matching `matrix-metal`'s playbook:

1. **Code-generation tests** run everywhere.  `cuda_emitter` is a
   pure function; snapshot tests pin the emitted source for a fixed
   `SpecKey`.
2. **Device tests** run only when `MATRIX_CUDA_TESTS=1` is set in
   the environment.  CI sets this on the NVIDIA-runner job and
   leaves it unset elsewhere.
3. **Above-layer tests** (`image-gpu-core`, `instagram-filters`)
   pick up CUDA automatically once registered; no per-test wiring
   needed.  This is the proof the abstraction held.

GitHub Actions: add a `build (ubuntu-latest, cuda)` matrix entry
gated on `runs-on: self-hosted-nvidia` (or `nvidia/cuda` container)
once we have a GPU runner.  Until then, only ring 1 runs in CI and
the others are local-developer-only.

## Security model

CUDA driver calls happen entirely in-process.  No network surface,
no privileged ops, no untrusted code execution.  The PTX we ship is
either hand-written or generated from `cuda_emitter.rs`; user data
flows through device buffers we allocated.  Same trust model as
`matrix-metal`.

NVRTC compiles **only strings we generated ourselves** — user input
is data, not code.  No template-injection path: `SpecKey` only
contains numeric fields, never user-supplied strings.

## Backward compatibility

This is an additive PR.  Every existing test must continue to pass
unchanged.  Adding `matrix-cuda` to `Runtime::register_default_executors`
should be invisible to users who don't have a CUDA-capable device.

Existing behaviour:

- macOS: only `matrix-cuda::cuda_backend()` is stubbed; no change.
- Linux without NVIDIA driver: `CudaExecutor::new()` returns
  `Err(CudaError::DriverNotFound)`; planner registration silently
  drops it; behaviour identical to today (CPU only).
- Linux with NVIDIA driver: CUDA shows up as a third executor, big
  ops migrate, small ones stay on CPU per cost model.

## Out of scope (V1 of MX06)

- **Multi-GPU.**  V1 picks the first device that responds to
  `cuDeviceGet(0)`.  V2 can shard.
- **Tensor cores / cuBLAS / cuDNN.**  V1 writes plain CUDA kernels.
  Once we have a real workload that justifies it we can layer in
  cuBLAS for MatMul.
- **Streams + overlap.**  V1 does synchronous dispatches.  V2 can
  pipeline H2D / compute / D2H per stream.
- **FP16 / BF16 / FP8.**  V1 is F32 only, same as Metal V1.
- **HIP / ROCm port.**  AMD GPUs would warrant a separate `matrix-rocm`
  crate — they share the source language (HIP is API-compatible with
  CUDA at the source level) but the driver and binary toolchain
  differ.  Future MX08 work.
- **Auto-tuning of block / grid sizes per shape.**  V1 uses fixed
  launch configs.

## Open questions

1. **Which CUDA driver version to require?**  Driver 12.x covers
   Ada Lovelace + Hopper; older drivers miss some PTX features.
   V1 proposal: require driver ≥ 11.4 (covers Turing onward, ~6
   years of cards as of 2026).  Anything older falls back via the
   `DriverNotFound` path.

2. **NVRTC vs ship-precompiled PTX.**  Phase 3 starts with NVRTC
   (simpler, hot-reload friendly).  If startup cost is a problem
   we can pre-compile PTX at build time.

3. **How does the bench CLI from MX05 phase-out know it ran on
   CUDA?**  `image_gpu_core::last_executor()` already returns the
   executor name; `matrix-cuda` will report `"matrix-cuda"`.  No
   change needed.

4. **What's the right `transfer_in_per_byte` default for PCIe gen
   5 systems (Lovelace + Sapphire Rapids)?**  Open — depends on
   how common these systems are by the time MX06 lands.  Phase 7
   ships PCIe gen 4 defaults and exposes the tuning knob.

## Cross-references

- **MX00** — narrow-waist architecture
- **MX01** — `matrix-ir::Op` wire tags consumed by `matrix-cuda`
- **MX02** — `compute-ir::ComputeGraph` is what the planner hands us
- **MX03** — executor-protocol (the trait MX06 implements)
- **MX04** — runtime planner / cost model
- **MX05** — tiered specialisation runtime (MX06 plugs into its
  `Specialiser` interface)
- **`matrix-metal` package** — the closest sibling implementation
- **Future MX07** — kernel fusion across adjacent ops (mentioned in
  MX05); will benefit both `matrix-metal` and `matrix-cuda`
- **Future MX08** — `matrix-rocm` (AMD GPUs) — uses the same
  template but talks HIP instead of CUDA
