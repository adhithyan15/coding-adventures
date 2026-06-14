//! # Inverse STFT (overlap-add reconstruction)
//!
//! **DSP05 Phase 3.**  Reconstructs a time-domain signal from
//! its STFT spectrogram via the standard overlap-add (OLA)
//! formula:
//!
//! ```text
//!     x_hat[n] = ( Σ_m  IFFT(STFT[:, m])[n - m·hop] · w[n - m·hop] )
//!               / ( Σ_m  w[n - m·hop]² )
//! ```
//!
//! Under COLA-satisfying choices (e.g. Hann window at
//! `hop = n_fft / 2`), `istft(stft(x))` recovers `x` exactly
//! up to FP noise.

use dsp_fft::{irfft_scalar, FftError};
use std::f32::consts::PI;

use crate::{StftError, WindowType};

/// Inverse short-time Fourier transform via overlap-add.
///
/// `spectrogram` is the flattened
/// `[num_frames, n_fft/2 + 1, 2]` row-major buffer that
/// [`crate::stft`] produces.  `output_length` is the desired
/// length of the reconstructed signal — typically the original
/// signal length passed to `stft`.
///
/// The same `n_fft`, `hop_length`, and `window` must be passed
/// that were used on the forward `stft`.
///
/// Output length equals `output_length`.  For COLA-satisfying
/// `(window, hop_length)` combos, `istft(stft(x))` recovers `x`
/// within `1e-4` relative tolerance.
pub fn istft(
    spectrogram: &[f32],
    n_fft: u32,
    hop_length: u32,
    window: WindowType,
    output_length: u32,
) -> Result<Vec<f32>, StftError> {
    if spectrogram.is_empty() {
        return Err(StftError::InvalidSpectrogram(
            "spectrogram is empty".into(),
        ));
    }
    if n_fft == 0 {
        return Err(StftError::InvalidParam("n_fft must be > 0".into()));
    }
    if hop_length == 0 {
        return Err(StftError::InvalidParam("hop_length must be > 0".into()));
    }
    if output_length == 0 {
        return Err(StftError::InvalidParam(
            "output_length must be > 0".into(),
        ));
    }

    let nf = n_fft as usize;
    let hop = hop_length as usize;
    let out_len = output_length as usize;
    let bins = nf / 2 + 1;
    let frame_floats = bins * 2;

    if spectrogram.len() % frame_floats != 0 {
        return Err(StftError::InvalidSpectrogram(format!(
            "spectrogram length {} is not a multiple of bins×2 = {}",
            spectrogram.len(),
            frame_floats
        )));
    }
    let num_frames = spectrogram.len() / frame_floats;

    let win = build_window(window, nf);

    // ── Output buffer and normalisation buffer.
    //
    //   out[n]  = Σ_m  irfft(frame m)[n - m·hop] · w[n - m·hop]
    //   norm[n] = Σ_m  w[n - m·hop]²
    let mut out = vec![0.0f32; out_len];
    let mut norm = vec![0.0f32; out_len];

    for m in 0..num_frames {
        let frame_start_global = m * hop;
        let frame_slice =
            &spectrogram[m * frame_floats..(m + 1) * frame_floats];
        // irfft returns a real Vec<f32> of length n_fft.
        let frame = irfft_scalar(frame_slice, n_fft).map_err(fft_err)?;
        debug_assert_eq!(frame.len(), nf);

        // Window the frame, then overlap-add into the output.
        for k in 0..nf {
            let n = frame_start_global + k;
            if n >= out_len {
                break;
            }
            let w = win[k];
            out[n] += frame[k] * w;
            norm[n] += w * w;
        }
    }

    // ── Normalise.  Avoid div-by-zero where the window
    //   coverage is near zero (at very edge samples for some
    //   window/hop combos) — leave those samples as 0.
    for n in 0..out_len {
        if norm[n] > 1e-10 {
            out[n] /= norm[n];
        }
    }
    Ok(out)
}

/// Build the same analysis/synthesis window the forward STFT
/// uses.  Duplicated here rather than imported from `lib.rs`
/// to keep modules independent — both copies follow the same
/// formulas (`scipy.signal.get_window` with `sym=True`).
fn build_window(window: WindowType, n_fft: usize) -> Vec<f32> {
    let m = (n_fft - 1) as f32;
    (0..n_fft)
        .map(|n| {
            let nn = n as f32;
            match window {
                WindowType::Rectangular => 1.0,
                WindowType::Hamming => 0.54 - 0.46 * (2.0 * PI * nn / m).cos(),
                WindowType::Hann => 0.5 * (1.0 - (2.0 * PI * nn / m).cos()),
                WindowType::Blackman => {
                    0.42 - 0.5 * (2.0 * PI * nn / m).cos()
                        + 0.08 * (4.0 * PI * nn / m).cos()
                }
            }
        })
        .collect()
}

