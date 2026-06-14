# DSP01 — Fast Fourier Transform (FFT / IFFT)

## Status

Draft.  V1 spec.  First concrete primitive in the DSP layer
([DSP00](DSP00-signal-processing-overview.md)).  Lowers to a
`matrix_ir::Graph` composed entirely of existing MX01 ops; adds no
new IR.  Subsequent DSP primitives (DCT, STFT, FFT-based
convolution) build on this one.

## Why FFT first

DSP00's roadmap puts FFT before everything else for a reason:

- **DCT-II / DCT-III lower to FFT.**  Once DSP01 ships, DSP02 is
  a thin wrapper.
- **FFT-based convolution** (the `> 64-tap` switchover in DSP04)
  is `O(N log N)` instead of `O(N · K)` — without FFT, DSP04 has
  no fast path.
- **STFT / spectrograms** (DSP05) are batched FFTs with windowing.
- The twiddle-factor table is the **canonical
  `RangeClass::Constant` workload** MX05 was designed to fold.
  Building FFT gives MX05 a real consumer beyond the synthetic
  benchmarks in MX05 Phase 5.

## What this spec covers

- The algorithmic choice (radix-2 Cooley-Tukey + Bluestein for
  arbitrary lengths).
- The public Rust API — function signatures, error types, the
  `ComplexTensor` helper from DSP00.
- How the FFT lowers into a `matrix_ir::Graph` of existing ops
  (twiddle tables as `Const`, butterflies as paired Mul/Add).
- Batched / multi-axis variants.
- Numerical-accuracy contract.
- A five-phase rollout, one PR per phase, mirroring the MX06
  playbook.

## Reading order

1. **MX01** — `matrix_ir::Op` vocabulary (`Add`, `Sub`, `Mul`,
   `MatMul`, `Const`, `Reshape`, `Transpose`).  Every FFT
   butterfly is a composition of these.
2. **MX02** — `ComputeGraph` placed-op format.
3. **MX05** — specialisation tier.  Phase 5 of DSP01 plugs in.
4. **DSP00** — layer-level conventions (interleaved complex,
   stateless primitives, etc.).
5. **This document**.

## Algorithm choice

### Radix-2 Cooley-Tukey for power-of-two N

Standard decimation-in-time radix-2 FFT.  For input length
`N = 2^k`:

```
for stage in 0..k:
    half = 1 << stage
    full = half * 2
    twiddle[j] = exp(-2πi · j / full) for j in 0..half
    for block_start in (0..N).step_by(full):
        for j in 0..half:
            t = twiddle[j] * x[block_start + j + half]
            x[block_start + j + half] = x[block_start + j] - t
            x[block_start + j]        = x[block_start + j] + t
```

Bit-reversal permutation is folded into the input gather step;
output is in natural order.

**Why radix-2 first**:

- Simplest correct implementation.  ~50 lines of scalar reference
  Rust we can sanity-check against.
- Twiddle tables for power-of-two sizes are exactly the data
  MX05's `Constant`-folding policy expects.
- 90% of real workloads (audio at 1024 / 2048 samples; image
  pyramids at 256 / 512 px; ML batches at 128 / 256) are power
  of two.

### Bluestein's algorithm for arbitrary N

Phase 3 work.  Recasts an `N`-point FFT as a convolution of length
`2N - 1`, padded out to the next power-of-2 ≥ `2N - 1`, then
internally calls the radix-2 path on the padded sequence.

We **do not** ship mixed-radix FFT (radix-3, radix-5, split-radix)
in V1.  Bluestein has slightly worse constants but handles every
length with one code path, and the constants stop mattering once
MX05 specialisation lands.

### Direction parameter

Forward FFT is `Σ_n x[n] · exp(-2πi · k · n / N)`.  Inverse FFT
is the same sum with `exp(+2πi · k · n / N)` and a `1/N` scale.

We expose `fft` and `ifft` as separate functions rather than a
boolean direction flag.  Mirrors `numpy`, `scipy.fft`, and
`rustfft`.

### Normalization convention

| Convention      | Forward scale | Inverse scale | Used by           |
| --------------- | ------------- | ------------- | ----------------- |
| Backward (V1)   | `1`           | `1/N`         | numpy default, scipy default, MATLAB |
| Forward         | `1/N`         | `1`           | -                 |
| Symmetric       | `1/√N`        | `1/√N`        | DFT-as-unitary    |

V1 ships "backward" (the most common default; numpy / scipy / MATLAB
agree).  A future `FftOptions::normalization` enum can add the
others when a real consumer asks.

## Public API

