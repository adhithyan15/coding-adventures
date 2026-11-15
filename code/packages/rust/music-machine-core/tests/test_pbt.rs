//! Property-based testing (PBT) and fuzzing (VER05/VER06).

use coding_adventures_test_bootstrap::fuzz::{bit_flip, byte_flip};
use coding_adventures_test_bootstrap::prng::SplitMix64;
use music_machine_core::{
    parse_portable_score, parse_portable_score_json, parse_score, render_portable_score_to_pcm,
    render_score_to_pcm,
};

#[test]
fn test_pbt_arbitrary_string_fuzz_never_panics() {
    let mut rng = SplitMix64::new(0xFEED_FACE_2026);

    for _ in 0..1000 {
        let len = (rng.next_u64() % 256) as usize;
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            bytes.push((rng.next_u64() % 256) as u8);
        }

        if let Ok(text) = std::str::from_utf8(&bytes) {
            // Must return Ok or Err, never panic
            let _ = parse_score(text);
            let _ = parse_portable_score(text);
            let _ = parse_portable_score_json(text);
        }
    }
}

#[test]
fn test_pbt_mutated_corpus_fuzz_never_panics() {
    let base_text = "\
format: music-machine-score/v2
title: Base Duet
ppq: 120
sample_rate: 44100
tempo 0 120
meter 0 4/4

instrument flute profile=flute_naive gain=0.8
instrument piano profile=piano_naive gain=0.6

track lead instrument=flute
track backing instrument=piano

event lead 0 120 note C4 velocity=0.8
event backing 0 120 note E4,G4 velocity=0.6
event lead 120 120 rest
";

    let mut rng = SplitMix64::new(0xBEEF_CAFE_1337);
    let base_bytes = base_text.as_bytes();

    for _ in 0..500 {
        let bit_offset = (rng.next_u64() % ((base_bytes.len() * 8) as u64)) as usize;
        let count = (rng.next_u64() % 16 + 1) as usize;
        let mutated = bit_flip(base_bytes, bit_offset, count);

        if let Ok(text) = std::str::from_utf8(&mutated) {
            let res = parse_portable_score(text);
            if let Ok(score) = res {
                let _ = render_portable_score_to_pcm(&score, 0);
            }
        }

        let byte_offset = (rng.next_u64() % (base_bytes.len() as u64)) as usize;
        let byte_mutated = byte_flip(base_bytes, byte_offset, (rng.next_u64() % 4 + 1) as usize);
        if let Ok(text) = std::str::from_utf8(&byte_mutated) {
            let res = parse_portable_score(text);
            if let Ok(score) = res {
                let _ = render_portable_score_to_pcm(&score, 0);
            }
        }
    }
}

#[test]
fn test_pbt_render_score_determinism() {
    let mut rng = SplitMix64::new(0xABCD_EF01_2345);
    let note_choices = ["C4", "D4", "E4", "F4", "G4", "A4", "B4", "C5"];
    let dur_choices = ["w", "h", "q", "e", "s"];

    for _ in 0..20 {
        let event_count = (rng.next_u64() % 8 + 1) as usize;
        let mut score_str = String::from("title: PBT Run\ntempo: 140\nmeter: 4/4\namplitude: 0.7\n");
        for _ in 0..event_count {
            let n = note_choices[(rng.next_u64() % note_choices.len() as u64) as usize];
            let d = dur_choices[(rng.next_u64() % dur_choices.len() as u64) as usize];
            score_str.push_str(&format!("{n}/{d} "));
        }

        let score = parse_score(&score_str).expect("generated valid score text");

        let run1 = render_score_to_pcm(&score, 0).expect("render run 1");
        let run2 = render_score_to_pcm(&score, 0).expect("render run 2");

        assert_eq!(run1.pcm_buffer.samples(), run2.pcm_buffer.samples());
        assert_eq!(run1.clipped_sample_count, run2.clipped_sample_count);
        assert_eq!(run1.rendered_notes.len(), run2.rendered_notes.len());
    }
}
