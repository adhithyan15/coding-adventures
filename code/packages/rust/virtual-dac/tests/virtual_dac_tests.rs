use coding_adventures_test_bootstrap::prng::SplitMix64;
use oscillator::OscillatorError;
use pcm_audio::{PcmAudio, PcmFormat, PCM16_MAX, PCM16_MIN};
use virtual_dac::{
    clamp_sample_index, pcm16_to_voltage, AnalogSignal, ContinuousSignal,
    LinearInterpolationDACSignal, ZeroOrderHoldDACSignal,
};

const EPS: f64 = 1e-12;

fn audio(samples: Vec<i16>, rate: f64, start: f64) -> PcmAudio {
    PcmAudio::from_i16(samples, PcmFormat::mono_16bit(rate).unwrap(), start).unwrap()
}

#[test]
fn voltage_mapping_is_asymmetric_full_scale() {
    let format = PcmFormat::default();
    assert_eq!(pcm16_to_voltage(0, &format), 0.0);
    assert!((pcm16_to_voltage(PCM16_MAX, &format) - 1.0).abs() < EPS);
    assert!((pcm16_to_voltage(PCM16_MIN, &format) + 1.0).abs() < EPS);
    let scaled = PcmFormat::new(44_100.0, 1, 16, 2.5).unwrap();
    assert!((pcm16_to_voltage(PCM16_MAX, &scaled) - 2.5).abs() < EPS);
    assert!((pcm16_to_voltage(PCM16_MIN, &scaled) + 2.5).abs() < EPS);
}

#[test]
fn zero_order_hold_holds_each_sample() {
    let dac = ZeroOrderHoldDACSignal::new(audio(vec![0, PCM16_MAX, PCM16_MIN, 0], 4.0, 0.0));
    assert_eq!(dac.value_at(0.0).unwrap(), 0.0);
    assert!((dac.value_at(0.25).unwrap() - 1.0).abs() < EPS);
    assert!((dac.value_at(0.49).unwrap() - 1.0).abs() < EPS);
    assert!((dac.value_at(0.5).unwrap() + 1.0).abs() < EPS);
    assert_eq!(dac.value_at(0.75).unwrap(), 0.0);
}

#[test]
fn outside_buffer_is_silent() {
    let dac = ZeroOrderHoldDACSignal::new(audio(vec![PCM16_MAX; 4], 4.0, 1.0));
    assert_eq!(dac.value_at(0.999).unwrap(), 0.0);
    assert_eq!(dac.value_at(2.0).unwrap(), 0.0);
    assert_eq!(dac.value_at(-100.0).unwrap(), 0.0);
    assert!((dac.value_at(1.0).unwrap() - 1.0).abs() < EPS);
}

#[test]
fn empty_buffer_is_silent() {
    let dac = ZeroOrderHoldDACSignal::new(audio(vec![], 8000.0, 0.0));
    assert_eq!(dac.value_at(0.0).unwrap(), 0.0);
    let lin = LinearInterpolationDACSignal::new(audio(vec![], 8000.0, 0.0));
    assert_eq!(lin.value_at(0.0).unwrap(), 0.0);
}

#[test]
fn non_finite_times_are_rejected() {
    let zoh = ZeroOrderHoldDACSignal::new(audio(vec![1, 2, 3], 3.0, 0.0));
    let lin = LinearInterpolationDACSignal::new(audio(vec![1, 2, 3], 3.0, 0.0));
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            zoh.value_at(bad),
            Err(OscillatorError::NonFiniteNumber { .. })
        ));
        assert!(matches!(
            lin.value_at(bad),
            Err(OscillatorError::NonFiniteNumber { .. })
        ));
    }
}

