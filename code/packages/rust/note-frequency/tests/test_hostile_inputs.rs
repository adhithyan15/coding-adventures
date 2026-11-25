//! Hostile input testing for CWE-400, CWE-190, and CWE-20 (VER15).

use coding_adventures_note_frequency::{
    note_to_frequency, note_to_frequency_with_reference, parse_note, validate_frequency,
    validate_reference_frequency, Note, MAX_FREQUENCY_HZ, MAX_NOTE_STRING_LENGTH, MAX_OCTAVE,
    MAX_REFERENCE_FREQUENCY_HZ, MIN_FREQUENCY_HZ, MIN_OCTAVE, MIN_REFERENCE_FREQUENCY_HZ,
};

#[test]
fn test_hostile_cwe_400_input_length_cap() {
    for len in [MAX_NOTE_STRING_LENGTH + 1, 1024, 100_000] {
        let mut huge = String::with_capacity(len);
        huge.push('A');
        for _ in 0..(len - 2) {
            huge.push('0');
        }
        huge.push('4');
        assert!(
            parse_note(&huge).is_err(),
            "oversized input of length {len} must fail"
        );
        assert!(
            note_to_frequency(&huge).is_err(),
            "oversized input of length {len} must fail"
        );
    }

    // Exact MAX_NOTE_STRING_LENGTH = 32 ("C#" + 29 zeros + "4") must succeed
    let boundary = format!("C#{}4", "0".repeat(MAX_NOTE_STRING_LENGTH - 3));
    assert_eq!(boundary.len(), MAX_NOTE_STRING_LENGTH);
    let parsed = parse_note(&boundary).expect("exact boundary length valid note");
    assert_eq!(parsed.letter, "C");
    assert_eq!(parsed.accidental, "#");
    assert_eq!(parsed.octave, 4);
}

#[test]
fn test_hostile_cwe_190_octave_bounds_and_no_overflow() {
    for bad_oct in [MIN_OCTAVE - 1, MAX_OCTAVE + 1, i32::MIN, i32::MAX, 10_000, -10_000] {
        assert!(
            Note::new("A", "", bad_oct).is_err(),
            "out-of-bounds octave {bad_oct} must fail"
        );
    }

    for bad_text in [
        format!("A{}", MAX_OCTAVE + 1),
        format!("A{}", MIN_OCTAVE - 1),
        "A2147483648".to_string(),
        "A-2147483649".to_string(),
        "A999999999".to_string(),
        "A-999999999".to_string(),
    ] {
        assert!(
            parse_note(&bad_text).is_err(),
            "out-of-bounds note {bad_text:?} must fail"
        );
    }

    // Manually constructed struct literals with extreme i32 octaves must not panic/overflow
    let max_lit = Note {
        letter: "A".to_string(),
        accidental: String::new(),
        octave: i32::MAX,
    };
    assert_eq!(max_lit.semitones_from_a4(), i32::MAX);
    assert!(max_lit.checked_semitones_from_a4().is_err());
    assert!(max_lit.checked_frequency().is_err());

    let min_lit = Note {
        letter: "A".to_string(),
        accidental: String::new(),
        octave: i32::MIN,
    };
    assert_eq!(min_lit.semitones_from_a4(), i32::MIN);
    assert!(min_lit.checked_semitones_from_a4().is_err());
    assert!(min_lit.checked_frequency().is_err());

    // Valid octave boundaries produce finite positive frequencies
    let low = parse_note(&format!("C{MIN_OCTAVE}")).expect("MIN_OCTAVE");
    let high = parse_note(&format!("B{MAX_OCTAVE}")).expect("MAX_OCTAVE");
    let low_hz = low.checked_frequency().expect("low finite");
    let high_hz = high.checked_frequency().expect("high finite");
    assert!(low_hz.is_finite() && low_hz > 0.0);
    assert!(high_hz.is_finite() && high_hz > 0.0);
    assert!(low_hz < high_hz);
}

#[test]
fn test_hostile_cwe_20_non_finite_and_out_of_range_frequencies() {
    for bad in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        -0.0,
        -440.0,
        MIN_FREQUENCY_HZ / 10.0,
        MAX_FREQUENCY_HZ * 10.0,
    ] {
        assert!(
            validate_frequency(bad).is_err(),
            "validate_frequency({bad}) must fail"
        );
    }

    for bad_ref in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        -440.0,
        MIN_REFERENCE_FREQUENCY_HZ - 0.1,
        MAX_REFERENCE_FREQUENCY_HZ + 1.0,
    ] {
        assert!(
            validate_reference_frequency(bad_ref).is_err(),
            "validate_reference_frequency({bad_ref}) must fail"
        );
        assert!(
            note_to_frequency_with_reference("A4", bad_ref).is_err(),
            "note_to_frequency_with_reference(A4, {bad_ref}) must fail"
        );
    }
}

#[test]
fn test_hostile_cwe_20_malformed_and_unicode_inputs() {
    for bad in [
        "",
        "A",
        "A-",
        "A#-",
        "A+4",
        " A4",
        "A4 ",
        "A4\n",
        "A\x004",
        "A٤",
        "İ4",
        "ß4",
        "C##4",
        "Dbb3",
        "E#4",
        "B#4",
        "Cb4",
        "Fb4",
        "H4",
    ] {
        assert!(parse_note(bad).is_err(), "malformed {bad:?} must fail");
    }

    for (letter, accidental) in [
        ("", ""),
        ("C#", ""),
        ("AA", ""),
        ("İ", ""),
        ("A", "##"),
        ("A", "x"),
        ("H", ""),
        ("E", "#"),
        ("B", "#"),
        ("C", "b"),
        ("F", "b"),
    ] {
        assert!(
            Note::new(letter, accidental, 4).is_err(),
            "Note::new({letter:?}, {accidental:?}, 4) must fail"
        );
    }

    // Manually constructed invalid spelling must not panic on chromatic_index()
    let bad_lit = Note {
        letter: "Z".to_string(),
        accidental: "##".to_string(),
        octave: 4,
    };
    assert_eq!(bad_lit.chromatic_index(), 0);
    assert!(bad_lit.checked_chromatic_index().is_none());
    assert!(bad_lit.checked_semitones_from_a4().is_err());

    // CWE-400: oversized letter/accidental payloads in Note::new or struct literals
    // must not amplify into multi-KB/MB error strings
    let huge_letter = "A".repeat(100_000);
    let err_letter = Note::new(&huge_letter, "", 4).expect_err("oversized letter must fail");
    assert!(
        err_letter.len() < 256,
        "error message amplified to {} bytes",
        err_letter.len()
    );

    let huge_accidental = "#".repeat(100_000);
    let err_acc = Note::new("A", &huge_accidental, 4).expect_err("oversized accidental must fail");
    assert!(
        err_acc.len() < 256,
        "error message amplified to {} bytes",
        err_acc.len()
    );

    let huge_lit = Note {
        letter: huge_letter,
        accidental: String::new(),
        octave: 4,
    };
    let err_lit = huge_lit
        .checked_semitones_from_a4()
        .expect_err("oversized struct literal must fail");
    assert!(
        err_lit.len() < 256,
        "error message amplified to {} bytes",
        err_lit.len()
    );
}
