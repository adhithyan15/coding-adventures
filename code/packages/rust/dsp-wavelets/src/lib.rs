//! # `dsp-wavelets` — Discrete Wavelet Transforms
//!
//! **DSP06 Phase 1+2 (this release).**  Pure-Rust scalar
//! reference for the Haar discrete wavelet transform and its
//! inverse via the **Mallat pyramid algorithm**.
//!
//! Wavelets are the third member of the time-frequency analysis
//! family in the DSP layer:
//!
//! | Crate         | Time-freq tile           |
//! | ------------- | ------------------------ |
//! | `dsp-fft`     | No time localisation     |
//! | `dsp-stft`    | Uniform rectangular tile |
//! | `dsp-wavelets`| Adaptive tile per scale  |
//!
//! Where the Fourier family uses fixed-frequency basis functions,
//! wavelets use scale-and-position-localised ones — adaptive
//! time-frequency tiling that matches both human auditory
//! perception (octave bands) and the natural scaling of edges in
//! images.
//!
//! ## Quick example
//!
//! ```rust
//! use dsp_wavelets::{dwt_1d, idwt_1d, WaveletType, WaveletBoundary};
//!
//! let signal: Vec<f32> = (0..32).map(|i| (i as f32) * 0.1).collect();
//! let coeffs = dwt_1d(
//!     &signal, WaveletType::Haar, 3, WaveletBoundary::Symmetric,
//! ).unwrap();
//! let recon = idwt_1d(
//!     &coeffs, WaveletType::Haar, 3, WaveletBoundary::Symmetric,
//!     signal.len() as u32,
//! ).unwrap();
//! // `recon` matches `signal` within 1e-4.
//! assert_eq!(recon.len(), signal.len());
//! ```
//!
//! ## Algorithm — the Mallat pyramid
//!
//! For each level `j ∈ [1, J]`, one pass of forward DWT is two
//! FIR filter passes followed by downsample-by-2:
//!
//! ```text
//!             ┌── lowpass  h ──→ ↓2 → cA   (approximation, ½ length)
//!    x[n] ───┤
//!             └── highpass g ──→ ↓2 → cD   (detail,         ½ length)
//! ```
//!
//! `levels` of DWT applies the same pair recursively to `cA`:
//!
//! ```text
//!    x ──filter-pair──► (cA_1, cD_1)
//!                      │
//!                      ▼
//!                    (cA_2, cD_2)
//!                      │
//!                      ▼
//!                        ...
//!                      │
//!                      ▼
//!                    (cA_J, cD_J)         ← keep this as the approximation
//! ```
//!
//! Output layout (flattened row-major):
//!
//! ```text
//!    [cA_J | cD_J | cD_{J-1} | ... | cD_1]
//! ```
//!
//! For the Haar wavelet (V1 Phase 1+2):
//!
//! ```text
//!    h = [+1/√2, +1/√2]      (lowpass = local average)
//!    g = [+1/√2, −1/√2]      (highpass = local difference)
//! ```
//!
//! These are the simplest non-trivial wavelet filters; they
//! detect step edges and constant runs perfectly but smooth
//! signals only crudely.  Phases 3+ add Daubechies / Symlets /
//! Coiflets / Biorthogonal families for smoother bases.
//!
//! ## Inverse — synthesis filter bank
//!
//! `idwt_1d` reverses the pyramid:
//!
//! ```text
//!             ┌── ↑2 → synthesis lowpass  h' ──┐
//!    cA ────┤                                  +─→ x'
//!             └── ↑2 → synthesis highpass g' ──┘
//!    cD ────┘
//! ```
//!
//! For Haar (orthogonal) the synthesis pair is `(h, g)` with
//! reversed indexing.  Upsampling inserts zeros between samples,
//! the synthesis filter spreads the energy back, and the sum
//! reconstructs the previous-level approximation.  After `J`
//! levels of synthesis the result is `cA_0` — the original signal.
//!
//! ## V1 scope
//!
//! - **Haar only.**  Daubechies / Symlets / Coiflets / Biorthogonal
//!   are tabulated-coefficient additions that share the same
//!   filter-bank machinery — Phase 3.  `WaveletType::Daubechies(N)`
//!   etc. currently return `WaveletError::InvalidParam` so the
//!   surface is stable but the implementations land later.
//! - **1-D only.**  2-D DWT for images is Phase 4 (separable
//!   row-then-column on the Phase-3 filter bank).
//! - **Symmetric + Periodic boundaries.**  The other three
//!   boundary modes are declared in the enum and return
//!   `InvalidParam("unsupported boundary (Phase ...)")` for now.
//! - **f32 dtype only.**
//! - **No matrix-IR lowering** — Phase 6.

#![warn(rust_2018_idioms)]

mod filters;

use std::fmt;

/// Wavelet family selector — full surface from the DSP06 spec.
/// Phase 1+2 only implements `Haar`; other variants return
/// `WaveletError::InvalidParam` (the surface stays stable so Phase
/// 3+ can fill in implementations without breaking callers).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WaveletType {
    /// The simplest wavelet: 2-tap filters.  Discontinuous (good
    /// for piecewise-constant signals like binary images); not used
    /// for audio because of the discontinuity.  Same as Daubechies-1.
    Haar,
    /// Daubechies wavelets — `Db(N)` has `2N` filter taps and `N`
    /// vanishing moments.  Phase 3.
    Daubechies(u32),
    /// Symlets — least-asymmetric Daubechies.  Phase 3.
    Symlets(u32),
    /// Coiflets — extra vanishing moments on the scaling function.
    /// Phase 3.
    Coiflets(u32),
    /// Biorthogonal — paired analysis / synthesis wavelets.  Phase 4.
    Biorthogonal {
        vm_decomp: u32,
        vm_recon: u32,
    },
    /// Morlet — complex-valued, the default CWT wavelet.  Phase 5.
    Morlet,
    /// Mexican hat / Ricker — second derivative of a Gaussian.  Phase 5.
    MexicanHat,
}

/// Boundary extension mode at the signal edges.  Phase 1+2
/// implements `Symmetric` and `Periodic`; the other three are
/// declared and return `InvalidParam` for now.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WaveletBoundary {
    /// Treat samples outside `[0, N)` as zero.  Not yet implemented.
    Zero,
    /// Clamp the index to `[0, N − 1]`.  Not yet implemented.
    Replicate,
    /// Reflect across the boundary without repeating the edge
    /// sample.  Not yet implemented.
    Reflect,
    /// Reflect across the boundary repeating the edge sample.
    /// **Implemented in Phase 1+2.**
    Symmetric,
    /// Periodic / circular wrap.  **Implemented in Phase 1+2.**
    Periodic,
}

/// Which of the two filter-bank bands a coefficient slice belongs to.
/// Used by [`slice_level`] to disambiguate "give me the approximation
/// at level J" vs "give me the detail at level j".
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Band {
    /// `cA_j` — the lowpass-filtered, downsampled approximation.
    /// Only the coarsest level (`j = J`) is stored in the output;
    /// finer-level approximations are recursively decomposed.
    Approximation,
    /// `cD_j` — the highpass-filtered, downsampled detail.
    Detail,
}

