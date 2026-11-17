//! Comprehensive JSON deserialization, deterministic serialization, and error testing (MUS04/VER04).

use music_machine_core::{
    parse_portable_score, parse_portable_score_json, MusicMachineError,
    MAX_JSON_DEPTH, MAX_SCORE_BYTES,
};
use std::fs;
use std::path::{Path, PathBuf};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/fixtures/music-machine")
        .join(name)
}

#[test]
fn test_canonical_json_roundtrip() {
    let json_path = fixture_path("canonical_score.json");
    let json_text = fs::read_to_string(&json_path).expect("read canonical_score.json");

    let score = parse_portable_score_json(&json_text).expect("parse canonical JSON");
    assert_eq!(score.title, "Tier 0 Canonical Score");
    assert_eq!(score.ppq, 120);
    assert_eq!(score.sample_rate_hz, 44100);
    assert_eq!(score.tracks.len(), 4);
    assert_eq!(score.instruments.len(), 4);
    assert_eq!(score.events.len(), 12);

    // Serialize to JSON and parse back
    let serialized = score.to_json();
    let re_parsed = parse_portable_score_json(&serialized).expect("parse serialized JSON");

    assert_eq!(score.title, re_parsed.title);
    assert_eq!(score.format_version, re_parsed.format_version);
    assert_eq!(score.ppq, re_parsed.ppq);
    assert_eq!(score.sample_rate_hz, re_parsed.sample_rate_hz);
    assert_eq!(score.tempo_events, re_parsed.tempo_events);
    assert_eq!(score.meter_events, re_parsed.meter_events);
    assert_eq!(score.instruments, re_parsed.instruments);
    assert_eq!(score.tracks, re_parsed.tracks);
    assert_eq!(score.events, re_parsed.events);
}

