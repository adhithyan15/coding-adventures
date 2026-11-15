use coding_adventures_test_bootstrap::prng::SplitMix64;
use oscillator::SineOscillator;
use virtual_speaker::{
    render_speaker_output, AnalogSignal, ClippingSpeakerSignal, ContinuousSignal,
    LinearSpeakerSignal, OscillatorError, DEFAULT_SPEAKER_GAIN, MAX_DURATION_SECONDS, MAX_SAMPLES,
    MAX_SAMPLE_RATE_HZ,
};

struct Constant(f64);

impl ContinuousSignal for Constant {
    fn value_at(&self, _t: f64) -> Result<f64, OscillatorError> {
        Ok(self.0)
    }
}

#[test]
fn default_gain_is_pass_through() {
    let sine = SineOscillator::new(1.0).unwrap();
    let speaker = LinearSpeakerSignal::with_default_gain(sine);
    assert_eq!(speaker.speaker_gain(), DEFAULT_SPEAKER_GAIN);
    for t in [0.0, 0.1, 0.25, 0.9] {
        assert_eq!(speaker.value_at(t).unwrap(), sine.value_at(t).unwrap());
    }
}

#[test]
fn gain_scales_and_may_invert() {
    let speaker = LinearSpeakerSignal::new(Constant(0.5), -2.0).unwrap();
    assert_eq!(speaker.value_at(0.0).unwrap(), -1.0);
    assert_eq!(speaker.analog_signal().0, 0.5);
}

#[test]
fn hostile_gain_and_time_are_rejected() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            LinearSpeakerSignal::new(Constant(1.0), bad),
            Err(OscillatorError::NonFiniteNumber { .. })
        ));
        assert!(ClippingSpeakerSignal::new(Constant(1.0), bad, 1.0).is_err());
        assert!(ClippingSpeakerSignal::new(Constant(1.0), 1.0, bad).is_err());
        let ok = LinearSpeakerSignal::with_default_gain(Constant(1.0));
        assert!(matches!(
            ok.value_at(bad),
            Err(OscillatorError::NonFiniteNumber { .. })
        ));
    }
    assert!(ClippingSpeakerSignal::new(Constant(1.0), 1.0, 0.0).is_err());
    assert!(ClippingSpeakerSignal::new(Constant(1.0), 1.0, -1.0).is_err());
}

#[test]
fn overflowing_product_is_rejected_linear_and_clipped_when_clipping() {
    let linear = LinearSpeakerSignal::new(Constant(f64::MAX), f64::MAX).unwrap();
    assert!(matches!(
        linear.value_at(0.0),
        Err(OscillatorError::NonFiniteNumber { .. })
    ));
    let clip = ClippingSpeakerSignal::new(Constant(f64::MAX), f64::MAX, 2.0).unwrap();
    assert_eq!(clip.value_at(0.0).unwrap(), 2.0);
    let clip_neg = ClippingSpeakerSignal::new(Constant(f64::MAX), -f64::MAX, 2.0).unwrap();
    assert_eq!(clip_neg.value_at(0.0).unwrap(), -2.0);
}

#[test]
fn clipping_speaker_limits_output() {
    let sine = SineOscillator::new(1.0).unwrap();
    let speaker = ClippingSpeakerSignal::new(sine, 4.0, 1.5).unwrap();
    assert_eq!(speaker.limit(), 1.5);
    assert_eq!(speaker.speaker_gain(), 4.0);
    assert_eq!(speaker.value_at(0.25).unwrap(), 1.5);
    assert_eq!(speaker.value_at(0.75).unwrap(), -1.5);
    assert_eq!(speaker.value_at(0.0).unwrap(), 0.0);
    let _ = speaker.analog_signal();
}

#[test]
fn render_is_bounded() {
    let speaker = LinearSpeakerSignal::with_default_gain(Constant(0.25));
    let buffer = render_speaker_output(&speaker, 0.5, 8.0, 1.0).unwrap();
    assert_eq!(buffer.sample_count(), 4);
    assert_eq!(buffer.start_time_seconds(), 1.0);
    assert!(buffer.samples().iter().all(|s| *s == 0.25));

    assert!(matches!(
        render_speaker_output(&speaker, 1.0, MAX_SAMPLE_RATE_HZ + 1.0, 0.0),
        Err(OscillatorError::SampleRateTooHigh { .. })
    ));
    assert!(matches!(
        render_speaker_output(&speaker, MAX_DURATION_SECONDS + 1.0, 1000.0, 0.0),
        Err(OscillatorError::DurationTooLong { .. })
    ));
    // MAX_DURATION_SECONDS * MAX_SAMPLE_RATE_HZ must itself fit in MAX_SAMPLES.
    assert!(MAX_DURATION_SECONDS * MAX_SAMPLE_RATE_HZ <= MAX_SAMPLES as f64);
    for bad in [f64::NAN, f64::INFINITY, -1.0] {
        assert!(render_speaker_output(&speaker, bad, 8.0, 0.0).is_err());
        assert!(render_speaker_output(&speaker, 1.0, bad, 0.0).is_err());
    }
    assert!(render_speaker_output(&speaker, 1.0, 8.0, f64::NAN).is_err());
}

#[test]
fn analog_signal_alias_is_usable() {
    let speaker = LinearSpeakerSignal::with_default_gain(Constant(1.0));
    let dynamic: &dyn AnalogSignal = &speaker;
    assert_eq!(dynamic.value_at(0.0).unwrap(), 1.0);
}

#[test]
fn pbt_linear_matches_product_and_clip_is_bounded() {
    let mut rng = SplitMix64::new(0x5EA4_0001);
    for _ in 0..500 {
        let v = (rng.bounded(2001) as f64 - 1000.0) / 100.0;
        let gain = (rng.bounded(2001) as f64 - 1000.0) / 100.0;
        let limit = 0.01 + rng.bounded(1000) as f64 / 100.0;
        let t = (rng.bounded(2001) as f64 - 1000.0) / 100.0;
        let lin = LinearSpeakerSignal::new(Constant(v), gain).unwrap();
        assert_eq!(lin.value_at(t).unwrap(), gain * v);
        let clip = ClippingSpeakerSignal::new(Constant(v), gain, limit).unwrap();
        let out = clip.value_at(t).unwrap();
        assert!(out.abs() <= limit);
        if (gain * v).abs() <= limit {
            assert_eq!(out, gain * v);
        }
    }
}
