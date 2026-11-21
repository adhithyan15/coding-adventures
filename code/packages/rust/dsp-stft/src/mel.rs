//! # Mel filterbank, mel spectrogram, and MFCC (DSP05 Phase 5)
//!
//! The canonical "speech features" pipeline.  Built on top of
//! [`crate::spectrogram`] (Phase 4) and `dsp-dct` (DSP02).
//!
//! ## The mel scale — why it exists
//!
//! Human pitch perception is approximately logarithmic in
//! frequency: doubling the frequency from 100 Hz → 200 Hz
//! sounds like the same step as 1000 Hz → 2000 Hz.  The mel
//! scale (Stevens / Volkmann / Newman, 1937) is a perceptual
//! re-scaling of frequency so that equal mel steps correspond to
//! equal perceptual steps.  The **HTK** approximation — the one
//! every speech toolkit uses — is:
//!
//! ```text
//!     mel(f)    = 2595 · log10(1 + f / 700)
//!     mel⁻¹(m) = 700  · (10^(m/2595) − 1)
//! ```
//!
//! Mel is nearly linear below ~1 kHz (where humans care about
//! pitch) and logarithmic above (where we mostly care about
//! formants and timbre).
//!
//! ## The mel filterbank
//!
//! A bank of `n_mels` overlapping triangular bandpass filters,
//! equally spaced **on the mel axis** between `fmin = 0` and
//! `fmax = sample_rate / 2` (Nyquist).  Each triangular filter
//! `m` has three Hz anchors: `(left[m], center[m], right[m])`
//! where `center[m] = left[m+1] = right[m-1]` — the filters tile
//! the spectrum with 50% overlap.
//!
//! Applied to a power spectrogram `|STFT|²`, this produces an
//! `[n_mels]` vector per frame — a low-dimensional, perceptually
//! shaped summary of the spectrum.
//!
//! ## MFCCs (Mel-Frequency Cepstral Coefficients)
//!
//! MFCCs are the **DCT-II of the log mel spectrogram**, keeping
//! the first `n_mfcc` coefficients.  Three things at once:
//!
//! 1. Mel pooling reshapes the spectrum perceptually.
//! 2. Logarithm matches loudness perception and decorrelates the
//!    source from the filter (the "cepstrum" trick).
//! 3. DCT-II compresses the now-smooth log spectrum into a few
//!    low-frequency coefficients — most of the signal energy
//!    lives in the first ~13 coefficients.
//!
//! For decades, MFCCs were the dominant input feature for ASR
//! (automatic speech recognition).  Modern deep models often
//! ingest the log mel spectrogram directly and let the network
//! learn its own cepstral compression — but MFCCs are still the
//! universal fallback and the standard for speaker ID, audio
//! classification, and most lightweight audio ML.

use crate::spectrogram::spectrogram;
use crate::{StftError, WindowType};
use dsp_dct::{dct, DctNorm, DctType};

/// Small floor added inside `log()` so silent bins map to a
/// finite (very negative) number instead of `-∞`.
const EPS: f32 = 1e-10;

/// Hz → mel, HTK convention.  `mel(0) = 0`, `mel(700) ≈ 781`.
///
/// Rejects negative or non-finite frequencies (CWE-20).
#[inline]
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn hz_to_mel(f: f32) -> Result<f32, StftError> {
    if !(f >= 0.0) || !f.is_finite() {
        return Err(StftError::InvalidParam(format!(
            "frequency must be finite and >= 0; got {}",
            f
        )));
    }
    let m = 2595.0 * (1.0 + f / 700.0).log10();
    if !m.is_finite() {
        return Err(StftError::InvalidParam(format!(
            "hz_to_mel overflowed for frequency {}",
            f
        )));
    }
    Ok(m)
}