#[test]
fn test_json_rejects_duplicate_object_members() {
    let json_path = fixture_path("canonical_score.json");
    let json_text = fs::read_to_string(&json_path).expect("read canonical_score.json");
    let score = parse_portable_score_json(&json_text)
        .expect("same member names in distinct objects remain valid");
    assert_eq!(score.title, "Tier 0 Canonical Score");

    let duplicate_outer = json_text.replacen(
        r#""title": "Tier 0 Canonical Score""#,
        r#""title": "Tier 0 Canonical Score", "title": "Tier 0 Canonical Score""#,
        1,
    );
    let duplicate_nested =
        json_text.replacen(r#""id": "lead""#, r#""id": "lead", "id": "lead""#, 1);
    let duplicate_escaped = json_text.replacen(
        r#""title": "Tier 0 Canonical Score""#,
        r#""title": "Tier 0 Canonical Score", "\u0074itle": "Tier 0 Canonical Score""#,
        1,
    );

    for malformed in [duplicate_outer, duplicate_nested, duplicate_escaped] {
        assert!(matches!(
            parse_portable_score_json(&malformed),
            Err(MusicMachineError::ParseError(message))
                if message == "duplicate JSON object member"
        ));
    }
}

#[test]
fn test_additive_and_drumkit_json_roundtrip() {
    let text = "\
format: music-machine-score/v2
title: Custom Additive & Kit
ppq: 120
sample_rate: 48000

instrument synth kind=additive harmonics=1:1.0,2:0.5,3:0.25 envelope=0.01,0.05,0.7,0.1 gain=0.6
instrument kit kind=drumkit kit=standard_kit gain=0.9

track t_synth instrument=synth
track t_drums instrument=kit

event t_synth 0 120 note C4 velocity=0.8
event t_drums 0 120 hit kick velocity=0.95
event t_drums 60 60 hit snare velocity=0.75
event t_synth 120 120 rest
";
    let score = parse_portable_score(text).expect("parse portable score");
    let json_text = score.to_json();
    let re_parsed = parse_portable_score_json(&json_text).expect("parse serialized JSON");

    assert_eq!(score.title, re_parsed.title);
    assert_eq!(score.sample_rate_hz, 48000);
    assert_eq!(score.instruments.len(), 2);
    assert_eq!(score.instruments[0].kind, "additive");
    assert_eq!(score.instruments[0].harmonics.as_ref().unwrap().len(), 3);
    assert_eq!(score.instruments[1].kind, "drumkit");
    assert_eq!(score.instruments[1].kit.as_deref(), Some("standard_kit"));
    assert_eq!(score.events.len(), 4);
    assert_eq!(score.events, re_parsed.events);
}

#[test]
fn test_malformed_json_errors() {
    // Empty text
    assert!(parse_portable_score_json("").is_err());

    // Unclosed object
    assert!(parse_portable_score_json("{\"title\": \"test\"").is_err());

    // Unclosed string
    assert!(parse_portable_score_json("{\"title\": \"test").is_err());

    // Trailing garbage
    assert!(parse_portable_score_json("{\"title\": \"test\"} extra").is_err());

    // Invalid number format
    assert!(parse_portable_score_json("{\"ppq\": 12.34.56}").is_err());

    // Missing colon
    assert!(parse_portable_score_json("{\"key\" \"value\"}").is_err());
}

#[test]
fn test_schema_and_domain_validation_errors() {
    // Non-object root
    assert!(matches!(
        parse_portable_score_json("[1, 2, 3]"),
        Err(MusicMachineError::InvalidArgument(_))
    ));

    // Unknown top-level key (strict schema validation)
    let unknown_key = r#"{
        "format_version": "music-machine-score/v2",
        "ppq": 120,
        "sample_rate_hz": 44100,
        "title": "Bad Key",
        "instruments": [],
        "tracks": [],
        "events": [],
        "tempo_events": [],
        "meter_events": [],
        "unexpected_key": "not allowed"
    }"#;
    assert!(matches!(
        parse_portable_score_json(unknown_key),
        Err(MusicMachineError::InvalidArgument(_))
    ));

    // Missing format_version
    let missing_format = r#"{
        "ppq": 120,
        "sample_rate_hz": 44100,
        "title": "Missing Format",
        "instruments": [],
        "tracks": [],
        "events": []
    }"#;
    assert!(matches!(
        parse_portable_score_json(missing_format),
        Err(MusicMachineError::InvalidArgument(_))
    ));

    // Unsupported format version
    let bad_format = r#"{
        "format_version": "music-machine-score/v99",
        "ppq": 120,
        "sample_rate_hz": 44100,
        "title": "Bad Format",
        "instruments": [],
        "tracks": [],
        "events": []
    }"#;
    assert!(matches!(
        parse_portable_score_json(bad_format),
        Err(MusicMachineError::InvalidArgument(_))
    ));

    // Missing ppq
    let missing_ppq = r#"{
        "format_version": "music-machine-score/v2",
        "sample_rate_hz": 44100,
        "title": "Missing PPQ",
        "instruments": [],
        "tracks": [],
        "events": []
    }"#;
    assert!(matches!(
        parse_portable_score_json(missing_ppq),
        Err(MusicMachineError::InvalidArgument(_))
    ));
}

#[test]
fn test_json_depth_limit_cwe_674() {
    // Generate deeply nested JSON arrays: [[[[...]]]]
    let depth = MAX_JSON_DEPTH + 10;
    let mut payload = String::new();
    for _ in 0..depth {
        payload.push('[');
    }
    payload.push_str("42");
    for _ in 0..depth {
        payload.push(']');
    }

    let res = parse_portable_score_json(&payload);
    assert!(matches!(res, Err(MusicMachineError::BudgetExceeded(_))));
}

#[test]
fn test_json_byte_limit_cwe_400() {
    // Generate oversized JSON exceeding MAX_SCORE_BYTES
    let mut payload = String::from("{\"title\": \"");
    payload.push_str(&"A".repeat(MAX_SCORE_BYTES + 100));
    payload.push_str("\"}");

    let res = parse_portable_score_json(&payload);
    assert!(matches!(res, Err(MusicMachineError::BudgetExceeded(_))));
}
