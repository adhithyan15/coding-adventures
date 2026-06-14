# DSP06 — Wavelet Transforms

## Layer overview

The `dsp-wavelets` crate is the wavelet sibling of the Fourier family
(DSP01 FFT, DSP02 DCT, DSP05 STFT).  Where Fourier-family transforms
decompose a signal into **fixed-frequency** sinusoids, wavelet
transforms decompose a signal into **scale-and-position localised**
basis functions — short bumps that can stretch (low frequency / large
scale) or shrink (high frequency / small scale).

```
Fourier  : Σ a_k · sin/cos(2π f_k t)        — perfect frequency, no time
STFT     : Σ a_{m,k} · sin/cos(2π f_k t) · w(t − m·hop)
                                            — fixed time-freq tile
Wavelet  : Σ a_{j,k} · ψ((t − k·2^j) / 2^j)
                                            — multi-scale time-freq tile
```

The STFT's window is fixed in width, so all frequencies share the same
time resolution.  A wavelet basis is **self-scaling** — low-frequency
basis functions are wide (good frequency resolution, poor time
resolution) while high-frequency basis functions are narrow (the
reverse).  This matches how transient events live in real signals:
percussion attacks need narrow time-domain wavelets, sustained bass
notes need wide ones.

### Why wavelets matter

- **Multi-resolution analysis.**  One pass of the discrete wavelet
  transform decomposes a signal into approximation (smooth, half the
  resolution) + detail (high-frequency residual) at every scale.  The
  pyramid structure makes coarse-to-fine algorithms natural.
- **Sparse representation of transients and edges.**  A wavelet basis
  concentrates most of a piecewise-smooth signal's energy in a few
  large coefficients — STFT and DCT spread the same energy across
  many small ones.  This is *the* property that makes wavelets the
  standard for compressing transient-heavy data.
- **Image compression (JPEG 2000).**  The 2-D DWT replaced the
  block-DCT (used by classical JPEG) in JPEG 2000 because it has no
  blocking artefacts and gives much better quality at low bitrates.
  Modern variants ship in DICOM medical imaging, the Adobe Camera
  Raw pipeline, and many texture-streaming systems.
- **Denoising.**  Soft-thresholding wavelet coefficients (zero out
  small coefficients, shrink the rest) gives a near-optimal estimator
  for Gaussian noise on piecewise-smooth signals — much better than
  any linear filter at preserving edges.
- **Time-localised analysis** of seismic data, EEG/ECG spikes, financial
  tick data, vibration sensors, gravitational-wave chirps —
  wavelets are the standard tool wherever events are sparse in time.
- **Feature extraction** for non-stationary classification: wavelet
  packet features, scattering networks (Bruna & Mallat), texture
  classification with wavelet energy histograms.

### How wavelets complement STFT (the DSP05 cousin)

The STFT trades time resolution for frequency resolution **uniformly**
across the spectrum.  A small `n_fft` gives sharp time edges but
blurry frequency bins; a large `n_fft` does the opposite.  A single
choice is wrong for any signal that has both fast transients (drum
hits) and slow content (bass notes).

The wavelet transform makes the trade-off **adaptive**: short, wide
wavelets at high frequency catch transients; long, narrow wavelets at
low frequency resolve sustained tones.  Mathematically, wavelet basis
functions have constant *relative* bandwidth (∆f/f = const), while
STFT bins have constant *absolute* bandwidth (∆f = const).  Constant
relative bandwidth matches both human auditory perception (octave
bands) and the natural scaling of edges in images (sharper edge →
narrower wavelet).

### Image-side use → reference ARCH01

JPEG 2000-style image wavelets are 2-D DWTs of pixel data, and a
future `image-wavelets` crate would expose them in `PixelContainer`
shape.  Per
[`ARCH01-img-dsp-routing.md`](./ARCH01-img-dsp-routing.md), any such
image-side wavelet wrapper is a **thin adapter** over this crate
(`dsp-wavelets`) — it converts `PixelContainer` ↔ flat `&[f32]`,
handles multi-channel iteration, sRGB ↔ linear conversion, and
alpha-premultiplied special-casing, and forwards the actual DWT to
`dwt_2d` here.