/// Mel → Hz, HTK convention.  Inverse of [`hz_to_mel`].
///
/// Rejects negative, non-finite, or overflow-inducing mel values (CWE-20).
#[inline]
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn mel_to_hz(m: f32) -> Result<f32, StftError> {
    if !(m >= 0.0) || !m.is_finite() {
        return Err(StftError::InvalidParam(format!(
            "mel value must be finite and >= 0; got {}",
            m
        )));
    }
    let hz = 700.0 * (10.0_f32.powf(m / 2595.0) - 1.0);
    if !hz.is_finite() {
        return Err(StftError::InvalidParam(format!(
            "mel_to_hz overflowed for mel {}",
            m
        )));
    }
    Ok(hz)
}

/// Build an `[n_mels, n_fft / 2 + 1]` triangular mel filterbank.
///
/// - `n_mels`: number of triangular filters.  Common choices:
///   40 (telephone-band speech), 80 (modern ASR / synthesis),
///   128 (music / audio classification).
/// - `n_fft`: FFT size that produced the spectrogram this
///   filterbank will multiply.  Determines the number of input
///   bins (`n_fft / 2 + 1`).
/// - `sample_rate`: signal sample rate in Hz, used to convert
///   FFT bin indices to physical frequencies.
///
/// Each row is normalised to sum to 1.0 (when a row has any
/// support at all — degenerate rows where `center == left` or
/// `right == center` may have zero support and sum to 0).  This
/// matches the "Slaney-area-1 / `librosa`-style `norm=None`-after-
/// renormalise" convention: the dot product
/// `mel_filterbank @ power_spectrogram` is then a weighted
/// average of the power bins.
///
/// Returns a flattened row-major `Vec<f32>` of length
/// `n_mels · (n_fft / 2 + 1)`.
///
/// `n_mels == 0` returns an empty `Vec`; `n_fft == 0` likewise
/// (no input bins).  Both degenerate cases are surfaced as
/// errors at the [`mel_spectrogram`] / [`mfcc`] layer where
/// they actually matter.
// The `!(x > 0.0)` form is a deliberate NaN-safe guard (see the inline comment):
// rewriting it as `x <= 0.0` would let NaN slip through. Behavior-preserving allow.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn mel_filterbank(
    n_mels: u32,
    n_fft: u32,
    sample_rate: f32,
) -> Vec<f32> {
    let n_mels_usize = n_mels as usize;
    let nf = n_fft as usize;
    if n_mels_usize == 0 || nf == 0 || n_mels_usize > crate::MAX_SAMPLES || nf > crate::MAX_FFT_LEN {
        return Vec::new();
    }
    let bins = nf / 2 + 1;
    let total_len = match n_mels_usize.checked_mul(bins) {
        Some(len) if len <= crate::MAX_SAMPLES => len,
        _ => return Vec::new(),
    };
    if !(sample_rate > 0.0) || !sample_rate.is_finite() {
        return Vec::new();
    }

    let n_mels = n_mels_usize;

    // n_mels + 2 equally-spaced anchor points on the mel axis,
    // converted back to Hz, then to fractional FFT bin indices.
    //
    // Anchor m gives `left[m] = hz_pts[m], center = hz_pts[m+1],
    // right = hz_pts[m+2]` — the three vertices of triangle m.
    let fmin = 0.0_f32;
    let fmax = sample_rate * 0.5; // Nyquist
    if !(fmax > 0.0) || !fmax.is_finite() {
        return Vec::new();
    }
    let bin_per_hz = (n_fft as f32) / sample_rate;
    if !bin_per_hz.is_finite() {
        return Vec::new();
    }
    let Ok(mel_min) = hz_to_mel(fmin) else {
        return Vec::new();
    };
    let Ok(mel_max) = hz_to_mel(fmax) else {
        return Vec::new();
    };
    let step = (mel_max - mel_min) / ((n_mels + 1) as f32);
    if !step.is_finite() {
        return Vec::new();
    }
    let mut bin_pts = Vec::with_capacity(n_mels + 2);
    for i in 0..n_mels + 2 {
        let mel = mel_min + step * (i as f32);
        let Ok(hz) = mel_to_hz(mel) else {
            return Vec::new();
        };
        let pt = hz * bin_per_hz;
        if !pt.is_finite() {
            return Vec::new();
        }
        bin_pts.push(pt);
    }

    let mut filterbank = vec![0.0_f32; total_len];

    // ── Triangular filter rasterisation ──────────────────────
    //
    // For triangle m with vertices (left, center, right):
    //
    //   weight(k) = (k - left)  / (center - left)   for left  ≤ k ≤ center
    //   weight(k) = (right - k) / (right - center)  for center ≤ k ≤ right
    //   weight(k) = 0                                otherwise
    //
    // Degenerate edges (center == left or right == center) are
    // guarded with a zero so we never divide by zero.
    for m in 0..n_mels {
        let left = bin_pts[m];
        let center = bin_pts[m + 1];
        let right = bin_pts[m + 2];
        for k in 0..bins {
            let kf = k as f32;
            let w = if kf < left || kf > right {
                0.0
            } else if kf <= center {
                if center > left {
                    (kf - left) / (center - left)
                } else {
                    0.0
                }
            } else if right > center {
                (right - kf) / (right - center)
            } else {
                0.0
            };
            filterbank[m * bins + k] = w;
        }
        // Normalise row to sum to 1.0 — guarantees mel pooling
        // is a weighted *average* of power bins.  If a row has
        // no support at all (extremely small n_fft / very large
        // n_mels can collapse adjacent vertices onto the same
        // integer bin), leave it as all-zeros.
        let row_sum: f32 = filterbank[m * bins..(m + 1) * bins]
            .iter()
            .sum();
        if row_sum > 0.0 {
            for k in 0..bins {
                filterbank[m * bins + k] /= row_sum;
            }
        }
    }
    filterbank
}