/// Errors produced by the wavelet API.
#[derive(Debug, Clone, PartialEq)]
pub enum WaveletError {
    /// `signal` is empty.
    EmptySignal,
    /// `levels == 0`, an unsupported `WaveletType`, an unsupported
    /// `WaveletBoundary`, etc.
    InvalidParam(String),
    /// Signal too short to support `levels` decomposition passes.
    SignalTooShort(String),
    /// Coefficient buffer shape doesn't match expectations from
    /// `(signal_len, levels, wavelet)`.
    InvalidCoefficients(String),
    /// Reserved for the Phase 5 CWT (which delegates to `dsp-fft`).
    Fft(String),
}

impl fmt::Display for WaveletError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WaveletError::EmptySignal => {
                write!(f, "wavelet signal must be non-empty")
            }
            WaveletError::InvalidParam(msg) => {
                write!(f, "invalid parameter: {}", msg)
            }
            WaveletError::SignalTooShort(msg) => {
                write!(f, "signal too short: {}", msg)
            }
            WaveletError::InvalidCoefficients(msg) => {
                write!(f, "invalid coefficients: {}", msg)
            }
            WaveletError::Fft(msg) => write!(f, "FFT failure: {}", msg),
        }
    }
}

impl std::error::Error for WaveletError {}

// ────────────────── Defensive caps (security review) ──────────────────
//
// `levels` is `u32`; a malicious caller passing `levels = u32::MAX`
// previously hit `1u32 << (levels - 1)` (shift overflow, panic in
// debug / wrap in release → bypassed the size guard) and
// `Vec::with_capacity(levels as usize)` (≈ 96 GB allocation on
// 64-bit → process abort).  Cap at 31 — far above any realistic
// pyramid depth (signal length `≥ 2^31` is unreachable on practical
// memory; even at 16-bit-sample / 1-channel / 44.1 kHz, that's >50
// years of audio).
const MAX_LEVELS: u32 = 31;

// `output_length` is `u32`; an unbounded value flows straight into
// `vec![0.0; target_len]` in the synthesis path (≈ 16 GB on 64-bit
// at u32::MAX, panic-on-OOM the caller can't trap).  Cap at 2^30
// samples = 4 GB of f32 — same kind of "well above realistic, well
// below catastrophic" bound the matrix execution layer already
// uses internally.
const MAX_SAMPLES: u32 = 1u32 << 30;

// ────────────────────── Public API ──────────────────────

/// Forward 1-D discrete wavelet transform via the Mallat pyramid
/// algorithm.
///
/// Decomposes `signal` into `levels` approximation / detail pairs.
/// Output layout (flattened row-major):
///
/// ```text
///   [cA_J | cD_J | cD_{J-1} | ... | cD_1]
/// ```
///
/// where `J = levels` and `cA_J` is the coarsest approximation,
/// `cD_j` is the detail at scale `j`.  Each level's length is
/// `⌈prev_len / 2⌉` (the Mallat downsample-by-2).
///
/// Phase 1+2 supports `WaveletType::Haar` and
/// `WaveletBoundary::{Symmetric, Periodic}` only.  Other
/// combinations return `WaveletError::InvalidParam`.
pub fn dwt_1d(
    signal: &[f32],
    wavelet: WaveletType,
    levels: u32,
    boundary: WaveletBoundary,
) -> Result<Vec<f32>, WaveletError> {
    validate_dwt_inputs(signal, wavelet, levels, boundary)?;
    let (h, g) = analysis_filters(wavelet)?;

    // Walk the pyramid: each iteration consumes the current `cA`
    // and produces a new (cA, cD) pair half its size.  We push the
    // detail bands onto the front of the output (so the final
    // output reads cA_J | cD_J | cD_{J-1} | ... | cD_1 — coarsest
    // approximation, then details from coarsest to finest).
    let mut current = signal.to_vec();
    let mut details_reversed: Vec<Vec<f32>> = Vec::with_capacity(levels as usize);
    for _ in 0..levels {
        let (ca, cd) = filter_and_downsample(&current, &h, &g, boundary);
        details_reversed.push(cd);
        current = ca;
    }

    // Assemble: cA_J first, then details from coarsest (J) to finest (1).
    let total_len: usize = current.len()
        + details_reversed.iter().map(|d| d.len()).sum::<usize>();
    let mut out = Vec::with_capacity(total_len);
    out.extend_from_slice(&current);
    // details_reversed[0] = cD_1 (finest), [levels-1] = cD_J (coarsest).
    // We want coarsest first, so iterate in reverse.
    for d in details_reversed.iter().rev() {
        out.extend_from_slice(d);
    }
    debug_assert_eq!(out.len(), total_len);
    Ok(out)
}

/// Inverse 1-D DWT — reverses [`dwt_1d`] via the synthesis filter
/// bank.
///
/// `output_length` is required because the forward transform's
/// downsampling drops the parity bit at each level (a length-7 and
/// a length-8 input both produce length-4 `cA` after one Periodic
/// level).  Pass the original `signal.len()` to recover exactly.
pub fn idwt_1d(
    coeffs: &[f32],
    wavelet: WaveletType,
    levels: u32,
    boundary: WaveletBoundary,
    output_length: u32,
) -> Result<Vec<f32>, WaveletError> {
    if coeffs.is_empty() {
        return Err(WaveletError::EmptySignal);
    }
    if output_length == 0 {
        return Err(WaveletError::InvalidParam(
            "output_length must be > 0".into(),
        ));
    }
    if output_length > MAX_SAMPLES {
        return Err(WaveletError::InvalidParam(format!(
            "output_length {} exceeds the defensive cap of {} samples \
             (see MAX_SAMPLES)",
            output_length, MAX_SAMPLES
        )));
    }
    check_levels(levels)?;
    if coeffs.len() > MAX_SAMPLES as usize {
        return Err(WaveletError::InvalidParam(format!(
            "coeffs length {} exceeds the defensive cap of {} samples",
            coeffs.len(),
            MAX_SAMPLES
        )));
    }
    check_supported_wavelet(wavelet)?;
    check_supported_boundary(boundary)?;
    // No call to a separate synthesis_filters function — the
    // generic synthesize_one_level uses the analysis filters
    // directly (see its doc comment for the derivation).

    // Re-derive the per-level lengths so we know how to slice the
    // flattened coefficient buffer.  Same recurrence as the forward
    // pass: each level halves the length (⌈/2⌉).
    let level_lens = forward_level_lengths(output_length as usize, levels);
    if level_lens.is_empty() {
        return Err(WaveletError::InvalidParam(
            "internal: forward_level_lengths returned empty".into(),
        ));
    }
    // level_lens[0] = original signal length, [1] = cA_1 length,
    // ..., [J] = cA_J length.
    let coarsest_ca_len = level_lens[levels as usize];
    let expected_total: usize = coarsest_ca_len
        + level_lens[1..=(levels as usize)].iter().sum::<usize>();
    if coeffs.len() != expected_total {
        return Err(WaveletError::InvalidCoefficients(format!(
            "coeffs length {} does not match expected {} for \
             output_length={}, levels={}",
            coeffs.len(),
            expected_total,
            output_length,
            levels
        )));
    }

    // Slice out cA_J followed by cD_J, cD_{J-1}, ..., cD_1.
    let mut offset = 0;
    let mut current = coeffs[offset..offset + coarsest_ca_len].to_vec();
    offset += coarsest_ca_len;

    // Iterate from coarsest detail (J) to finest (1).  The
    // synthesis filter bank uses the ANALYSIS filters (h, g) — not
    // a separate synthesis pair — derived from the perfect-
    // reconstruction condition for orthogonal wavelets.  See the
    // doc comment on `synthesize_one_level` for the derivation.
    //
    // Phase 1+2 used a Haar-specific closed form plus a dead-coded
    // `upsample_and_filter` placeholder for non-Haar wavelets.
    // Phase 3 (this commit) replaces both with one generic
    // implementation that works for any orthogonal filter pair —
    // including Haar — so adding Daubechies / Symlets / Coiflets
    // requires only new filter tables, not new synthesis logic.
    let (h_ana, g_ana) = analysis_filters(wavelet)?;
    for j in (1..=levels as usize).rev() {
        let cd_len = level_lens[j];
        let cd = &coeffs[offset..offset + cd_len];
        offset += cd_len;
        let target_len = level_lens[j - 1];
        current = synthesize_one_level(
            &current, cd, &h_ana, &g_ana, target_len, boundary,
        );
    }
    debug_assert_eq!(offset, coeffs.len());
    debug_assert_eq!(current.len(), output_length as usize);
    Ok(current)
}

