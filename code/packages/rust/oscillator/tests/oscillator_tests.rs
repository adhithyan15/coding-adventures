use coding_adventures_test_bootstrap::prng::SplitMix64;
use oscillator::{
    fractional_part, nyquist_frequency, sample_count_for_duration, time_at_sample,
    ContinuousSignal, OscillatorError, PulseOscillator, SampleBuffer, SawtoothOscillator,
    SineOscillator, SquareOscillator, TriangleOscillator, UniformSampler, MAX_DURATION_SECONDS,
    MAX_SAMPLES, MAX_SAMPLE_RATE_HZ,
};

const TOLERANCE: f64 = 1e-9;

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOLERANCE,
        "got {actual}, expected {expected}, diff: {}",
        (actual - expected).abs()
    );
}

// --- OSC00 §18 Cross-Language Parity Vectors ---

#[test]
fn test_osc00_sine_parity_default() {
    let osc = SineOscillator::new(1.0).unwrap();
    assert_close(osc.value_at(0.00).unwrap(), 0.0);
    assert_close(osc.value_at(0.25).unwrap(), 1.0);
    assert_close(osc.value_at(0.50).unwrap(), 0.0);
    assert_close(osc.value_at(0.75).unwrap(), -1.0);
    assert_close(osc.value_at(1.00).unwrap(), 0.0);
}

#[test]
fn test_osc00_sine_parity_with_amplitude_and_offset() {
    let osc = SineOscillator::with_options(1.0, 2.0, 0.0, 3.0).unwrap();
    assert_close(osc.value_at(0.00).unwrap(), 3.0);
    assert_close(osc.value_at(0.25).unwrap(), 5.0);
    assert_close(osc.value_at(0.75).unwrap(), 1.0);
}

#[test]
fn test_osc00_sine_parity_with_phase() {
    let osc = SineOscillator::with_options(1.0, 1.0, 0.25, 0.0).unwrap();
    assert_close(osc.value_at(0.00).unwrap(), 1.0);
    assert_close(osc.value_at(0.25).unwrap(), 0.0);
}

#[test]
fn test_osc00_square_parity() {
    let osc = SquareOscillator::with_options(2.0, 0.0, 1.0, 0.5, 0.0).unwrap();
    assert_eq!(osc.value_at(0.000).unwrap(), 1.0);
    assert_eq!(osc.value_at(0.125).unwrap(), 1.0);
    assert_eq!(osc.value_at(0.250).unwrap(), 0.0);
    assert_eq!(osc.value_at(0.375).unwrap(), 0.0);
    assert_eq!(osc.value_at(0.500).unwrap(), 1.0);
}

#[test]
fn test_osc00_uniform_sampler_parity() {
    let signal = SineOscillator::new(1.0).unwrap();
    let sampler = UniformSampler::new(4.0).unwrap();
    let buffer = sampler.sample(&signal, 1.0).unwrap();

    assert_eq!(buffer.sample_count(), 4);
    assert_close(buffer.sample_period_seconds(), 0.25);
    assert_close(buffer.duration_seconds(), 1.0);
    assert_close(sampler.nyquist_frequency(), 2.0);

    let samples = buffer.samples();
    assert_close(samples[0], 0.0);
    assert_close(samples[1], 1.0);
    assert_close(samples[2], 0.0);
    assert_close(samples[3], -1.0);
}

// --- Waveform Tests (Pulse, Sawtooth, Triangle) ---

#[test]
fn test_pulse_oscillator() {
    let pulse = PulseOscillator::with_options(1.0, 0.25, 2.0, 0.0, 5.0).unwrap();
    // pos < 0.25 -> 5.0 + 2.0 = 7.0
    // pos >= 0.25 -> 5.0 - 2.0 = 3.0
    assert_close(pulse.value_at(0.10).unwrap(), 7.0);
    assert_close(pulse.value_at(0.24).unwrap(), 7.0);
    assert_close(pulse.value_at(0.25).unwrap(), 3.0);
    assert_close(pulse.value_at(0.80).unwrap(), 3.0);
}

