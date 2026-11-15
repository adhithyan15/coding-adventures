//! Hostile input validation and fail-closed bounds tests (CWE-400 / CWE-190 / CWE-20).

use musical_instruments::{
    get_gm_program, get_instrument, parse_note, render_instrument_note, render_instrument_note_pcm,
    AdsrEnvelope, HarmonicPartial, InstrumentProfile, InstrumentSignal, Note, RenderOptions,
    VariationProfile, DEFAULT_MAX_PARTIAL_COUNT, DEFAULT_MAX_SAMPLE_COUNT,
    MAX_NOTE_DURATION_SECONDS, MAX_PARTIALS, MAX_SAMPLE_COUNT, MAX_SAMPLE_RATE_HZ,
};

#[test]
fn test_exported_constants_and_defaults() {
    assert_eq!(MAX_PARTIALS, 64);
    assert_eq!(DEFAULT_MAX_PARTIAL_COUNT, 64);
    assert_eq!(MAX_NOTE_DURATION_SECONDS, 60.0);
    assert_eq!(MAX_SAMPLE_RATE_HZ, 192_000.0);
    assert_eq!(MAX_SAMPLE_COUNT, 16_777_216);
    assert_eq!(DEFAULT_MAX_SAMPLE_COUNT, 10_000_000);
}

#[test]
fn test_hostile_non_finite_inputs_fail_closed() {
    let non_finites = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];

    for &bad in &non_finites {
        // Duration
        let opts = RenderOptions::default();
        assert!(render_instrument_note("A4", bad, &opts).is_err());
        assert!(render_instrument_note_pcm("A4", bad, &opts).is_err());

        // Amplitude
        let opts_amp = RenderOptions::new().amplitude(bad);
        assert!(render_instrument_note("A4", 0.01, &opts_amp).is_err());

        // Sample rate
        let opts_sr = RenderOptions::new().sample_rate_hz(bad);
        assert!(render_instrument_note("A4", 0.01, &opts_sr).is_err());

        // Start time
        let opts_st = RenderOptions::new().start_time_seconds(bad);
        assert!(render_instrument_note("A4", 0.01, &opts_st).is_err());

        // Partial
        assert!(HarmonicPartial::new(bad, 1.0).is_err());
        assert!(HarmonicPartial::new(1.0, bad).is_err());
        assert!(HarmonicPartial::with_phase(1.0, 1.0, bad).is_err());

        // Envelope
        assert!(AdsrEnvelope::new(bad, 0.01, 0.5, 0.01).is_err());
        assert!(AdsrEnvelope::new(0.01, bad, 0.5, 0.01).is_err());
        assert!(AdsrEnvelope::new(0.01, 0.01, bad, 0.01).is_err());
        assert!(AdsrEnvelope::new(0.01, 0.01, 0.5, bad).is_err());

        let env = AdsrEnvelope::new(0.01, 0.01, 0.5, 0.01).unwrap();
        assert!(env.rendered_duration_seconds(bad).is_err());
        assert!(env.value_at(bad, 1.0).is_err());
        assert!(env.value_at(0.01, bad).is_err());

        // Signal
        let sine_inst = get_instrument("sine").unwrap();
        assert!(InstrumentSignal::new(bad, sine_inst.clone(), 0.1, 1.0, 44100.0, None).is_err());
        assert!(InstrumentSignal::new(440.0, sine_inst.clone(), bad, 1.0, 44100.0, None).is_err());
        assert!(InstrumentSignal::new(440.0, sine_inst.clone(), 0.1, bad, 44100.0, None).is_err());
        assert!(InstrumentSignal::new(440.0, sine_inst.clone(), 0.1, 1.0, bad, None).is_err());

        // Variation
        assert!(VariationProfile::new(bad, 0.0, 0.0, None, 0.0).is_err());
        assert!(VariationProfile::new(0.0, bad, 0.0, None, 0.0).is_err());
        assert!(VariationProfile::new(0.0, 0.0, bad, None, 0.0).is_err());
        assert!(VariationProfile::new(0.0, 0.0, 0.0, None, bad).is_err());
    }
}

#[test]
fn test_cwe400_max_partials_bound() {
    let mut too_many = Vec::new();
    for i in 0..=MAX_PARTIALS {
        too_many.push(HarmonicPartial::new((i + 1) as f64, 1.0).unwrap());
    }
    let env = AdsrEnvelope::new(0.01, 0.01, 0.5, 0.01).unwrap();
    assert!(InstrumentProfile::new(
        "overloaded",
        "Overloaded",
        "additive",
        0.5,
        too_many,
        env,
        VariationProfile::default(),
        None,
        "custom",
    )
    .is_err());

    let opts = RenderOptions::new().max_partial_count(MAX_PARTIALS + 1);
    assert!(render_instrument_note("A4", 0.01, &opts).is_err());

    // Render rejects when instrument has more partials than max_partial_count
    let opts_low = RenderOptions::new()
        .instrument_id("flute_naive")
        .max_partial_count(1);
    assert!(render_instrument_note("A4", 0.01, &opts_low).is_err());
}

