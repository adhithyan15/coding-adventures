# Changelog — dsp-wavelets

## 0.3.0 — 2026-05-17

### Added — DSP06 Phase 3b (partial — Db6 and Db8)

Two more Daubechies wavelets land:

- **Db6** (12 taps, 6 vanishing moments) — the canonical
  "longer-than-toy" orthogonal wavelet.  6 vanishing moments
  perfectly suppress polynomial signals up to degree 5 (on
  non-Periodic boundaries — see Phase 4 for Symmetric).
- **Db8** (16 taps, 8 vanishing moments) — the standard
  high-order Daubechies, common in audio compression and
  scientific signal denoising.

Both pass the orthogonal-wavelet invariants (`Σ h = √2` within
`5e-4`, `Σ h² = 1` within `5e-4`) and round-trip through
`idwt_1d(dwt_1d(x))` within `1e-3` under Periodic boundary on
128-sample sinusoid test signals.

#### Source of coefficients

Imported from PyWavelets'
`_extensions/c/wavelets_coeffs.template.h` table via WebFetch,
cross-checked against the Wikipedia "Orthogonal Daubechies
coefficients" table (after rescaling from the "normalized to
sum 2" convention Wikipedia uses to the "normalized to sum √2"
convention pywt / this crate use).

#### Why Sym6, Sym8, Coif2, Coif3 are still deferred

The same WebFetch returned data for these wavelets but the values
did NOT pass the strict orthogonal-wavelet invariants at f32
precision:

- **Sym6** — `Σ h = 1.658` (should be `√2 ≈ 1.414`), 17% off.
  Source data was likely from a different normalisation
  convention or has a copy-paste error.
- **Sym8** — `Σ h = 1.416` vs `1.4142`, error `2e-3` above the
  `5e-4` tolerance.  Close but not close enough — likely
  truncation in the source rather than a wrong filter.
- **Coif2** — source returned 6 values instead of 12 (the
  scaling-function half; the wavelet-function half was missing).
- **Coif3** — source returned 9 values instead of 18 (same
  truncation pattern).

Phase 3a's strict invariant tests catch all four cases.  Rather
than relax the tolerance (which would hide the issue), the
deferred variants stay behind `WaveletError::InvalidParam` until
a clean source ships — likely a build.rs that calls Python
PyWavelets directly to dump verified values into a vendored CSV.

#### Public API change

None.  Identical surface to 0.2.0.  Only the `analysis_lowpass`
dispatch in `src/filters.rs` adds two new arms.

#### New tests — 2

- `db6_round_trip_periodic` — 128-sample sinusoid, 2 levels,
  Periodic, central region within `1e-3`.
- `db8_round_trip_periodic` — same setup, Db8.

Plus the existing `Σ h = √2`, `Σ h² = 1`, and `Σ g = 0`
invariant loops in `src/filters.rs` were extended to include
the new variants.

All 28 unit tests + 1 doctest pass (26 from Phases 1+2+3a +
2 new Db6/Db8 round-trips).

## 0.2.0 — 2026-05-17

### Added — DSP06 Phase 3a (Daubechies / Symlets / Coiflets — verified subset)

Three new orthogonal wavelet families join Haar:

- **Daubechies** — `Db2` (4 taps, 2 vanishing moments), `Db4` (8 taps, 4 vm).
- **Symlets** — `Sym4` (8 taps, 4 vm) — least-asymmetric Db4.
- **Coiflets** — `Coif1` (6 taps).

All four pass round-trip tests within `1e-3` under `Periodic`
boundary, plus DC-suppression on the constant-signal test.

#### Why "Phase 3a" not "Phase 3"

The DSP06 spec's Phase 3 plan called for the full Db2/4/6/8 +
Sym4/6/8 + Coif1/2/3 set.  This PR ships the smaller verified
subset (Db2, Db4, Sym4, Coif1) where the tabulated coefficients
satisfy the orthogonal-wavelet invariants (`Σ h = √2` within
`5e-4`, `Σ h² = 1` within `5e-4`) at f32 precision.

