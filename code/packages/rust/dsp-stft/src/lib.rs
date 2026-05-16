//! # `dsp-stft` — Short-Time Fourier Transform
//!
//! **DSP05 Phase 1 + 2 (this release).**  Scalar reference
//! Short-Time Fourier Transform — sliding-window FFT for
//! time-frequency analysis.  Per-frame FFT is delegated to
//! [`dsp_fft::rfft_scalar`]; the analysis window comes from
//! [`dsp_filters::WindowType`].
//!
//! ## Quick example
//!
//! ```rust
//! use dsp_stft::{stft, WindowType};
//!
//! let signal: Vec<f32> = (0..1024).map(|n| (n as f32) * 0.01).collect();
//! let spec = stft(&signal, 256, 128, WindowType::Hann).unwrap();
//! // Layout: [num_frames, n_fft/2 + 1, 2] interleaved [re, im]
//! // num_frames = 1 + (1024 - 256) / 128 = 7
//! // bins per frame = 256 / 2 + 1 = 129
//! // total floats = 7 * 129 * 2 = 1806
//! assert_eq!(spec.len(), 7 * 129 * 2);
//! ```
//!
//! ## Algorithm — strict-mode framing
//!
//! For each frame `m ∈ [0, num_frames)` where
//! `num_frames = 1 + (N - n_fft) / hop_length`:
//!
//! 1. Extract frame: `signal[m * hop_length .. m * hop_length + n_fft]`.
//! 2. Multiply by analysis window `w[n]`.
//! 3. Run `rfft` on the windowed frame → length `n_fft/2 + 1`
//!    complex spectrum.
//! 4. Append to output (row-major `[num_frames, n_fft/2+1, 2]`).
//!
//! V1 uses **strict framing** — no centred padding, only
//! frames that fit entirely inside the signal.  Centred-padding
//! mode (matching librosa / scipy defaults) is a Phase 4+
//! follow-up.

#![warn(rust_2018_idioms)]

use dsp_fft::{rfft_scalar, FftError};
pub use dsp_filters::WindowType;
use std::f32::consts::PI;
use std::fmt;

/// Compute the short-time Fourier transform of `signal`.
///
/// - `n_fft`: FFT size per frame (window length too).
/// - `hop_length`: how many samples to advance between frames.
///   `hop_length = n_fft / 2` is the common 50% overlap.
/// - `window`: analysis window function — see
///   [`WindowType`].
///
/// Returns a flattened row-major
/// `[num_frames, n_fft/2 + 1, 2]` `Vec<f32>` holding the
/// interleaved-complex spectrogram, where:
///
/// ```text
///     num_frames = 1 + (signal.len() - n_fft) / hop_length
/// ```
///
/// (strict-mode framing — V1 ships this only; centred padding
/// is a future phase).
pub fn stft(
    signal: &[f32],
    n_fft: u32,
    hop_length: u32,
    window: WindowType,
) -> Result<Vec<f32>, StftError> {
    if signal.is_empty() {
        return Err(StftError::EmptySignal);
    }
    if n_fft == 0 {
        return Err(StftError::InvalidParam("n_fft must be > 0".into()));
    }
    if hop_length == 0 {
        return Err(StftError::InvalidParam("hop_length must be > 0".into()));
    }
    let n = signal.len();
    let nf = n_fft as usize;
    let hop = hop_length as usize;

    if n < nf {
        return Err(StftError::SignalTooShort(format!(
            "signal length {} < n_fft {} (strict-mode framing)",
            n, nf
        )));
    }

    // num_frames = 1 + (N - n_fft) / hop_length  (strict mode).
    //
    // n - nf >= 0 (checked above), so integer division gives the
    // floor we want.
    let num_frames = 1 + (n - nf) / hop;
    let bins = nf / 2 + 1;
    let out_len = num_frames * bins * 2;

    // ── Pre-compute the analysis window once.
    //
    // The window is symmetric; we sample it across the full
    // n_fft length (M = n_fft - 1 in the standard "periodic"
    // formulas — we use the "symmetric" convention here, which
    // matches scipy.signal.get_window's default sym=True).
    let win = build_window(window, nf);

    // ── Process each frame.
    let mut out = Vec::with_capacity(out_len);
    let mut windowed = vec![0.0f32; nf];
    for m in 0..num_frames {
        let frame_start = m * hop;
        // Multiply frame by window.
        for k in 0..nf {
            windowed[k] = signal[frame_start + k] * win[k];
        }
        // Per-frame rfft.
        let spectrum = rfft_scalar(&windowed).map_err(fft_err)?;
        // spectrum is length 2 * bins (interleaved [re, im]).
        debug_assert_eq!(spectrum.len(), 2 * bins);
        out.extend_from_slice(&spectrum);
    }
    debug_assert_eq!(out.len(), out_len);
    Ok(out)
}