#[test]
fn test_sawtooth_oscillator() {
    let saw = SawtoothOscillator::new(1.0).unwrap();
    assert_close(saw.value_at(0.00).unwrap(), -1.0);
    assert_close(saw.value_at(0.25).unwrap(), -0.5);
    assert_close(saw.value_at(0.50).unwrap(), 0.0);
    assert_close(saw.value_at(0.75).unwrap(), 0.5);
    // Just before 1.0
    assert_close(saw.value_at(0.99).unwrap(), 0.98);
}

#[test]
fn test_triangle_oscillator() {
    let tri = TriangleOscillator::new(1.0).unwrap();
    assert_close(tri.value_at(0.00).unwrap(), 0.0);
    assert_close(tri.value_at(0.25).unwrap(), 1.0);
    assert_close(tri.value_at(0.50).unwrap(), 0.0);
    assert_close(tri.value_at(0.75).unwrap(), -1.0);
    assert_close(tri.value_at(1.00).unwrap(), 0.0);
}

#[test]
fn test_zero_frequency_signals() {
    let sine = SineOscillator::new(0.0).unwrap();
    assert_close(sine.value_at(0.0).unwrap(), 0.0);
    assert_close(sine.value_at(100.0).unwrap(), 0.0);

    let square = SquareOscillator::new(0.0).unwrap();
    assert_eq!(square.value_at(0.0).unwrap(), 1.0);
    assert_eq!(square.value_at(50.0).unwrap(), 1.0);
}

#[test]
fn test_nyquist_frequency_helper() {
    assert_close(nyquist_frequency(44_100.0).unwrap(), 22_050.0);
    assert_close(nyquist_frequency(48_000.0).unwrap(), 24_000.0);
    assert!(nyquist_frequency(0.0).is_err());
    assert!(nyquist_frequency(-100.0).is_err());
}

#[test]
fn test_fractional_part_negative_numbers() {
    assert_close(fractional_part(0.25), 0.25);
    assert_close(fractional_part(-0.25), 0.75);
    assert_close(fractional_part(-1.0), 0.0);
    assert_close(fractional_part(1.0), 0.0);
}

#[test]
fn test_sample_count_rounding_tolerance() {
    // 0.01 * 48000.0 is 480.0
    let count = sample_count_for_duration(0.01, 48_000.0).unwrap();
    assert_eq!(count, 480);

    // 0.0 duration
    let count_zero = sample_count_for_duration(0.0, 44_100.0).unwrap();
    assert_eq!(count_zero, 0);
}

// --- CWE-400 / CWE-190 / CWE-20 Bounds and Hostile Inputs ---

#[test]
fn test_cwe400_sample_rate_bounds() {
    assert!(UniformSampler::new(MAX_SAMPLE_RATE_HZ).is_ok());
    let err = UniformSampler::new(MAX_SAMPLE_RATE_HZ + 1.0).unwrap_err();
    assert!(matches!(err, OscillatorError::SampleRateTooHigh { .. }));

    let err_neg = UniformSampler::new(-1.0).unwrap_err();
    assert!(matches!(err_neg, OscillatorError::InvalidParameter { .. }));
}

#[test]
fn test_cwe400_duration_bounds() {
    assert!(sample_count_for_duration(MAX_DURATION_SECONDS, 1000.0).is_ok());
    let err = sample_count_for_duration(MAX_DURATION_SECONDS + 1.0, 1000.0).unwrap_err();
    assert!(matches!(err, OscillatorError::DurationTooLong { .. }));
}

#[test]
fn test_cwe400_sample_count_bounds() {
    let sampler = UniformSampler::new(192_000.0).unwrap();
    let osc = SineOscillator::new(440.0).unwrap();
    let err = sampler
        .sample_count(&osc, MAX_SAMPLES + 1)
        .unwrap_err();
    assert!(matches!(err, OscillatorError::SampleCountTooLarge { .. }));
}