The kernel math, filter coefficients, the Mallat pyramid algorithm,
boundary extension modes, and the eventual matrix-IR lowering all
live in this crate exactly once.  The image layer does not
re-implement them.  This is the same pattern `image-convolution` (when
it's written) follows against `dsp-conv`, and that future
`image-fft` / `image-dct` follow against `dsp-fft` / `dsp-dct`.

---

## Public API

```rust
// Wavelet family selector.  Each variant fixes a specific filter pair
// (low-pass + high-pass analysis + low-pass + high-pass synthesis).
//
// The "N" parameter on parameterised families (Daubechies, Symlets,
// Coiflets) is the number of *vanishing moments* — higher N gives a
// smoother basis function with longer filter support.  V1 ships the
// commonly-used small N values; extending to higher N is a
// configuration-table change.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WaveletType {
    /// The simplest wavelet: 2-tap filters.  Discontinuous (good for
    /// piecewise-constant signals like binary images); not used for
    /// audio because of the discontinuity.  Same as Daubechies-1.
    Haar,

    /// Daubechies wavelets — `Db(N)` has `2N` filter taps and `N`
    /// vanishing moments.  Orthogonal, compactly supported, no
    /// closed-form expression but tabulated filter coefficients.
    /// V1 ships N ∈ {1, 2, 4, 6, 8}; `Db1 == Haar`.
    Daubechies(u32),

    /// Symlets — `Sym(N)` is the *least asymmetric* Daubechies wavelet
    /// of the same support length, fixing the strong phase asymmetry
    /// of Daubechies.  V1 ships N ∈ {2, 4, 6, 8}.
    Symlets(u32),

    /// Coiflets — like Symlets but with additional vanishing moments
    /// on the scaling function (not just the wavelet).  Good for
    /// numerical analysis applications.  V1 ships N ∈ {1, 2, 3}.
    Coiflets(u32),

    /// Biorthogonal — paired analysis / synthesis wavelets.  Allows
    /// linear-phase symmetric filters (which orthogonal wavelets
    /// can't have except for Haar).  V1 ships `Bior(2,2)`, `Bior(4,4)`,
    /// `Bior(6,8)` — the JPEG 2000 reversible 5/3 and irreversible
    /// 9/7 wavelets.
    Biorthogonal { vm_decomp: u32, vm_recon: u32 },

    /// Morlet wavelet — complex-valued, ψ(t) = π^(-1/4) · e^(iω₀t) ·
    /// e^(-t²/2).  The default wavelet for the continuous wavelet
    /// transform.  `ω₀` is fixed to the canonical 6.0 (good
    /// time-frequency localisation).
    Morlet,

    /// Mexican hat / Ricker wavelet — second derivative of a
    /// Gaussian, ψ(t) = (1 − t²) · e^(-t²/2).  Real-valued; common
    /// in seismic exploration and edge detection.
    MexicanHat,
}

/// Boundary extension mode at the signal edges, used by every
/// filter-bank routine.  Same family as `dsp-conv::PaddingMode` so
/// downstream image wrappers can reuse the enum semantically.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WaveletBoundary {
    /// Treat samples outside `[0, N)` as zero.  Simple, but injects
    /// artificial step edges at the boundary.
    Zero,
    /// Clamp the index to `[0, N − 1]`.  Replicates the edge sample.
    Replicate,
    /// Reflect across the boundary without repeating the edge sample
    /// (`...c, b, a | a, b, c, ...` → `...c, b, a | b, c, ...`).
    Reflect,
    /// Reflect across the boundary repeating the edge sample
    /// (`...c, b, a | a, b, c, ...` → `...c, b, a | a, b, c, ...`).
    Symmetric,
    /// Periodic / circular wrap.  Mathematically exact for FFT-paired
    /// operations.
    Periodic,
}

// ─────────────────────── 1-D DWT ───────────────────────

/// Forward 1-D discrete wavelet transform via the Mallat pyramid
/// algorithm.
///
/// Decomposes `signal` into a hierarchy of `levels` approximation /
/// detail pairs.  Output layout (flattened row-major, see § "Output
/// layout" below):
///
/// ```text
///   [cA_J, cD_J, cD_{J-1}, ..., cD_1]
/// ```
///
/// where `J = levels` and `cA_J` is the coarsest approximation,
/// `cD_j` is the detail at scale `j`, and each level's length is
/// half of the previous (per the downsample-by-2 in the Mallat
/// algorithm).
///
/// Returns `Err(WaveletError::SignalTooShort)` if the signal is too
/// short to support `levels` decomposition passes.
pub fn dwt_1d(
    signal: &[f32],
    wavelet: WaveletType,
    levels: u32,
    boundary: WaveletBoundary,
) -> Result<Vec<f32>, WaveletError>;

/// Inverse 1-D DWT — reverses [`dwt_1d`] via the synthesis filter
/// bank.  `output_length` is required because the forward transform's
/// downsampling drops the parity bit at each level (a length-7 and a
/// length-8 input both produce length-4 cA after one level).
pub fn idwt_1d(
    coeffs: &[f32],
    wavelet: WaveletType,
    levels: u32,
    boundary: WaveletBoundary,
    output_length: u32,
) -> Result<Vec<f32>, WaveletError>;

// ─────────────────────── 2-D DWT ───────────────────────

/// Forward 2-D DWT for images — separable row-then-column DWT.
/// `image` is row-major `[n_rows, n_cols]` flattened.
///
/// At each level the four "sub-bands" are stacked into a quad-tree
/// quarter of the output:
///
/// ```text
///   ┌─────────────────────┐
///   │ LL (cA)  │ HL (cH)  │
///   │----------+----------│
///   │ LH (cV)  │ HH (cD)  │
///   └─────────────────────┘
/// ```
///
/// then `LL` is recursively decomposed `levels` more times.  Output
/// layout matches `pywt.wavedec2` flattened (see § "Output layout").
///
/// JPEG 2000 uses `dwt_2d` with `WaveletType::Biorthogonal { 5, 3 }`
/// (reversible, lossless mode) or `{ 9, 7 }` (irreversible, lossy
/// mode).
pub fn dwt_2d(
    image: &[f32],
    n_rows: u32,
    n_cols: u32,
    wavelet: WaveletType,
    levels: u32,
    boundary: WaveletBoundary,
) -> Result<Vec<f32>, WaveletError>;

/// Inverse 2-D DWT — reverses [`dwt_2d`].
pub fn idwt_2d(
    coeffs: &[f32],
    n_rows: u32,
    n_cols: u32,
    wavelet: WaveletType,
    levels: u32,
    boundary: WaveletBoundary,
) -> Result<Vec<f32>, WaveletError>;

// ───────────────────── Continuous WT ─────────────────────

/// Continuous wavelet transform (CWT).  At each scale `s ∈ scales`
/// and each time index `n`, computes the inner product of the signal
/// with a scaled+translated wavelet.
///
/// Output shape `[scales.len(), signal.len()]` flattened row-major —
/// the canonical "scaleogram" (time × scale heat map).  Used for
/// chirp detection, frequency-drift analysis, and any visualisation
/// where the STFT's fixed time-frequency tile is too rigid.
///
/// V1 uses FFT-based convolution under the hood for `O(N log N)` per
/// scale (delegates to `dsp_fft::fft_via_runtime`).  Naive direct
/// convolution would be `O(N · max_filter_length)`, slower for
/// realistic settings.
///
/// Wavelet choices for CWT: `WaveletType::Morlet` is the default
/// for time-frequency analysis; `WaveletType::MexicanHat` for edge /
/// spike detection.  The DWT-style discrete wavelets are not valid
/// CWT mothers (no analytic form, irregular sampling).
pub fn cwt(
    signal: &[f32],
    scales: &[f32],
    wavelet: WaveletType,
) -> Result<Vec<f32>, WaveletError>;

// ───────────────────── Errors ────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum WaveletError {
    EmptySignal,
    /// `levels == 0`, unsupported wavelet parameter, etc.
    InvalidParam(String),
    /// Signal too short to support `levels` decomposition passes for
    /// this wavelet's filter length.
    SignalTooShort(String),
    /// Coefficient buffer shape doesn't match `(n_rows, n_cols,
    /// levels, wavelet)`.
    InvalidCoefficients(String),
    /// Wraps a `dsp_fft::FftError` from the CWT's FFT-based
    /// convolution path.
    Fft(String),
}
```

All public functions return `Result<Vec<f32>, WaveletError>` — same
convention every other `dsp-*` crate follows.

### Output layout

The Mallat pyramid for `dwt_1d` with `levels = J` returns a
concatenation:

```text
   [cA_J | cD_J | cD_{J-1} | ... | cD_1]