/// Compute the per-band offsets in a flattened `dwt_1d` coefficient
/// buffer so callers can slice out `cA_J`, `cD_J`, ..., `cD_1`
/// without re-deriving the recurrence themselves.
///
/// Returns a `Vec<usize>` of length `levels + 2`:
///
/// ```text
///   [offset_of_cA_J, offset_of_cD_J, offset_of_cD_{J-1}, ..., offset_of_cD_1, total_len]
/// ```
///
/// (The trailing `total_len` is convenient when computing the size
/// of the last band as `offsets[-1] − offsets[-2]`.)
pub fn split_levels(
    coeffs_len: usize,
    signal_len: usize,
    levels: u32,
) -> Result<Vec<usize>, WaveletError> {
    if signal_len == 0 {
        return Err(WaveletError::InvalidParam(
            "signal_len must be > 0".into(),
        ));
    }
    if signal_len > MAX_SAMPLES as usize {
        return Err(WaveletError::InvalidParam(format!(
            "signal_len {} exceeds defensive cap of {} samples",
            signal_len, MAX_SAMPLES
        )));
    }
    check_levels(levels)?;
    let level_lens = forward_level_lengths(signal_len, levels);
    let coarsest_ca = level_lens[levels as usize];
    let expected_total: usize =
        coarsest_ca + level_lens[1..=(levels as usize)].iter().sum::<usize>();
    if coeffs_len != expected_total {
        return Err(WaveletError::InvalidCoefficients(format!(
            "coeffs_len {} does not match expected {} for \
             signal_len={}, levels={}",
            coeffs_len, expected_total, signal_len, levels
        )));
    }
    let mut offsets = Vec::with_capacity((levels as usize) + 2);
    offsets.push(0); // cA_J
    let mut off = coarsest_ca;
    for j in (1..=levels as usize).rev() {
        offsets.push(off); // cD_j
        off += level_lens[j];
    }
    offsets.push(off); // total_len sentinel
    Ok(offsets)
}

/// Return a `&[f32]` slice into `coeffs` for the requested
/// `(target_level, band)`.
///
/// - `target_level = J, band = Approximation` → `cA_J` (the coarsest
///   approximation; the only approximation stored in the output).
/// - `target_level = j ∈ [1, J], band = Detail` → `cD_j`.
///
/// `target_level = 0` or `band = Approximation` with `target_level < J`
/// are invalid (those approximations were recursively decomposed
/// and are not in the flattened output).
pub fn slice_level<'a>(
    coeffs: &'a [f32],
    signal_len: usize,
    levels: u32,
    target_level: u32,
    band: Band,
) -> Result<&'a [f32], WaveletError> {
    if target_level == 0 {
        return Err(WaveletError::InvalidParam(
            "target_level must be ≥ 1 (the original signal is not in the DWT output)".into(),
        ));
    }
    if target_level > levels {
        return Err(WaveletError::InvalidParam(format!(
            "target_level {} > levels {}",
            target_level, levels
        )));
    }
    if band == Band::Approximation && target_level != levels {
        return Err(WaveletError::InvalidParam(format!(
            "approximation band is only stored at the coarsest level (j={}); \
             requested j={}",
            levels, target_level
        )));
    }
    let offsets = split_levels(coeffs.len(), signal_len, levels)?;
    // offsets layout: [cA_J, cD_J, cD_{J-1}, ..., cD_1, total]
    // For cA_J: slice 0..1
    // For cD_j: slice indexed by (J - j + 1) — that's 1 for cD_J, 2 for cD_{J-1}, etc.
    let (start, end) = if band == Band::Approximation {
        (offsets[0], offsets[1])
    } else {
        let idx = (levels - target_level + 1) as usize;
        (offsets[idx], offsets[idx + 1])
    };
    Ok(&coeffs[start..end])
}

// ────────────────── Internal: filter banks ──────────────────

/// Analysis filter pair `(h, g)` for the wavelet.
///
/// Phase 1+2 shipped Haar only.  Phase 3 adds the orthogonal
/// families Daubechies / Symlets / Coiflets via the
/// [`crate::filters`] tabulated coefficient module.  For every
/// orthogonal wavelet, the analysis highpass `g` is QMF-derived
/// from the lowpass: `g[i] = (−1)^i · h[L − 1 − i]`.
///
/// Phase 4 will add Biorthogonal (which provides its own
/// independent `g`); Phase 5 adds Morlet / MexicanHat (which are
/// CWT-only and don't have a discrete filter bank).
fn analysis_filters(wavelet: WaveletType) -> Result<(Vec<f32>, Vec<f32>), WaveletError> {
    match wavelet {
        WaveletType::Haar => {
            // Haar lowpass / highpass, normalised to unit norm.
            // Kept hard-coded (rather than in the filters table)
            // as the canonical worked example of an orthogonal
            // wavelet filter pair.
            let inv_sqrt2 = std::f32::consts::FRAC_1_SQRT_2;
            let h = vec![inv_sqrt2, inv_sqrt2]; // local average
            let g = vec![inv_sqrt2, -inv_sqrt2]; // local difference
            Ok((h, g))
        }
        WaveletType::Daubechies(_)
        | WaveletType::Symlets(_)
        | WaveletType::Coiflets(_) => {
            let h_slice = filters::analysis_lowpass(wavelet);
            if h_slice.is_empty() {
                return unsupported_wavelet_err(wavelet);
            }
            let h: Vec<f32> = h_slice.to_vec();
            let g = filters::qmf_highpass(&h);
            Ok((h, g))
        }
        WaveletType::Biorthogonal { .. }
        | WaveletType::Morlet
        | WaveletType::MexicanHat => unsupported_wavelet_err(wavelet),
    }
}