fn fft_err(e: FftError) -> StftError {
    StftError::Fft(format!("{:?}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stft;

    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        let scale = a.abs().max(b.abs()).max(1.0);
        (a - b).abs() <= scale * tol
    }

    fn assert_close(a: &[f32], b: &[f32], tol: f32) {
        assert_eq!(a.len(), b.len(), "length mismatch");
        for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
            assert!(
                approx_eq(*x, *y, tol),
                "mismatch at {}: {} vs {} (tol {})",
                i,
                x,
                y,
                tol
            );
        }
    }

    // ── error paths ────────────────────────────────────────────

    #[test]
    fn istft_rejects_empty_spectrogram() {
        let err = istft(&[], 64, 32, WindowType::Hann, 128).unwrap_err();
        assert!(matches!(err, StftError::InvalidSpectrogram(_)));
    }

    #[test]
    fn istft_rejects_zero_n_fft() {
        let err =
            istft(&[0.0; 16], 0, 32, WindowType::Hann, 128).unwrap_err();
        assert!(matches!(err, StftError::InvalidParam(_)));
    }

    #[test]
    fn istft_rejects_zero_hop() {
        let err =
            istft(&[0.0; 16], 64, 0, WindowType::Hann, 128).unwrap_err();
        assert!(matches!(err, StftError::InvalidParam(_)));
    }

    #[test]
    fn istft_rejects_zero_output_length() {
        let err = istft(&[0.0; 16], 64, 32, WindowType::Hann, 0).unwrap_err();
        assert!(matches!(err, StftError::InvalidParam(_)));
    }

    #[test]
    fn istft_rejects_misaligned_spectrogram() {
        // n_fft=64 → bins=33 → 66 floats per frame. Length 100
        // is not a multiple of 66.
        let err =
            istft(&[0.0; 100], 64, 32, WindowType::Hann, 128).unwrap_err();
        assert!(matches!(err, StftError::InvalidSpectrogram(_)));
    }

    // ── round-trip (COLA) ──────────────────────────────────────

    #[test]
    fn istft_recovers_signal_with_hann_hop_half() {
        // Hann window at hop = n_fft / 2 satisfies COLA exactly,
        // so the istft(stft(x)) round-trip should recover the
        // signal within FP tolerance.  We compare the central
        // portion of the recovered signal (skip the edges where
        // boundary effects matter).
        let n_fft = 256u32;
        let hop = 128u32;
        let signal: Vec<f32> = (0..2048)
            .map(|i| ((i as f32) * 0.05).sin() + 0.3 * ((i as f32) * 0.02).cos())
            .collect();
        let spec = stft(&signal, n_fft, hop, WindowType::Hann).unwrap();
        let recovered = istft(
            &spec,
            n_fft,
            hop,
            WindowType::Hann,
            signal.len() as u32,
        )
        .unwrap();
        // Central portion: skip the first and last n_fft/2
        // samples (boundary transients where overlap-add
        // hasn't fully built up).
        let pad = (n_fft / 2) as usize;
        assert_close(
            &recovered[pad..(signal.len() - pad)],
            &signal[pad..(signal.len() - pad)],
            1e-3,
        );
    }

    #[test]
    fn istft_output_length_matches_request() {
        // The output length is whatever the caller asks for,
        // regardless of how many frames the spectrogram has.
        let n_fft = 128u32;
        let hop = 64u32;
        let signal = vec![0.5f32; 512];
        let spec = stft(&signal, n_fft, hop, WindowType::Hann).unwrap();
        let recovered =
            istft(&spec, n_fft, hop, WindowType::Hann, 512).unwrap();
        assert_eq!(recovered.len(), 512);
    }

    #[test]
    fn istft_constant_signal_round_trip_under_hann() {
        // Constant signal with COLA-satisfying Hann/hop=n_fft/2:
        // each sample in the central region should match the
        // original constant.
        let n_fft = 128u32;
        let hop = 64u32;
        let signal = vec![1.5f32; 1024];
        let spec = stft(&signal, n_fft, hop, WindowType::Hann).unwrap();
        let recovered =
            istft(&spec, n_fft, hop, WindowType::Hann, 1024).unwrap();
        let pad = (n_fft / 2) as usize;
        for n in pad..(1024 - pad) {
            assert!(
                approx_eq(recovered[n], 1.5, 1e-3),
                "n={}: got {}, expected 1.5",
                n,
                recovered[n]
            );
        }
    }
}