```

where each block has length `⌈prev_len / 2⌉` (with parity governed by
the boundary mode).  This matches the "wavedec" flattened layout in
PyWavelets, scipy.signal, and MATLAB Wavelet Toolbox — the de facto
standard.  Helpers `split_levels(...)` and `slice_level(...)` will
ship in Phase 2 so callers don't have to compute offsets by hand.

For 2-D, the per-level quad `[LL | HL | LH | HH]` is concatenated
from coarsest to finest the same way.  Spelling-out:

```text
Level-J quad followed by level-(J-1) detail-only quad
(LL only appears at the coarsest level — every other level only stores
HL, LH, HH because their LL got recursively decomposed):

   [LL_J | HL_J | LH_J | HH_J | HL_{J-1} | LH_{J-1} | HH_{J-1} | ...
    ... | HL_1 | LH_1 | HH_1]
```

---

## Algorithm

### Mallat pyramid (DWT)

For 1-D, one level of DWT is two FIR filter passes followed by
downsample-by-2:

```text
            ┌── lowpass h ──→ ↓2 → cA   (approximation, ½ length)
   x[n] ───┤
            └── highpass g ─→ ↓2 → cD   (detail,         ½ length)
```

`levels` of DWT applies the same pair recursively to `cA`:

```text
   x ──filter-pair──► (cA_1, cD_1)
                     │
                     ▼
                   (cA_2, cD_2)
                     │
                     ▼
                       ...
                     │
                     ▼
                   (cA_J, cD_J)         ← keep this as the approximation
