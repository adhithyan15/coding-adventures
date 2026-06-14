# DSP02 — Discrete Cosine Transform (DCT)

**Status**: V1 spec (this document is Phase 0).

**Scope**: a new `dsp-dct` crate providing the DCT-II and DCT-III
primitives (the most-used pair — DCT-II is forward, DCT-III is
its inverse).  Optionally DCT-I and DCT-IV.  Built on top of
`dsp-fft` so it lifts to the matrix execution layer for free
once Metal / CUDA claim Slice + Concat.

## Why a DCT?

The DCT is the spectral primitive that powers most of practical
multimedia compression and analysis:

- **JPEG** — every 8×8 block goes through a 2-D DCT-II before
  quantisation.  Decoder runs the inverse (2-D DCT-III).
- **MP3 / AAC / Vorbis** — frequency-domain audio coding uses
  MDCT (modified DCT), which is built on top of a DCT-IV.
- **Speech recognition** — MFCCs (mel-frequency cepstral
  coefficients), the dominant feature vector for two decades,
  are the DCT-II of the log-mel spectrum.
- **Image hashing / pHash** — the perceptual hash used by
  Apple's CSAM scanner and most "is this image a near-duplicate"
  tools is the top-left 8×8 block of the DCT of a downscaled
  grayscale image.
- **Heat-equation solvers**, **PDE pseudospectral methods**, and
  any other "natural Neumann boundary conditions" computation.

When OpenCV / Pillow / scipy expose `dct(x)`, that's almost
always DCT-II (often "ortho" normalised), and the inverse is
DCT-III.  Our V1 will match that contract exactly.

## DCT types

All four classical DCT types are real-valued linear transforms
on real-valued input.  They differ in how they handle the
boundaries of the implicit periodic extension of the input.

For a length-`N` real input `x[n]`, `n = 0..N-1`:

### DCT-I

```text
    X[k] = Σ_{n=0..N-1}  x[n] · cos( π · n · k / (N - 1) )
```

For `k = 0..N-1`.  Boundary: even reflection at both ends with
the endpoint samples *not* duplicated.  Requires `N ≥ 2`.

### DCT-II (most common)

```text
    X[k] = Σ_{n=0..N-1}  x[n] · cos( π · k · (2n + 1) / (2N) )
```

For `k = 0..N-1`.  Boundary: even reflection at both ends with
endpoint samples duplicated (samples land at half-integer
positions).  Defined for all `N ≥ 1`.  This is what scipy /
numpy / MATLAB call simply "the DCT" by default.

### DCT-III (inverse of DCT-II under "ortho" norm)

```text
    X[k] = x[0] / 2 + Σ_{n=1..N-1}  x[n] · cos( π · n · (2k + 1) / (2N) )
```

For `k = 0..N-1`.  Boundary: odd reflection at the right end.
Under "ortho" normalisation, DCT-III is the exact inverse of
DCT-II.  This is what scipy's `idct(type=3)` and `idct` default
to.

### DCT-IV

```text
    X[k] = Σ_{n=0..N-1}  x[n] · cos( π · (2k + 1) · (2n + 1) / (4N) )
```

For `k = 0..N-1`.  Symmetric about both ends but at half-integer
shifts on each side.  Used inside the MDCT for audio coding.

## V1 scope

**Phase 1 (spec — this document)**: defines API + algorithm +
phase plan.

**Phase 2 (scalar reference)**: DCT-II and DCT-III in pure Rust,
using `dsp-fft`'s `fft_scalar` as the substrate.  Both `"none"`
(unnormalised) and `"ortho"` (orthonormal) conventions
supported, matching scipy / numpy.

**Phase 3 (matrix-ir lowered)**: emits a `matrix_ir::Graph`
that computes DCT-II / DCT-III through `dsp-fft`'s
`fft_via_runtime`, so the whole transform lifts onto the matrix
execution layer.  CPU now, GPU once Metal / CUDA claim Slice +
Concat.  Same lift story as DSP01's `fft_via_runtime`.

**Phase 4 (2-D DCT)**: `dct_2d` / `idct_2d` operating on `[H, W]`
real tensors.  Implements the standard row-then-column factorisation
(2-D DCT = 1-D DCT along axis 0 + 1-D DCT along axis 1).  This
unlocks JPEG / pHash / image-domain spectral filters.

