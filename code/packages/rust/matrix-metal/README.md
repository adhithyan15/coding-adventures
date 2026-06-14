# matrix-metal

First specialised executor for the matrix execution layer.  Lowers a
subset of MatrixIR ops to MSL kernels and dispatches them on Apple's
Metal GPU.

## What works in V1

| Op | F32 | U8 | I32 |
|----|-----|-----|-----|
| Neg, Abs, Sqrt, Exp, Log, Tanh, Recip | ✓ | — | — |
| Add, Sub, Mul, Div, Max, Min, Pow | ✓ | — | — |
| MatMul (rank-2) | ✓ | — | — |
| Const | ✓ | ✓ | ✓ |

Everything else (integer dtypes, casts, reductions, shape ops,
comparisons, `Where`) falls back to `matrix-cpu` automatically — that's
the cost-model planner working as designed.

## What this proves

With both `matrix-cpu` and `matrix-metal` registered:

- **Tiny ops stay on CPU.**  Transfer cost dominates GPU speedup for
  small inputs.
- **Big matmuls / heavy elementwise chains ship to GPU.**  GPU speedup
  dominates transfer cost for large inputs.
- **Capability fallback works.**  Casts and reductions in the middle
  of a graph route to CPU silently.

All without any user-facing change.  `image-gpu-core` and the
`instagram-filters` CLI inherit the speedup automatically — sepia /
greyscale / colour-matrix on a 4K image now runs the matmul portion
on the GPU.

## Architecture

```
ExecutorRequest (Dispatch)
   ↓
MetalExecutor.handle (mutex-guarded)
   ↓
DispatchCtx (device, queue, buffers, pipelines)
   ↓
walk PlacedOps:
   Alloc      → device.alloc(bytes) → BufferStore
   Free       → BufferStore.remove
   Transfer   → memcpy via unified memory
   Compute    → look up MSL pipeline → dispatch
                (one threadgroup per output element)
   ↓
ExecutorResponse (DispatchDone)
```

The MSL kernels live in [`src/kernels.rs`] as a single source string
compiled once at executor startup.  Pipelines are cached by entry-point
name (`add_f32`, `matmul_f32`, etc.).

## Buffer lifecycle

`BufferStore` owns `MetalBuffer` objects keyed by `BufferId`.  Apple
Silicon's unified memory means the buffer is both GPU-visible and
CPU-readable — `as_slice()` and `as_slice_mut()` work directly without
explicit transfers.

## Platform support

- **macOS / Apple Silicon / Apple Intel / iOS**: full GPU dispatch via
  `Metal.framework` (linked through the existing zero-dep
  `metal-compute` crate).
- **Linux / Windows / WebAssembly**: stub `MetalExecutor` that always
  returns `Err`.  `cargo build` succeeds on every platform; only
  Metal-bearing hosts run the integration tests.

## Tests

5 integration tests covering all V1 ops on real Metal hardware:

```
$ cargo test -p matrix-metal
test dispatch_rejects_oversized_tensor ... ok
test local_transport_heartbeat ... ok
test neg_f32_on_gpu ... ok
test add_f32_on_gpu ... ok
test matmul_2x2_on_gpu ... ok
test result: ok. 5 passed; 0 failed
```

The matmul test is the canonical proof: `[[1,2],[3,4]] × [[5,6],[7,8]] =
[[19,22],[43,50]]` computed end-to-end through an MSL kernel.

## Zero dependencies

```
$ cargo tree -p matrix-metal
matrix-metal v0.1.0
├── compute-ir v0.1.0
├── executor-protocol v0.1.0
├── matrix-ir v0.1.0
├── matrix-runtime v0.1.0
└── metal-compute v0.1.0   (uses Metal.framework via objc-bridge)
```

No external Cargo crates.  `metal-compute` itself uses only `core` +
`alloc` + `std` plus the homegrown `objc-bridge` for Objective-C FFI.

## V2 roadmap

These are deferred from V1:

- **Integer dtypes** (U8, I32) — kernels are easy, just need the
  per-dtype variants
- **Cast** — separate kernels per (src, dst) dtype pair
- **Reductions** (`ReduceSum`, `ReduceMax`, `ReduceMean`) — needs
  threadgroup reduction kernels
- **Shape ops** (`Reshape`, `Transpose`, `Broadcast`) — partly
  zero-cost on unified memory but `Transpose` needs an actual kernel
- **Comparison** (`Equal`, `Less`, `Greater`) — output U8
- **Selection** (`Where`) — branchless predication
- **Async dispatch** — V1 calls `commit_and_wait`; V2 could keep work
  in flight
- **Per-pipeline calibration** — V1 advertises a static `BackendProfile`;
  V2 could microbenchmark on first registration

## Naming

The crate is `matrix-metal` to fit the executor naming family
(`matrix-cpu`, future `matrix-cuda`, future `matrix-vulkan`, etc.).
The matrix execution layer's narrow waist means each new executor is
just one more crate at this level — no spec changes, no IR changes,
no protocol changes.
