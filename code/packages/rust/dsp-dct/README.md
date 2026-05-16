# dsp-dct

**DSP02 Phase 1+2** — scalar reference DCT-II / DCT-III for
the DSP layer.

The Discrete Cosine Transform (DCT) is the spectral primitive
behind JPEG, MP3, MFCCs, pHash, and `scipy.fft.dct` /
`scipy.fft.idct`.  V1 ships the two most-used variants:

- **DCT-II** — the canonical "forward" DCT (`scipy.fft.dct(type=2)`).
- **DCT-III** — the inverse of DCT-II under the `Ortho`
  normalisation (`scipy.fft.idct(type=3)`).

Both `None` (un-normalised) and `Ortho` (orthonormal, mutual
inverse) normalisation conventions are supported, matching
scipy / numpy / MATLAB.

```rust
use dsp_dct::{dct, idct, DctType, DctNorm};

let signal = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

// Forward DCT-II under Ortho — the JPEG / MFCC convention.
let coeffs = dct(&signal, DctType::II, DctNorm::Ortho).unwrap();

// Inverse via DCT-III under Ortho — recovers the input.
let recovered = idct(&coeffs, DctType::III, DctNorm::Ortho).unwrap();
```

## Algorithm

### DCT-II — Makhoul reduction (FFT-based)

1. **Pre-shuffle** the length-`N` real input into a length-`N`
   real sequence `y` by interleaving even and reversed-odd samples.
2. **FFT(y)** of length `N` — runs through `dsp-fft::fft_scalar`.
3. **Twiddle multiply + real-part double**:
   `X[k] = 2 · Re(Y[k] · exp(-iπk/(2N)))`.
4. Apply `Ortho` or `None` normalisation.

`O(N log N)` time, `O(N)` memory.  Works for any `N ≥ 1`
(`dsp-fft` handles non-power-of-two sizes via Bluestein).

### DCT-III — naive O(N²) inverse (Phase 2)

Phase 2 ships the textbook double-sum:

```text
    X[k] = x[0]/2 + Σ_{n=1..N-1} x[n] · cos(πn(2k+1)/(2N))
```

Plus the matching `Ortho` rescaling so that
`idct(dct(x, II, Ortho), III, Ortho) ≈ x` exactly.

Phase 3 will lower DCT-III to FFT (the spec's Algorithm — DCT-III
via FFT section: build a complex spectrum, IFFT, un-shuffle).
The naive form is correct, simple, and easy to verify; the
FFT-based version is a correctness-preserving optimisation that
also unlocks matrix-ir lowering.

## Public API

```rust
pub enum DctType { II, III }
pub enum DctNorm { None, Ortho }

pub fn dct(signal: &[f32], dct_type: DctType, norm: DctNorm)
    -> Result<Vec<f32>, DctError>;

pub fn idct(signal: &[f32], dct_type: DctType, norm: DctNorm)
    -> Result<Vec<f32>, DctError>;

pub enum DctError {
    InvalidInput(String),
    EmptyInput,
    Fft(String),
}
```

`dct(_, II, _)` is the standard DCT-II.  `idct(_, III, _)`
inverts `dct(_, II, _)` under matching norm.  In V1 the convention
is: pass `DctType::III` to invert a DCT-II forward, and vice
versa.  Phase 4 will add `dct_2d` / `idct_2d` for image work
(JPEG block transform, perceptual hashes).

## Phase scope

| Phase  | Lands                                                | Status |
| ------ | ---------------------------------------------------- | ------ |
| 0      | Spec (`code/specs/DSP02-dct.md`)                     | landed |
| **1+2** | **Crate skeleton + scalar DCT-II/III + tests**      | **this PR (0.1.0)** |
| 3      | Matrix-ir-lowered DCT-II/III via `dsp-fft::fft_via_runtime` | pending |
| 4      | 2-D `dct_2d` / `idct_2d` for image / JPEG workloads  | pending |
| 5      | Loeffler-style specialised 8-point DCT-II emitter    | pending |

## Tests

`cargo test -p dsp-dct` exercises:

- Error paths: empty input.
- Closed-form DCT-II: impulse → cosine sequence; DC → single bin.
- Cross-check against a naive O(N²) DCT-II oracle for N ∈ {2, 3, 4, 5, 8, 16}.
- Round-trip `idct(dct(x, II, Ortho), III, Ortho) ≈ x` for
  N ∈ {1, 2, 8, 16, 31, 64}.
- Round-trip with `None` norm + manual `2/N` rescale.
- DC and impulse sanity at non-pow2 N.