#[test]
fn test_cwe20_non_finite_inputs_rejected() {
    // NaN
    assert!(matches!(
        SineOscillator::new(f64::NAN).unwrap_err(),
        OscillatorError::NonFiniteNumber { .. }
    ));
    assert!(matches!(
        SquareOscillator::new(f64::NAN).unwrap_err(),
        OscillatorError::NonFiniteNumber { .. }
    ));
    assert!(matches!(
        UniformSampler::new(f64::NAN).unwrap_err(),
        OscillatorError::NonFiniteNumber { .. }
    ));

    // +Inf, -Inf
    assert!(matches!(
        SineOscillator::new(f64::INFINITY).unwrap_err(),
        OscillatorError::NonFiniteNumber { .. }
    ));
    assert!(matches!(
        SineOscillator::new(f64::NEG_INFINITY).unwrap_err(),
        OscillatorError::NonFiniteNumber { .. }
    ));

    // ValueAt with NaN
    let osc = SineOscillator::new(440.0).unwrap();
    assert!(matches!(
        osc.value_at(f64::NAN).unwrap_err(),
        OscillatorError::NonFiniteNumber { .. }
    ));
    assert!(matches!(
        osc.value_at(f64::INFINITY).unwrap_err(),
        OscillatorError::NonFiniteNumber { .. }
    ));
}

#[test]
fn test_invalid_duty_cycle_rejected() {
    let err_zero = SquareOscillator::with_options(1.0, -1.0, 1.0, 0.0, 0.0).unwrap_err();
    assert!(matches!(err_zero, OscillatorError::InvalidParameter { .. }));

    let err_one = SquareOscillator::with_options(1.0, -1.0, 1.0, 1.0, 0.0).unwrap_err();
    assert!(matches!(err_one, OscillatorError::InvalidParameter { .. }));
}

#[test]
fn test_sample_buffer_bounds() {
    let buffer = SampleBuffer::new(vec![0.1, 0.2, 0.3], 10.0, 0.0).unwrap();
    assert_eq!(buffer.sample_count(), 3);
    assert_close(buffer.time_at(0).unwrap(), 0.0);
    assert_close(buffer.time_at(1).unwrap(), 0.1);
    assert_close(buffer.time_at(2).unwrap(), 0.2);
    assert!(matches!(
        buffer.time_at(3).unwrap_err(),
        OscillatorError::IndexOutOfBounds { .. }
    ));
}

// --- SplitMix64 Property-Based Tests ---

#[test]
fn test_pbt_splitmix64_sine_properties() {
    let mut rng = SplitMix64::new(0x1337_cafe_beef_4400);

    for _ in 0..1000 {
        let freq = (rng.next_u64() % 20_000) as f64; // 0 to 20,000 Hz
        let amp = ((rng.next_u64() % 1000) as f64) / 100.0; // 0.0 to 10.0
        let phase = ((rng.next_u64() % 1000) as f64) / 1000.0; // 0.0 to 1.0
        let offset = (((rng.next_u64() % 2000) as f64) - 1000.0) / 100.0; // -10.0 to 10.0

        let osc = SineOscillator::with_options(freq, amp, phase, offset).unwrap();

        // Sample at 5 random times
        for _ in 0..5 {
            let t = ((rng.next_u64() % 100_000) as f64) / 10_000.0; // 0.0 to 10.0 s
            let val = osc.value_at(t).unwrap();

            // Value must always be bounded by [offset - amp - eps, offset + amp + eps]
            assert!(
                val >= offset - amp - TOLERANCE && val <= offset + amp + TOLERANCE,
                "val: {val}, range: [{}, {}]",
                offset - amp,
                offset + amp
            );
        }
    }
}

#[test]
fn test_pbt_splitmix64_sampler_consistency() {
    let mut rng = SplitMix64::new(0xdead_beef_feed_face);

    for _ in 0..100 {
        let freq = 10.0 + (rng.next_u64() % 1000) as f64;
        let osc = TriangleOscillator::new(freq).unwrap();

        let sample_rate = 1000.0 + (rng.next_u64() % 4000) as f64;
        let sampler = UniformSampler::new(sample_rate).unwrap();

        let count = (rng.next_u64() % 200 + 1) as usize;
        let start_time = ((rng.next_u64() % 1000) as f64) / 100.0;

        let buffer = sampler
            .sample_count_with_start_time(&osc, count, start_time)
            .unwrap();
        assert_eq!(buffer.sample_count(), count);

        // Every sample in buffer must exactly match individual point evaluation
        for (i, &sample_val) in buffer.samples().iter().enumerate() {
            let t = time_at_sample(i, sample_rate, start_time).unwrap();
            let expected = osc.value_at(t).unwrap();
            assert_close(sample_val, expected);
        }
    }
}
