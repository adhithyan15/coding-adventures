//! Property-based testing for `dsp-stft` using SplitMix64 PRNG (DSP05, VER15).

use coding_adventures_test_bootstrap::prng::SplitMix64;
use dsp_stft::mel::{hz_to_mel, mel_to_hz};
use dsp_stft::{
    istft, log_spectrogram, mel_filterbank, mel_spectrogram, mfcc, spectrogram, stft,
    stft_via_runtime, WindowType,
};

fn sample_f32(rng: &mut SplitMix64) -> f32 {
    let u = rng.next_u64();
    let mantissa = (u & 0x00ff_ffff) as f32 / (0x0100_0000 as f32);
    (mantissa * 2.0) - 1.0
}

#[test]
fn test_pbt_stft_istft_cola_round_trip() {
    let mut rng = SplitMix64::new(0xD505_1111_2222_3333);
    for &n_fft in &[32u32, 64, 128] {
        let hop = n_fft / 2;
        for _ in 0..10 {
            let n = (n_fft as usize) * (3 + rng.bounded(6) as usize);
            let sig: Vec<f32> = (0..n).map(|_| sample_f32(&mut rng)).collect();

            let spec = stft(&sig, n_fft, hop, WindowType::Hann).expect("stft");
            let rec = istft(&spec, n_fft, hop, WindowType::Hann, n as u32).expect("istft");
            assert_eq!(rec.len(), n);
            assert!(rec.iter().all(|v| v.is_finite()));

            let pad = n_fft as usize;
            for i in pad..(n - pad) {
                assert!(
                    (rec[i] - sig[i]).abs() < 1e-3,
                    "COLA mismatch at {i}: {} vs {}",
                    rec[i],
                    sig[i]
                );
            }
        }
    }
}

#[test]
fn test_pbt_spectrogram_mel_and_mfcc_invariants() {
    let mut rng = SplitMix64::new(0xD505_4444_5555_6666);
    for _ in 0..20 {
        let n_fft = 64u32;
        let hop = 32u32;
        let n_mels = 12u32;
        let n_mfcc = 6u32;
        let sr = 8000.0 + (rng.bounded(40_000) as f32);
        let n = 128 + (rng.bounded(256) as usize);
        let sig: Vec<f32> = (0..n).map(|_| sample_f32(&mut rng)).collect();

        let expected_frames = 1 + (n - n_fft as usize) / hop as usize;
        let bins = (n_fft as usize) / 2 + 1;

        let pow_spec = spectrogram(&sig, n_fft, hop, WindowType::Hann).expect("spectrogram");
        assert_eq!(pow_spec.len(), expected_frames * bins);
        assert!(pow_spec.iter().all(|&v| v.is_finite() && v >= 0.0));

        let log_spec = log_spectrogram(&sig, n_fft, hop, WindowType::Hann).expect("log_spec");
        assert_eq!(log_spec.len(), pow_spec.len());
        assert!(log_spec.iter().all(|v| v.is_finite()));

        let mel_spec =
            mel_spectrogram(&sig, n_fft, hop, n_mels, sr, WindowType::Hann).expect("mel_spec");
        assert_eq!(mel_spec.len(), expected_frames * n_mels as usize);
        assert!(mel_spec.iter().all(|&v| v.is_finite() && v >= 0.0));

        let coeffs =
            mfcc(&sig, n_fft, hop, n_mels, n_mfcc, sr, WindowType::Hann).expect("mfcc");
        assert_eq!(coeffs.len(), expected_frames * n_mfcc as usize);
        assert!(coeffs.iter().all(|v| v.is_finite()));
    }
}

#[test]
fn test_pbt_hz_mel_roundtrip_and_filterbank_weights() {
    let mut rng = SplitMix64::new(0xD505_7777_8888_9999);
    for _ in 0..100 {
        let hz = (rng.next_u64() & 0xFFFF) as f32 * 0.3;
        let mel = hz_to_mel(hz).expect("hz_to_mel");
        let back = mel_to_hz(mel).expect("mel_to_hz");
        let tol = hz.max(1.0) * 1e-4;
        assert!(
            (hz - back).abs() <= tol,
            "hz->mel->hz mismatch: {hz} -> {mel} -> {back}"
        );
    }

    let fb = mel_filterbank(24, 512, 16_000.0);
    assert_eq!(fb.len(), 24 * 257);
    assert!(fb.iter().all(|&w| w.is_finite() && w >= 0.0));
}

#[test]
fn test_pbt_matrix_ir_matches_scalar_stft() {
    let mut rng = SplitMix64::new(0xD505_AAAA_BBBB_CCCC);
    let windows = [
        WindowType::Rectangular,
        WindowType::Hann,
        WindowType::Hamming,
        WindowType::Blackman,
    ];
    for &win in &windows {
        let n = 64;
        let sig: Vec<f32> = (0..n).map(|_| sample_f32(&mut rng)).collect();
        let scalar = stft(&sig, 32, 16, win).expect("scalar stft");
        let via_rt = stft_via_runtime(&sig, 32, 16, win).expect("stft_via_runtime");
        assert_eq!(scalar.len(), via_rt.len());
        for (i, (&a, &b)) in scalar.iter().zip(&via_rt).enumerate() {
            assert!(
                (a - b).abs() < 1e-4,
                "window {:?} idx {i}: scalar {a} vs runtime {b}",
                win
            );
        }
    }
}
