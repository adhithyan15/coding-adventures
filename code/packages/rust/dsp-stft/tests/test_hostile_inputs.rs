//! Hostile input testing for CWE-20, CWE-190, CWE-400, and CWE-674 in `dsp-stft` (VER15).

use dsp_stft::mel::{hz_to_mel, mel_to_hz};
use dsp_stft::{
    build_stft_graph, istft, log_spectrogram, mel_filterbank, mel_spectrogram, mfcc, spectrogram,
    stft, stft_via_runtime, StftError, WindowType, MAX_FFT_LEN, MAX_FRAMES, MAX_SAMPLES,
};

#[test]
fn test_hostile_cwe_20_non_finite_and_domain_errors() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(matches!(
            stft(&[1.0, bad], 2, 1, WindowType::Hann),
            Err(StftError::InvalidParam(_))
        ));
        assert!(matches!(
            istft(&[1.0, bad, 0.0, 0.0], 2, 1, WindowType::Hann, 4),
            Err(StftError::InvalidSpectrogram(_))
        ));
        assert!(matches!(
            spectrogram(&[1.0, bad], 2, 1, WindowType::Hann),
            Err(StftError::InvalidParam(_))
        ));
        assert!(matches!(
            log_spectrogram(&[1.0, bad], 2, 1, WindowType::Hann),
            Err(StftError::InvalidParam(_))
        ));
        assert!(matches!(
            mel_spectrogram(&[1.0, bad], 2, 1, 4, 16000.0, WindowType::Hann),
            Err(StftError::InvalidParam(_))
        ));
        assert!(matches!(
            mfcc(&[1.0, bad], 2, 1, 4, 2, 16000.0, WindowType::Hann),
            Err(StftError::InvalidParam(_))
        ));
        assert!(matches!(
            stft_via_runtime(&[1.0, bad], 2, 1, WindowType::Hann),
            Err(StftError::InvalidParam(_))
        ));
        assert!(mel_filterbank(40, 512, bad).is_empty());
        assert!(matches!(hz_to_mel(bad), Err(StftError::InvalidParam(_))));
        assert!(matches!(mel_to_hz(bad), Err(StftError::InvalidParam(_))));
    }

    assert!(matches!(hz_to_mel(-1.0), Err(StftError::InvalidParam(_))));
    assert!(matches!(mel_to_hz(-1.0), Err(StftError::InvalidParam(_))));
    // mel_to_hz overflow guard: 10^(1e6 / 2595) overflows f32 to Inf
    assert!(matches!(
        mel_to_hz(1_000_000.0),
        Err(StftError::InvalidParam(_))
    ));
}

#[test]
fn test_hostile_cwe_20_subnormal_sample_rate_and_power_overflow() {
    // Subnormal / tiny sample_rate causes n_fft / sample_rate -> Inf -> 0 * Inf = NaN if unguarded.
    for tiny_sr in [0.0_f32, -0.0, -16000.0, f32::from_bits(1), 1e-40_f32, f32::MIN_POSITIVE] {
        let fb = mel_filterbank(40, 512, tiny_sr);
        assert!(
            fb.is_empty(),
            "tiny/non-positive sample_rate {tiny_sr:e} must return empty filterbank"
        );
        assert!(matches!(
            mel_spectrogram(&[0.1; 512], 512, 256, 40, tiny_sr, WindowType::Hann),
            Err(StftError::InvalidParam(_))
        ));
    }

    // Power spectrogram overflow: finite input [1e20; 64] produces finite STFT (~6.4e21)
    // whose |STFT|^2 overflows f32 to Inf (and mel_spectrogram would compute 0 * Inf = NaN).
    let hot = vec![1e20_f32; 64];
    assert!(matches!(
        spectrogram(&hot, 64, 32, WindowType::Rectangular),
        Err(StftError::InvalidParam(_))
    ));
    assert!(matches!(
        log_spectrogram(&hot, 64, 32, WindowType::Rectangular),
        Err(StftError::InvalidParam(_))
    ));
    assert!(matches!(
        mel_spectrogram(&hot, 64, 32, 16, 16000.0, WindowType::Rectangular),
        Err(StftError::InvalidParam(_))
    ));
    assert!(matches!(
        mfcc(&hot, 64, 32, 16, 8, 16000.0, WindowType::Rectangular),
        Err(StftError::InvalidParam(_))
    ));

    // Normal MFCC and stft_via_runtime output length & finiteness regression check.
    let quiet = vec![0.0_f32; 64];
    let m = mfcc(&quiet, 64, 32, 16, 8, 16000.0, WindowType::Hann).expect("quiet mfcc");
    assert_eq!(m.len(), 8);
    assert!(m.iter().all(|v| v.is_finite()));
    let rt = stft_via_runtime(&[0.25_f32; 64], 32, 16, WindowType::Hann).expect("stft_via_runtime");
    assert_eq!(rt.len(), 3 * 17 * 2);
    assert!(rt.iter().all(|v| v.is_finite()));
}

