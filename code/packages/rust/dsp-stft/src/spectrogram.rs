//! # Magnitude and log spectrograms (DSP05 Phase 4)
//!
//! Convenience helpers that compute the STFT and then collapse
//! the complex spectrum into a real-valued magnitude or
//! log-magnitude image — the form used for plotting, machine
//! learning input features, and most audio analysis pipelines.

use crate::{stft, StftError, WindowType};

/// Power spectrogram — `|STFT|²` at each time-frequency bin.
///
/// Returns a flattened row-major `[num_frames, n_fft/2 + 1]`
/// `Vec<f32>` of non-negative real values.  Length is
/// `num_frames · (n_fft/2 + 1)` (i.e. half the size of the
/// complex spectrogram from `stft`, since we collapse the two
/// interleaved `[re, im]` lanes into one magnitude lane).
///
/// Useful for energy visualisation, voice activity detection,
/// audio fingerprinting, etc.
pub fn spectrogram(
    signal: &[f32],
    n_fft: u32,
    hop_length: u32,
    window: WindowType,
) -> Result<Vec<f32>, StftError> {
    let complex_spec = stft(signal, n_fft, hop_length, window)?;
    let nf = n_fft as usize;
    let bins = nf / 2 + 1;
    let frame_floats = bins * 2;
    debug_assert!(complex_spec.len() % frame_floats == 0);
    let num_frames = complex_spec.len() / frame_floats;
    let mut out = Vec::with_capacity(num_frames * bins);
    for m in 0..num_frames {
        let frame =
            &complex_spec[m * frame_floats..(m + 1) * frame_floats];
        for k in 0..bins {
            let re = frame[2 * k];
            let im = frame[2 * k + 1];
            out.push(re * re + im * im);
        }
    }
    debug_assert_eq!(out.len(), num_frames * bins);
    Ok(out)
}

/// Log-power spectrogram — `log(|STFT|² + ε)`.
///
/// Uses `ε = 1e-10` to keep the logarithm finite at silent
/// bins (`|STFT|² == 0` would otherwise produce `-∞`).  Output
/// shape and length match [`spectrogram`].
///
/// Used as the input feature for MFCCs (after the mel
/// filterbank in Phase 5), for spectrograms in plotting / ML
/// pipelines (where the log compression makes quiet structure
/// visible), and for perceptual loss functions in audio
/// neural networks.
pub fn log_spectrogram(
    signal: &[f32],
    n_fft: u32,
    hop_length: u32,
    window: WindowType,
) -> Result<Vec<f32>, StftError> {
    const EPS: f32 = 1e-10;
    let mut power = spectrogram(signal, n_fft, hop_length, window)?;
    for v in &mut power {
        *v = (*v + EPS).ln();
    }
    Ok(power)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── spectrogram ────────────────────────────────────────────

    #[test]
    fn spectrogram_is_non_negative() {
        // |STFT|² ≥ 0 by construction.
        let signal: Vec<f32> =
            (0..1024).map(|i| ((i as f32) * 0.1).sin()).collect();
        let spec =
            spectrogram(&signal, 256, 128, WindowType::Hann).unwrap();
        for &v in &spec {
            assert!(v >= 0.0, "negative bin: {}", v);
        }
    }

    #[test]
    fn spectrogram_length_matches_num_frames_times_bins() {
        let n: usize = 1024;
        let n_fft = 256u32;
        let hop = 128u32;
        let signal = vec![0.0f32; n];
        let spec =
            spectrogram(&signal, n_fft, hop, WindowType::Hann).unwrap();
        let expected_frames = 1 + (n - n_fft as usize) / hop as usize;
        let bins = (n_fft as usize) / 2 + 1;
        assert_eq!(spec.len(), expected_frames * bins);
    }

    #[test]
    fn spectrogram_of_zero_signal_is_all_zero() {
        let signal = vec![0.0f32; 1024];
        let spec =
            spectrogram(&signal, 256, 128, WindowType::Hann).unwrap();
        for &v in &spec {
            assert_eq!(v, 0.0);
        }
    }

    // ── log spectrogram ────────────────────────────────────────

    #[test]
    fn log_spectrogram_is_finite() {
        // log(power + ε) is always finite (ε > 0 prevents -∞).
        let signal: Vec<f32> =
            (0..1024).map(|i| ((i as f32) * 0.07).cos()).collect();
        let log_spec =
            log_spectrogram(&signal, 256, 128, WindowType::Hann).unwrap();
        for &v in &log_spec {
            assert!(v.is_finite(), "non-finite value: {}", v);
        }
    }

    #[test]
    fn log_spectrogram_of_zero_signal_is_near_log_eps() {
        // log(0 + 1e-10) ≈ -23.0259 — finite, just very small.
        let signal = vec![0.0f32; 1024];
        let log_spec =
            log_spectrogram(&signal, 256, 128, WindowType::Hann).unwrap();
        let expected = (1e-10f32).ln(); // ≈ -23.0259
        for &v in &log_spec {
            assert!((v - expected).abs() < 1e-4, "got {}", v);
        }
    }

    #[test]
    fn log_spectrogram_length_matches_spectrogram() {
        let signal: Vec<f32> = (0..1024).map(|i| (i as f32) * 0.01).collect();
        let spec = spectrogram(&signal, 256, 128, WindowType::Hann).unwrap();
        let log_spec =
            log_spectrogram(&signal, 256, 128, WindowType::Hann).unwrap();
        assert_eq!(spec.len(), log_spec.len());
    }
}