The longer filters (Db6, Db8, Sym6, Sym8, Coif2, Coif3) are
declared in `src/filters.rs` with `#[allow(dead_code)]`
placeholders — the constants I wrote from memory have small
errors (~5e-3 per coefficient) that fail the strict invariants
even though they would still round-trip within ~1e-2.  Below the
crate's `1e-4` accuracy contract.

**Phase 3b (next PR)** will import the verified PyWavelets
`filter_bank` table values via a build.rs script reading from a
CSV (or a vendored coefficient table file), bringing the full
Db/Sym/Coif set online without changing the public API — those
variants currently return
`WaveletError::InvalidParam("unsupported wavelet ... ")`.

#### Architectural change — generic synthesis filter bank

Phase 1+2 used a **Haar-specific closed form** for the inverse
(`y[2k] = (cA[k] − cD[k])/√2`, `y[2k+1] = (cA[k] + cD[k])/√2`)
and gated the longer-filter synthesis behind `#[cfg(test)]` /
`#[allow(dead_code)]` placeholders.

Phase 3 replaces this with a **generic `synthesize_one_level`**
that works for any orthogonal wavelet filter pair, derived from
first principles:

```text
   y[n] = Σ_m ( h[2m + 1 − n] · cA[m] + g[2m + 1 − n] · cD[m] )
```

where `m` ranges over values such that `2m + 1 − n ∈ [0, L − 1]`.
This reduces to the Haar closed form for length-2 filters
(verified by all Phase 1+2 round-trip tests continuing to pass)
and handles arbitrary filter lengths for Phase 3+.

The implementation uses the **analysis** filters `(h, g)`
directly — for orthogonal wavelets the "synthesis filters" from
the textbook are just the analysis filters with indices reversed,
and after the reversal the convolution direction also flips, so
the two reversals cancel.  Pleasingly symmetric.

#### Public API change

None.  The public surface (`dwt_1d`, `idwt_1d`, `split_levels`,
`slice_level`, all enums) is byte-for-byte identical to 0.1.0.
Only the dispatch arms in `analysis_filters` /
`check_supported_wavelet` / `filter_length_for` recognise the new
wavelet variants.

#### QMF highpass derivation

For each orthogonal wavelet, the analysis highpass `g` is
derived from the lowpass via the QMF relation:

```text
   g[i] = (−1)^i · h[L − 1 − i]
```

implemented as `filters::qmf_highpass(h)`.  Cross-checked by the
"highpass filters sum to zero" test in `src/filters.rs`.

#### New tests — 9

- `db2_round_trip_periodic`
- `db4_round_trip_periodic`
- `sym4_round_trip_periodic`
- `coif1_round_trip_periodic`
- `db2_dwt_of_constant_signal_has_small_detail` (Db2's 2 vm
  perfectly suppresses constants)
- `lowpass_filters_sum_to_sqrt_2` (universal invariant)
- `lowpass_filters_have_unit_energy` (universal invariant)
- `highpass_filters_sum_to_zero` (cross-checks QMF derivation)
- `unsupported_variants_return_empty` (Phase 3b-deferred wavelets
  explicitly return EMPTY rather than silently using approximate
  coefficients)

Plus the existing `rejects_unsupported_wavelet` test was
expanded to cover all currently-unsupported variants
(odd-N Daubechies, Phase 3b-deferred wavelets, Biorthogonal,
Morlet, MexicanHat).

All 26 unit tests + 1 doctest pass (17 from Phase 1+2 + 4 in
`src/filters.rs` + 5 new in `src/lib.rs`).

#### File structure

New module: `src/filters.rs` — tabulated coefficient constants
+ QMF derivation + invariant tests.  Keeps the verbose
coefficient arrays out of `src/lib.rs`.

#### What this phase does NOT include

- Phase 3b: Db6/8, Sym6/8, Coif2/3 with verified coefficients.
- Phase 4: 2-D DWT + JPEG 2000 biorthogonal wavelets.
- Phase 5: CWT (Morlet, MexicanHat) via dsp-fft.
- Phase 6: matrix-IR-lowered DWT.
- Round-trip-exact `Symmetric` boundary for non-Haar wavelets
  (requires proper convolution-boundary stencils — Phase 4
  will land them anyway for biorthogonal wavelets).
