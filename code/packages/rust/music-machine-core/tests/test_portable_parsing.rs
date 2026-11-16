//! Portable multi-track score parsing and formatting tests (MUS04 / MUS07).

use music_machine_core::{parse_portable_score, render_portable_score_to_pcm, ScoreEventKind};

const SIMPLE_DUET: &str = "\
format: music-machine-score/v2
title: Simple Duet
ppq: 240
sample_rate: 44100
tempo 0 120
meter 0 4/4

instrument flute profile=flute_naive gain=0.8
instrument piano profile=piano_naive gain=0.6

track lead instrument=flute
track backing instrument=piano

event lead 0 240 note C4 velocity=0.8
event backing 0 240 note E4,G4 velocity=0.6
event lead 240 240 note D4 velocity=0.8
event backing 240 240 note F4,A4 velocity=0.6
event lead 480 240 rest
";

#[test]
fn test_parse_simple_duet() {
    let score = parse_portable_score(SIMPLE_DUET).expect("parse duet");
    assert_eq!(score.title, "Simple Duet");
    assert_eq!(score.ppq, 240);
    assert_eq!(score.sample_rate_hz, 44100);
    assert_eq!(score.tempo_events.len(), 1);
    assert_eq!(score.tempo_events[0].bpm, 120.0);
    assert_eq!(score.meter_events.len(), 1);
    assert_eq!(score.meter_events[0].meter, "4/4");
    assert_eq!(score.instruments.len(), 2);
    assert_eq!(score.tracks.len(), 2);
    assert_eq!(score.events.len(), 5);

    // Chords verification
    assert_eq!(score.events[1].notes.len(), 2);
    assert_eq!(score.events[1].notes, vec!["E4", "G4"]);

    // Rest event
    assert_eq!(score.events[4].kind, ScoreEventKind::Rest);

    // Render verification
    let rendered = render_portable_score_to_pcm(&score, 0).expect("render duet");
    assert!(rendered.pcm_buffer.sample_count() > 0);
    assert_eq!(rendered.rendered_notes.len(), 6); // 1 + 2 + 1 + 2 = 6 notes
}

#[test]
fn test_parse_additive_and_drum_instruments() {
    let text = "\
format: music-machine-score/v2
title: Custom Synth & Drums
ppq: 120

instrument synth kind=additive harmonics=1:1.0,2:0.5 envelope=0.01,0.05,0.7,0.1 gain=0.5
instrument drums kind=drumkit kit=standard_kit gain=0.8

track t_synth instrument=synth
track t_drums instrument=drums

event t_synth 0 120 note C4 velocity=0.7
event t_drums 0 120 hit kick velocity=0.9
event t_drums 120 120 hit snare velocity=0.8
";

    let score = parse_portable_score(text).expect("parse additive and drums");
    assert_eq!(score.instruments[0].kind, "additive");
    let harmonics = score.instruments[0].harmonics.as_ref().unwrap();
    assert_eq!(harmonics.len(), 2);
    assert_eq!(harmonics[0].frequency_multiplier, 1.0);
    assert_eq!(harmonics[1].frequency_multiplier, 2.0);

    assert_eq!(score.instruments[1].kind, "drumkit");
    assert_eq!(score.instruments[1].kit.as_deref(), Some("standard_kit"));

    let rendered = render_portable_score_to_pcm(&score, 0).expect("render synth & drums");
    assert!(rendered.pcm_buffer.sample_count() > 0);
    assert_eq!(rendered.rendered_notes.len(), 1);
    assert_eq!(rendered.rendered_hits.len(), 2);
}

#[test]
fn test_to_text_roundtrip() {
    let score = parse_portable_score(SIMPLE_DUET).expect("parse duet");
    let text_repr = score.to_text();
    let re_parsed = parse_portable_score(&text_repr).expect("re-parse serialized text");

    assert_eq!(score.title, re_parsed.title);
    assert_eq!(score.ppq, re_parsed.ppq);
    assert_eq!(score.sample_rate_hz, re_parsed.sample_rate_hz);
    assert_eq!(score.instruments.len(), re_parsed.instruments.len());
    assert_eq!(score.tracks.len(), re_parsed.tracks.len());
    assert_eq!(score.events.len(), re_parsed.events.len());
}

#[test]
fn test_validation_errors() {
    // Missing format header
    assert!(parse_portable_score("title: No Format\n").is_err());
    // Wrong format version
    assert!(parse_portable_score("format: music-machine-score/v1\n").is_err());
    // Duplicate instrument ID
    let dup_inst = "\
format: music-machine-score/v2
instrument lead profile=flute_naive
instrument lead profile=piano_naive
";
    assert!(parse_portable_score(dup_inst).is_err());

    // Duplicate track ID
    let dup_track = "\
format: music-machine-score/v2
instrument lead profile=flute_naive
track mel instrument=lead
track mel instrument=lead
";
    assert!(parse_portable_score(dup_track).is_err());

    // Track references nonexistent instrument
    let bad_track = "\
format: music-machine-score/v2
track mel instrument=nonexistent
";
    assert!(parse_portable_score(bad_track).is_err());

    // Event references nonexistent track
    let bad_event = "\
format: music-machine-score/v2
instrument lead profile=flute_naive
track mel instrument=lead
event nonexistent 0 120 note C4
";
    assert!(parse_portable_score(bad_event).is_err());

    // Note event on drumkit track
    let note_on_drum = "\
format: music-machine-score/v2
instrument drums kind=drumkit kit=standard_kit
track r instrument=drums
event r 0 120 note C4
";
    let score = parse_portable_score(note_on_drum).expect("parsed structure");
    assert!(render_portable_score_to_pcm(&score, 0).is_err());

    // Hit event on melodic track
    let hit_on_flute = "\
format: music-machine-score/v2
instrument flute profile=flute_naive
track m instrument=flute
event m 0 120 hit kick
";
    let score = parse_portable_score(hit_on_flute).expect("parsed structure");
    assert!(render_portable_score_to_pcm(&score, 0).is_err());
}
