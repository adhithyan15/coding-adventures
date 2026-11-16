//! Golden parity checks against the Python reference implementation.

use musical_instruments::{
    all_gm_programs, all_instruments, get_gm_program, get_instrument, instrument_for_gm_program,
    parse_note, render_instrument_note, AdsrEnvelope, RenderOptions, VariationProfile,
};

#[test]
fn test_all_21_presets_present_and_valid() {
    let instruments = all_instruments();
    assert_eq!(instruments.len(), 21);

    let expected_ids = [
        "sine",
        "silence",
        "flute_naive",
        "clarinet_naive",
        "violin_naive",
        "piano_naive",
        "pluck_naive",
        "brass_naive",
        "organ_naive",
        "mallet_naive",
        "celesta_naive",
        "glockenspiel_naive",
        "vibraphone_naive",
        "marimba_naive",
        "xylophone_naive",
        "tubular_bells_naive",
        "timpani_naive",
        "kalimba_naive",
        "synth_lead_naive",
        "synth_pad_naive",
        "effect_naive",
    ];

    for id in &expected_ids {
        let inst = get_instrument(id).unwrap();
        assert_eq!(inst.id, *id);
        assert!(!inst.display_name.is_empty());
        assert!(inst.gain >= 0.0 && inst.gain <= 1.0);
        if inst.synthesis_kind != "silence" {
            assert!(!inst.harmonic_profile.is_empty());
        }
    }
}

#[test]
fn test_gm_catalog_parity() {
    let programs = all_gm_programs();
    assert_eq!(programs.len(), 128);

    assert_eq!(programs[0].program, 1);
    assert_eq!(programs[0].name, "Acoustic Grand Piano");
    assert_eq!(programs[0].family, "piano");
    assert_eq!(programs[0].instrument_id, "piano_naive");

    assert_eq!(programs[73].program, 74);
    assert_eq!(programs[73].name, "Flute");
    assert_eq!(programs[73].family, "pipe");
    assert_eq!(programs[73].instrument_id, "flute_naive");

    assert_eq!(programs[127].program, 128);
    assert_eq!(programs[127].name, "Gunshot");
    assert_eq!(programs[127].family, "sound effects");
    assert_eq!(programs[127].instrument_id, "effect_naive");

    assert_eq!(get_gm_program(41).unwrap().name, "Violin");
    assert_eq!(instrument_for_gm_program(41).unwrap().id, "violin_naive");
}

#[test]
fn test_adsr_envelope_golden_stages() {
    let envelope = AdsrEnvelope::new(0.1, 0.1, 0.4, 0.2).unwrap();

    let tol = 1e-12;
    assert!((envelope.value_at(0.0, 1.0).unwrap() - 0.0).abs() < tol);
    assert!((envelope.value_at(0.05, 1.0).unwrap() - 0.5).abs() < tol);
    assert!((envelope.value_at(0.1, 1.0).unwrap() - 1.0).abs() < tol);
    assert!((envelope.value_at(0.2, 1.0).unwrap() - 0.4).abs() < tol);
    assert!((envelope.value_at(0.8, 1.0).unwrap() - 0.4).abs() < tol);
    assert!((envelope.value_at(1.1, 1.0).unwrap() - 0.2).abs() < tol);
    assert!((envelope.value_at(1.2, 1.0).unwrap() - 0.0).abs() < tol);
}

#[test]
fn test_note_parsing_and_frequency() {
    let a4 = parse_note("A4").unwrap();
    assert!((a4.frequency() - 440.0).abs() < 1e-9);

    let c4 = parse_note("C4").unwrap();
    assert!((c4.frequency() - 261.6255653005986).abs() < 1e-9);

    let a0 = parse_note("A0").unwrap();
    assert!((a0.frequency() - 27.5).abs() < 1e-9);
}

#[test]
fn test_golden_flute_render_parity() {
    let opts = RenderOptions::new()
        .instrument_id("flute_naive")
        .sample_rate_hz(8000.0)
        .amplitude(0.25);

    let rendered = render_instrument_note("A4", 0.01, &opts).unwrap();

    assert_eq!(rendered.floating_samples.sample_count(), 720);
    assert_eq!(rendered.pcm_buffer.sample_count(), 720);

    let expected_floats = [
        0.0,
        0.0001406552305823993,
        0.0005004458099154665,
        0.0009352695660518945,
        0.0013054222550062105,
    ];
    let expected_pcm = [0, 5, 16, 31, 43];

    for i in 0..5 {
        let actual = rendered.floating_samples.samples()[i];
        let diff = (actual - expected_floats[i]).abs();
        assert!(
            diff < 1e-12,
            "float sample {i} mismatch: got {actual}, expected {}",
            expected_floats[i]
        );
        assert_eq!(
            rendered.pcm_buffer.samples()[i],
            expected_pcm[i],
            "pcm sample {i} mismatch"
        );
    }
}

#[test]
fn test_golden_variation_violin_render_parity() {
    let var = VariationProfile::new(15.0, 0.2, 5.0, Some(424242), 0.1).unwrap();
    let opts = RenderOptions::new()
        .instrument_id("violin_naive")
        .sample_rate_hz(16000.0)
        .variation_profile(var);

    let rendered = render_instrument_note("C4", 0.05, &opts).unwrap();

    assert_eq!(rendered.floating_samples.sample_count(), 2720);
    assert_eq!(rendered.pcm_buffer.sample_count(), 2720);

    let expected_floats = [
        0.0,
        0.00011990929119070181,
        0.0004563997455449683,
        0.0009455230941164347,
        0.0014983460312601403,
    ];
    let expected_pcm = [0, 4, 15, 31, 49];

    for i in 0..5 {
        let actual = rendered.floating_samples.samples()[i];
        let diff = (actual - expected_floats[i]).abs();
        assert!(
            diff < 1e-12,
            "float sample {i} mismatch: got {actual}, expected {}",
            expected_floats[i]
        );
        assert_eq!(
            rendered.pcm_buffer.samples()[i],
            expected_pcm[i],
            "pcm sample {i} mismatch"
        );
    }
}