#[test]
fn test_hostile_cwe_190_and_cwe_400_caps_before_allocation() {
    // Empty inputs
    assert_eq!(
        stft(&[], 64, 32, WindowType::Hann),
        Err(StftError::EmptySignal)
    );
    assert!(matches!(
        istft(&[], 64, 32, WindowType::Hann, 64),
        Err(StftError::InvalidSpectrogram(_))
    ));

    // Zero / out-of-bounds params
    assert!(matches!(
        stft(&[1.0; 64], 0, 32, WindowType::Hann),
        Err(StftError::InvalidParam(_))
    ));
    assert!(matches!(
        stft(&[1.0; 64], 64, 0, WindowType::Hann),
        Err(StftError::InvalidParam(_))
    ));
    assert!(matches!(
        stft(&[1.0; 64], (MAX_FFT_LEN as u32) + 1, 32, WindowType::Hann),
        Err(StftError::InvalidParam(_))
    ));
    assert!(matches!(
        istft(&[0.0; 66], (MAX_FFT_LEN as u32) + 1, 32, WindowType::Hann, 64),
        Err(StftError::InvalidParam(_))
    ));
    assert!(matches!(
        istft(&[0.0; 66], 64, 32, WindowType::Hann, (MAX_SAMPLES as u32) + 1),
        Err(StftError::InvalidParam(_))
    ));
    assert!(matches!(
        mel_spectrogram(&[1.0; 64], 0, 32, 16, 16000.0, WindowType::Hann),
        Err(StftError::InvalidParam(_))
    ));
    assert!(matches!(
        mel_spectrogram(&[1.0; 64], 64, 32, 0, 16000.0, WindowType::Hann),
        Err(StftError::InvalidParam(_))
    ));
    assert!(matches!(
        mfcc(&[1.0; 64], 64, 32, 16, 0, 16000.0, WindowType::Hann),
        Err(StftError::InvalidParam(_))
    ));
    assert!(matches!(
        mfcc(&[1.0; 64], 64, 32, 16, 17, 16000.0, WindowType::Hann),
        Err(StftError::InvalidParam(_))
    ));

    // Matrix-IR graph caps: u32::MAX signal_length and > MAX_FRAMES rejected before graph build.
    assert!(matches!(
        build_stft_graph(u32::MAX, 64, 32, WindowType::Hann),
        Err(StftError::InvalidParam(_))
    ));
    assert!(matches!(
        build_stft_graph((MAX_FRAMES as u32) + 10, 2, 1, WindowType::Hann),
        Err(StftError::InvalidParam(_))
    ));
    assert!(mel_filterbank((MAX_SAMPLES as u32) + 1, 64, 16000.0).is_empty());

    // Oversized spectrogram in istft rejected by MAX_SAMPLES warrant bound.
    let oversized = vec![0.0_f32; MAX_SAMPLES + 1];
    assert!(matches!(
        istft(&oversized, 64, 32, WindowType::Hann, 64),
        Err(StftError::InvalidParam(_))
    ));
}

#[test]
fn test_hostile_cwe_674_iterative_stft_and_istft() {
    let signal: Vec<f32> = (0..16384).map(|i| ((i as f32) * 0.01).sin()).collect();
    let spec = stft(&signal, 256, 128, WindowType::Hann).expect("iterative stft");
    let recon = istft(&spec, 256, 128, WindowType::Hann, signal.len() as u32)
        .expect("iterative istft");
    assert_eq!(recon.len(), signal.len());
    assert!(recon.iter().all(|x| x.is_finite()));
}
