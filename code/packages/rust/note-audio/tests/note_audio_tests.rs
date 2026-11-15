use coding_adventures_test_bootstrap::prng::SplitMix64;
use note_audio::{
    midi_to_frequency, note_to_frequency, render_note, render_note_to_sound_chain,
    render_note_with_limit, validate_frequency, ContinuousSignal, NoteAudioError, NoteEvent,
    NotePitch, Waveform, DEFAULT_AMPLITUDE, DEFAULT_SAMPLE_RATE_HZ, MAX_DURATION_SECONDS,
    MAX_SAMPLE_RATE_HZ,
};
use note_frequency::parse_note;

const EPS: f64 = 1e-6;

#[test]
fn midi_to_frequency_reference_notes() {
    // A4 = MIDI 69 -> 440.0 Hz
    let a4 = midi_to_frequency(69).unwrap();
    assert!((a4 - 440.0).abs() < EPS);

    // C4 (Middle C) = MIDI 60 -> ~261.6256 Hz
    let c4 = midi_to_frequency(60).unwrap();
    assert!((c4 - 261.625565).abs() < 1e-3);

    // A0 = MIDI 21 -> 27.5 Hz
    let a0 = midi_to_frequency(21).unwrap();
    assert!((a0 - 27.5).abs() < EPS);

    // C8 = MIDI 108 -> ~4186.01 Hz
    let c8 = midi_to_frequency(108).unwrap();
    assert!((c8 - 4186.009).abs() < 1e-2);

    // Lowest MIDI 0 and Highest MIDI 127
    assert!(midi_to_frequency(0).is_ok());
    assert!(midi_to_frequency(127).is_ok());
}

#[test]
fn note_to_frequency_names() {
    assert!((note_to_frequency("A4").unwrap() - 440.0).abs() < EPS);
    assert!((note_to_frequency("C4").unwrap() - 261.625565).abs() < 1e-3);
    assert!((note_to_frequency("A#4").unwrap() - 466.163761).abs() < 1e-3);
    assert!((note_to_frequency("Bb4").unwrap() - 466.163761).abs() < 1e-3);
    assert!(note_to_frequency("H4").is_err());
    assert!(note_to_frequency("").is_err());
}

#[test]
fn note_pitch_variants() {
    let p_name = NotePitch::from("A4");
    assert!((p_name.to_frequency().unwrap() - 440.0).abs() < EPS);

    let parsed = parse_note("C4").unwrap();
    let p_parsed = NotePitch::from(parsed);
    assert!((p_parsed.to_frequency().unwrap() - 261.625565).abs() < 1e-3);

    let p_midi = NotePitch::from(69u8);
    assert!((p_midi.to_frequency().unwrap() - 440.0).abs() < EPS);

    let p_freq = NotePitch::from(440.0);
    assert!((p_freq.to_frequency().unwrap() - 440.0).abs() < EPS);
}

#[test]
fn render_a4_keeps_every_layer_visible() {
    let chain = render_note_to_sound_chain("A4", 0.01).unwrap();

    assert_eq!(chain.note_event().pitch(), &NotePitch::from("A4"));
    assert_eq!(chain.frequency_hz(), 440.0);
    assert_eq!(chain.note_event().amplitude(), DEFAULT_AMPLITUDE);
    assert_eq!(
        chain.floating_samples().sample_rate_hz(),
        DEFAULT_SAMPLE_RATE_HZ
    );
    assert_eq!(chain.floating_samples().sample_count(), 441);
    assert_eq!(chain.pcm_audio().sample_count(), 441);
    assert_eq!(chain.pcm_audio().samples()[0], 0);

    // Oscillator is non-zero after phase advance
    let val_first = chain.oscillator().value_at(1.0 / DEFAULT_SAMPLE_RATE_HZ).unwrap();
    assert!(val_first > 0.0);

    // Consuming returns PCM audio
    let pcm = chain.clone().into_pcm_audio();
    assert_eq!(pcm.sample_count(), 441);
}

#[test]
fn render_with_offset_start_time() {
    let event = NoteEvent::new("A4", 0.005)
        .unwrap()
        .with_start_time(10.0)
        .unwrap();
    let chain = render_note(&event, 8000.0).unwrap();

    assert_eq!(chain.floating_samples().start_time_seconds(), 10.0);
    assert_eq!(chain.pcm_audio().start_time_seconds(), 10.0);
    assert_eq!(chain.floating_samples().sample_count(), 40);
}

#[test]
fn zero_duration_produces_empty_valid_chain() {
    let chain = render_note_to_sound_chain("A4", 0.0).unwrap();
    assert_eq!(chain.floating_samples().sample_count(), 0);
    assert_eq!(chain.pcm_audio().sample_count(), 0);
}

