//! Hostile input and resource exhaustion tests (CWE-400, CWE-190, CWE-20, CWE-674, CWE-1333).

use music_machine_core::{
    parse_portable_score, parse_portable_score_file, parse_portable_score_json, parse_score,
    parse_score_file, validate_finite, validate_identifier, validate_meter,
    validate_unit_float, MusicMachineError, PortableScoreEvent, ScoreEventKind,
    MAX_EVENTS, MAX_INSTRUMENTS, MAX_JSON_DEPTH, MAX_LINES, MAX_LINE_LENGTH, MAX_PARTIALS,
    MAX_SCORE_BYTES, MAX_TRACKS,
};

#[test]
fn test_oversized_score_bytes_cwe_400() {
    // Single-line score parser byte limit
    let huge_line = "C4/q ".repeat((MAX_SCORE_BYTES / 5) + 10);
    assert!(huge_line.len() > MAX_SCORE_BYTES);
    let err = parse_score(&huge_line).expect_err("should reject oversized bytes");
    assert!(matches!(err, MusicMachineError::BudgetExceeded(_)));

    // Portable score parser byte limit
    let mut huge_portable = String::from("format: music-machine-score/v2\n");
    huge_portable.push_str(&"   \n".repeat((MAX_SCORE_BYTES / 4) + 10));
    let err = parse_portable_score(&huge_portable).expect_err("should reject oversized portable text");
    assert!(matches!(err, MusicMachineError::BudgetExceeded(_)));
}

#[test]
fn test_oversized_line_length_cwe_400() {
    let long_line = "A".repeat(MAX_LINE_LENGTH + 1);
    let err = parse_score(&long_line).expect_err("should reject oversized line length");
    assert!(matches!(err, MusicMachineError::BudgetExceeded(_)));

    let portable_long_line = format!("format: music-machine-score/v2\ntitle: {}\n", "A".repeat(MAX_LINE_LENGTH + 1));
    let err = parse_portable_score(&portable_long_line).expect_err("should reject oversized portable line");
    assert!(matches!(err, MusicMachineError::BudgetExceeded(_)));
}

#[test]
fn test_max_events_cwe_400() {
    // Single line score with > MAX_EVENTS tokens
    let tokens = "C4/q ".repeat(MAX_EVENTS + 1);
    let err = parse_score(&tokens).expect_err("should reject too many events");
    assert!(matches!(err, MusicMachineError::BudgetExceeded(_)));

    // Portable score with > MAX_EVENTS events
    let mut port = String::from(
        "format: music-machine-score/v2\n\
         instrument lead profile=flute_naive\n\
         track mel instrument=lead\n",
    );
    for i in 0..=(MAX_EVENTS + 1) {
        port.push_str(&format!("event mel {i} 1 note C4\n"));
    }
    let err = parse_portable_score(&port).expect_err("should reject too many events");
    assert!(matches!(err, MusicMachineError::BudgetExceeded(_)));
}

#[test]
fn test_max_instruments_cwe_400() {
    let mut port = String::from("format: music-machine-score/v2\n");
    for i in 0..=(MAX_INSTRUMENTS + 1) {
        port.push_str(&format!("instrument inst_{i} profile=flute_naive\n"));
    }
    let err = parse_portable_score(&port).expect_err("should reject too many instruments");
    assert!(matches!(err, MusicMachineError::BudgetExceeded(_)));
}

#[test]
fn test_max_tracks_cwe_400() {
    let mut port = String::from(
        "format: music-machine-score/v2\n\
         instrument lead profile=flute_naive\n",
    );
    for i in 0..=(MAX_TRACKS + 1) {
        port.push_str(&format!("track trk_{i} instrument=lead\n"));
    }
    let err = parse_portable_score(&port).expect_err("should reject too many tracks");
    assert!(matches!(err, MusicMachineError::BudgetExceeded(_)));
}

#[test]
fn test_max_partials_cwe_400() {
    let mut harmonics = Vec::new();
    for i in 1..=(MAX_PARTIALS + 2) {
        harmonics.push(format!("{i}:0.1"));
    }
    let port = format!(
        "format: music-machine-score/v2\n\
         instrument huge kind=additive harmonics={}\n",
        harmonics.join(",")
    );
    let err = parse_portable_score(&port).expect_err("should reject too many partials");
    assert!(matches!(err, MusicMachineError::BudgetExceeded(_)));
}

#[test]
fn test_non_finite_floats_cwe_20() {
    assert!(validate_finite("nan", f64::NAN).is_err());
    assert!(validate_finite("inf", f64::INFINITY).is_err());
    assert!(validate_finite("neg_inf", f64::NEG_INFINITY).is_err());

    assert!(validate_unit_float("gain", f64::NAN).is_err());
    assert!(validate_unit_float("gain", -0.01).is_err());
    assert!(validate_unit_float("gain", 1.01).is_err());
    assert!(validate_unit_float("gain", 0.0).is_ok());
    assert!(validate_unit_float("gain", 1.0).is_ok());
}