Lives in a new crate **`dsp-fft`** with a companion
**`dsp-complex`** crate for the shared `ComplexTensor` helper
(DSP02–DSP05 will reuse it).

### `dsp-complex`

```rust
/// A tensor whose last axis is size-2 (real, imag).  Lives on the
/// device chosen by the planner; behaves like any other Tensor
/// otherwise.
pub struct ComplexTensor {
    /// Interior shape is `[.., 2]`; the trailing `2` is the
    /// `[real, imag]` axis.
    inner: Tensor,
}

impl ComplexTensor {
    pub fn from_real(real: Tensor) -> Self;
    pub fn from_real_imag(real: Tensor, imag: Tensor) -> Result<Self, _>;
    pub fn real(&self) -> Tensor;
    pub fn imag(&self) -> Tensor;
    pub fn magnitude(&self) -> Tensor;
    pub fn phase(&self) -> Tensor;
    pub fn conjugate(&self) -> Self;
    pub fn shape_without_complex_axis(&self) -> &[u32];
}
```

The helpers above produce small MatrixIR graphs — `real()` is a
slice on the last axis at index 0; `magnitude()` is
`sqrt(re² + im²)`, etc.  They run on whatever backend the planner
picks.

### `dsp-fft`

```rust
/// Compute the forward 1-D FFT along the last axis of `signal`.
///
/// - `signal` may be real (shape `[..., N]`, dtype F32) or
///   complex (shape `[..., N, 2]`, dtype F32).  Real inputs are
///   wrapped as ComplexTensor with imag=0 before transform.
/// - Returns a ComplexTensor of shape `[..., N, 2]`.
/// - V1 supports any N ≥ 1.  Power-of-two N is the fast path
///   (radix-2); other N is Bluestein (Phase 3).
///
/// `rt` is the matrix-runtime that hosts the resulting graph.
pub fn fft(rt: &Runtime, signal: &Tensor) -> Result<ComplexTensor, FftError>;

/// Inverse FFT.  Inverse of `fft` — `ifft(fft(x))` returns `x`
/// (within tolerance, see "Numerical accuracy" below).
pub fn ifft(rt: &Runtime, spectrum: &ComplexTensor) -> Result<ComplexTensor, FftError>;

/// Real-input FFT.  Exploits conjugate symmetry: only the first
/// `N/2 + 1` bins are returned, since `X[k] = conj(X[N - k])`
/// for real input.  Saves nearly 50% of memory and compute.
///
/// Input shape `[..., N]` → output shape `[..., N/2 + 1, 2]`.
pub fn rfft(rt: &Runtime, signal: &Tensor) -> Result<ComplexTensor, FftError>;

/// Inverse real FFT.  Takes the `N/2 + 1` half-spectrum back to
/// a real signal of length `N`.  Output is a real Tensor, not
/// ComplexTensor.
pub fn irfft(
    rt: &Runtime,
    half_spectrum: &ComplexTensor,
    output_length: u32,
) -> Result<Tensor, FftError>;

#[derive(Debug)]
pub enum FftError {
    /// Last axis is empty or has dtype != F32.
    InvalidInput(String),
    /// `irfft`'s explicit length is incompatible with the input
    /// half-spectrum size.
    LengthMismatch { half_spectrum_len: u32, requested: u32 },
    /// Graph construction or planner returned an error.
    Compute(matrix_runtime::PlannerError),
}
```

Notes:

- Every function takes `&Runtime`.  We don't expose a
  graph-builder-only path in V1 — that's a future optimisation
  for callers that want to pre-plan and cache the FFT graph
  outside the hot loop.  When that need arrives we'll add
  `fft_graph(signal_shape) -> Graph` siblings.
- Errors are typed.  No panics on malformed input.

## Lowering to MatrixIR

The radix-2 FFT becomes a sequence of `log₂(N)` butterfly stages.
Each stage is the same shape — only the twiddle indices and the
block stride differ.

Per stage, the lowered graph looks like:

```
// inputs at this stage: x[..., N, 2] (complex tensor)
// twiddle constant: w[..., half, 2]  (precomputed at graph build)

// 1. Split x into the "even" and "odd" halves for this stage.
//    This is a Reshape + Transpose + Slice combo that picks
//    indices 0..half and half..full from each block.
even = slice(x, axis = -2, range = 0..half)
odd  = slice(x, axis = -2, range = half..full)

// 2. Complex-multiply odd by twiddle.  On interleaved [re, im]:
//    (a + bi)(c + di) = (ac - bd) + (ad + bc)i
//    Implemented as four Mul + two Sub/Add over the real/imag
//    components.
t_re = odd.re * w.re - odd.im * w.im
t_im = odd.re * w.im + odd.im * w.re

// 3. Combine into next-stage layout.
next_first  = (even.re + t_re, even.im + t_im)
next_second = (even.re - t_re, even.im - t_im)

// 4. Concatenate back to a single tensor.
next_x = concat(next_first, next_second, axis = -2)
```

