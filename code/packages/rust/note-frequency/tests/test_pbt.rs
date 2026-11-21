//! Property-based testing for note-frequency using SplitMix64 PRNG (MUS00, VER15).

use coding_adventures_note_frequency::{
    note_to_frequency, note_to_frequency_with_reference, parse_note, Note, MAX_OCTAVE, MIN_OCTAVE,
};
use coding_adventures_test_bootstrap::prng::SplitMix64;

type Spelling = (&'static str, &'static str);

const VALID_SPELLINGS: [Spelling; 17] = [
    ("C", ""),
    ("C", "#"),
    ("D", "b"),
    ("D", ""),
    ("D", "#"),
    ("E", "b"),
    ("E", ""),
    ("F", ""),
    ("F", "#"),
    ("G", "b"),
    ("G", ""),
    ("G", "#"),
    ("A", "b"),
    ("A", ""),
    ("A", "#"),
    ("B", "b"),
    ("B", ""),
];

const ENHARMONIC_PAIRS: [(Spelling, Spelling); 5] = [
    (("C", "#"), ("D", "b")),
    (("D", "#"), ("E", "b")),
    (("F", "#"), ("G", "b")),
    (("G", "#"), ("A", "b")),
    (("A", "#"), ("B", "b")),
];

#[test]
fn test_pbt_octave_doubling_and_semitone_linearity() {
    let mut rng = SplitMix64::new(0xA440_1234_5678_9ABC);
    let span = (MAX_OCTAVE - MIN_OCTAVE) as u64; // [MIN_OCTAVE, MAX_OCTAVE - 1]
    for _ in 0..250 {
        let (letter, accidental) =
            VALID_SPELLINGS[rng.bounded(VALID_SPELLINGS.len() as u64) as usize];
        let octave = MIN_OCTAVE + rng.bounded(span) as i32;

        let lower = Note::new(letter, accidental, octave).expect("lower note");
        let upper = Note::new(letter, accidental, octave + 1).expect("upper note");

        assert_eq!(upper.semitones_from_a4() - lower.semitones_from_a4(), 12);
        let ratio = upper.frequency() / lower.frequency();
        let rel_err = (ratio - 2.0).abs() / 2.0;
        assert!(rel_err < 1e-12, "octave ratio {ratio} != 2.0");
    }
}

#[test]
fn test_pbt_enharmonic_equivalence_and_roundtrip() {
    let mut rng = SplitMix64::new(0xCAFE_BABE_DEAD_BEEF);
    let octave_count = (MAX_OCTAVE - MIN_OCTAVE + 1) as u64;
    for _ in 0..250 {
        let ((l1, a1), (l2, a2)) =
            ENHARMONIC_PAIRS[rng.bounded(ENHARMONIC_PAIRS.len() as u64) as usize];
        let octave = MIN_OCTAVE + rng.bounded(octave_count) as i32;

        let sharp = Note::new(l1, a1, octave).expect("sharp note");
        let flat = Note::new(l2, a2, octave).expect("flat note");

        assert_eq!(sharp.chromatic_index(), flat.chromatic_index());
        assert_eq!(sharp.semitones_from_a4(), flat.semitones_from_a4());
        let rel_err = (sharp.frequency() - flat.frequency()).abs() / sharp.frequency();
        assert!(rel_err < 1e-12, "enharmonic mismatch: {rel_err}");

        let reparsed = parse_note(&sharp.to_string()).expect("roundtrip parse");
        assert_eq!(reparsed, sharp);
        let lower_hz = note_to_frequency(&sharp.to_string().to_lowercase()).expect("lowercase");
        let lower_err = (lower_hz - sharp.frequency()).abs() / sharp.frequency();
        assert!(lower_err < 1e-12);
    }
}

#[test]
fn test_pbt_reference_pitch_scaling() {
    let mut rng = SplitMix64::new(0x1357_2468_ACE0_BDF1);
    for _ in 0..150 {
        let (letter, accidental) =
            VALID_SPELLINGS[rng.bounded(VALID_SPELLINGS.len() as u64) as usize];
        let octave = -20 + rng.bounded(41) as i32;
        let label = format!("{letter}{accidental}{octave}");
        let a4_hz = 220.0 + ((rng.next_u64() & 0xFFFF) as f64) * (660.0 / 65535.0);

        let f_std = note_to_frequency(&label).expect("std freq");
        let f_alt = note_to_frequency_with_reference(&label, a4_hz).expect("alt freq");
        let ratio = f_alt / f_std;
        let expected = a4_hz / 440.0;
        let rel_err = (ratio - expected).abs() / expected;
        assert!(rel_err < 1e-12, "ratio {ratio} != {expected}");
    }
}

#[test]
fn test_pbt_hostile_random_strings_fail_closed() {
    let mut rng = SplitMix64::new(0xFEED_FACE_0102_0304);
    let alphabet = b"ABCDEFGHabcdefg#b-+0123456789xyz \t\n\x00";
    for _ in 0..500 {
        let len = rng.bounded(48) as usize;
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            bytes.push(alphabet[rng.bounded(alphabet.len() as u64) as usize]);
        }
        let candidate = String::from_utf8_lossy(&bytes);
        if let Ok(note) = parse_note(&candidate) {
            let freq = note.checked_frequency().expect("valid note frequency");
            assert!(freq.is_finite() && freq > 0.0);
            assert!((MIN_OCTAVE..=MAX_OCTAVE).contains(&note.octave));
        }
    }
}
