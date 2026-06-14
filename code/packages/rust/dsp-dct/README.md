# dsp-dct

Scalar reference Discrete Cosine Transform for the DSP layer.

The DCT is the spectral primitive behind JPEG, MP3, MFCCs, pHash,
and `scipy.fft.dct` / `scipy.fft.idct`.  As of **0.2.0 (DSP02
Phase 4)** this crate ships:

- **1-D DCT-II** (`scipy.fft.dct(type=2)`) — the canonical "forward" DCT.
- **1-D DCT-III** (`scipy.fft.idct(type=3)`) — inverse under `Ortho`.
- **2-D `dct_2d` / `idct_2d`** — for image / JPEG workloads
  (any `(H, W)` ≥ `(1, 1)`).

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
| 1+2    | Crate skeleton + scalar DCT-II/III + tests           | landed (0.1.0) |
| 3      | Matrix-ir-lowered DCT-II/III via `dsp-fft::fft_via_runtime` | pending |
| **4**  | **2-D `dct_2d` / `idct_2d` for image / JPEG workloads** | **this PR (0.2.0)** |
| 5      | Loeffler-style specialised 8-point DCT-II emitter    | pending |

## 2-D DCT (Phase 4)

```rust
use dsp_dct::{dct_2d, idct_2d, DctType, DctNorm};

// 8×8 JPEG-style block.
let block: Vec<f32> = (0..64).map(|i| (i as f32) * 0.1).collect();
let coeffs = dct_2d(&block, 8, 8, DctType::II, DctNorm::Ortho).unwrap();
let recovered = idct_2d(&coeffs, 8, 8, DctType::III, DctNorm::Ortho).unwrap();
// `recovered` matches `block` within 1e-3.
```

The 2-D DCT is **separable** — applying the 1-D DCT to each
row, then to each column, gives the exact 2-D result.  Works
for any `(H, W) ≥ (1, 1)`; both axes can be non-power-of-two
(Bluestein along each).

## Tests

`cargo test -p dsp-dct` — 26 unit tests + 1 doctest:

- 14 from Phase 1+2 (1-D): error paths, closed-form, naive
  cross-check, Ortho / None round-trips.
- 12 from Phase 4 (2-D, this release): error paths, 8×8 DC /
  impulse closed-form, naive cross-check at 4×4 and 8×8,
  Ortho round-trips at 8×8 / 16×16 / 8×16 / 3×5.