After `log₂(N)` stages, `x` is the spectrum (with bit-reversal
applied via an initial Gather op; see "Bit-reversal" below).

Twiddle tables are `Const` tensors built once at graph build:

```rust
fn twiddles(n: u32, direction: Direction) -> Vec<f32> {
    let sign = match direction { Forward => -1.0, Inverse => 1.0 };
    let mut t = Vec::with_capacity((n as usize / 2) * 2);
    for k in 0..(n / 2) {
        let theta = sign * 2.0 * PI * (k as f32) / (n as f32);
        t.push(theta.cos());  // real
        t.push(theta.sin());  // imag
    }
    t
}
```

The constant payload is exactly the kind MX05's policy folds:
identical bytes across every dispatch, ranges identical, slot
stable.  Phase 5 of this spec wires the folded-twiddle hook.

### Bit-reversal

V1 implements bit-reversal as a `Gather` op (an existing MX01
shape op): build a `[N]`-shaped `u32` index tensor at graph build
time and gather from `x` through it as the first stage.

This adds one transfer to the graph but no kernel — the gather is
already a primitive matrix-cpu / matrix-metal / matrix-cuda
supports.

### Real-FFT optimisation

`rfft` uses the standard pack-and-unpack trick:

1. Treat an even-length real signal `x[0..N]` as a complex
   signal `y[k] = x[2k] + i · x[2k + 1]` of length `N/2`.
2. Compute a length-`N/2` complex FFT of `y` (one stage cheaper
   than a full-length transform).
3. Unpack the `N/2 + 1` real-FFT bins from the length-`N/2`
   complex result via additional butterfly stage.

Phase 4 work; the spec just records the algorithm choice.

## Batched FFT

The public API treats the FFT axis as "the last axis", same as
numpy.  All preceding axes are batch axes:

- Input `[B, N]` → output `[B, N, 2]`.
- Input `[B0, B1, N]` → output `[B0, B1, N, 2]`.

The graph builder broadcasts the twiddle table across batch axes
implicitly; the kernel is the same.  Batched FFT is the dominant
workload (per-channel audio FFTs, per-frame STFT, multi-image
analysis), so V1 supports it from day one.

## Numerical-accuracy contract

For length `N ≤ 64K`, dtype F32:

- **Forward-then-inverse round-trip**:
  `‖ifft(fft(x)) - x‖∞ ≤ 1e-5 · ‖x‖∞`
- **Impulse spectrum**: `fft([1.0, 0.0, …, 0.0]) ≈ [1.0, 1.0, …]`
  exactly (within ULP rounding of the constant 1.0).
- **DC spectrum**: `fft([1, 1, …, 1]) ≈ [N, 0, …, 0]`
  exactly.
- **Single-bin sinusoid**:
  `fft(cos(2π · k₀ · n / N))` has magnitude `N/2` concentrated at
  bins `±k₀`, magnitude `< 1e-4 · N` elsewhere.

Cross-backend (CPU vs Metal vs CUDA) agreement: within `1e-4`
relative tolerance (Apple-Silicon Metal and consumer NVIDIA
GPUs differ on f32 transcendentals by a few ULPs per stage; over
`log₂(N) = 16` stages the error accumulates).

For larger `N` (or post-V1 f64), tolerance budgets are
spec-extended.

## Phased rollout

One PR per phase, mirroring MX06's playbook.  Each phase is
self-contained and passes CI on every platform.

