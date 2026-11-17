use coding_adventures_test_bootstrap::prng::SplitMix64;
use oscillator::{SineOscillator, UniformSampler};
use pcm_audio::{
    encode_sample_buffer, float_to_pcm16, pcm16_to_f32, safe_round_sample, samples_to_pcm_buffer,
    PcmAudio, PcmError, PcmFormat, DEFAULT_BIT_DEPTH, DEFAULT_CHANNEL_COUNT,
    DEFAULT_FULL_SCALE_VOLTAGE, DEFAULT_SAMPLE_RATE_HZ, PCM16_MAX, PCM16_MIN,
};

const TOLERANCE: f64 = 1e-9;

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOLERANCE,
        "got {actual}, expected {expected}"
    );
}

#[test]
fn test_defaults_and_constants_are_visible() {
    assert_close(DEFAULT_SAMPLE_RATE_HZ, 44_100.0);
    assert_eq!(DEFAULT_CHANNEL_COUNT, 1);
    assert_eq!(DEFAULT_BIT_DEPTH, 16);
    assert_close(DEFAULT_FULL_SCALE_VOLTAGE, 1.0);
    assert_eq!(PCM16_MIN, -32_768);
    assert_eq!(PCM16_MAX, 32_767);
}

#[test]
fn test_pcm_format_validation() {
    let format = PcmFormat::new(48_000.0, 1, 16, 1.0).unwrap();
    assert_eq!(format.minimum_integer(), -32_768);
    assert_eq!(format.maximum_integer(), 32_767);
    assert_eq!(format.sample_width_bytes(), 2);
    assert_eq!(format.integer_sample_rate().unwrap(), 48_000);

    // Invalid parameters
    assert!(PcmFormat::new(0.0, 1, 16, 1.0).is_err());
    assert!(PcmFormat::new(-44100.0, 1, 16, 1.0).is_err());
    assert!(PcmFormat::new(44100.0, 0, 16, 1.0).is_err());
    assert!(PcmFormat::new(44100.0, 9, 16, 1.0).is_err()); // > MAX_CHANNELS
    assert!(PcmFormat::new(44100.0, 1, 24, 1.0).is_err()); // unsupported bit depth
    assert!(PcmFormat::new(44100.0, 1, 16, 0.0).is_err()); // voltage <= 0
}

#[test]
fn test_integer_sample_rate_rejects_fractional_rates() {
    let format = PcmFormat::new(44_100.5, 1, 16, 1.0).unwrap();
    assert!(matches!(
        format.integer_sample_rate().unwrap_err(),
        PcmError::IntegerSampleRateRequired { .. }
    ));
}

#[test]
fn test_float_to_pcm16_clipping_and_exact_points() {
    assert_eq!(float_to_pcm16(0.0).unwrap(), (0, false));
    assert_eq!(float_to_pcm16(1.0).unwrap(), (32767, false));
    assert_eq!(float_to_pcm16(-1.0).unwrap(), (-32768, false));
    assert_eq!(float_to_pcm16(2.0).unwrap(), (32767, true));
    assert_eq!(float_to_pcm16(-2.0).unwrap(), (-32768, true));
}

#[test]
fn test_float_to_pcm16_rejects_non_finite_samples() {
    assert!(matches!(
        float_to_pcm16(f64::NAN).unwrap_err(),
        PcmError::NonFiniteSample { .. }
    ));
    assert!(matches!(
        float_to_pcm16(f64::INFINITY).unwrap_err(),
        PcmError::NonFiniteSample { .. }
    ));
    assert!(matches!(
        float_to_pcm16(f64::NEG_INFINITY).unwrap_err(),
        PcmError::NonFiniteSample { .. }
    ));
}

#[test]
fn test_encode_sample_buffer_from_oscillator() {
    let osc = SineOscillator::new(1.0).unwrap();
    let sampler = UniformSampler::new(4.0).unwrap();
    let sample_buffer = sampler.sample(&osc, 1.0).unwrap();

    let pcm = encode_sample_buffer(&sample_buffer).unwrap();

    assert_eq!(pcm.sample_count(), 4);
    assert_eq!(pcm.clipped_sample_count(), 0);
    assert_close(pcm.sample_period_seconds(), 0.25);
    assert_close(pcm.duration_seconds(), 1.0);

    let samples = pcm.samples();
    assert_eq!(samples[0], 0);
    assert_eq!(samples[1], 32767);
    assert_eq!(samples[2], 0);
    assert_eq!(samples[3], -32768);
}

#[test]
fn test_samples_to_pcm_buffer_helper() {
    let samples = vec![0.0, 0.5, -0.5];
    let pcm = samples_to_pcm_buffer(&samples, 3.0).unwrap();

    assert_eq!(pcm.samples(), &[0, 16384, -16384]);
    assert_close(pcm.duration_seconds(), 1.0);
}

#[test]
fn test_pcm_to_little_endian_bytes() {
    let format = PcmFormat::mono_16bit(3.0).unwrap();
    let pcm = PcmAudio::from_i16(vec![0, 32767, -32768], format, 0.0).unwrap();

    let bytes = pcm.to_little_endian_bytes().unwrap();
    assert_eq!(bytes, &[0x00, 0x00, 0xFF, 0x7F, 0x00, 0x80]);
}

#[test]
fn test_pcm_audio_time_at_and_indexing() {
    let format = PcmFormat::mono_16bit(10.0).unwrap();
    let pcm = PcmAudio::from_i16(vec![100, 200, 300], format, 5.0).unwrap();

    assert_close(pcm.time_at(0).unwrap(), 5.0);
    assert_close(pcm.time_at(1).unwrap(), 5.1);
    assert_close(pcm.time_at(2).unwrap(), 5.2);
    assert!(matches!(
        pcm.time_at(3).unwrap_err(),
        PcmError::IndexOutOfBounds { .. }
    ));
}

// --- SplitMix64 Property-Based Tests ---

#[test]
fn test_pbt_splitmix64_quantization_round_trip() {
    let mut rng = SplitMix64::new(0xfeed_beef_c001_d00d);

    for _ in 0..10_000 {
        // Generate random float in [-1.0, 1.0]
        let raw_u64 = rng.next_u64() % 2_000_001;
        let original_f = ((raw_u64 as f64) - 1_000_000.0) / 1_000_000.0;

        let (pcm_int, clipped) = safe_round_sample(original_f).unwrap();
        assert!(!clipped);

        let reconstructed = pcm16_to_f32(pcm_int);
        let error = (reconstructed as f64 - original_f).abs();

        // 16-bit quantization step size is 1 / 32768 ≈ 3.05e-5
        assert!(
            error <= 1.0 / 32767.0 + 1e-6,
            "quantization error {error} exceeded limit for sample {original_f}"
        );
    }
}