#[test]
fn test_cwe400_max_note_duration_bound() {
    let opts = RenderOptions::default();
    assert!(render_instrument_note("A4", 60.1, &opts).is_err());

    let sine_inst = get_instrument("sine").unwrap();
    assert!(InstrumentSignal::new(440.0, sine_inst, 60.1, 1.0, 44100.0, None).is_err());

    assert!(AdsrEnvelope::new(60.1, 0.01, 0.5, 0.01).is_err());
    assert!(AdsrEnvelope::new(0.01, 60.1, 0.5, 0.01).is_err());
    assert!(AdsrEnvelope::new(0.01, 0.01, 0.5, 60.1).is_err());

    let env = AdsrEnvelope::new(0.01, 0.01, 0.5, 0.01).unwrap();
    assert!(env.rendered_duration_seconds(60.1).is_err());
    assert!(env.value_at(0.01, 60.1).is_err());
}

#[test]
fn test_cwe400_max_sample_rate_hz_bound() {
    let opts_too_high = RenderOptions::new().sample_rate_hz(192_001.0);
    assert!(render_instrument_note("A4", 0.01, &opts_too_high).is_err());

    let opts_huge = RenderOptions::new().sample_rate_hz(1e12);
    assert!(render_instrument_note("A4", 0.01, &opts_huge).is_err());

    let sine_inst = get_instrument("sine").unwrap();
    assert!(InstrumentSignal::new(440.0, sine_inst, 0.1, 1.0, 192_001.0, None).is_err());
}

#[test]
fn test_cwe400_max_sample_count_bound() {
    let opts_over_max = RenderOptions::new().max_sample_count(MAX_SAMPLE_COUNT + 1);
    assert!(render_instrument_note("A4", 0.01, &opts_over_max).is_err());

    // Budget exceeded when note would produce too many samples
    let opts_budget = RenderOptions::new().max_sample_count(10);
    assert!(render_instrument_note("A4", 1.0, &opts_budget).is_err());
}

#[test]
fn test_invalid_gm_programs_fail() {
    assert!(get_gm_program(0).is_err());
    assert!(get_gm_program(129).is_err());
    assert!(get_gm_program(255).is_err());
}

#[test]
fn test_zero_velocity_amplitude_renders_zeros() {
    let opts_zero = RenderOptions::new()
        .instrument_id("flute_naive")
        .amplitude(0.0);
    let rendered = render_instrument_note("A4", 0.01, &opts_zero).unwrap();

    for &s in rendered.floating_samples.samples() {
        assert_eq!(s, 0.0);
    }
    for &p in rendered.pcm_buffer.samples() {
        assert_eq!(p, 0);
    }
}

#[test]
fn test_silence_preset_renders_all_zeros() {
    let opts_silence = RenderOptions::new().instrument_id("silence");
    let rendered = render_instrument_note("C4", 0.01, &opts_silence).unwrap();

    for &s in rendered.floating_samples.samples() {
        assert_eq!(s, 0.0);
    }
    for &p in rendered.pcm_buffer.samples() {
        assert_eq!(p, 0);
    }
}

#[test]
fn test_nyquist_frequency_rejection() {
    // Note at 500 Hz cannot be represented at 800 Hz sample rate (Nyquist = 400 Hz)
    let opts = RenderOptions::new().sample_rate_hz(800.0);
    assert!(render_instrument_note("A4", 0.01, &opts).is_err());
}

#[test]
fn test_invalid_note_strings_fail_closed() {
    let opts = RenderOptions::default();
    assert!(render_instrument_note("InvalidNote", 0.01, &opts).is_err());
    assert!(render_instrument_note("H4", 0.01, &opts).is_err());
    assert!(render_instrument_note("", 0.01, &opts).is_err());
    assert!(render_instrument_note("A", 0.01, &opts).is_err());
    assert!(render_instrument_note("A#b4", 0.01, &opts).is_err());
}

#[test]
fn test_cwe190_octave_overflow_protection() {
    // Extreme octaves must fail closed without integer overflow panics
    assert!(Note::new("A", "", i32::MAX).is_err());
    assert!(Note::new("A", "", i32::MIN).is_err());
    assert!(Note::new("A", "", 21).is_err());
    assert!(Note::new("A", "", -11).is_err());

    assert!(parse_note("A2147483647").is_err());
    assert!(parse_note("A-2147483648").is_err());
    assert!(parse_note("A999999999999999").is_err());

    let opts = RenderOptions::default();
    assert!(render_instrument_note("A2147483647", 0.01, &opts).is_err());
}

#[test]
fn test_cwe400_instrument_signal_direct_construction_bounds() {
    // Construct profile with excessive partials bypassing normal constructor
    let mut too_many = Vec::new();
    for i in 0..=MAX_PARTIALS {
        too_many.push(HarmonicPartial::new((i + 1) as f64, 1.0).unwrap());
    }
    let dummy_profile = InstrumentProfile {
        id: "bypassed".to_string(),
        display_name: "Bypassed".to_string(),
        synthesis_kind: "additive".to_string(),
        gain: 1.0,
        harmonic_profile: too_many,
        envelope_profile: AdsrEnvelope::new(0.01, 0.01, 0.5, 0.01).unwrap(),
        variation_profile: VariationProfile::default(),
        gm_program: None,
        family: "custom".to_string(),
    };

    // InstrumentSignal::new must reject before allocating
    assert!(InstrumentSignal::new(440.0, dummy_profile, 0.1, 1.0, 44100.0, None).is_err());
}

#[test]
fn test_cwe20_signal_value_at_finiteness_guard() {
    let flute = get_instrument("flute_naive").unwrap();
    // Huge pitch cents jitter that would make effective_fundamental_hz non-finite
    let var = VariationProfile::new(1e300, 0.0, 0.0, Some(42), 0.0).unwrap();
    assert!(InstrumentSignal::new(440.0, flute, 0.1, 1.0, 44100.0, Some(var)).is_err());
}
