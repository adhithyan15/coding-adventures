# DSP00 — Signal Processing Layer (overview)

## Status

Draft.  V1 spec.  Sits **above** the matrix execution layer
([MX01–MX06](MX00-matrix-execution-overview.md)) and **below** any
future computer-vision / audio-processing / scientific-analysis
facades the user builds on top.  Adds no new ops to MX01's
`matrix-ir`; instead, DSP primitives compose existing matrix ops
into the textbook transforms (FFT, DCT, FIR/IIR filters, …).

## Why this layer exists

MX01–MX06 give us:

- A tensor IR and planner that can fan compute out to CPU / Apple
  GPU / NVIDIA GPU.
- A specialisation tier (MX05) that folds runtime-stable constants
  into hot kernels.

Signal-processing workloads sit naturally on top of that:

- Almost every DSP primitive is a sequence of dense linear-algebra
  operations (matrix multiplies, complex multiplies, elementwise
  ops, gather/scatter).  These are exactly the ops MX optimises.
- Twiddle-factor tables, filter taps, and window functions are
  **stable across calls** — exactly the constant-folding pattern
  MX05's policy was built for.  An FFT loop running a thousand
  times re-uses the same twiddle table every iteration.
- DSP workloads benefit massively from GPU when they're big
  (large FFTs, batched STFTs, image-domain filtering).  MX's
  cost-model planner already picks the right backend per call.

What this layer adds: **a vocabulary of named DSP primitives that
internally produce MatrixIR graphs.**  Users see
`fft(x)` / `dct(x)` / `convolve(x, h)`; the runtime sees the same
generic matrix ops we already know how to schedule.

## What this layer is NOT

- **Not a new IR.**  We do not extend `matrix-ir` with FFT /
  convolution / filter ops.  Adding domain-specific ops to a
  general-purpose tensor IR is a trap (each new op needs lowering
  on every backend, breaks the narrow-waist contract).  DSP
  primitives are *libraries that emit MatrixIR graphs*, the same
  way `image-gpu-core` emits MatrixIR for filters.
- **Not a planner.**  The MX04 planner is the one place we make
  placement decisions.  DSP primitives accept a `&Runtime` and
  return a `Graph`; whoever calls them decides whether to run it,
  cache it, profile it, or replan it.
- **Not a wholesale audio library.**  We don't ship realtime
  audio I/O, format codecs, or DAW-style effects routing.  Those
  layers (`audio-device-*`, `wave`, future audio-graph) consume
  DSP primitives the same way `image-gpu-core` consumes
  `matrix-runtime`.
- **Not stateful.**  Every primitive is a pure function from input
  tensors to a `Graph` describing the transform.  Stateful
  things — IIR filter memory between calls, STFT overlap-add
  state — live in thin wrapper structs that hold the underlying
  pure primitives plus a `Vec<f32>` of carry-over samples.

## Reading order

1. **MX00** — the narrow-waist architecture this layer is built on.
2. **MX01–MX03** — the IR, the planner-facing wire format, and the
   ops we'll compose.
3. **MX05** — the specialisation tier that will fold twiddle tables
   and filter taps into hot kernels.
4. **This document** — the layer overview.
5. **DSP01 onwards** — per-primitive specs (each a single PR).

## Architecture

```
┌────────────────────────────────────────────────────────────────┐
│ User code                                                       │
│   let spectrum = dsp_fft::fft(&runtime, signal)?;               │
│   let filtered = dsp_filters::iir_lowpass(&runtime, sig, cutoff)?;
└─────────────────┬──────────────────────────────────────────────┘
                  │  builds matrix_ir::Graph
                  ▼
┌────────────────────────────────────────────────────────────────┐
│ DSP primitive crates (this layer)                              │
│  ┌──────────┐ ┌──────────┐ ┌──────────────┐ ┌────────────────┐ │
│  │ dsp-fft  │ │ dsp-dct  │ │ dsp-filters  │ │ dsp-conv       │ │
│  │ (DSP01)  │ │ (DSP02)  │ │ (DSP03)      │ │ (DSP04)        │ │
│  └──────────┘ └──────────┘ └──────────────┘ └────────────────┘ │
└─────────────────┬──────────────────────────────────────────────┘
                  │  matrix_ir::Graph
                  ▼
┌────────────────────────────────────────────────────────────────┐
│ matrix-runtime planner + executors (MX01–MX06)                  │
└────────────────────────────────────────────────────────────────┘
```

Every DSP crate is:

- A thin, **pure-Rust** library with no FFI of its own.
- Returns a `Graph` (or executes it through a `&Runtime` if the
  caller passes one).
- Self-contained tests: bit-exact CPU oracle in pure Rust,
  property tests via `proptest`/`quickcheck` for round-trips
  (e.g. ifft(fft(x)) ≈ x).
- No special CI dependencies — runs on every platform, exercises
  whatever backends the host has via MX.

## Naming convention

Crates: `dsp-<primitive>` (`dsp-fft`, `dsp-dct`, `dsp-filters`, …).

Public functions: noun-named primitives where the math is canonical
(`fft`, `ifft`, `dct2`, `dct3`), verb-named when they take config
parameters (`iir_lowpass(rt, sig, cutoff)`, `fir_design(taps,
window)`).

## Complex-number convention

DSP needs complex numbers.  Two options:

| Option | Storage | Pros | Cons |
| --- | --- | --- | --- |
| **A. Interleaved f32** | Real signal `[N]` → complex spectrum `[N, 2]` (last axis is `[real, imag]`) | Reuses every MX op as-is; no IR change | Manual `[:, 0]` / `[:, 1]` indexing in user code; AoS layout pessimises some GPU kernels |
| **B. Planar f32** | Two parallel `[N]` tensors `re` and `im` | Better memory pattern for GPU; aligns with cuFFT and Apple's vDSP | Every primitive's signature returns two tensors; harder to compose with generic matrix ops |
| **C. New `Complex32` dtype** | Single tensor `[N]`, dtype = Complex32 | Cleanest user API | Cross-cutting change to `matrix-ir`, every executor, every emitter, every cost model |

**Decision: V1 uses option A (interleaved).**  Justification:

- Zero churn to MX01–MX06.  Every executor already handles `f32
  [N, 2]` without any change.
- The cost-model penalty vs option B is small for V1 sizes
  (≤ 64K-point FFT).  When that becomes the bottleneck we revisit.
- Option C is the right long-term target, but the IR change is
  too disruptive to land alongside FFT primitives.  Filed as a
  future spec (DSP-future-complex-dtype).

User-facing helpers will mask the indexing:

```rust
pub struct ComplexTensor { real_imag: Tensor /* [..., 2] */ }
impl ComplexTensor {
    pub fn real(&self) -> Tensor;
    pub fn imag(&self) -> Tensor;
    pub fn magnitude(&self) -> Tensor;
    pub fn phase(&self) -> Tensor;
}
```

So user code reads `spectrum.magnitude()`, not `slice spectrum on
axis -1 at index 0 and 1, square, sum, sqrt`.

## Roadmap

V1 ships four primitive families.  Each gets a spec PR, then phased
implementation PRs the way MX06 did.  Order chosen for dependency
flow — FFT unlocks DCT, STFT, fast convolution; filters depend on
none of the others.

| Spec  | Primitive                                            | Phases planned | Rationale |
| ----- | ---------------------------------------------------- | -------------- | --------- |
| DSP01 | FFT / IFFT (radix-2 Cooley-Tukey + Bluestein)         | 3 (kernel, integration, perf) | Foundation: DCT and STFT lower to FFT in V2. |
| DSP02 | DCT-II / DCT-III (via FFT)                            | 1              | JPEG / audio compression workloads. |
| DSP03 | FIR + IIR filters (Direct Form I, biquad cascades)    | 2              | Audio / sensor processing.  No FFT dependency. |
| DSP04 | 1-D and 2-D convolution / correlation (time-domain + FFT-based switchover) | 2 | Bridge to image processing; planner picks time vs frequency domain by length. |
| DSP05 | (future) STFT / spectrograms                          | tbd            | Built on DSP01.  Audio analysis. |
| DSP06 | (future) Wavelets                                     | tbd            | Lower priority. |

## Backend strategy per primitive

Within MX's existing framework:

- **First implementation always lowers to generic matrix ops.**
  This guarantees correctness across CPU / Metal / CUDA without
  per-backend code.
- **Hot primitives get specialised emitters in MX05's vocabulary.**
  A 1024-point FFT with stable twiddle tables is exactly the
  workload MX05's `RangeClass::Constant` was designed for —
  the twiddle bytes are unchanging across thousands of calls.
- **Pre-baked transforms** for canonical sizes (FFT-8 / FFT-16
  with literal-folded twiddles) are a Phase 3 optimisation.
  Reuse the `cuda_emitter` / `msl_emitter` pattern from MX06 /
  MX05 Phase 4.

DSP primitives do **not** introduce new backend-specific code.
Every optimisation rides on MX's existing infrastructure.

## Versioning and stability

- The user-facing function signatures (e.g. `fft(rt, signal)
  -> Result<ComplexTensor, ...>`) are stable from v1.0.
- The `Graph` shape any primitive produces is **not stable** —
  callers must not rely on op-by-op identity.  Future versions
  may reorganise the graph to expose more parallelism, more
  fusion opportunity, or different specialisation hooks.
- Behaviour is bit-exact within a backend across patch releases;
  cross-backend results may differ within a documented ULP
  tolerance (FFT is famously sensitive; DSP03 IIR cascades
  even more so).

## Test strategy

- **Pure-Rust scalar reference** per primitive.  Lives alongside
  the matrix-lowering implementation.  Every test compares the
  matrix-graph result to the scalar reference within tolerance.
- **Round-trip identities**: `ifft(fft(x)) ≈ x`,
  `idct(dct(x)) ≈ x`, `conv(x, identity) == x`.  Property tests.
- **Cross-backend agreement** within tolerance: run each primitive
  on CPU and (when available) Metal / CUDA; compare.  Device
  tests gate on the same env vars MX06's tests use.
- **Known-good vectors** for FFT (impulse, DC, single-bin
  sinusoids) and DCT (JPEG reference matrices) — these are the
  canonical sanity checks every DSP library ships.

## Cross-references

- **MX00** — narrow-waist architecture this layer sits on.
- **MX01** — ops we compose.
- **MX04** — planner that places the resulting graphs.
- **MX05** — the specialisation tier that will fold twiddle
  factors / filter taps.
- **IMG00** — image-domain library.  DSP04 (convolution) is the
  natural meeting point: image filtering and 1-D signal
  filtering share kernels.
- **Future DSP-future-complex-dtype** — `Complex32` IR addition
  (post-V1).
- **Future CV00** — computer-vision / OpenCV-equivalent facade.
  Will compose `image-gpu-core` + `dsp-conv` + `dsp-fft` + future
  feature-detection primitives.