/// Build an analysis window of length `n_fft` per the
/// `WindowType`.  Same formulas as
/// [`dsp_filters::design`] — Rectangular, Hamming, Hann,
/// Blackman.  All windows are normalised to peak ≤ 1 and
/// span the closed interval `[0, n_fft - 1]`.
fn build_window(window: WindowType, n_fft: usize) -> Vec<f32> {
    let m = (n_fft - 1) as f32;
    (0..n_fft)
        .map(|n| {
            let nn = n as f32;
            match window {
                WindowType::Rectangular => 1.0,
                WindowType::Hamming => {
                    0.54 - 0.46 * (2.0 * PI * nn / m).cos()
                }
                WindowType::Hann => {
                    0.5 * (1.0 - (2.0 * PI * nn / m).cos())
                }
                WindowType::Blackman => {
                    0.42 - 0.5 * (2.0 * PI * nn / m).cos()
                        + 0.08 * (4.0 * PI * nn / m).cos()
                }
            }
        })
        .collect()
}

/// Errors produced by the STFT API.
#[derive(Debug, Clone, PartialEq)]
pub enum StftError {
    /// `signal` is empty.
    EmptySignal,
    /// `n_fft == 0`, `hop_length == 0`, etc.
    InvalidParam(String),
    /// Strict-mode framing requires `signal.len() >= n_fft`.
    SignalTooShort(String),
    /// Reserved for the Phase 3 `istft` API.
    InvalidSpectrogram(String),
    /// Wraps a `dsp_fft::FftError` from the per-frame FFT.
    Fft(String),
}

impl fmt::Display for StftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StftError::EmptySignal => write!(f, "STFT signal must be non-empty"),
            StftError::InvalidParam(msg) => write!(f, "invalid parameter: {}", msg),
            StftError::SignalTooShort(msg) => write!(f, "signal too short: {}", msg),
            StftError::InvalidSpectrogram(msg) => {
                write!(f, "invalid spectrogram: {}", msg)
            }
            StftError::Fft(msg) => write!(f, "FFT failure: {}", msg),
        }
    }
}

impl std::error::Error for StftError {}

fn fft_err(e: FftError) -> StftError {
    StftError::Fft(format!("{:?}", e))
}