- `Zero`, `Replicate`, `Reflect` boundary modes (still deferred).

## 0.1.0 — 2026-05-16

### Added — DSP06 Phase 1+2 (crate skeleton + scalar Haar DWT)

First release.  The Haar discrete wavelet transform — the simplest
member of the wavelet family and the canonical worked example for
the Mallat pyramid algorithm.  This phase establishes the crate
skeleton and the public API surface; Phases 3+ extend it with more
wavelet families, 2-D, CWT, and matrix-IR lowering without
breaking the API.

#### Public API

```rust
pub use WaveletType;       // Haar / Daubechies(N) / Symlets(N) / Coiflets(N) /
                            // Biorthogonal{vm_d, vm_r} / Morlet / MexicanHat
pub use WaveletBoundary;   // Zero / Replicate / Reflect / Symmetric / Periodic
pub use WaveletError;      // EmptySignal / InvalidParam / SignalTooShort /
                            // InvalidCoefficients / Fft
pub use Band;              // Approximation / Detail

pub fn dwt_1d(signal: &[f32], wavelet: WaveletType, levels: u32,
              boundary: WaveletBoundary)
    -> Result<Vec<f32>, WaveletError>;

pub fn idwt_1d(coeffs: &[f32], wavelet: WaveletType, levels: u32,
               boundary: WaveletBoundary, output_length: u32)
    -> Result<Vec<f32>, WaveletError>;

pub fn split_levels(coeffs_len: usize, signal_len: usize, levels: u32)
    -> Result<Vec<usize>, WaveletError>;

pub fn slice_level<'a>(coeffs: &'a [f32], signal_len: usize,
                       levels: u32, target_level: u32, band: Band)
    -> Result<&'a [f32], WaveletError>;
```

The full enum surface (every `WaveletType`, every `WaveletBoundary`)
is declared even though only `Haar` (and only `Symmetric` / `Periodic`
boundaries) are actually implemented in this phase — unimplemented
variants return `WaveletError::InvalidParam("unsupported ...
(Phase ...)")`.  Pinning the enum surface up front means later phases
can fill in the implementations without changing the public type
signature or risking enum-variant additions that break callers.

#### Algorithm

Standard Mallat pyramid for the Haar wavelet:

- Analysis filter pair `h = [1/√2, 1/√2]` (lowpass, local average)
  and `g = [1/√2, −1/√2]` (highpass, local difference).
- One level: filter with `h` → downsample by 2 → `cA`; filter with
  `g` → downsample by 2 → `cD`.
- `J` levels: recursively apply to `cA`.
- Output layout `[cA_J | cD_J | cD_{J-1} | ... | cD_1]`, flattened
  row-major.  Total length matches `signal.len()` exactly (Mallat
  is sample-count-preserving for Haar with even-length boundaries;
  odd lengths get a ⌈/2⌉ split per level, still adding to
  `signal.len()`).

Inverse:

- Upsample each band by 2 (insert zeros between samples).
- Filter `cA_{j+1}` with synthesis lowpass + `cD_{j+1}` with
  synthesis highpass.  Haar is its own synthesis, so the same
  `(h, g)` pair is reversed.
- Sum the two results → `cA_j`.  Repeat until `cA_0` = the original
  signal.

#### Boundary modes (Phase 1+2)

- **Symmetric** — mirror across the boundary repeating the edge
  sample (`...c, b, a | a, b, c...` → `...c, b, a | a, b, c...`).
  The default for most wavelet workflows because it avoids the
  artificial edge content that `Zero` injects.  Implemented.
- **Periodic** — circular wrap.  Mathematically exact for
  FFT-paired operations and the canonical mode for testing
  perfect-reconstruction round-trips.  Implemented.
- **Zero / Replicate / Reflect** — declared in the enum, return
  `InvalidParam("unsupported boundary (Phase ...)")` for now.
  Will land in a later phase once a consumer asks for them.

#### Helpers