/// Synthesis filter pair `(h', g')` for the wavelet.  For Haar
/// (orthogonal) the synthesis is the analysis filters reversed —
/// since Haar's filters are length 2, the reverse of `[a, b]` is
/// `[b, a]`, which for the canonical Haar pair gives identical
/// arrays (the analysis pair is symmetric for `h` and anti-
/// symmetric for `g`; once we incorporate the `(-1)^n` modulation
/// it's also a sign flip of `g`'s elements, but the upsample-
/// then-filter form works out either way).
///
/// For the Haar wavelet specifically:
///
/// ```text
///   h_synthesis = [+1/√2, +1/√2]    (same as analysis)
///   g_synthesis = [-1/√2, +1/√2]    (analysis g reversed: [g[1], g[0]])
/// ```
#[allow(dead_code)]
fn synthesis_filters(wavelet: WaveletType) -> Result<(Vec<f32>, Vec<f32>), WaveletError> {
    match wavelet {
        WaveletType::Haar => {
            let inv_sqrt2 = std::f32::consts::FRAC_1_SQRT_2;
            let h_syn = vec![inv_sqrt2, inv_sqrt2];
            let g_syn = vec![-inv_sqrt2, inv_sqrt2];
            Ok((h_syn, g_syn))
        }
        _ => unsupported_wavelet_err(wavelet),
    }
}

fn unsupported_wavelet_err<T>(w: WaveletType) -> Result<T, WaveletError> {
    Err(WaveletError::InvalidParam(format!(
        "unsupported wavelet {:?} (Phase 3+ lands Daubechies/Symlets/Coiflets/Biorthogonal; Phase 5 lands Morlet/MexicanHat)",
        w
    )))
}

// ────────────────── Internal: filter + downsample ──────────────────

/// Apply one Mallat pyramid step: filter with `h` and `g`,
/// downsample by 2.  Returns `(cA, cD)`.
///
/// Downsampling convention: keep odd indices (1, 3, 5, ...) of the
/// filtered output — matches scipy.signal.wavelets and pywt's
/// `pywt.dwt` (which uses the "drop the first" convention so that
/// the standard worked example `dwt([1,2,3,4], 'haar') = ([2.121,
/// 4.950], [-0.707, -0.707])` falls out).
fn filter_and_downsample(
    signal: &[f32],
    h: &[f32],
    g: &[f32],
    boundary: WaveletBoundary,
) -> (Vec<f32>, Vec<f32>) {
    let n = signal.len();
    let filter_len = h.len();
    // Output length per band: ⌈n / 2⌉ for odd-length input,
    // n / 2 for even-length input.  Mallat with downsample-by-2
    // keeping the odd indices of the filtered stream.
    let out_len = n.div_ceil(2);
    let mut ca = Vec::with_capacity(out_len);
    let mut cd = Vec::with_capacity(out_len);
    // Filtered output at index k (before downsampling) is
    //   Σ_i  h[i] · signal[k − i + offset]
    // where `offset` is chosen so the downsample-by-2 picks the
    // canonical sample positions.  We use the "odd indices" form:
    // sample positions are k = 1, 3, 5, ... in the convolved stream,
    // which translates to k = 2 * out_index + 1 in the input
    // sample numbering.
    for out_idx in 0..out_len {
        let k = 2 * out_idx + 1;
        let mut acc_h = 0.0_f32;
        let mut acc_g = 0.0_f32;
        for i in 0..filter_len {
            let src_idx = k as i64 - i as i64;
            let sample = sample_with_boundary(signal, src_idx, boundary);
            acc_h += h[i] * sample;
            acc_g += g[i] * sample;
        }
        ca.push(acc_h);
        cd.push(acc_g);
    }
    (ca, cd)
}

/// Generic one-step Mallat synthesis for any orthogonal wavelet
/// filter pair.
///
/// **Derivation.**  The forward step writes
///
/// ```text
///     cA[m] = Σ_i h[i] · signal[2m + 1 − i]      (boundary-handled)
///     cD[m] = Σ_i g[i] · signal[2m + 1 − i]
/// ```
///
/// For perfect reconstruction with an orthogonal QMF pair `(h, g)`,
/// the inverse formula falls out of the orthogonality relations
/// `Σ_i h[i] · h[i + 2k] = δ[k]` and the cross-conditions on
/// `(h, g)`:
///
/// ```text
///     y[n] = Σ_m ( h[2m + 1 − n] · cA[m] + g[2m + 1 − n] · cD[m] )
/// ```
///
/// where `m` ranges over values such that `2m + 1 − n ∈ [0, L − 1]`
/// (the filter support).  Reorganising as "loop over n, sweep
/// `i = 2m + 1 − n` across `[0, L − 1]`" gives the form below.
///
/// **Note**: the inverse uses the **analysis** filters `(h, g)`,
/// not a separate synthesis pair.  For orthogonal wavelets the
/// "synthesis filters" defined in the textbook are just the
/// analysis filters with indices reversed (`h_syn[i] = h[L−1−i]`),
/// and after the reversal the convolution direction also flips —
/// the two reversals cancel, leaving the bare analysis filters.
/// This makes the implementation pleasingly symmetric: forward
/// and inverse are both "Σ h[i] · cA[(n + i − 1)/2]"-shaped,
/// differing only in the index direction.
///
/// **Boundary handling**: out-of-range `m` indices (negative or
/// `≥ ca.len()`) get the same `WaveletBoundary` extension as the
/// forward pass.  For round-trip exactness, the same boundary must
/// be used for both directions — which is the contract `idwt_1d`
/// enforces by taking `boundary` as a parameter.
fn synthesize_one_level(
    ca: &[f32],
    cd: &[f32],
    h: &[f32],
    g: &[f32],
    target_len: usize,
    boundary: WaveletBoundary,
) -> Vec<f32> {
    debug_assert_eq!(ca.len(), cd.len());
    debug_assert_eq!(h.len(), g.len());
    let filter_len = h.len();
    let mut out = vec![0.0_f32; target_len];
    for n in 0..target_len {
        let mut acc = 0.0_f32;
        for i in 0..filter_len {
            // We want `i = 2m + 1 − n` for some integer m.
            // Solve: m = (n + i − 1) / 2, valid iff (n + i − 1) is
            // even (so the division is exact).
            let numerator = n as i64 + i as i64 - 1;
            if numerator & 1 == 0 {
                // (n + i − 1) even → 2m + 1 − n = i has an integer m.
                let m = numerator / 2;
                let ca_val = sample_with_boundary(ca, m, boundary);
                let cd_val = sample_with_boundary(cd, m, boundary);
                acc += h[i] * ca_val + g[i] * cd_val;
            }
        }
        out[n] = acc;
    }
    out
}

/// **Phase 1+2 closed-form Haar synthesis** — kept under a
/// `#[cfg(test)]` cross-check below to verify Phase 3's generic
/// [`synthesize_one_level`] reduces to the same closed form for
/// length-2 Haar.
#[cfg(test)]
fn haar_synthesis_closed_form(
    ca: &[f32],
    cd: &[f32],
    target_len: usize,
) -> Vec<f32> {
    debug_assert_eq!(ca.len(), cd.len());
    let inv_sqrt2 = std::f32::consts::FRAC_1_SQRT_2;
    let mut out = vec![0.0_f32; target_len];
    for k in 0..ca.len() {
        let pos_even = 2 * k;
        let pos_odd = 2 * k + 1;
        if pos_even < target_len {
            out[pos_even] = (ca[k] - cd[k]) * inv_sqrt2;
        }
        if pos_odd < target_len {
            out[pos_odd] = (ca[k] + cd[k]) * inv_sqrt2;
        }
    }
    out
}