**Phase 5 (perf)**: specialised emitters for canonical sizes
(N = 8 is JPEG's hot path; specialised 8-point DCT-II emitters
collapse the FFT-based construction into ~22 multiplies and ~28
adds — Loeffler's algorithm).

**Out of V1 scope**:

- DCT-I, DCT-IV.  Both are addressable with the same FFT-based
  construction but are far less commonly used; defer until a
  real consumer asks.
- MDCT.  Lives in DSP05 (STFT / overlap-add framework).
- Integer / fixed-point DCT.  The reference is always `f32`;
  integer kernels are MX05 territory.
- Negative N, complex input, arbitrary axis selection (we
  always operate along the last axis).

## Algorithm — DCT-II via FFT

The standard reduction (Makhoul 1980; also covered in numpy /
scipy):

1. **Pre-shuffle** the length-`N` real input `x[0..N-1]` into a
   length-`N` real sequence `y` that interleaves even and reversed
   odd samples:

   ```text
       y[n] = x[2n]            for n = 0..(N - 1) / 2
       y[N - 1 - n] = x[2n + 1] for n = 0..(N - 1) / 2
   ```

   (For even `N` this fills `y` exactly; for odd `N` the middle
   sample is just `x[N - 1]`.)

2. **FFT** the result: `Y[k] = FFT(y)[k]` for `k = 0..N-1`.

3. **Twiddle multiply + take real part**:

   ```text
       X[k] = 2 · Re( Y[k] · exp(-iπ · k / (2N)) )
   ```

   This `2 · Re(Y[k] · twiddle[k])` is real-valued by
   construction (the imaginary part cancels because of the
   pre-shuffle's symmetry).

4. **Normalise** depending on the convention requested.  For
   `"ortho"` mode (the scipy default and the only mode that
   makes DCT-III the inverse of DCT-II):

   ```text
       X[0]   *= sqrt(1 / (4N))     ← extra √2 factor for DC bin
       X[k>0] *= sqrt(1 / (2N))
   ```

   For `"none"` mode (the un-normalised form, matching
   scipy.fft.dct(type=2, norm=None) and JPEG's pre-quantisation
   coefficients):

   ```text
       no scaling — X[k] is the raw sum
   ```

## Algorithm — DCT-III via FFT

Symmetric to DCT-II: pre-shuffle, FFT, twiddle, real-part
extract, normalise.  The pre-shuffle is the inverse of DCT-II's:

1. Build a length-`N` complex sequence:

   ```text
       Y[0]   =  x[0]
       Y[k]   = (x[k] - i · x[N - k]) · exp(+iπ · k / (2N))   for k = 1..N-1
   ```

2. IFFT to get a length-`N` complex sequence `y`.

3. Un-shuffle:

   ```text
       X[2n]     = Re(y[n])           for n = 0..(N - 1) / 2
       X[2n + 1] = Re(y[N - 1 - n])   for n = 0..(N - 1) / 2
   ```

4. Normalise to match the input's convention.

`idct(dct(x))` is the identity within `1e-4` relative tolerance
for `N ≤ 64K`, f32 dtype — same contract as DSP01 / `dsp-fft`.

## Public API

Lives in a new crate **`dsp-dct`** depending on `dsp-fft` and
`dsp-complex`.

```rust
/// Which DCT variant to compute.  V1 ships II and III; I and
/// IV are deferred.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DctType {
    II,
    III,
}

/// Normalisation convention.  `Ortho` makes DCT-II and DCT-III
/// mutual inverses (and makes the transform unitary).  `None`
/// gives the un-normalised raw cosine sum (what JPEG uses
/// before quantisation).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DctNorm {
    None,
    Ortho,
}

/// Forward DCT.  `signal` is a length-`N` real f32 slice; output
/// is a length-`N` real f32 vector.
pub fn dct(signal: &[f32], dct_type: DctType, norm: DctNorm)
    -> Result<Vec<f32>, DctError>;

/// Inverse DCT.  Takes a length-`N` real f32 slice (assumed to
/// be the output of `dct` with the *same* `dct_type` and
/// `norm`), returns the recovered length-`N` real signal.
///
/// In V1 the convention is: pass DCT-III as `dct_type` to invert
/// a DCT-II forward, and vice versa (under `Ortho` norm they
/// are mutual inverses).
pub fn idct(signal: &[f32], dct_type: DctType, norm: DctNorm)
    -> Result<Vec<f32>, DctError>;

#[derive(Debug, Clone, PartialEq)]
pub enum DctError {
    InvalidInput(String),
    /// V1 (Phase 2) requires N ≥ 1.  Empty input is rejected.
    EmptyInput,
    /// Wraps `dsp_fft::FftError` from the underlying FFT call.
    Fft(String),
}
```

Phase 3 will add matrix-ir-lowered variants `dct_via_runtime`
and `build_dct_graph_with_input` analogous to `dsp-fft`'s.

Phase 4 will add:

```rust
pub fn dct_2d(image: &[f32], height: u32, width: u32,
              dct_type: DctType, norm: DctNorm)
    -> Result<Vec<f32>, DctError>;
pub fn idct_2d(image: &[f32], height: u32, width: u32,
               dct_type: DctType, norm: DctNorm)
    -> Result<Vec<f32>, DctError>;
```

operating on row-major `[H, W]` real f32 buffers.

## Normalisation reference table

| Convention | DCT-II X[0] scale     | DCT-II X[k>0] scale  | DCT-III X[k] scale  |
| ---------- | --------------------- | -------------------- | ------------------- |
| None       | 1                     | 1                    | 1                   |
| Ortho      | √(1 / (4N))           | √(1 / (2N))          | √(1 / (4N)) on X[0]; √(1 / (2N)) on X[k>0] |

When both directions use `Ortho`, `idct(dct(x)) == x` exactly
(within FP noise).  When both use `None`, you have to scale by
`2/N` somewhere (typically the inverse) to round-trip — we
document this and let the caller pick.

## Numerical accuracy contract

Same as DSP01:

- `idct(dct(x))` round-trips within `1e-5` relative tolerance
  for `N ≤ 64K`, f32 dtype, under `Ortho` normalisation.
- DCT-II of an impulse `[1, 0, …, 0]` matches the closed form
  `X[k] = cos(π · k / (2N))` (or its `Ortho`-scaled version)
  bit-for-bit modulo ULP.
- DCT-II of DC `[1, 1, …, 1]` concentrates at bin 0 (`X[0] = N`
  un-normalised; `X[0] = √N` under `Ortho`; all other bins
  near zero).

The reference implementation in Phase 2 (`dct_scalar`) is the
oracle for all later phases.

## Testing strategy

Per `CLAUDE.md` (>80% coverage target, library standard 95%+):

**Phase 2 tests (scalar reference)**:

- Error paths: empty input.
- Closed-form DCT-II:
  - Impulse → cosine sequence.
  - DC → single bin (or scaled-single-bin under Ortho).
  - `dct(linear ramp)` matches the known DCT-II of `n` (analytic
    expression involves Dirichlet-like kernels).
- Round-trip `idct(dct(x))` under both `None` (with explicit
  rescale) and `Ortho` (no rescale needed) at N ∈ {1, 2, 8, 16,
  31, 64, 100, 256}.  N = 31 and N = 100 stress non-power-of-two.
- Cross-check against scipy's reference output (stored as
  golden test vectors) for two or three representative cases.

**Phase 3 tests (matrix-ir lowered)**:

- `dct_via_runtime` matches `dct_scalar` within `1e-4` tolerance
  for N ∈ {2, 4, 8, 16}.
- Round-trip end-to-end through the runtime path.

**Phase 4 tests (2-D)**:

- 8×8 DCT-II / DCT-III round-trip (the JPEG block size).
- Cross-check against the JPEG spec's reference 8×8 DCT
  coefficients for the DC-only and impulse blocks.

## Phase plan

| Phase  | Lands                                                        | Risk                       |
| ------ | ------------------------------------------------------------ | -------------------------- |
| 0      | Spec (this document)                                         | Low.                       |
| 1      | `dsp-dct` crate skeleton (Cargo.toml, README, CHANGELOG, lib.rs stub) | Low. |
| 2      | Scalar `dct` / `idct` for types II + III, both norms.  Closed-form + round-trip + scipy cross-check tests. | Low — pure Rust, oracle vs scipy. |
| 3      | Matrix-ir-lowered `dct_via_runtime` + `build_dct_graph_with_input`.  Plans through `matrix-runtime`, dispatches on `matrix-cpu`. | Medium — first DSP02 graph build. Reuses the dsp-fft graph builders. |
| 4      | 2-D DCT-II / DCT-III via row-then-column factorisation.  `dct_2d` / `idct_2d` public API.  JPEG 8×8 closed-form cross-check. | Medium — first 2-D DSP primitive. |
| 5      | MX05-style specialised 8-point DCT-II emitter (Loeffler).  Same pattern as MX05 folded FFT twiddles. | Medium-high — perf-only. |

Phases 1-4 may merge as standalone PRs or be bundled
opportunistically; Phase 5 is its own thing.

## Dependencies

- `dsp-fft` — for `fft_scalar` (Phase 2), `fft_via_runtime` and
  `build_fft_graph_with_input` (Phase 3).
- `dsp-complex` — for the intermediate complex tensor in the
  Phase 3 / 4 matrix-ir builders (the twiddle multiply step).
- `matrix-ir`, `matrix-runtime`, `matrix-cpu`, `compute-ir`,
  `executor-protocol` — for Phase 3.  (Same set as `dsp-fft`
  already pulls in.)

No FFI, no `unsafe`.

## Open questions (defer until V1 lands)

1. **Type-I and type-IV.**  Worth shipping?  Type-IV unlocks
   MDCT, which we'd need for DSP05's STFT framework anyway.
   Decide when DSP05 starts.
2. **Lazy DCT** (compute only some output bins).  The pHash use
   case only needs the top-left 8 bins of a 32×32 DCT — pulling
   that off without computing the full DCT is a real perf win.
   Phase 5+.
3. **Integer-domain DCT** for fixed-point pipelines.  Outside
   `dsp-dct`'s scope; lives in MX05's specialised emitters.