```

The output is `[cA_J | cD_J | cD_{J-1} | ... | cD_1]`.

`(h, g)` are the **analysis filter pair** specific to the wavelet
family.  For Haar (V1 Phase 1):

```text
   h = [+1/√2, +1/√2]              (lowpass = local average)
   g = [+1/√2, −1/√2]              (highpass = local difference)
```

For Daubechies-2 (Db2, Phase 3):

```text
   h = [(1+√3)/(4√2),  (3+√3)/(4√2),  (3−√3)/(4√2),  (1−√3)/(4√2)]
   g = quadrature mirror of h
```

Higher-N Daubechies, Symlets, and Coiflets filters are tabulated
constants — they have no closed-form expression but are computable
once and embedded as static arrays.  The V1 phase plan installs the
common ones; users wanting longer filters can extend the table.

### Inverse DWT — synthesis filter bank

The inverse is the dual:

```text
            ┌── ↑2 → synthesis lowpass  h' ──┐
   cA ────┤                                  +─→ x'
            └── ↑2 → synthesis highpass g' ──┘
   cD ────┘
```

For orthogonal wavelets (Daubechies, Symlets, Coiflets) `h' = reverse(h)`,
`g' = reverse(g)`.  For biorthogonal wavelets `(h', g')` are a
**different** filter pair tabulated alongside `(h, g)`.

Perfect reconstruction (PR) holds when the four filters satisfy the
**Smith-Barnwell conditions**:

```text
   H(z) · H'(z) + H(−z) · H'(−z) = 2
   G(z) · G'(z) + G(−z) · G'(−z) = 2
```

The orthogonal families are designed to satisfy these by construction;
the biorthogonal table includes both pairs.  V1 tests PR numerically
(`idwt(dwt(x)) ≈ x`) for every shipped wavelet.

### 2-D DWT — separable row-then-column