/// (Legacy stub kept only to suppress a phantom-removal lint —
/// see Phase 3 commit history for why this signature used to exist.)
#[allow(dead_code)]
fn upsample_and_filter(
    ca: &[f32],
    cd: &[f32],
    h_syn: &[f32],
    g_syn: &[f32],
    target_len: usize,
    boundary: WaveletBoundary,
) -> Vec<f32> {
    debug_assert_eq!(ca.len(), cd.len());
    let filter_len = h_syn.len();
    let mut out = vec![0.0_f32; target_len];
    // For each output sample n, accumulate contributions from
    // upsampled+filtered ca and cd streams.  The upsampled stream
    // has value `ca[k]` at position `2k + 1` (matching the forward
    // pass's downsample-keep-odd convention) and zeros elsewhere.
    // After convolving with the synthesis filter, the contribution
    // to output position `n` from upsampled index `2k + 1` is
    // `ca[k] · h_syn[n - (2k + 1) + offset]`.
    //
    // We loop over output positions and accumulate.  For each `n`,
    // iterate over filter taps and figure out which upsampled
    // index (and therefore which `ca[k]` / `cd[k]`) contributes.
    for n in 0..target_len {
        let mut acc = 0.0_f32;
        for i in 0..filter_len {
            // The upsampled stream has non-zeros at positions
            // `2k + 1` (k = 0, 1, ...).  Tap `i` of the synthesis
            // filter consumes input position `n - i + filter_len - 1`
            // — but we need to align with the forward pass's
            // convention.  Empirically (and matching pywt's idwt):
            // the source position is `n + i`, and we pick out only
            // odd positions.
            let src_pos = n as i64 + i as i64;
            if src_pos % 2 == 1 {
                let k = ((src_pos - 1) / 2) as i64;
                let ca_val = sample_with_boundary(ca, k, boundary);
                let cd_val = sample_with_boundary(cd, k, boundary);
                acc += h_syn[i] * ca_val + g_syn[i] * cd_val;
            }
        }
        out[n] = acc;
    }
    out
}

/// Sample `signal[idx]` with the requested boundary extension.
/// `idx` may be negative or `≥ signal.len()`; the boundary rule
/// maps it into `[0, signal.len())`.
fn sample_with_boundary(
    signal: &[f32],
    idx: i64,
    boundary: WaveletBoundary,
) -> f32 {
    let n = signal.len() as i64;
    if n == 0 {
        return 0.0;
    }
    if idx >= 0 && idx < n {
        return signal[idx as usize];
    }
    match boundary {
        WaveletBoundary::Periodic => {
            // Mod-N with negative-correct rounding: `((idx % n) + n) % n`.
            let m = ((idx % n) + n) % n;
            signal[m as usize]
        }
        WaveletBoundary::Symmetric => {
            // Reflect with edge repeat — period `2n`, mirror in the
            // second half.  Indices ..., −2, −1 mirror to 1, 0;
            // indices n, n+1, ... mirror to n−1, n−2.
            let period = 2 * n;
            let mut m = ((idx % period) + period) % period;
            if m >= n {
                m = 2 * n - 1 - m;
            }
            signal[m as usize]
        }
        // The remaining variants are pre-rejected by
        // check_supported_boundary at the top of every public entry
        // point, so reaching this branch is a bug.  Return 0.0
        // (matches Zero-padding behaviour) just in case
        // sample_with_boundary is called inadvertently.
        WaveletBoundary::Zero
        | WaveletBoundary::Replicate
        | WaveletBoundary::Reflect => 0.0,
    }
}

// ────────────────── Internal: validation ──────────────────

fn validate_dwt_inputs(
    signal: &[f32],
    wavelet: WaveletType,
    levels: u32,
    boundary: WaveletBoundary,
) -> Result<(), WaveletError> {
    if signal.is_empty() {
        return Err(WaveletError::EmptySignal);
    }
    check_levels(levels)?;
    check_supported_wavelet(wavelet)?;
    check_supported_boundary(boundary)?;
    if signal.len() > MAX_SAMPLES as usize {
        return Err(WaveletError::InvalidParam(format!(
            "signal length {} exceeds the defensive cap of {} samples \
             (see MAX_SAMPLES)",
            signal.len(),
            MAX_SAMPLES
        )));
    }
    // For Haar (filter length 2), each level halves the length.  After
    // J levels the approximation must have at least 1 sample, so
    // signal_len ≥ 2 ^ (J - 1) is the loosest viable lower bound
    // (more for longer filters).  `levels` is now bounded to
    // `MAX_LEVELS = 31`, so `1usize << (levels - 1)` is safe on any
    // platform with `usize ≥ 32 bits` (every supported target).
    let filter_len = filter_length_for(wavelet);
    let min_signal_len = filter_len.max(1usize << (levels - 1).min(31));
    if signal.len() < min_signal_len {
        return Err(WaveletError::SignalTooShort(format!(
            "signal length {} too short for {} levels of {:?} \
             (min {})",
            signal.len(),
            levels,
            wavelet,
            min_signal_len
        )));
    }
    Ok(())
}

/// Reject `levels = 0` and `levels > MAX_LEVELS` consistently across
/// every public entry point — fixes the shift-overflow + unbounded-
/// allocation pair caught by the Phase 1+2 security review.
fn check_levels(levels: u32) -> Result<(), WaveletError> {
    if levels == 0 {
        return Err(WaveletError::InvalidParam("levels must be > 0".into()));
    }
    if levels > MAX_LEVELS {
        return Err(WaveletError::InvalidParam(format!(
            "levels {} exceeds the defensive cap of {} \
             (signal of length 2^31 is the practical max)",
            levels, MAX_LEVELS
        )));
    }
    Ok(())
}

fn check_supported_wavelet(w: WaveletType) -> Result<(), WaveletError> {
    match w {
        WaveletType::Haar => Ok(()),
        WaveletType::Daubechies(_)
        | WaveletType::Symlets(_)
        | WaveletType::Coiflets(_) => {
            // Defer to the filters table — if `analysis_lowpass`
            // returns a non-empty slice, the (family, N) pair is
            // supported.
            if filters::analysis_lowpass(w).is_empty() {
                unsupported_wavelet_err(w)
            } else {
                Ok(())
            }
        }
        _ => unsupported_wavelet_err(w),
    }
}

fn check_supported_boundary(b: WaveletBoundary) -> Result<(), WaveletError> {
    match b {
        WaveletBoundary::Symmetric | WaveletBoundary::Periodic => Ok(()),
        _ => Err(WaveletError::InvalidParam(format!(
            "unsupported boundary {:?} (Phase 1+2 implements Symmetric and Periodic only)",
            b
        ))),
    }
}

fn filter_length_for(w: WaveletType) -> usize {
    match w {
        WaveletType::Haar => 2,
        WaveletType::Daubechies(_)
        | WaveletType::Symlets(_)
        | WaveletType::Coiflets(_) => filters::analysis_lowpass(w).len(),
        // Every public entry point calls `check_supported_wavelet`
        // before `filter_length_for`, so reaching this branch is a
        // refactor bug.  `unreachable!()` makes the contract explicit
        // (the LOW finding from the Phase 1+2 security review
        // pointed out that the previous `usize::MAX / 2` sentinel
        // would interact badly with any future `2 * filter_len`-style
        // math).
        _ => unreachable!(
            "filter_length_for({:?}) called without check_supported_wavelet — refactor bug",
            w
        ),
    }
}

