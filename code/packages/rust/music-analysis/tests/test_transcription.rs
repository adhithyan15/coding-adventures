//! Unit tests for sample-to-score transcription round-trip (MUS08, MUS04).

use coding_adventures_music_analysis::{
    transcribe_pcm_buffer, transcribe_pcm_to_portable_score, transcribe_samples, AnalysisError,
    TranscriptionOptions,
};
use music_machine_core::{
    parse_portable_score, render_portable_score_to_pcm, PORTABLE_FORMAT_VERSION,
};
use pcm_audio::{PcmAudio, PcmFormat};

const TRIAD_SCORE_TEXT: &str = "\
format: music-machine-score/v2
title: Round Trip Triad
ppq: 480
sample_rate: 44100
tempo 0 120
meter 0 4/4

instrument lead profile=sine gain=0.8
track melody instrument=lead

event melody 0 480 note C4 velocity=0.8
event melody 480 480 note E4 velocity=0.8
event melody 960 480 note G4 velocity=0.8
";

#[test]
fn test_transcribe_portable_score_round_trip() {
    let original_score = parse_portable_score(TRIAD_SCORE_TEXT).expect("parse triad score");
    let rendered =
        render_portable_score_to_pcm(&original_score, 0).expect("render score to pcm buffer");

    let opts = TranscriptionOptions {
        bpm: 120.0,
        ppq: 480,
        quantize_grid_beats: 0.25,
        title: "Transcribed Triad".to_string(),
        instrument_id: "sine".to_string(),
        track_id: "melody".to_string(),
    };

    let transcribed =
        transcribe_pcm_buffer(&rendered.pcm_buffer, Some(&opts)).expect("transcribe score");

    assert_eq!(transcribed.format_version, PORTABLE_FORMAT_VERSION);
    assert_eq!(transcribed.title, "Transcribed Triad");
    assert_eq!(transcribed.ppq, 480);
    assert_eq!(transcribed.sample_rate_hz, 44100);

    // Verify notes and ticks match
    assert_eq!(transcribed.events.len(), 3);

    let e0 = &transcribed.events[0];
    assert_eq!(e0.notes, vec!["C4"]);
    assert_eq!(e0.start_tick, 0);
    assert_eq!(e0.duration_tick, 480);

    let e1 = &transcribed.events[1];
    assert_eq!(e1.notes, vec!["E4"]);
    assert_eq!(e1.start_tick, 480);
    assert_eq!(e1.duration_tick, 480);

    let e2 = &transcribed.events[2];
    assert_eq!(e2.notes, vec!["G4"]);
    assert_eq!(e2.start_tick, 960);
    assert_eq!(e2.duration_tick, 480);
}

#[test]
fn test_transcribe_raw_float_sequence() {
    let sr = 44100.0;
    let bpm = 120.0;
    // Create 0.5s of A4 (440 Hz)
    let n = (0.5 * sr) as usize;
    let phase_inc = 2.0 * std::f64::consts::PI * 440.0 / sr;
    let sig: Vec<f64> = (0..n).map(|i| 0.8 * ((i as f64) * phase_inc).sin()).collect();

    let opts = TranscriptionOptions {
        bpm,
        ..Default::default()
    };
    let score = transcribe_samples(&sig, sr, Some(&opts)).expect("transcribe float samples");
    assert_eq!(score.events.len(), 1);
    let event = &score.events[0];
    assert_eq!(event.notes, vec!["A4"]);
    assert_eq!(event.start_tick, 0);
}

#[test]
fn test_transcribe_pcm16_integer_sequence() {
    let sr = 44100.0;
    let bpm = 120.0;
    // Create 0.5s of C4 (261.63 Hz) in 16-bit PCM integer range
    let n = (0.5 * sr) as usize;
    let phase_inc = 2.0 * std::f64::consts::PI * 261.63 / sr;
    let sig: Vec<f64> = (0..n)
        .map(|i| 24000.0 * ((i as f64) * phase_inc).sin())
        .collect();

    let opts = TranscriptionOptions {
        bpm,
        ..Default::default()
    };
    let score = transcribe_samples(&sig, sr, Some(&opts)).expect("transcribe int sequence");
    assert_eq!(score.events.len(), 1);
    assert_eq!(score.events[0].notes, vec!["C4"]);
}

#[test]
fn test_transcribe_silence() {
    let sr = 44100.0;
    let sig = vec![0.0; 8000];
    let score = transcribe_samples(&sig, sr, None).expect("transcribe silence");
    assert!(score.events.is_empty());
}

#[test]
fn test_transcribe_pcm_buffer_metadata() {
    let sr = 22050.0;
    let format = PcmFormat::mono_16bit(sr).expect("pcm format");
    // 0.5s tone
    let n = (0.5 * sr) as usize;
    let phase_inc = 2.0 * std::f64::consts::PI * 440.0 / sr;
    let int_samples: Vec<i16> = (0..n)
        .map(|i| (20000.0 * ((i as f64) * phase_inc).sin()) as i16)
        .collect();
    let buf = PcmAudio::from_i16(int_samples, format, 0.0).expect("pcm audio");

    // When passing PcmAudio buffer, sample_rate_hz is inferred automatically
    let score = transcribe_pcm_buffer(&buf, None).expect("transcribe buffer");
    assert_eq!(score.sample_rate_hz, 22050);
    assert_eq!(score.events.len(), 1);
    assert_eq!(score.events[0].notes, vec!["A4"]);
}

#[test]
fn test_transcribe_invalid_inputs() {
    assert!(matches!(
        transcribe_pcm_to_portable_score(&[1.0][..], None, None),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        transcribe_samples(&[1.0], 500.0, None),
        Err(AnalysisError::InvalidInput(_))
    ));
    let bad_bpm = TranscriptionOptions {
        bpm: 0.0,
        ..Default::default()
    };
    assert!(matches!(
        transcribe_samples(&[1.0], 44100.0, Some(&bad_bpm)),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        transcribe_samples(&[f64::NAN], 44100.0, None),
        Err(AnalysisError::InvalidInput(_))
    ));
}