| Phase | Lands | Risk |
| ----- | ----- | ---- |
| **1** | **This spec.** Docs only. | Low. |
| 2 | `dsp-complex` crate (ComplexTensor helper) + `dsp-fft` crate skeleton with **scalar reference implementation** of `fft` / `ifft` for power-of-2 sizes.  Property tests for round-trip + known vectors.  No matrix lowering yet — reference is pure-Rust f32 math.  This gives us a known-good oracle for Phase 3 onwards. | Low. |
| 3 | `dsp-fft` lowers `fft` / `ifft` to a `matrix_ir::Graph`.  Power-of-2 sizes only.  Cross-backend tests (CPU baseline; Metal + CUDA when available).  Bluestein still not landed. | Medium — first real graph build outside MX. |
| 4 | Arbitrary lengths via Bluestein (uses Phase 3's power-of-2 path internally).  `rfft` / `irfft`. | Medium. |
| 5 | MX05 specialisation hook: a `Specialiser` impl that produces fully-unrolled butterfly kernels for canonical sizes (N ∈ {8, 16, 32, 64, 128, 256, 512, 1024}) with twiddle factors baked in as literal floats.  Same emitter pattern `cuda_emitter` / `msl_emitter` use.  Optional — non-canonical sizes continue to use the generic graph from Phase 3. | Medium-high (NVRTC / MSL code gen). |

A V2 spec (post-V1) covers: mixed-radix, GPU-tile-optimised
butterflies, FP16 input precision, streaming FFT (overlap-save
for very long inputs), distributed multi-device FFT.  None are
in scope for V1.

## Crate layout

```
code/packages/rust/
  dsp-complex/          ← Phase 2
    Cargo.toml
    src/lib.rs            ← ComplexTensor struct, real/imag/magnitude/phase
    src/tests/          ← property tests
    CHANGELOG.md
    BUILD / BUILD_windows
    required_capabilities.json   ← (none — pure Rust over matrix-ir)
    README.md

  dsp-fft/              ← Phase 2 onward
    Cargo.toml
    src/lib.rs            ← public fft / ifft / rfft / irfft
    src/scalar.rs         ← Phase 2 scalar reference (oracle for tests)
    src/radix2.rs         ← Phase 3 matrix-ir lowering for power-of-2
    src/bluestein.rs      ← Phase 4 arbitrary-length wrapper
    src/specialiser.rs    ← Phase 5 MX05 hook
    src/tests/
    CHANGELOG.md
    BUILD / BUILD_windows
    required_capabilities.json
    README.md
```

Both crates added to `code/packages/rust/Cargo.toml` workspace
members alphabetically (`dsp-complex` and `dsp-fft` land between
`dot-parser` and `excel-lexer`).

### Dependencies

- `dsp-complex`: `matrix-ir`, `matrix-runtime`.  Nothing else.
- `dsp-fft`: `matrix-ir`, `matrix-runtime`, `dsp-complex`.
  No FFI, no `unsafe`, no external crates.  Twiddle math uses
  `f32::cos` / `f32::sin` from `core` (or `std` if needed).

## Out of scope for DSP01 V1

- **Mixed-radix FFT.**  Bluestein covers it via padding; the
  constants are slightly worse but identical-shape kernels.
- **F64 support.**  Real workloads we'll ship before V2 (audio,
  image processing) use F32.  F64 is one matrix-ir dtype change
  away when needed.
- **Streaming / overlap-save.**  For audio realtime work the user
  composes `dsp-fft` with their own buffering.  DSP05 (STFT) will
  introduce an opinionated stream wrapper.
- **Hand-vectorised CPU SIMD.**  matrix-cpu's generic kernels
  are already auto-vectorised by LLVM.  Phase 5's specialisation
  emitter improves on this on metal / CUDA; CPU stays generic
  until measurements justify per-platform SIMD.
- **Halton / windowed FFTs.**  Windowing is a separate
  elementwise pre-multiply the user composes upstream.  DSP05
  will offer canned window functions.

## Open questions

1. **Where does the bit-reversal permutation index tensor live —
   in the graph as a Const, or built lazily by the executor?**
   V1: as a `Const`.  Folded into the graph at build time so the
   planner can pin it to the device.  Future work could share
   the permutation across calls of the same N.

2. **Should we expose a "no-bit-reversal" output mode** (so a
   caller doing a frequency-domain operation followed by an
   inverse FFT can skip both bit-reversals)?  Not in V1.  The
   `fft_graph` shape will let advanced callers do this manually
   in a future spec.

3. **How does Phase 5's specialiser interact with batched
   inputs?**  V1: the emitter produces a kernel parameterised on
   batch axes via a flat `bytes` argument.  Twiddles are folded;
   batch loop iterates at runtime.  We may revisit if a real
   workload shows this is the bottleneck.

## Cross-references

- **MX00** — narrow-waist architecture.
- **MX01** — `matrix_ir::Op` vocabulary used in the lowering.
- **MX04** — planner.  Picks per-stage placement.
- **MX05** — specialisation tier.  Phase 5 plugs in.
- **MX06** — `matrix-cuda` will run the specialised kernels on
  NVIDIA hardware (once Phase 5 lands).
- **[DSP00](DSP00-signal-processing-overview.md)** — layer
  overview.
- **Future DSP02** — DCT, built on this spec.
- **Future DSP04** — convolution, switches to FFT-based for
  large kernels.
- **Future DSP05** — STFT, batched repeat-FFT with windowing.
