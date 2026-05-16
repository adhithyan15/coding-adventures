# Changelog — dsp-dct

## 0.1.0 — 2026-05-15

### Added — DSP02 Phase 1 + 2 (crate skeleton + scalar DCT-II / DCT-III)

Initial release.  The pure-Rust scalar oracle that Phase 3
(matrix-ir lowered) and Phase 4 (2-D) will test against.

#### Public API

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

#### DCT-II — Makhoul reduction via `dsp-fft::fft_scalar`

1. Pre-shuffle `x` into `y` (even samples in order, odd samples
   reversed).
2. FFT(y) of length N — uses `dsp_fft::fft_scalar` for power-of-two
   N and falls through to Bluestein for non-pow2.
3. Twiddle multiply: `X[k] = 2 · Re(Y[k] · exp(-iπk/(2N)))`.
4. Apply `Ortho` (`X[0] *= √(1/(4N))`, `X[k>0] *= √(1/(2N))`)
   or `None` (no scaling).

`O(N log N)` time, `O(N)` memory.  Works for any `N ≥ 1`.

#### DCT-III — naive O(N²) inverse (Phase 2)

```text
    X[k] = x[0]/2 + Σ_{n=1..N-1} x[n] · cos(πn(2k+1)/(2N))
```

Phase 2 keeps the textbook double-sum form for clarity.  Under
`Ortho`, the input is rescaled so that
`idct(dct(x, II, Ortho), III, Ortho) = x` exactly.  Phase 3
will lower DCT-III to FFT (the spec's "Algorithm — DCT-III via
FFT" section) so it lifts onto the matrix execution layer.

#### Tests

23 unit tests pass:

Error paths (3):
- `dct_rejects_empty_input`
- `idct_rejects_empty_input`
- Invalid DCT type combos (rejected by the type-system enum, but
  asserted dynamically too).

Closed-form known vectors (4):
- `dct_ii_of_impulse_matches_cosine_sequence` (N = 8).
- `dct_ii_of_dc_concentrates_at_bin_0` (N = 8).
- `dct_ii_of_dc_n3` (non-power-of-two).
- `dct_ii_ortho_of_dc_returns_sqrt_n` (N = 16, Ortho convention).

Naive cross-check (6):
- `dct_ii_matches_naive_dft` for N ∈ {2, 3, 4, 5, 8, 16}.

Round-trips under Ortho (6):
- `round_trip_ortho_n_X` for N ∈ {1, 2, 8, 16, 31, 64}.

Round-trips under None (4):
- `round_trip_none_with_explicit_rescale` for N ∈ {2, 8, 16, 32}
  (the un-normalised pair requires a `2/N` rescale to close the
  loop, which the test applies explicitly).

#### Dependencies

- `dsp-fft` — for the length-N FFT inside DCT-II.
- `dsp-complex` — for re-export consistency (not used directly
  in Phase 2; reserved for the matrix-ir Phase 3 builder).

No FFI, no `unsafe`, no external crates.

#### What this phase does NOT include

- Phase 3: matrix-ir-lowered `dct_via_runtime` /
  `build_dct_graph_with_input`.
- Phase 4: 2-D `dct_2d` / `idct_2d` for JPEG / pHash / image
  workloads.
- Phase 5: Loeffler-style specialised 8-point DCT-II emitter.
- DCT-I, DCT-IV, MDCT.  Deferred per the spec.