#[test]
fn test_identifier_validation_cwe_20_cwe_1333() {
    // Valid identifiers
    assert!(validate_identifier("test", "lead").is_ok());
    assert!(validate_identifier("test", "track_1").is_ok());
    assert!(validate_identifier("test", "_hidden").is_ok());
    assert!(validate_identifier("test", "my-track-name").is_ok());

    // Invalid identifiers
    assert!(validate_identifier("test", "").is_err());
    assert!(validate_identifier("test", "1track").is_err());
    assert!(validate_identifier("test", "track with spaces").is_err());
    assert!(validate_identifier("test", "track/slash").is_err());
    assert!(validate_identifier("test", "track:colon").is_err());
    assert!(validate_identifier("test", "track$special").is_err());
    assert!(validate_identifier("test", "track\nnewline").is_err());
}

#[test]
fn test_meter_validation_cwe_20_cwe_1333() {
    // Valid meters
    assert!(validate_meter("4/4").is_ok());
    assert!(validate_meter("3/4").is_ok());
    assert!(validate_meter("6/8").is_ok());
    assert!(validate_meter("12/8").is_ok());
    assert!(validate_meter("1/4").is_ok());

    // Invalid meters
    assert!(validate_meter("").is_err());
    assert!(validate_meter("0/4").is_err());
    assert!(validate_meter("4/0").is_err());
    assert!(validate_meter("-4/4").is_err());
    assert!(validate_meter("4/-4").is_err());
    assert!(validate_meter("4/4/4").is_err());
    assert!(validate_meter("abc").is_err());
    assert!(validate_meter("4").is_err());
    assert!(validate_meter("04/4").is_err());
}

#[test]
fn test_invalid_notes_and_durations_cwe_20() {
    // Invalid note token
    assert!(parse_score("Z4/q").is_err());
    assert!(parse_score("C99/q").is_err());
    assert!(parse_score("H3/q").is_err());

    // Invalid duration token
    assert!(parse_score("C4/x").is_err());
    assert!(parse_score("C4/123").is_err());
    assert!(parse_score("C4/").is_err());

    // Unrecognized directive
    assert!(parse_score("unknown_directive: 123\nC4/q").is_err());
}

#[test]
fn test_json_hostile_nesting_cwe_674() {
    // Depth limit of 32
    let mut deep_object = String::new();
    for i in 0..(MAX_JSON_DEPTH + 5) {
        deep_object.push_str(&format!("{{\"nest_{i}\": "));
    }
    deep_object.push_str("42");
    for _ in 0..(MAX_JSON_DEPTH + 5) {
        deep_object.push('}');
    }

    let err = parse_portable_score_json(&deep_object).expect_err("should reject excessive JSON depth");
    assert!(matches!(err, MusicMachineError::BudgetExceeded(_)));
}

#[test]
fn test_zero_division_and_overflow_guards_cwe_190() {
    // Zero tempo rejection
    assert!(parse_score("tempo: 0\nC4/q").is_err());
    assert!(parse_score("tempo: -120\nC4/q").is_err());

    // Zero sample rate
    assert!(parse_score("sample_rate: 0\nC4/q").is_err());

    // Extreme PPQ
    let text = "format: music-machine-score/v2\nppq: 100000000\n";
    assert!(parse_portable_score(text).is_err());
}

#[test]
fn test_max_lines_cwe_400() {
    let mut text = String::from("format: music-machine-score/v2\n");
    for _ in 0..=(MAX_LINES + 5) {
        text.push('\n');
    }
    let err = parse_portable_score(&text).expect_err("should reject too many lines");
    assert!(matches!(err, MusicMachineError::BudgetExceeded(_)));
}

#[test]
fn test_tick_addition_overflow_cwe_190() {
    // 1. Direct construction of PortableScoreEvent with overflow
    let err = PortableScoreEvent::new(
        "track1",
        u64::MAX,
        1,
        ScoreEventKind::Note,
        vec!["C4".to_string()],
        1.0,
        0,
        None,
    )
    .expect_err("should reject u64 overflow in PortableScoreEvent");
    assert!(matches!(err, MusicMachineError::BudgetExceeded(_)));

    // 2. Parsed portable score with tick range overflow
    let text = "\
format: music-machine-score/v2
instrument lead profile=flute_naive
track mel instrument=lead
event mel 18446744073709551615 1 note C4
";
    let err = parse_portable_score(text).expect_err("should reject u64 overflow during score parse");
    assert!(matches!(err, MusicMachineError::BudgetExceeded(_)));
}

#[test]
fn test_path_traversal_rejection_cwe_22() {
    // Parent directory traversal
    let err1 = parse_score_file("../test.score").expect_err("should reject parent directory traversal");
    assert!(matches!(err1, MusicMachineError::InvalidArgument(_)));

    let err2 = parse_portable_score_file("foo/../bar.score").expect_err("should reject .. component");
    assert!(matches!(err2, MusicMachineError::InvalidArgument(_)));

    // Empty path
    let err3 = parse_score_file("").expect_err("should reject empty path");
    assert!(matches!(err3, MusicMachineError::InvalidArgument(_)));
}