#[test]
fn clamp_sample_index_is_total() {
    assert_eq!(clamp_sample_index(-5.0, 10), 0);
    assert_eq!(clamp_sample_index(f64::NAN, 10), 0);
    assert_eq!(clamp_sample_index(f64::NEG_INFINITY, 10), 0);
    assert_eq!(clamp_sample_index(3.0, 10), 3);
    assert_eq!(clamp_sample_index(9.0, 10), 9);
    assert_eq!(clamp_sample_index(10.0, 10), 9);
    assert_eq!(clamp_sample_index(1e300, 10), 9);
    assert_eq!(clamp_sample_index(f64::INFINITY, 10), 9);
    assert_eq!(clamp_sample_index(5.0, 0), 0);
    assert_eq!(clamp_sample_index(5.0, 1), 0);
}

#[test]
fn rounding_at_right_edge_holds_last_sample() {
    // 0.1 * 3 = 0.30000000000000004 > 0.3: time just below `end` can still
    // produce position == count after rounding; must clamp, not panic.
    let n = 10usize;
    let rate = 10.0;
    let dac = ZeroOrderHoldDACSignal::new(audio(vec![7; n], rate, 0.0));
    let end = n as f64 / rate;
    let t = f64::from_bits(end.to_bits() - 1);
    assert!((dac.value_at(t).unwrap() - 7.0 / 32767.0).abs() < EPS);
}

#[test]
fn linear_interpolation_midpoints() {
    let lin = LinearInterpolationDACSignal::new(audio(vec![0, PCM16_MAX, 0, PCM16_MIN], 4.0, 0.0));
    assert_eq!(lin.value_at(0.0).unwrap(), 0.0);
    assert!((lin.value_at(0.125).unwrap() - 0.5).abs() < 1e-9);
    assert!((lin.value_at(0.25).unwrap() - 1.0).abs() < EPS);
    assert!((lin.value_at(0.375).unwrap() - 0.5).abs() < 1e-9);
    // Final segment holds the last sample.
    assert!((lin.value_at(0.9).unwrap() + 1.0).abs() < 1e-9);
}

#[test]
fn trait_alias_and_accessors() {
    let dac = ZeroOrderHoldDACSignal::new(audio(vec![1, 2], 2.0, 0.0));
    let signal: &dyn AnalogSignal = &dac;
    let _: &dyn ContinuousSignal = signal;
    assert_eq!(dac.pcm_audio().sample_count(), 2);
    assert_eq!(dac.clone().into_pcm_audio().samples(), &[1, 2]);
}

#[test]
fn pbt_outputs_bounded_and_never_panic() {
    let mut rng = SplitMix64::new(0xDAC0_0001);
    for _ in 0..200 {
        let count = rng.bounded(64) as usize;
        let samples: Vec<i16> = (0..count).map(|_| rng.next_u64() as u16 as i16).collect();
        let rate = 1.0 + rng.bounded(48_000) as f64;
        let start = (rng.bounded(2000) as f64 - 1000.0) / 100.0;
        let pcm = audio(samples, rate, start);
        let zoh = ZeroOrderHoldDACSignal::new(pcm.clone());
        let lin = LinearInterpolationDACSignal::new(pcm.clone());
        for _ in 0..50 {
            let t = start + (rng.bounded(4000) as f64 - 500.0) / 1000.0 * pcm.duration_seconds().max(1e-3);
            let a = zoh.value_at(t).unwrap();
            let b = lin.value_at(t).unwrap();
            assert!((-1.0..=1.0).contains(&a), "zoh {a}");
            assert!((-1.0..=1.0).contains(&b), "lin {b}");
        }
        // Exact edges.
        let end = start + pcm.duration_seconds();
        assert!(zoh.value_at(end).unwrap() == 0.0);
        assert!(lin.value_at(end).unwrap() == 0.0);
        assert!(zoh.value_at(f64::from_bits(end.to_bits().wrapping_sub(1))).is_ok());
    }
}

#[test]
fn pbt_clamp_index_in_range() {
    let mut rng = SplitMix64::new(42);
    for _ in 0..1000 {
        let count = 1 + rng.bounded(1000) as usize;
        let raw = f64::from_bits(rng.next_u64());
        let idx = clamp_sample_index(raw, count);
        assert!(idx < count);
    }
}