```text
   image  ──row DWT──→  ┌── L (row-lowpass) ─┐
                        │                      ├── col DWT ──→ ┌── LL ──┐
                        │                      │                │── HL ──┤
                        └── H (row-highpass) ─┤                ┌── LH ──┤
                                              │                │── HH ──┘
                                              └── col DWT ──┘
```

Row pass produces `(L, H)`, each `n_rows × (n_cols/2)`.  Column pass
on each gives the four sub-bands `(LL, HL, LH, HH)`, each `(n_rows/2)
× (n_cols/2)`.  `LL` is recursively decomposed.

Cost: `O(n_rows · n_cols)` per level, dominated by the four filter
passes (each linear in image area).  Compared to the 2-D STFT or 2-D
DFT (`O(n_rows · n_cols · log(n_rows · n_cols))`), the DWT is
asymptotically faster for moderate-to-large images.

### CWT — FFT-based

For each scale `s`, compute

```text
   W_s[n] = (1/√s) · Σ_t x[t] · ψ*((t − n) / s)
```

which is a convolution of the signal with a scaled+translated
wavelet.  V1 uses FFT-based convolution (multiply in frequency,
inverse FFT) for `O(N log N)` per scale — delegated to
`dsp_fft::fft_via_runtime` so the heavy lifting reuses the DSP01
matrix-IR-lowered FFT.

Output shape is `[len(scales), N]` flattened row-major.

---

## Numerical accuracy contract

- `idwt_1d(dwt_1d(x, w, J, B), w, J, B, N) ≈ x` within `1e-4`
  relative tolerance for `N ≤ 64K`, `J ≤ 6`, f32 dtype, every
  shipped `WaveletType` and `WaveletBoundary` except `Zero` (zero
  padding injects artificial edge content that breaks exact PR at
  the boundary — the central region still round-trips within
  tolerance).
- `idwt_2d(dwt_2d(image, w, J, B), w, J, B)` round-trips within
  `1e-4` for `n_rows · n_cols ≤ 64K`, `J ≤ 4`.
- DWT of a constant signal: all detail coefficients are ≤ `1e-6` at
  every level (Haar in particular gives exactly 0 in exact
  arithmetic).
- DWT of a Dirac delta at index `n`: at level `J`, exactly one
  approximation coefficient is non-zero (with magnitude
  `2^(-J/2)`), and detail coefficients are bounded by the filter
  support length.
- CWT of a pure sinusoid `cos(ω₀ t)` with Morlet wavelet at scale
  `s ≈ ω_morlet / ω₀` peaks at that scale and is small elsewhere.

These are the standard PR + impulse + sinusoid + zero-signal sanity
checks shipped by PyWavelets, scipy.signal.wavelets, and the MATLAB
Wavelet Toolbox.

---

## Phase plan

| Phase | Lands                                                          | Risk |
|-------|----------------------------------------------------------------|------|
| 0     | Spec (this document)                                           | Low. |
| 1+2   | Crate skeleton + **scalar Haar DWT + Haar IDWT** (1-D only) + tests; `WaveletType`, `WaveletBoundary`, `WaveletError` enums established; output-layout helpers (`split_levels`, `slice_level`) | Low. |
| 3     | Daubechies (Db2, Db4, Db6, Db8) + Symlets (Sym4, Sym6, Sym8) + Coiflets (Coif1, Coif2, Coif3) — all share the same filter-bank machinery from Phase 1+2, this phase only adds tabulated filter coefficients + dispatch | Low. |
| 4     | **2-D DWT** (`dwt_2d` / `idwt_2d`) — separable row-then-column on the filter bank from Phases 1-3.  Adds Biorthogonal { 5, 3 } and { 9, 7 } (JPEG 2000 wavelets) since 2-D is where they're most useful. | Medium — quad-tree layout indexing has to match pywt. |
| 5     | **CWT** (`cwt` + `WaveletType::Morlet`, `MexicanHat`) — FFT-based convolution via `dsp_fft::fft_via_runtime`.  Scaleogram for time-frequency visualisation. | Medium — Morlet has complex output, needs `dsp-complex` interleaved layout. |
| 6     | **Matrix-IR-lowered** `dwt_1d` / `dwt_2d` — emits `matrix_ir::Graph` that runs the filter bank through `Mul` / `Slice` / `Concat` ops.  Lifts onto GPU once matrix-metal / matrix-cuda claim the relevant ops.  Same pattern `dsp-stft` Phase 6 established. | Medium — downsampling is `Slice` with stride 2, well within matrix-ir's reach. |

