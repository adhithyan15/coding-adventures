//! V1 beginner score parsing and rendering tests (MUS01 / MUS03).

use music_machine_core::{
    parse_score, render_score_to_pcm, HAPPY_BIRTHDAY_TEXT,
    ScoreEventKind,
};

#[test]
fn test_parse_happy_birthday_fixture() {
    let score = parse_score(HAPPY_BIRTHDAY_TEXT).expect("parse Happy Birthday score");
    assert_eq!(score.tempo_bpm, 120.0);
    assert_eq!(score.meter, "3/4");
    assert_eq!(score.sample_rate_hz, 44100);
    assert_eq!(score.instrument_id, "sine");
    assert!((score.amplitude - 0.18).abs() < 1e-6);

    // 28 musical events in Happy Birthday fixture (25 notes + 3 rests)
    assert_eq!(score.events.len(), 28);

    // Total beats and duration
    let beats = score.total_duration_beats();
    assert!(beats > 0.0);
    let dur = score.total_duration_seconds();
    assert!(dur > 0.0);

    // Render to PCM
    let rendered = render_score_to_pcm(&score, 0).expect("render Happy Birthday score");
    assert_eq!(rendered.pcm_buffer.format().sample_rate_hz(), 44100.0);
    assert!(rendered.pcm_buffer.sample_count() > 0);
    assert_eq!(rendered.rendered_notes.len(), 25);
}

#[test]
fn test_parse_score_directives() {
    let text = "tempo: 140\nmeter: 4/4\ninstrument: flute_naive\namplitude: 0.7\nsample_rate: 22050\nC4/q D4/e E4/e\n";
    let score = parse_score(text).expect("parse custom directives");
    assert_eq!(score.tempo_bpm, 140.0);
    assert_eq!(score.meter, "4/4");
    assert_eq!(score.instrument_id, "flute_naive");
    assert!((score.amplitude - 0.7).abs() < 1e-6);
    assert_eq!(score.sample_rate_hz, 22050);
    assert_eq!(score.events.len(), 3);
}

#[test]
fn test_parse_rests_and_dotted_durations() {
    let text = "tempo: 120\nC4/q. r/e D4/h r/w\n";
    let score = parse_score(text).expect("parse rests and dots");
    assert_eq!(score.events.len(), 4);

    assert_eq!(score.events[0].kind, ScoreEventKind::Note);
    assert_eq!(score.events[0].beat_count, 1.5); // dotted quarter = 1.0 * 1.5 = 1.5

    assert_eq!(score.events[1].kind, ScoreEventKind::Rest);
    assert_eq!(score.events[1].beat_count, 0.5); // eighth = 0.5

    assert_eq!(score.events[2].kind, ScoreEventKind::Note);
    assert_eq!(score.events[2].beat_count, 2.0); // half = 2.0

    assert_eq!(score.events[3].kind, ScoreEventKind::Rest);
    assert_eq!(score.events[3].beat_count, 4.0); // whole = 4.0
}

#[test]
fn test_parse_drumkit_and_hits() {
    let text = "drumkit: standard_kit\nkick/q snare/e r/e\n";
    let score = parse_score(text).expect("parse drumkit");
    assert_eq!(score.drumkit.as_deref(), Some("standard_kit"));
    assert_eq!(score.events.len(), 3);
    assert_eq!(score.events[0].kind, ScoreEventKind::Hit);
    assert_eq!(score.events[0].hit_id.as_deref(), Some("kick"));
    assert_eq!(score.events[1].kind, ScoreEventKind::Hit);
    assert_eq!(score.events[1].hit_id.as_deref(), Some("snare"));
    assert_eq!(score.events[2].kind, ScoreEventKind::Rest);

    let rendered = render_score_to_pcm(&score, 0).expect("render drum hits");
    assert!(rendered.pcm_buffer.sample_count() > 0);
    assert_eq!(rendered.rendered_hits.len(), 2);
}

#[test]
fn test_score_parser_invalid_tokens() {
    // Unknown directive
    assert!(parse_score("foo: bar\nC4/q\n").is_err());
    // Invalid tempo
    assert!(parse_score("tempo: -120\nC4/q\n").is_err());
    assert!(parse_score("tempo: abc\nC4/q\n").is_err());
    // Invalid meter
    assert!(parse_score("meter: 0/4\nC4/q\n").is_err());
    assert!(parse_score("meter: 4\nC4/q\n").is_err());
    // Invalid amplitude
    assert!(parse_score("amplitude: 1.5\nC4/q\n").is_err());
    // Invalid note token
    assert!(parse_score("H4/q\n").is_err());
    assert!(parse_score("C4/x\n").is_err());
    // Multiple dots
    assert!(parse_score("C4/q..\n").is_err());
}