/// Mel spectrogram — `mel_filterbank @ |STFT|²`.
///
/// Computes the power spectrogram (Phase 4) and then projects
/// each frame's `[n_fft/2 + 1]` power vector through the
/// `[n_mels, n_fft/2 + 1]` mel filterbank, producing an
/// `[num_frames, n_mels]` matrix flattened row-major.
///
/// The "what you see in a Spotify-style audio visualiser"
/// representation.  Also the standard input for modern audio /
/// speech neural networks (after taking `log10` of it).
// The `!(x > 0.0)` form is a deliberate NaN-safe guard (see the inline comment):
// rewriting it as `x <= 0.0` would let NaN slip through. Behavior-preserving allow.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn mel_spectrogram(
    signal: &[f32],
    n_fft: u32,
    hop_length: u32,
    n_mels: u32,
    sample_rate: f32,
    window: WindowType,
) -> Result<Vec<f32>, StftError> {
    if n_mels == 0 {
        return Err(StftError::InvalidParam(
            "n_mels must be > 0".into(),
        ));
    }
    if (n_mels as usize) > crate::MAX_SAMPLES {
        return Err(StftError::InvalidParam(format!(
            "n_mels {} exceeds MAX_SAMPLES {}",
            n_mels, crate::MAX_SAMPLES
        )));
    }
    if !(sample_rate > 0.0) || !sample_rate.is_finite() {
        return Err(StftError::InvalidParam(
            "sample_rate must be positive and finite".into(),
        ));
    }
    if n_fft == 0 {
        return Err(StftError::InvalidParam("n_fft must be > 0".into()));
    }
    if (n_fft as usize) > crate::MAX_FFT_LEN {
        return Err(StftError::InvalidParam(format!(
            "n_fft {} exceeds MAX_FFT_LEN {}",
            n_fft, crate::MAX_FFT_LEN
        )));
    }
    let bins = (n_fft as usize) / 2 + 1;
    let nm = n_mels as usize;
    let fb_len = nm.checked_mul(bins).ok_or_else(|| {
        StftError::InvalidParam("mel filterbank size overflow".into())
    })?;
    if fb_len > crate::MAX_SAMPLES {
        return Err(StftError::InvalidParam(format!(
            "mel filterbank size {} exceeds MAX_SAMPLES {}",
            fb_len, crate::MAX_SAMPLES
        )));
    }
    let power = spectrogram(signal, n_fft, hop_length, window)?;
    debug_assert_eq!(power.len() % bins, 0);
    let num_frames = power.len() / bins;
    let out_len = num_frames.checked_mul(nm).ok_or_else(|| {
        StftError::InvalidParam("mel spectrogram output length overflow".into())
    })?;
    if out_len > crate::MAX_SAMPLES {
        return Err(StftError::InvalidParam(format!(
            "mel spectrogram length {} exceeds MAX_SAMPLES {}",
            out_len, crate::MAX_SAMPLES
        )));
    }
    let fb = mel_filterbank(n_mels, n_fft, sample_rate);
    if fb.len() != fb_len {
        return Err(StftError::InvalidParam(
            "mel_filterbank generation failed".into(),
        ));
    }

    let mut out = vec![0.0_f32; out_len];
    for t in 0..num_frames {
        let p_row = &power[t * bins..(t + 1) * bins];
        for m in 0..nm {
            let fb_row = &fb[m * bins..(m + 1) * bins];
            let mut acc = 0.0_f32;
            for k in 0..bins {
                acc += fb_row[k] * p_row[k];
            }
            if !acc.is_finite() {
                return Err(StftError::InvalidParam(
                    "mel spectrogram bin overflowed to non-finite float".into(),
                ));
            }
            out[t * nm + m] = acc;
        }
    }
    Ok(out)
}