Phases 1+2 typically bundle (per the precedent set by every other
`dsp-*` crate).  Phase 3 may itself split into 3a (Daubechies), 3b
(Symlets), 3c (Coiflets) if the implementation PR feels too large.

The final shipped crate `dsp-wavelets` reaches its Phase-6 endpoint
with the same architecture as `dsp-stft`: scalar reference + matrix-IR
lowered, both publicly exposed, both numerically equivalent within
f32 tolerance.

---

## Dependencies

- `dsp-fft` — for the CWT's FFT-based convolution path (Phase 5) and
  potentially for future implementations of the *fast wavelet
  transform via FFT* variants.  Also for general consistency with
  the other transform crates.
- `dsp-complex` — for the Morlet wavelet's complex-valued output
  (Phase 5).
- `matrix-ir`, `matrix-runtime`, `matrix-cpu`, `compute-ir`,
  `executor-protocol` — Phase 6 only.  Same set every other matrix-IR-
  lowered `dsp-*` crate depends on.

No FFI, no `unsafe`, no external crates.

---

## Out of scope

Explicitly **not** in V1:

- **Wavelet packet transforms.**  The DWT only decomposes the
  approximation branch recursively; wavelet packets decompose both
  branches, giving a richer (but exponentially larger) basis.
  Useful for audio classification and some compression schemes;
  worth a separate spec (DSP06b?) if there's demand.
- **Lifting scheme optimisations.**  The factorisation of any
  wavelet filter bank into a sequence of "predict-then-update" steps
  via Daubechies-Sweldens gives in-place computation and `2×`-ish
  speedup over the convolution form.  V1 uses the straightforward
  filter-bank form because it's far easier to verify against
  reference implementations; the lifting form can land later
  without an API change.
- **Stationary / undecimated DWT (SWT).**  Skips the downsample step,
  producing a translation-invariant representation at the cost of
  `J × N` storage.  Useful for denoising; orthogonal to V1.
- **Ridgelets, curvelets, shearlets.**  Newer transforms with better
  directional selectivity for images; each is its own large topic.
- **Wavelet-based denoising / compression** *as a public API*.  V1
  ships only the transforms.  Downstream packages
  (`dsp-denoise`, future `image-jpeg2000`) can implement
  thresholding, quantisation, and codestream encoding on top of
  these primitives.
- **Centred / "periodisation" boundary modes** beyond the five listed.
  Easy to add in a later phase once the V1 modes are settled.
- **Streaming / real-time wavelet transforms.**  V1 is batch (whole
  signal at once).  A streaming `DwtState::feed(...)` would let
  callers process audio block-by-block; future work if there's
  demand from a streaming consumer.

---

## Relation to other DSP crates

| Crate           | What it computes                          | Time-freq tile           |
|-----------------|-------------------------------------------|--------------------------|
| `dsp-fft`       | DFT of the whole signal                   | No time localisation     |
| `dsp-dct`       | Real-cosine variant of DFT                | No time localisation     |
| `dsp-stft`      | Windowed FFT, fixed window size           | Uniform rectangular tile |
| `dsp-wavelets`  | Self-scaling basis decomposition          | Adaptive tile per scale  |
| `dsp-conv`      | General filter application                | Underlying primitive     |
| `dsp-filters`   | FIR / IIR filter design + application     | Underlying primitive     |
| `dsp-complex`   | Interleaved [re, im] tensor type          | Type infrastructure      |

DSP06 closes the time-frequency analysis trio (Fourier → STFT →
wavelets) in the DSP layer.  Image-side and ML-side consumers
(JPEG 2000 codec, scattering networks, wavelet denoising,
spike-detection in neuroscience) wrap these primitives without
re-deriving them, per ARCH01.
