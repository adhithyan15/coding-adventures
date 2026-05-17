# dsp-wavelets

Scalar reference wavelet transforms for the DSP layer.  As of
**0.3.0 (DSP06 Phase 3b partial)** the crate ships **Haar +
Daubechies (Db2, Db4, Db6, Db8) + Symlets (Sym4) + Coiflets
(Coif1)** — the four orthogonal wavelet families wired through
a generic Mallat pyramid filter bank that works for any
orthogonal wavelet:

- **`dwt_1d`** — forward 1-D DWT via Mallat pyramid.
- **`idwt_1d`** — inverse 1-D DWT (synthesis filter bank).
- **`split_levels`** / **`slice_level`** — helpers for unpacking
  the flattened `[cA_J | cD_J | cD_{J-1} | ... | cD_1]` output
  layout.

The wavelet sibling of `dsp-fft` / `dsp-stft`.  Where the Fourier
family uses fixed-frequency basis functions, wavelets use
scale-and-position-localised ones — adaptive time-frequency tiling
that matches both human auditory perception (octave bands) and
the natural scaling of edges in images.

```rust
use dsp_wavelets::{dwt_1d, idwt_1d, WaveletType, WaveletBoundary};

let signal: Vec<f32> = (0..32).map(|i| (i as f32) * 0.1).collect();

// 3 levels of Haar decomposition with symmetric boundary.
let coeffs = dwt_1d(&signal, WaveletType::Haar, 3, WaveletBoundary::Symmetric).unwrap();

// Round-trip back to the original signal.
let recon = idwt_1d(
    &coeffs, WaveletType::Haar, 3, WaveletBoundary::Symmetric, signal.len() as u32,
).unwrap();
// recon matches signal within 1e-4 relative tolerance.
```

## Algorithm — Mallat pyramid

For each level `j ∈ [1, J]`:

1. **Lowpass filter** the input with `h = [1/√2, 1/√2]` (local
   average).
2. **Highpass filter** the input with `g = [1/√2, −1/√2]` (local
   difference).
3. **Downsample by 2** — keep every second sample.

The lowpass result `cA_j` (approximation) feeds the next level;
the highpass result `cD_j` (detail) is kept for the output.  After
`J` levels the approximation `cA_J` is also kept.  Output layout:

```text
   [cA_J | cD_J | cD_{J-1} | ... | cD_1]
```

The inverse mirrors this: upsample by 2 (insert zeros), filter
with synthesis pair `(h, g)` (Haar is its own synthesis — same
filters reversed), sum.

V1 supports `WaveletBoundary::Symmetric` (mirror across the
boundary, repeating the edge sample) and `WaveletBoundary::Periodic`
(circular wrap).  Other boundary modes return
`WaveletError::InvalidParam("unsupported boundary (Phase ...)")`
for now and land in later phases.

## Public API

```rust
pub fn dwt_1d(
    signal: &[f32],
    wavelet: WaveletType,
    levels: u32,
    boundary: WaveletBoundary,
) -> Result<Vec<f32>, WaveletError>;

pub fn idwt_1d(
    coeffs: &[f32],
    wavelet: WaveletType,
    levels: u32,
    boundary: WaveletBoundary,
    output_length: u32,
) -> Result<Vec<f32>, WaveletError>;

pub fn split_levels(
    coeffs_len: usize,
    signal_len: usize,
    levels: u32,
) -> Result<Vec<usize>, WaveletError>;

pub fn slice_level<'a>(
    coeffs: &'a [f32],
    signal_len: usize,
    levels: u32,
    target_level: u32,
    band: Band,
) -> Result<&'a [f32], WaveletError>;

pub enum WaveletType {
    Haar,
    Daubechies(u32),
    Symlets(u32),
    Coiflets(u32),
    Biorthogonal { vm_decomp: u32, vm_recon: u32 },
    Morlet,
    MexicanHat,
}

pub enum WaveletBoundary {
    Zero,
    Replicate,
    Reflect,
    Symmetric,
    Periodic,
}

pub enum Band {
    Approximation,
    Detail,
}

pub enum WaveletError {
    EmptySignal,
    InvalidParam(String),
    SignalTooShort(String),
    InvalidCoefficients(String),
    Fft(String),
}
```

## Phase scope

| Phase  | Lands                                                | Status |
| ------ | ---------------------------------------------------- | ------ |
| 0      | Spec (`code/specs/DSP06-wavelets.md`)                | landed |
| 1+2    | Crate skeleton + scalar Haar DWT / IDWT             | landed (0.1.0) |
| 3a     | Db2, Db4, Sym4, Coif1 (verified-coefficient subset) | landed (0.2.0) |
| **3b** | **Db6, Db8 (PyWavelets-imported); Sym6/Sym8/Coif2/Coif3 still deferred** | **this PR (0.3.0)** |
| 4      | 2-D DWT + JPEG 2000 biorthogonal wavelets           | pending |
| 5      | CWT (Morlet, MexicanHat) via FFT                     | pending |
| 6      | Matrix-IR-lowered `dwt_1d` / `dwt_2d`                | pending |

See [`DSP06-wavelets.md`](../../../specs/DSP06-wavelets.md) for the
full algorithm and accuracy contract.  For the image-side wrapper
rule (any future `image-wavelets` is a thin adapter over this
crate, not a reimplementation), see
[`ARCH01-img-dsp-routing.md`](../../../specs/ARCH01-img-dsp-routing.md).

## Tests

`cargo test -p dsp-wavelets` exercises:

- Error paths: empty signal, `levels = 0`, signal too short for
  requested levels, unsupported wavelet variants, unsupported
  boundary modes.
- Output layout: `dwt_1d(x, Haar, J, B).len() == x.len()`
  (Mallat pyramid is sample-count-preserving) for both Symmetric
  and Periodic boundaries.
- Perfect reconstruction: `idwt_1d(dwt_1d(x))` round-trips within
  `1e-4` for both boundaries, several N (4, 8, 16, 32, 100), and
  several J (1, 2, 3).
- Constant signal: every detail coefficient is ≤ `1e-6` at every
  level (Haar gives exactly 0 in exact arithmetic).
- Dirac delta: DWT concentrates the impulse in one approximation
  coefficient at the coarsest level.
- Haar pair against a hand-worked reference vector.