/// MFCC — Mel-Frequency Cepstral Coefficients.
///
/// `mfcc[t, c] = DCT-II_ortho( log(mel_spectrogram[t, :] + ε) )[c]`
/// for `c ∈ [0, n_mfcc)`.
///
/// Output layout: row-major `[num_frames, n_mfcc]`, flattened
/// to a `Vec<f32>` of length `num_frames · n_mfcc`.
///
/// `n_mfcc` is typically 12 or 13 (for speech), occasionally up
/// to 40.  Requires `n_mfcc ≤ n_mels` (you can't keep more DCT
/// coefficients than you have inputs to the DCT).
pub fn mfcc(
    signal: &[f32],
    n_fft: u32,
    hop_length: u32,
    n_mels: u32,
    n_mfcc: u32,
    sample_rate: f32,
    window: WindowType,
) -> Result<Vec<f32>, StftError> {
    if n_mfcc == 0 {
        return Err(StftError::InvalidParam(
            "n_mfcc must be > 0".into(),
        ));
    }
    if n_mfcc > n_mels {
        return Err(StftError::InvalidParam(format!(
            "n_mfcc {} > n_mels {} (cannot keep more DCT coefficients \
             than DCT inputs)",
            n_mfcc, n_mels
        )));
    }
    let mel = mel_spectrogram(
        signal, n_fft, hop_length, n_mels, sample_rate, window,
    )?;
    let nm = n_mels as usize;
    let nc = n_mfcc as usize;
    debug_assert_eq!(mel.len() % nm, 0);
    let num_frames = mel.len() / nm;

    // log compression — silent bins map to log(ε) ≈ -23, not -∞.
    let mut log_mel = Vec::with_capacity(mel.len());
    for &v in &mel {
        let lm = (v + EPS).ln();
        if !lm.is_finite() {
            return Err(StftError::InvalidParam(
                "log mel spectrogram bin became non-finite".into(),
            ));
        }
        log_mel.push(lm);
    }

    // DCT-II per frame, keep first n_mfcc coefficients.
    let total_mfcc_len = num_frames.checked_mul(nc).ok_or_else(|| {
        StftError::InvalidParam("MFCC output length overflow".into())
    })?;
    if total_mfcc_len > crate::MAX_SAMPLES {
        return Err(StftError::InvalidParam(format!(
            "MFCC output length {} exceeds MAX_SAMPLES {}",
            total_mfcc_len, crate::MAX_SAMPLES
        )));
    }
    let mut out = Vec::with_capacity(total_mfcc_len);
    for t in 0..num_frames {
        let row = &log_mel[t * nm..(t + 1) * nm];
        let coeffs = dct(row, DctType::II, DctNorm::Ortho)
            .map_err(|e| StftError::Fft(format!("{:?}", e)))?;
        debug_assert_eq!(coeffs.len(), nm);
        if !coeffs[..nc].iter().all(|x| x.is_finite()) {
            return Err(StftError::InvalidParam(
                "MFCC output contains non-finite float".into(),
            ));
        }
        out.extend_from_slice(&coeffs[..nc]);
    }
    debug_assert_eq!(out.len(), num_frames * nc);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── mel_filterbank shape / properties ──────────────────────

    #[test]
    fn mel_filterbank_shape_matches_n_mels_times_bins() {
        let n_fft = 512u32;
        let n_mels = 40u32;
        let fb = mel_filterbank(n_mels, n_fft, 16_000.0);
        let bins = (n_fft as usize) / 2 + 1;
        assert_eq!(fb.len(), (n_mels as usize) * bins);
    }

    #[test]
    fn mel_filterbank_is_non_negative() {
        let fb = mel_filterbank(40, 512, 16_000.0);
        for &w in &fb {
            assert!(w >= 0.0, "negative weight: {}", w);
        }
    }

    #[test]
    fn mel_filterbank_rows_sum_to_one() {
        // Each well-formed triangular row is row-normalised to
        // sum to 1.  Use n_mels small enough relative to bins
        // that no triangle collapses to zero support.
        let n_fft = 1024u32;
        let n_mels = 40u32;
        let fb = mel_filterbank(n_mels, n_fft, 22_050.0);
        let bins = (n_fft as usize) / 2 + 1;
        for m in 0..(n_mels as usize) {
            let row_sum: f32 =
                fb[m * bins..(m + 1) * bins].iter().sum();
            assert!(
                (row_sum - 1.0).abs() < 1e-5,
                "row {} sum = {} (expected 1.0)",
                m,
                row_sum
            );
        }
    }

    #[test]
    fn mel_filterbank_zero_n_mels_returns_empty() {
        let fb = mel_filterbank(0, 512, 16_000.0);
        assert!(fb.is_empty());
    }

    #[test]
    fn mel_filterbank_zero_n_fft_returns_empty() {
        let fb = mel_filterbank(40, 0, 16_000.0);
        assert!(fb.is_empty());
    }

    #[test]
    fn mel_filterbank_nan_sample_rate_returns_empty() {
        // Defensive NaN guard — NaN sample_rate would propagate
        // through hz↔mel conversions and poison every weight.
        let fb = mel_filterbank(40, 512, f32::NAN);
        assert!(fb.is_empty());
    }

    #[test]
    fn mel_filterbank_non_positive_sample_rate_returns_empty() {
        let fb_zero = mel_filterbank(40, 512, 0.0);
        let fb_neg = mel_filterbank(40, 512, -16_000.0);
        let fb_inf = mel_filterbank(40, 512, f32::INFINITY);
        assert!(fb_zero.is_empty());
        assert!(fb_neg.is_empty());
        assert!(fb_inf.is_empty());
    }

    // ── mel_spectrogram ────────────────────────────────────────

    #[test]
    fn mel_spectrogram_output_shape_matches_num_frames_times_n_mels() {
        let signal = vec![0.5_f32; 4096];
        let n_fft = 512u32;
        let hop = 256u32;
        let n_mels = 40u32;
        let out = mel_spectrogram(
            &signal,
            n_fft,
            hop,
            n_mels,
            16_000.0,
            WindowType::Hann,
        )
        .unwrap();
        let expected_frames =
            1 + (signal.len() - n_fft as usize) / hop as usize;
        assert_eq!(out.len(), expected_frames * (n_mels as usize));
    }

    #[test]
    fn mel_spectrogram_is_non_negative() {
        // mel_filterbank entries ≥ 0 and power ≥ 0, so the
        // product is ≥ 0 in every bin.
        let signal: Vec<f32> = (0..4096)
            .map(|i| ((i as f32) * 0.07).sin())
            .collect();
        let out = mel_spectrogram(
            &signal,
            512,
            256,
            40,
            16_000.0,
            WindowType::Hann,
        )
        .unwrap();
        for &v in &out {
            assert!(v >= 0.0, "negative mel bin: {}", v);
        }
    }

    #[test]
    fn mel_spectrogram_of_zero_signal_is_all_zero() {
        let signal = vec![0.0_f32; 4096];
        let out = mel_spectrogram(
            &signal,
            512,
            256,
            40,
            16_000.0,
            WindowType::Hann,
        )
        .unwrap();
        for &v in &out {
            assert_eq!(v, 0.0, "non-zero mel bin from zero signal: {}", v);
        }
    }

    #[test]
    fn mel_spectrogram_rejects_zero_n_mels() {
        let signal = vec![0.5_f32; 1024];
        let err = mel_spectrogram(
            &signal,
            256,
            128,
            0, // n_mels
            16_000.0,
            WindowType::Hann,
        )
        .unwrap_err();
        assert!(matches!(err, StftError::InvalidParam(_)));
    }

    #[test]
    fn mel_spectrogram_rejects_non_positive_sample_rate() {
        let signal = vec![0.5_f32; 1024];
        let err = mel_spectrogram(
            &signal,
            256,
            128,
            40,
            0.0, // sample_rate
            WindowType::Hann,
        )
        .unwrap_err();
        assert!(matches!(err, StftError::InvalidParam(_)));
    }

    #[test]
    fn mel_spectrogram_propagates_stft_errors() {
        // signal shorter than n_fft → SignalTooShort from stft.
        let signal = vec![0.5_f32; 32];
        let err = mel_spectrogram(
            &signal,
            512, // n_fft > signal.len()
            256,
            40,
            16_000.0,
            WindowType::Hann,
        )
        .unwrap_err();
        assert!(matches!(err, StftError::SignalTooShort(_)));
    }

    // ── mfcc ───────────────────────────────────────────────────

    #[test]
    fn mfcc_output_shape_matches_num_frames_times_n_mfcc() {
        let signal: Vec<f32> = (0..4096)
            .map(|i| ((i as f32) * 0.05).cos())
            .collect();
        let n_fft = 512u32;
        let hop = 256u32;
        let n_mels = 40u32;
        let n_mfcc = 13u32;
        let out = mfcc(
            &signal,
            n_fft,
            hop,
            n_mels,
            n_mfcc,
            16_000.0,
            WindowType::Hann,
        )
        .unwrap();
        let expected_frames =
            1 + (signal.len() - n_fft as usize) / hop as usize;
        assert_eq!(out.len(), expected_frames * (n_mfcc as usize));
    }

    #[test]
    fn mfcc_rejects_zero_n_mfcc() {
        let signal = vec![0.5_f32; 1024];
        let err = mfcc(
            &signal,
            256,
            128,
            40,
            0, // n_mfcc
            16_000.0,
            WindowType::Hann,
        )
        .unwrap_err();
        assert!(matches!(err, StftError::InvalidParam(_)));
    }

    #[test]
    fn mfcc_rejects_n_mfcc_greater_than_n_mels() {
        let signal = vec![0.5_f32; 1024];
        let err = mfcc(
            &signal,
            256,
            128,
            13, // n_mels
            20, // n_mfcc > n_mels
            16_000.0,
            WindowType::Hann,
        )
        .unwrap_err();
        assert!(matches!(err, StftError::InvalidParam(_)));
    }

    #[test]
    fn mfcc_is_finite_for_zero_signal() {
        // Zero signal → zero mel → log(0 + ε) = log(ε) ≈ -23
        // → DCT of a constant vector is finite (the DC bin
        // accumulates the energy, others are zero).  Must not
        // produce NaN or ±∞.
        let signal = vec![0.0_f32; 4096];
        let out = mfcc(
            &signal,
            512,
            256,
            40,
            13,
            16_000.0,
            WindowType::Hann,
        )
        .unwrap();
        for &v in &out {
            assert!(v.is_finite(), "non-finite MFCC: {}", v);
        }
    }

    #[test]
    fn mel_spectrogram_rejects_oversized_filterbank_product() {
        let signal = vec![0.5_f32; 1024];
        // n_fft = 65536 -> bins = 32769. If n_mels = 1000, 32769 * 1000 = 32,769,000 > MAX_SAMPLES (16,777,216).
        let err = mel_spectrogram(
            &signal,
            65536,
            32768,
            1000,
            16_000.0,
            WindowType::Hann,
        );
        assert!(err.is_err());
    }
}