- `split_levels(coeffs_len, signal_len, levels)` — returns the
  per-band offsets in the flattened coefficient vector so callers
  can slice out `cA_J`, `cD_J`, ..., `cD_1` without computing
  offsets by hand.
- `slice_level(coeffs, signal_len, levels, target_level, band)` —
  returns a `&[f32]` slice for the requested level and band
  (Approximation or Detail).  `target_level = 0` is the original
  signal level (only valid for IDWT inputs); `target_level = 1..J`
  are detail levels; `target_level = J` is the only valid
  approximation level.

#### Defensive caps (from security review)

Two HIGH findings + one MEDIUM, all rooted in unbounded `u32`
parameters reaching internal shifts and allocations:

- **`MAX_LEVELS = 31`** at every public entry point (`dwt_1d`,
  `idwt_1d`, `split_levels`, `slice_level`).  Without this cap,
  `levels ≥ 33` triggered `1u32 << (levels - 1)` shift overflow
  (panic in debug / wrap to 0 in release, silently bypassing the
  size guard), and `Vec::with_capacity(levels as usize)` with
  `levels = u32::MAX` was a ~96 GB capacity request → OOM abort.
- **`MAX_SAMPLES = 1 << 30`** on `signal.len()`, `output_length`,
  and `coeffs.len()`.  Caps the largest allocation
  `vec![0.0; target_len]` in the synthesis path at 4 GB of f32 —
  well above any realistic audio / signal workload, well below
  the OOM cliff.
- `filter_length_for` defensive arm changed from `usize::MAX / 2`
  sentinel to `unreachable!()` to make the
  "always-gated-by-check_supported_wavelet" contract explicit.

Two new tests pin the rejections in place
(`rejects_levels_above_max`, `rejects_output_length_above_max`).

#### Unit tests — 16

Error paths (7):
- `rejects_empty_signal`
- `rejects_zero_levels`
- `rejects_signal_too_short_for_levels`
- `rejects_unsupported_wavelet`
- `rejects_unsupported_boundary`
- `rejects_levels_above_max`        ← from security review
- `rejects_output_length_above_max` ← from security review

Output contract (2):
- `output_length_matches_signal_length_periodic` (powers of 2)
- `output_length_matches_signal_length_symmetric` (odd lengths)

Closed-form / known vectors (3):
- `haar_dwt_matches_hand_worked_reference` — `[1, 2, 3, 4]` under
  Haar with `levels=1, Periodic` matches scipy/pywt reference output.
- `dwt_of_constant_signal_has_zero_detail` — every detail
  coefficient ≤ `1e-6` for a flat signal at every level.
- `dwt_of_dirac_delta_concentrates_at_one_coefficient` — single
  non-zero approximation coefficient at the coarsest level.

Perfect reconstruction (4):
- `idwt_of_dwt_recovers_signal_periodic_powers_of_2` — N ∈ {4, 8, 16, 32}, J ∈ {1, 2, 3}.
- `idwt_of_dwt_recovers_signal_symmetric_powers_of_2` — same.
- `idwt_of_dwt_recovers_signal_periodic_odd_length` — N = 17.
- `idwt_of_dwt_recovers_signal_symmetric_odd_length` — N = 17.

All 17 unit tests + 1 doctest pass.

#### Dependencies

None (no FFI, no `unsafe`, no external crates).  Phase 5 will pull
in `dsp-fft` (CWT via FFT-based convolution) and `dsp-complex`
(Morlet complex output); Phase 6 will pull in the `matrix-*` set
(same as `dsp-stft` Phase 6).

#### What this phase does NOT include

- Phase 3: Daubechies, Symlets, Coiflets filter coefficients.
- Phase 4: 2-D DWT + JPEG 2000 biorthogonal wavelets (5/3, 9/7).
- Phase 5: CWT (Morlet, MexicanHat) via FFT.
- Phase 6: matrix-IR-lowered `dwt_1d` / `dwt_2d`.
- `Zero`, `Replicate`, `Reflect` boundary modes.
- Wavelet packets, lifting scheme, SWT, ridgelets/curvelets/shearlets.
- Streaming / real-time.