// ─────────────────────────── Tests ───────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        let scale = a.abs().max(b.abs()).max(1.0);
        (a - b).abs() <= scale * tol
    }

    // ── error paths ────────────────────────────────────────────

    #[test]
    fn stft_rejects_empty_signal() {
        let err = stft(&[], 64, 32, WindowType::Hann).unwrap_err();
        assert_eq!(err, StftError::EmptySignal);
    }

    #[test]
    fn stft_rejects_zero_n_fft() {
        let err = stft(&[1.0; 128], 0, 32, WindowType::Hann).unwrap_err();
        assert!(matches!(err, StftError::InvalidParam(_)));
    }

    #[test]
    fn stft_rejects_zero_hop() {
        let err = stft(&[1.0; 128], 64, 0, WindowType::Hann).unwrap_err();
        assert!(matches!(err, StftError::InvalidParam(_)));
    }

    #[test]
    fn stft_rejects_signal_shorter_than_n_fft() {
        let err = stft(&[1.0; 32], 64, 16, WindowType::Hann).unwrap_err();
        assert!(matches!(err, StftError::SignalTooShort(_)));
    }

    // ── output length contract ─────────────────────────────────

    #[test]
    fn stft_output_length_matches_num_frames() {
        // N = 1024, n_fft = 256, hop = 128:
        //   num_frames = 1 + (1024 - 256) / 128 = 1 + 6 = 7
        //   bins = 256/2 + 1 = 129
        //   total floats = 7 * 129 * 2 = 1806
        let signal = vec![0.5f32; 1024];
        let out = stft(&signal, 256, 128, WindowType::Hann).unwrap();
        assert_eq!(out.len(), 7 * 129 * 2);
    }

    #[test]
    fn stft_num_frames_matches_formula() {
        for &(n, n_fft, hop) in &[
            (128usize, 64u32, 32u32),     // 3 frames
            (256, 128, 64),               // 3 frames
            (1024, 256, 128),             // 7 frames
            (44100, 1024, 512),           // 84 frames
            (100, 100, 1),                // 1 frame (exactly fits)
        ] {
            let signal = vec![0.0f32; n];
            let out = stft(&signal, n_fft, hop, WindowType::Rectangular).unwrap();
            let expected_frames = 1 + (n - n_fft as usize) / hop as usize;
            let bins = (n_fft as usize) / 2 + 1;
            assert_eq!(
                out.len(),
                expected_frames * bins * 2,
                "N={}, n_fft={}, hop={}",
                n,
                n_fft,
                hop
            );
        }
    }

    // ── closed-form ────────────────────────────────────────────

    #[test]
    fn stft_with_rectangular_window_matches_per_frame_rfft() {
        // Rectangular window = no windowing.  STFT should match
        // calling rfft directly on each frame.
        let n = 512;
        let n_fft = 64;
        let hop = 32;
        let signal: Vec<f32> = (0..n).map(|i| ((i as f32) * 0.1).sin()).collect();
        let stft_out = stft(&signal, n_fft, hop, WindowType::Rectangular).unwrap();
        let num_frames = 1 + (n - n_fft as usize) / hop as usize;
        let bins = (n_fft as usize) / 2 + 1;
        for m in 0..num_frames {
            let frame_start = m * (hop as usize);
            let frame = &signal[frame_start..frame_start + n_fft as usize];
            let expected_spec = dsp_fft::rfft_scalar(frame).unwrap();
            let actual_spec =
                &stft_out[m * bins * 2..(m + 1) * bins * 2];
            for (i, (a, b)) in expected_spec.iter().zip(actual_spec.iter()).enumerate() {
                assert!(
                    approx_eq(*a, *b, 1e-5),
                    "frame {} bin {}: {} vs {}",
                    m,
                    i,
                    a,
                    b
                );
            }
        }
    }

    #[test]
    fn stft_of_constant_signal_concentrates_at_low_frequency_bins() {
        // Constant signal under a Hann window has its energy
        // concentrated in the Hann window's mainlobe, centred
        // at DC (bin 0).  The Hann mainlobe spans roughly the
        // first 4 bins, so:
        //   - bin 0 (DC) has the largest magnitude (≈ sum(Hann))
        //   - bins 1-3 are inside the mainlobe (significant)
        //   - bin 4+ are sidelobes (much smaller — at least 10×
        //     less than DC for a 256-point Hann)
        let n = 1024;
        let n_fft = 256u32;
        let hop = 128u32;
        let signal = vec![1.0f32; n];
        let spec = stft(&signal, n_fft, hop, WindowType::Hann).unwrap();
        let bins = (n_fft as usize) / 2 + 1;
        // Inspect frame 0.
        let dc_re = spec[0];
        let dc_im = spec[1];
        let dc_mag = (dc_re * dc_re + dc_im * dc_im).sqrt();
        assert!(
            dc_mag > 100.0,
            "DC magnitude = {}, expected > 100 (sum of 256-pt Hann)",
            dc_mag
        );
        // Bins 4+ are outside the Hann mainlobe and should be
        // at least 10× smaller than the DC peak.
        for k in 4..bins {
            let re = spec[2 * k];
            let im = spec[2 * k + 1];
            let mag = (re * re + im * im).sqrt();
            assert!(
                mag < dc_mag * 0.1,
                "bin {} mag = {} (> 10% of DC {})",
                k,
                mag,
                dc_mag
            );
        }
    }

    #[test]
    fn stft_of_pure_sinusoid_peaks_at_expected_bin() {
        // 440 Hz sinusoid at 44100 Hz sample rate, n_fft = 1024:
        //   peak bin ≈ 440 * 1024 / 44100 = 10.2 → bin 10 or 11.
        let sample_rate = 44_100.0f32;
        let freq = 440.0f32;
        let n_fft = 1024u32;
        let n = 4096;
        let signal: Vec<f32> = (0..n)
            .map(|i| (2.0 * PI * freq * (i as f32) / sample_rate).sin())
            .collect();
        let spec = stft(&signal, n_fft, n_fft / 4, WindowType::Hann).unwrap();
        let bins = (n_fft as usize) / 2 + 1;
        // Inspect a middle frame (skip edge transients).
        let frame_idx = 5; // arbitrary middle frame
        let frame = &spec[frame_idx * bins * 2..(frame_idx + 1) * bins * 2];
        // Find peak bin by magnitude.
        let mut peak_bin = 0;
        let mut peak_mag = 0.0f32;
        for k in 0..bins {
            let mag = (frame[2 * k] * frame[2 * k]
                + frame[2 * k + 1] * frame[2 * k + 1])
                .sqrt();
            if mag > peak_mag {
                peak_mag = mag;
                peak_bin = k;
            }
        }
        // Expected: bin ~10.
        assert!(
            peak_bin == 10 || peak_bin == 11,
            "peak bin {} (expected ~10 for 440 Hz @ 44100, n_fft=1024)",
            peak_bin
        );
    }

    #[test]
    fn stft_with_hop_equal_to_n_fft_gives_disjoint_frames() {
        // hop = n_fft means no overlap.  num_frames = N / n_fft
        // exactly (when N is a multiple of n_fft).
        let n = 1024;
        let n_fft = 256u32;
        let hop = 256u32; // = n_fft, no overlap
        let signal = vec![0.0f32; n];
        let out = stft(&signal, n_fft, hop, WindowType::Hann).unwrap();
        let expected_frames = 1 + (n - n_fft as usize) / hop as usize;
        assert_eq!(expected_frames, 4);
        let bins = (n_fft as usize) / 2 + 1;
        assert_eq!(out.len(), expected_frames * bins * 2);
    }

    // ── numerical sanity ───────────────────────────────────────

    #[test]
    fn stft_with_hann_window_attenuates_high_frequencies() {
        // High-frequency tone (close to Nyquist): rectangular
        // window has the full FFT magnitude at the tone's bin,
        // Hann window should have a smaller magnitude there
        // (Hann's main-lobe is wider and lower than rect's).
        let n = 1024;
        let n_fft = 256u32;
        // Frequency at 0.4 * Nyquist of n_fft.
        let bin_idx = (0.4 * (n_fft as f32 / 2.0)) as usize;
        let normalized_freq = bin_idx as f32 / n_fft as f32;
        let signal: Vec<f32> = (0..n)
            .map(|i| (2.0 * PI * normalized_freq * (i as f32)).sin())
            .collect();
        let spec_rect =
            stft(&signal, n_fft, 128, WindowType::Rectangular).unwrap();
        let spec_hann = stft(&signal, n_fft, 128, WindowType::Hann).unwrap();
        let bins = (n_fft as usize) / 2 + 1;
        let frame_idx = 3; // arbitrary middle frame
        let rect_frame = &spec_rect[frame_idx * bins * 2..(frame_idx + 1) * bins * 2];
        let hann_frame = &spec_hann[frame_idx * bins * 2..(frame_idx + 1) * bins * 2];
        let rect_mag = (rect_frame[2 * bin_idx] * rect_frame[2 * bin_idx]
            + rect_frame[2 * bin_idx + 1] * rect_frame[2 * bin_idx + 1])
            .sqrt();
        let hann_mag = (hann_frame[2 * bin_idx] * hann_frame[2 * bin_idx]
            + hann_frame[2 * bin_idx + 1] * hann_frame[2 * bin_idx + 1])
            .sqrt();
        assert!(
            hann_mag < rect_mag,
            "Hann mag {} >= Rect mag {} (expected Hann to attenuate)",
            hann_mag,
            rect_mag
        );
    }
}