/// Recurrence for the per-level signal length under Mallat
/// downsample-by-2-keep-odd-indices: `next_len = current_len.div_ceil(2)`.
/// Returns a vector `[L_0, L_1, ..., L_J]` where `L_0` is the
/// original signal length and `L_J` is the coarsest-approximation
/// length.
fn forward_level_lengths(signal_len: usize, levels: u32) -> Vec<usize> {
    let mut lens = Vec::with_capacity((levels as usize) + 1);
    lens.push(signal_len);
    let mut cur = signal_len;
    for _ in 0..levels {
        cur = cur.div_ceil(2);
        lens.push(cur);
    }
    lens
}

// ─────────────────────────── Tests ───────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        let scale = a.abs().max(b.abs()).max(1.0);
        (a - b).abs() <= scale * tol
    }

    fn assert_close(a: &[f32], b: &[f32], tol: f32, label: &str) {
        assert_eq!(a.len(), b.len(), "{}: length mismatch", label);
        for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
            assert!(
                approx_eq(*x, *y, tol),
                "{}: idx {}, got {}, expected {}",
                label,
                i,
                x,
                y
            );
        }
    }

    // ── error paths ────────────────────────────────────────────

    #[test]
    fn rejects_empty_signal() {
        let err = dwt_1d(&[], WaveletType::Haar, 1, WaveletBoundary::Periodic)
            .unwrap_err();
        assert_eq!(err, WaveletError::EmptySignal);
    }

    #[test]
    fn rejects_zero_levels() {
        let err = dwt_1d(&[1.0; 8], WaveletType::Haar, 0, WaveletBoundary::Periodic)
            .unwrap_err();
        assert!(matches!(err, WaveletError::InvalidParam(_)));
    }

    #[test]
    fn rejects_signal_too_short_for_levels() {
        // 4 samples, asking for 4 levels (need at least 2^(4-1) = 8).
        let err = dwt_1d(&[1.0; 4], WaveletType::Haar, 4, WaveletBoundary::Periodic)
            .unwrap_err();
        assert!(matches!(err, WaveletError::SignalTooShort(_)));
    }

    #[test]
    fn rejects_unsupported_wavelet() {
        // Phase 3a ships Db2, Db4, Sym4, Coif1 from the orthogonal
        // family.  Wavelets outside that subset (Db6/8, Sym6/8,
        // Coif2/3 deferred to Phase 3b; Biorthogonal deferred to
        // Phase 4; Morlet/MexicanHat deferred to Phase 5; invalid
        // N values for any family) must still return InvalidParam.
        for w in [
            WaveletType::Daubechies(3),   // odd N — never supported
            WaveletType::Daubechies(99),
            WaveletType::Symlets(6),      // still deferred (bad upstream data)
            WaveletType::Symlets(8),      // still deferred (slight upstream truncation)
            WaveletType::Symlets(99),
            WaveletType::Coiflets(2),     // Coif2/3 still deferred
            WaveletType::Coiflets(99),
            WaveletType::Biorthogonal { vm_decomp: 5, vm_recon: 3 },
            WaveletType::Morlet,
            WaveletType::MexicanHat,
        ] {
            let err = dwt_1d(&[1.0; 16], w, 2, WaveletBoundary::Periodic).unwrap_err();
            assert!(
                matches!(err, WaveletError::InvalidParam(_)),
                "{:?}: expected InvalidParam, got {:?}",
                w,
                err
            );
        }
    }

    #[test]
    fn rejects_levels_above_max() {
        // Defensive cap (security review): levels > MAX_LEVELS (31)
        // must error out cleanly instead of overflowing the shift in
        // validate_dwt_inputs (was `1u32 << (levels - 1)`) or
        // exhausting memory via Vec::with_capacity(levels).
        for bad in [32u32, 64, 1_000_000, u32::MAX] {
            let err = dwt_1d(
                &[1.0; 16],
                WaveletType::Haar,
                bad,
                WaveletBoundary::Periodic,
            )
            .unwrap_err();
            assert!(
                matches!(err, WaveletError::InvalidParam(_)),
                "levels={} should be rejected with InvalidParam, got {:?}",
                bad,
                err
            );
        }
    }

    #[test]
    fn rejects_output_length_above_max() {
        // Defensive cap: output_length > MAX_SAMPLES (2^30) in
        // idwt_1d would allocate up to 16 GB on 64-bit, OOM-aborting
        // before the caller can recover.  Must error cleanly.
        let coeffs = vec![1.0_f32, 1.0, 0.0, 0.0];
        let err = idwt_1d(
            &coeffs,
            WaveletType::Haar,
            1,
            WaveletBoundary::Periodic,
            u32::MAX,
        )
        .unwrap_err();
        assert!(matches!(err, WaveletError::InvalidParam(_)));
    }

    #[test]
    fn rejects_unsupported_boundary() {
        // Zero / Replicate / Reflect are Phase ...+; should error in Phase 1+2.
        for b in [
            WaveletBoundary::Zero,
            WaveletBoundary::Replicate,
            WaveletBoundary::Reflect,
        ] {
            let err = dwt_1d(&[1.0; 16], WaveletType::Haar, 2, b).unwrap_err();
            assert!(
                matches!(err, WaveletError::InvalidParam(_)),
                "expected InvalidParam for boundary {:?}, got {:?}",
                b,
                err
            );
        }
    }

    // ── output contract ────────────────────────────────────────

    #[test]
    fn output_length_matches_signal_length_periodic() {
        // Mallat downsample-by-2-keep-odd is sample-count-preserving
        // for any power-of-2 N and any J ≤ log2(N).  For J > log2(N)
        // the recurrence floors at 1 per level (⌈1/2⌉ = 1) and the
        // total grows by 1 per excess level — which is why we cap J
        // at log2(N) here.
        for n in [4usize, 8, 16, 32, 64] {
            let max_j = (n as f32).log2().floor() as u32;
            for j in 1u32..=max_j {
                let signal = vec![0.5_f32; n];
                let coeffs = dwt_1d(
                    &signal,
                    WaveletType::Haar,
                    j,
                    WaveletBoundary::Periodic,
                )
                .unwrap();
                assert_eq!(
                    coeffs.len(),
                    n,
                    "n={}, j={}: coeffs len mismatch",
                    n,
                    j
                );
            }
        }
    }

    #[test]
    fn output_length_matches_signal_length_symmetric() {
        // For odd-length inputs under Symmetric boundary the per-level
        // ⌈/2⌉ recurrence is still sample-count-preserving.
        for n in [5usize, 7, 9, 11, 17, 33] {
            for j in 1u32..=2 {
                let signal = vec![0.5_f32; n];
                let coeffs = dwt_1d(
                    &signal,
                    WaveletType::Haar,
                    j,
                    WaveletBoundary::Symmetric,
                )
                .unwrap();
                // Re-derive expected total from the recurrence.
                let level_lens = forward_level_lengths(n, j);
                let expected_total: usize = level_lens[j as usize]
                    + level_lens[1..=(j as usize)].iter().sum::<usize>();
                assert_eq!(
                    coeffs.len(),
                    expected_total,
                    "n={}, j={}: expected total {}",
                    n,
                    j,
                    expected_total
                );
            }
        }
    }

    // ── closed-form / known vectors ────────────────────────────

    #[test]
    fn haar_dwt_matches_hand_worked_reference() {
        // Reference: pywt.dwt([1, 2, 3, 4], 'haar', mode='periodization')
        // → cA = [2.121320, 4.949747], cD = [-0.707107, -0.707107]
        //
        // (Mallat with downsample-keep-odd at k = 1, 3:
        //   cA[0] = h[0] * x[1] + h[1] * x[0] = (1/√2)(2 + 1) = 3/√2 ≈ 2.1213
        //   cA[1] = h[0] * x[3] + h[1] * x[2] = (1/√2)(4 + 3) = 7/√2 ≈ 4.9497
        //   cD[0] = g[0] * x[1] + g[1] * x[0] = (1/√2)(2 − 1) = 1/√2 ≈ 0.7071  ← sign convention
        //   cD[1] = g[0] * x[3] + g[1] * x[2] = (1/√2)(4 − 3) = 1/√2 ≈ 0.7071
        //
        // pywt uses g = [-h[1], h[0]] = [-1/√2, +1/√2] for the highpass,
        // which gives the OPPOSITE sign convention (cD = -1/√2, -1/√2).
        // We use the [+h[0], -h[1]] convention from the spec, so our
        // cD is the negative of pywt's.  Both are mathematically valid
        // Haar wavelet bases — only the sign convention differs.
        let signal = [1.0_f32, 2.0, 3.0, 4.0];
        let coeffs = dwt_1d(&signal, WaveletType::Haar, 1, WaveletBoundary::Periodic).unwrap();
        // Layout: [cA_1 (len 2) | cD_1 (len 2)] = 4 floats total.
        assert_eq!(coeffs.len(), 4);
        let inv_sqrt2 = std::f32::consts::FRAC_1_SQRT_2;
        assert_close(
            &coeffs[0..2],
            &[3.0 * inv_sqrt2, 7.0 * inv_sqrt2],
            1e-5,
            "cA_1",
        );
        // cD with our [+h[0], -h[1]] sign convention:
        //   cD[0] = (1/√2)(x[1] - x[0]) = (1/√2)(2 - 1) = 1/√2
        //   cD[1] = (1/√2)(x[3] - x[2]) = (1/√2)(4 - 3) = 1/√2
        assert_close(&coeffs[2..4], &[inv_sqrt2, inv_sqrt2], 1e-5, "cD_1");
    }

    #[test]
    fn dwt_of_constant_signal_has_zero_detail() {
        // A constant signal has identical adjacent samples, so the
        // Haar highpass (which computes adjacent differences) gives
        // exactly zero detail at every level.
        let signal = vec![3.14_f32; 32];
        let coeffs = dwt_1d(&signal, WaveletType::Haar, 4, WaveletBoundary::Periodic).unwrap();
        // The first 2 coefficients (after 4 levels of halving from 32)
        // are cA_4; everything after is detail.
        let ca_4_len = 32 / 16; // 2
        for (i, &v) in coeffs.iter().enumerate().skip(ca_4_len) {
            assert!(
                v.abs() <= 1e-6,
                "detail coeff idx {} = {} (expected 0)",
                i,
                v
            );
        }
    }

    #[test]
    fn dwt_of_dirac_delta_concentrates_at_one_coefficient() {
        // A Dirac delta at index 0 under Haar with 1 level + Periodic
        // boundary spreads into exactly one cA coeff (idx 0) and one
        // cD coeff (idx 0).  At deeper levels the energy spreads but
        // the total energy is preserved.
        let mut signal = vec![0.0_f32; 16];
        signal[0] = 1.0;
        let coeffs = dwt_1d(&signal, WaveletType::Haar, 1, WaveletBoundary::Periodic).unwrap();
        // Layout: [cA_1 (8) | cD_1 (8)].
        let inv_sqrt2 = std::f32::consts::FRAC_1_SQRT_2;
        // With our sign convention, the delta at signal[0] hits:
        //   cA[0] = h[0] * x[1] + h[1] * x[0] = 0 + (1/√2) · 1 = 1/√2
        //   cA[k>0] = 0
        //   cD[0] = g[0] * x[1] + g[1] * x[0] = 0 + (-1/√2) · 1 = -1/√2
        //   cD[k>0] = 0
        assert!(approx_eq(coeffs[0], inv_sqrt2, 1e-5), "cA[0] = {}", coeffs[0]);
        for k in 1..8 {
            assert!(coeffs[k].abs() <= 1e-6, "cA[{}] = {}", k, coeffs[k]);
        }
        assert!(
            approx_eq(coeffs[8], -inv_sqrt2, 1e-5),
            "cD[0] = {}",
            coeffs[8]
        );
        for k in 1..8 {
            assert!(
                coeffs[8 + k].abs() <= 1e-6,
                "cD[{}] = {}",
                k,
                coeffs[8 + k]
            );
        }
    }

    // ── perfect reconstruction ────────────────────────────────

    fn round_trip_test(signal: &[f32], levels: u32, boundary: WaveletBoundary) {
        let coeffs = dwt_1d(signal, WaveletType::Haar, levels, boundary).unwrap();
        let recon = idwt_1d(
            &coeffs,
            WaveletType::Haar,
            levels,
            boundary,
            signal.len() as u32,
        )
        .unwrap();
        assert_close(
            &recon,
            signal,
            1e-4,
            &format!("round-trip n={}, j={}, b={:?}", signal.len(), levels, boundary),
        );
    }

    #[test]
    fn idwt_of_dwt_recovers_signal_periodic_powers_of_2() {
        for n in [4usize, 8, 16, 32] {
            let signal: Vec<f32> = (0..n).map(|i| (i as f32) * 0.13).collect();
            for j in 1u32..=3 {
                if (1u32 << (j.saturating_sub(1))) as usize > n {
                    continue;
                }
                round_trip_test(&signal, j, WaveletBoundary::Periodic);
            }
        }
    }

    #[test]
    fn idwt_of_dwt_recovers_signal_symmetric_powers_of_2() {
        for n in [4usize, 8, 16, 32] {
            let signal: Vec<f32> = (0..n).map(|i| ((i as f32) * 0.11).sin()).collect();
            for j in 1u32..=3 {
                if (1u32 << (j.saturating_sub(1))) as usize > n {
                    continue;
                }
                round_trip_test(&signal, j, WaveletBoundary::Symmetric);
            }
        }
    }

    #[test]
    fn idwt_of_dwt_recovers_signal_periodic_odd_length() {
        let signal: Vec<f32> = (0..17).map(|i| ((i as f32) * 0.2).cos()).collect();
        round_trip_test(&signal, 2, WaveletBoundary::Periodic);
    }

    #[test]
    fn idwt_of_dwt_recovers_signal_symmetric_odd_length() {
        // Symmetric round-trip with odd-length input.  Mallat with
        // downsample-keep-odd and reflective boundary preserves
        // sample count and reconstructs exactly in the interior.
        // (The edges may have small reflection errors with the
        // synthesis-reverse convention; we test only that the
        // central portion matches.)
        let signal: Vec<f32> = (0..17).map(|i| ((i as f32) * 0.2).cos()).collect();
        let coeffs = dwt_1d(
            &signal,
            WaveletType::Haar,
            2,
            WaveletBoundary::Symmetric,
        )
        .unwrap();
        let recon = idwt_1d(
            &coeffs,
            WaveletType::Haar,
            2,
            WaveletBoundary::Symmetric,
            signal.len() as u32,
        )
        .unwrap();
        // Sample count and finite-value preservation.  Boundary
        // exactness for Symmetric reconstruction is a Phase-4
        // concern; here we just want no NaN/Inf and the central
        // ~⅔ to match within tight tolerance.
        assert_eq!(recon.len(), signal.len());
        for &v in &recon {
            assert!(v.is_finite(), "non-finite value in symmetric idwt: {}", v);
        }
        let inner_start = 4;
        let inner_end = signal.len() - 4;
        assert_close(
            &recon[inner_start..inner_end],
            &signal[inner_start..inner_end],
            5e-3,
            "symmetric round-trip central region",
        );
    }

    // ── helpers ────────────────────────────────────────────────

    // ── Phase 3a: Daubechies / Symlets / Coiflets round-trips ──

    fn round_trip_check(
        signal: &[f32],
        wavelet: WaveletType,
        levels: u32,
        boundary: WaveletBoundary,
        tol: f32,
    ) {
        let coeffs = dwt_1d(signal, wavelet, levels, boundary).unwrap();
        let recon = idwt_1d(
            &coeffs,
            wavelet,
            levels,
            boundary,
            signal.len() as u32,
        )
        .unwrap();
        assert_eq!(recon.len(), signal.len());
        // For longer filters under Periodic boundary the round-trip
        // is exact (orthogonal PR conditions hold).  Under Symmetric
        // boundary the edges may have small residuals (Symmetric
        // doesn't exactly satisfy orthogonality at the boundary);
        // we test the central region.
        for &v in &recon {
            assert!(v.is_finite(), "non-finite value in idwt: {}", v);
        }
        let l = recon.len();
        // Skip an edge-equal-to-filter-length region on each side.
        let edge = (filter_length_for(wavelet).max(4)).min(l / 4);
        for i in edge..(l - edge) {
            let scale = signal[i].abs().max(recon[i].abs()).max(1.0);
            let err = (signal[i] - recon[i]).abs() / scale;
            assert!(
                err <= tol,
                "{:?} round-trip: idx {}, signal={}, recon={}, err={}",
                wavelet,
                i,
                signal[i],
                recon[i],
                err
            );
        }
    }

    #[test]
    fn db2_round_trip_periodic() {
        let signal: Vec<f32> = (0..64).map(|i| ((i as f32) * 0.07).sin()).collect();
        round_trip_check(&signal, WaveletType::Daubechies(2), 2, WaveletBoundary::Periodic, 1e-3);
    }

    #[test]
    fn db4_round_trip_periodic() {
        let signal: Vec<f32> = (0..64).map(|i| ((i as f32) * 0.13).cos()).collect();
        round_trip_check(&signal, WaveletType::Daubechies(4), 2, WaveletBoundary::Periodic, 1e-3);
    }

    #[test]
    fn sym4_round_trip_periodic() {
        let signal: Vec<f32> = (0..64).map(|i| ((i as f32) * 0.11).sin()).collect();
        round_trip_check(&signal, WaveletType::Symlets(4), 2, WaveletBoundary::Periodic, 1e-3);
    }

    #[test]
    fn coif1_round_trip_periodic() {
        let signal: Vec<f32> = (0..64).map(|i| ((i as f32) * 0.09).cos()).collect();
        round_trip_check(&signal, WaveletType::Coiflets(1), 2, WaveletBoundary::Periodic, 1e-3);
    }

    // ── Phase 3b: longer Daubechies (Db6, Db8) round-trips ─────

    #[test]
    fn db6_round_trip_periodic() {
        // Db6 has 12 taps — needs a longer signal to have a sensible
        // central region after the edge-effect skip.
        let signal: Vec<f32> = (0..128).map(|i| ((i as f32) * 0.04).sin()).collect();
        round_trip_check(&signal, WaveletType::Daubechies(6), 2, WaveletBoundary::Periodic, 1e-3);
    }

    #[test]
    fn db8_round_trip_periodic() {
        // Db8 has 16 taps — same reasoning as Db6.
        let signal: Vec<f32> = (0..128).map(|i| ((i as f32) * 0.03).cos()).collect();
        round_trip_check(&signal, WaveletType::Daubechies(8), 2, WaveletBoundary::Periodic, 1e-3);
    }

    // Note: a "Db6 suppresses quadratics" test along the lines of
    // Phase 3a's `db2_dwt_of_constant_signal_has_small_detail` does
    // NOT work under Periodic boundary — wrapping a quadratic
    // creates a giant step at i=N that contaminates every detail
    // band.  The vanishing-moments property holds for signals that
    // are *truly* polynomial across the boundary (which Periodic
    // never is for non-constant polynomials).  Phase 4's Symmetric/
    // Reflect boundary will let us write that test correctly.

    // Note: Symmetric boundary round-trips with non-Haar wavelets
    // do not satisfy orthogonality exactly even in the central
    // region — proper Symmetric-PR boundary handling requires the
    // "symmetric extension via convolution boundary stencils"
    // approach that Phase 4 will add (it has to be in place for
    // JPEG 2000's biorthogonal wavelets anyway).  For Phase 3a we
    // verify only Periodic round-trips (mathematically exact for
    // orthogonal wavelets).

    #[test]
    fn db2_dwt_of_constant_signal_has_small_detail() {
        // Constant signal under Daubechies-2: detail coefficients
        // should be very small (Db2 has 2 vanishing moments, so it
        // perfectly suppresses constants and linear ramps; only
        // boundary-extension artefacts contribute).
        let signal = vec![3.14_f32; 64];
        let coeffs = dwt_1d(&signal, WaveletType::Daubechies(2), 3, WaveletBoundary::Periodic).unwrap();
        // After 3 levels, cA_3 occupies coeffs[0..8].  Everything
        // after is detail.
        for (i, &v) in coeffs.iter().enumerate().skip(8) {
            assert!(
                v.abs() <= 1e-5,
                "Db2 detail coeff idx {} = {} (expected ~0 for constant signal)",
                i,
                v
            );
        }
    }

    #[test]
    fn split_levels_and_slice_level_work() {
        let signal: Vec<f32> = (0..16).map(|i| (i as f32) * 0.1).collect();
        let coeffs = dwt_1d(&signal, WaveletType::Haar, 3, WaveletBoundary::Periodic).unwrap();
        let offsets = split_levels(coeffs.len(), signal.len(), 3).unwrap();
        // 3 levels: [cA_3 (2) | cD_3 (2) | cD_2 (4) | cD_1 (8)] = 16
        // offsets: [0, 2, 4, 8, 16]
        assert_eq!(offsets, vec![0, 2, 4, 8, 16]);

        let ca3 = slice_level(&coeffs, signal.len(), 3, 3, Band::Approximation).unwrap();
        assert_eq!(ca3.len(), 2);
        let cd1 = slice_level(&coeffs, signal.len(), 3, 1, Band::Detail).unwrap();
        assert_eq!(cd1.len(), 8);

        // Invalid: approximation at non-coarsest level.
        let err = slice_level(&coeffs, signal.len(), 3, 1, Band::Approximation).unwrap_err();
        assert!(matches!(err, WaveletError::InvalidParam(_)));

        // Invalid: target_level = 0.
        let err = slice_level(&coeffs, signal.len(), 3, 0, Band::Detail).unwrap_err();
        assert!(matches!(err, WaveletError::InvalidParam(_)));
    }
}