#[test]
fn all_waveforms_render_successfully() {
    let waveforms = [
        Waveform::Sine,
        Waveform::Square,
        Waveform::Pulse(0.25),
        Waveform::Sawtooth,
        Waveform::Triangle,
    ];

    for wf in waveforms {
        let event = NoteEvent::new(440.0, 0.01)
            .unwrap()
            .with_waveform(wf);
        let chain = render_note(&event, 8000.0).unwrap();
        assert_eq!(chain.pcm_audio().sample_count(), 80);
        assert_eq!(chain.note_event().waveform(), wf);
    }
}

#[test]
fn nyquist_violation_rejected() {
    // 440 Hz with 800 Hz sample rate: Nyquist is 400 Hz -> must fail
    let event = NoteEvent::new(440.0, 0.1).unwrap();
    let res = render_note(&event, 800.0);
    assert!(matches!(
        res,
        Err(NoteAudioError::NyquistViolation {
            frequency_hz,
            nyquist_hz
        }) if (frequency_hz - 440.0).abs() < EPS && (nyquist_hz - 400.0).abs() < EPS
    ));

    // Exactly at Nyquist (frequency == rate / 2) must also fail
    let res_exact = render_note(&event, 880.0);
    assert!(matches!(res_exact, Err(NoteAudioError::NyquistViolation { .. })));

    // Strictly below Nyquist (rate = 881 Hz) succeeds
    let res_ok = render_note(&event, 881.0);
    assert!(res_ok.is_ok());
}

#[test]
fn budget_limit_exceeded() {
    let event = NoteEvent::new("A4", 1.0).unwrap();
    let res = render_note_with_limit(&event, 8000.0, 100);
    assert!(matches!(
        res,
        Err(NoteAudioError::SampleBudgetExceeded {
            requested: 8000,
            limit: 100
        })
    ));
}

#[test]
fn hostile_inputs_rejected() {
    // Non-finite duration
    assert!(NoteEvent::new("A4", f64::NAN).is_err());
    assert!(NoteEvent::new("A4", f64::INFINITY).is_err());
    assert!(NoteEvent::new("A4", -0.5).is_err());
    assert!(NoteEvent::new("A4", MAX_DURATION_SECONDS + 1.0).is_err());

    // Non-finite amplitude
    let ev = NoteEvent::new("A4", 0.1).unwrap();
    assert!(ev.clone().with_amplitude(f64::NAN).is_err());
    assert!(ev.clone().with_amplitude(-1.0).is_err());

    // Non-finite start time
    assert!(ev.clone().with_start_time(f64::NAN).is_err());
    assert!(ev.clone().with_start_time(f64::INFINITY).is_err());

    // Non-finite or non-positive frequency
    assert!(validate_frequency(f64::NAN).is_err());
    assert!(validate_frequency(f64::INFINITY).is_err());
    assert!(validate_frequency(0.0).is_err());
    assert!(validate_frequency(-100.0).is_err());

    // Invalid note name
    assert!(NoteEvent::new("Z9", 0.1).is_err());
    assert!(NoteEvent::new("", 0.1).is_err());

    // Hostile sample rates
    assert!(render_note(&ev, f64::NAN).is_err());
    assert!(render_note(&ev, 0.0).is_err());
    assert!(render_note(&ev, -8000.0).is_err());
    assert!(render_note(&ev, MAX_SAMPLE_RATE_HZ + 1.0).is_err());
}

#[test]
fn pbt_deterministic_random_notes_never_panic() {
    let mut rng = SplitMix64::new(0x4E07_0001);
    let midi_notes = [21, 36, 48, 60, 69, 72, 84, 96, 108];

    for _ in 0..100 {
        let midi = midi_notes[rng.bounded(midi_notes.len() as u64) as usize];
        let duration = (rng.bounded(100) as f64 + 1.0) / 1000.0; // 0.001..0.1s
        let sample_rate = [8000.0, 16000.0, 22050.0, 44100.0, 48000.0][rng.bounded(5) as usize];
        let amp = (rng.bounded(100) as f64) / 100.0;

        let event = NoteEvent::new(midi, duration)
            .unwrap()
            .with_amplitude(amp)
            .unwrap();

        let chain = render_note(&event, sample_rate).unwrap();
        assert_eq!(chain.frequency_hz(), midi_to_frequency(midi).unwrap());
        assert!(!chain.pcm_audio().samples().is_empty());
    }
}

#[test]
fn error_display() {
    let e = NoteAudioError::InvalidNote("test".to_string());
    assert!(e.to_string().contains("invalid note: test"));
    let _: &dyn std::error::Error = &e;
}
