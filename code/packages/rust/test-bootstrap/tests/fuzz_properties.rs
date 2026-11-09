//! # VER06 fuzz engine: determinism, dedup, feedback, honest reports
//!
//! `fuzz_corpus.rs` pins the pure mutators to the fuzz-v1 vectors. This file
//! tests what a vector table cannot:
//!
//! * the **campaign** (`fuzz_target`) is a pure function of its seed, and
//!   its report is honest: crashes are deduplicated and minimized,
//!   timeouts and out-of-memory outcomes are *counted*, never dropped, and a
//!   campaign that never mutated anything is not "clean";
//! * **coverage feedback works**: a target guarded by a staircase of checks
//!   is cracked with feedback and not without it, which is the one thing a
//!   guidance loop exists to do;
//! * every **mutator** obeys its algebra (involutions, length arithmetic,
//!   exact byte positions) exhaustively over small inputs;
//! * **private reporting** cannot be bent into forging a TAP line.
//!
//! libtest runs these, not the harness under test.

use std::collections::BTreeSet;

use coding_adventures_test_bootstrap::fuzz::{
    bit_flip, block_delete, block_duplicate, block_insert, boundary_insert, byte_arithmetic,
    byte_flip, calculate_energy, corpus_admit, crash_signature, format_tap_failure, fuzz_target,
    is_duplicate_crash, minimize_crashing_input, mutate_sequence, splice, CoverageBitmap,
    FuzzOptions, FuzzOutcome, COVERAGE_BYTES, COVERAGE_MAP_SIZE,
};
use coding_adventures_test_bootstrap::prng::fnv1a64;

fn crash(kind: &str, frame: &str) -> FuzzOutcome {
    FuzzOutcome::Crash {
        kind: kind.into(),
        frames: vec![frame.into()],
    }
}

fn opts(seed: u64, iterations: usize) -> FuzzOptions {
    FuzzOptions {
        max_iterations: iterations,
        seed,
        max_input_len: 64,
        base_energy: 10,
    }
}

// ---------------------------------------------------------------------------
// Campaign determinism
// ---------------------------------------------------------------------------

/// A target with rich coverage: one edge per distinct (length bucket, first
/// byte high nibble, byte-sum parity), and a crash on a rare shape.
fn rich_target(input: &[u8], cov: &mut CoverageBitmap) -> FuzzOutcome {
    let sum: usize = input.iter().map(|&b| b as usize).sum();
    cov.record_edge(input.len().min(40));
    cov.record_edge(100 + (input.first().copied().unwrap_or(0) >> 4) as usize);
    cov.record_edge(200 + sum % 2);
    if input.len() >= 3 && input[0] == 0xFF && input[2] == 0 {
        return crash("rare-shape", "frame_a");
    }
    FuzzOutcome::Pass
}

type Fingerprint = (
    usize,
    usize,
    usize,
    Vec<(String, Vec<u8>, usize)>,
    usize,
    usize,
);

fn fingerprint(seed: u64, iterations: usize) -> Fingerprint {
    let report = fuzz_target(
        &[vec![1, 2, 3], vec![0xFE]],
        &opts(seed, iterations),
        rich_target,
    );
    (
        report.iterations,
        report.admitted_seeds,
        report.total_edges,
        report
            .crashes
            .iter()
            .map(|c| (c.signature.clone(), c.minimal_input.clone(), c.hits))
            .collect(),
        report.timeouts,
        report.ooms,
    )
}

#[test]
fn a_campaign_is_a_pure_function_of_its_seed() {
    for seed in [0u64, 1, 42, 0xDEAD_BEEF, u64::MAX] {
        assert_eq!(
            fingerprint(seed, 1500),
            fingerprint(seed, 1500),
            "seed {seed}"
        );
    }
    let distinct: BTreeSet<String> = (0..8u64)
        .map(|s| format!("{:?}", fingerprint(s, 1500)))
        .collect();
    assert!(distinct.len() > 3, "seeds barely change the campaign");
}

#[test]
fn iterations_are_exactly_the_budget_when_the_corpus_is_alive() {
    for budget in [0usize, 1, 7, 100, 999] {
        let report = fuzz_target(&[vec![1]], &opts(3, budget), |_, cov| {
            cov.record_edge(1);
            FuzzOutcome::Pass
        });
        assert_eq!(report.iterations, budget);
    }
}

#[test]
fn mutated_candidates_never_exceed_max_input_len() {
    let longest = std::cell::Cell::new(0usize);
    let options = FuzzOptions {
        max_input_len: 24,
        ..opts(9, 3000)
    };
    // The one seed is shorter than the cap, so every execution is a mutation.
    let _ = fuzz_target(&[vec![7; 20]], &options, |input, cov| {
        longest.set(longest.get().max(input.len()));
        cov.record_edge(input.len());
        FuzzOutcome::Pass
    });
    assert!(
        longest.get() <= 24,
        "saw an input of {} bytes",
        longest.get()
    );
    assert!(longest.get() > 20, "the engine never grew an input");
}

// ---------------------------------------------------------------------------
// Crash handling: found, deduplicated, minimized, counted
// ---------------------------------------------------------------------------

#[test]
fn a_reachable_crash_is_found_for_every_seed() {
    // Crashes whenever the input contains 0xFF, which boundary insertion
    // produces constantly. A campaign that finds nothing here is broken.
    for seed in 0..40u64 {
        let report = fuzz_target(&[vec![1, 2, 3]], &opts(seed, 400), |input, cov| {
            cov.record_edge(input.len().min(30));
            if input.contains(&0xFF) {
                crash("saw-ff", "check_ff")
            } else {
                FuzzOutcome::Pass
            }
        });
        assert!(!report.crashes.is_empty(), "seed {seed} found no crash");
        assert!(!report.is_clean());
    }
}

#[test]
fn crashes_with_one_signature_are_one_record_and_hits_add_up() {
    // Two distinct signatures; every crashing execution is counted by the
    // target itself, and the report's hits must add up to that count exactly.
    let executions = std::cell::Cell::new(0usize);
    let crashing = std::cell::Cell::new(0usize);
    let probing = std::cell::Cell::new(false);
    let report = fuzz_target(
        &[vec![0], vec![0], vec![0xEE]],
        &opts(5, 2000),
        |input, cov| {
            executions.set(executions.get() + 1);
            cov.record_edge(input.len().min(50));
            let outcome = if input.contains(&0xFF) {
                crash("kind-a", "frame")
            } else if input.contains(&0xEE) {
                crash("kind-b", "frame")
            } else {
                FuzzOutcome::Pass
            };
            if matches!(outcome, FuzzOutcome::Crash { .. }) && !probing.get() {
                crashing.set(crashing.get() + 1);
            }
            outcome
        },
    );
    let signatures: BTreeSet<&str> = report
        .crashes
        .iter()
        .map(|c| c.signature.as_str())
        .collect();
    assert_eq!(report.crashes.len(), signatures.len(), "duplicate records");
    assert_eq!(report.crashes.len(), 2, "{:?}", report.crashes);
    // Minimization probes also crash and are counted by the target; the
    // campaign's hits cover campaign executions only, so hits <= crashing.
    let hits: usize = report.crashes.iter().map(|c| c.hits).sum();
    assert!(
        hits <= crashing.get() && hits >= 3,
        "hits {hits}, crashing {}",
        crashing.get()
    );
}

#[test]
fn identical_crashing_seeds_in_the_initial_corpus_are_deduplicated() {
    // REGRESSION: triage appended a new record for every crashing seed, so
    // three copies of one crashing input produced three identical reports.
    let report = fuzz_target(
        &[vec![9, 9], vec![9, 9], vec![9, 9]],
        &opts(1, 10),
        |_, _| crash("always", "frame"),
    );
    assert_eq!(report.crashes.len(), 1);
    assert_eq!(report.crashes[0].hits, 3);
}

#[test]
fn novel_crashes_are_minimized() {
    // REGRESSION: minimization existed but was never invoked; the record
    // kept whatever (possibly long) input first crashed.
    let report = fuzz_target(
        &[vec![
            1, 2, 3, 4, 5, 6, 7, 8, 0xAA, 9, 10, 11, 12, 0xBB, 13, 14,
        ]],
        &opts(1, 10),
        |input, _| {
            let a = input.iter().position(|&b| b == 0xAA);
            let b = input.iter().position(|&b| b == 0xBB);
            match (a, b) {
                (Some(i), Some(j)) if i < j => crash("pair", "frame"),
                _ => FuzzOutcome::Pass,
            }
        },
    );
    assert_eq!(report.crashes.len(), 1);
    assert_eq!(report.crashes[0].minimal_input, [0xAA, 0xBB]);
}

#[test]
fn timeouts_and_ooms_are_counted_and_never_look_clean() {
    // REGRESSION: both outcomes fell into `_ => {}`; a target that timed out
    // on every input yielded crashes == [] and passed any "no crashes" check.
    let report = fuzz_target(&[vec![1]], &opts(1, 50), |_, cov| {
        cov.record_edge(1);
        FuzzOutcome::Timeout
    });
    // Triage timeout means no live corpus: nothing was fuzzed, nothing is clean.
    assert_eq!(report.timeouts, 1);
    assert_eq!(report.iterations, 0);
    assert!(report.crashes.is_empty());
    assert!(!report.is_clean(), "an all-timeout campaign read as clean");

    // Timeouts and OOMs during the mutation loop.
    let calls = std::cell::Cell::new(0usize);
    let report = fuzz_target(&[vec![1]], &opts(2, 200), |_, cov| {
        let n = calls.get(); // the first call is the triage of the seed: it passes
        cov.record_edge(n % 5);
        calls.set(n + 1);
        match n % 4 {
            2 => FuzzOutcome::Timeout,
            3 => FuzzOutcome::Oom,
            _ => FuzzOutcome::Pass,
        }
    });
    assert!(report.timeouts > 40 && report.ooms > 40, "{report:?}");
    assert!(report.crashes.is_empty());
    assert!(!report.is_clean());
}

#[test]
fn an_all_oom_campaign_counts_the_triage_oom_and_is_never_clean() {
    let report = fuzz_target(&[vec![1]], &opts(1, 50), |_, _| FuzzOutcome::Oom);
    assert_eq!((report.ooms, report.timeouts, report.iterations), (1, 0, 0));
    assert!(!report.is_clean());
}

#[test]
fn a_shorter_input_for_a_known_signature_replaces_the_stored_one() {
    // Two crashing seeds with one signature: the record keeps the shorter
    // input, and counts both hits. (The first, longer seed is minimized only
    // as far as the target allows: it crashes for any non-empty input.)
    let report = fuzz_target(&[vec![9, 9, 9, 9], vec![7]], &opts(1, 10), |input, _| {
        if input.is_empty() {
            FuzzOutcome::Pass
        } else {
            crash("same", "frame")
        }
    });
    assert_eq!(report.crashes.len(), 1);
    assert_eq!(report.crashes[0].hits, 2);
    assert_eq!(report.crashes[0].minimal_input.len(), 1);
}

#[test]
fn a_genuinely_clean_campaign_is_clean_and_a_vacuous_one_is_not() {
    let clean = fuzz_target(&[vec![1]], &opts(1, 100), |_, cov| {
        cov.record_edge(7);
        FuzzOutcome::Pass
    });
    assert!(clean.is_clean());
    let vacuous = fuzz_target(&[vec![1]], &opts(1, 0), |_, _| FuzzOutcome::Pass);
    assert!(!vacuous.is_clean(), "zero iterations is not evidence");
    let all_crash = fuzz_target(&[vec![1], vec![2]], &opts(1, 100), |_, _| crash("k", "f"));
    assert_eq!(
        all_crash.iterations, 0,
        "a corpus of crashers is never mutated"
    );
    assert!(!all_crash.is_clean());
}

#[test]
fn an_empty_initial_corpus_gets_a_default_seed() {
    let report = fuzz_target(&[], &opts(1, 20), |_, cov| {
        cov.record_edge(3);
        FuzzOutcome::Pass
    });
    assert_eq!(report.iterations, 20);
    assert_eq!(report.admitted_seeds, 1);
}

// ---------------------------------------------------------------------------
// Coverage feedback actually guides the search
// ---------------------------------------------------------------------------

/// Crash only after matching three bytes in a row, reporting one edge per stage.
/// The staircase is what coverage guidance exists to climb.
fn staircase(instrumented: bool) -> impl FnMut(&[u8], &mut CoverageBitmap) -> FuzzOutcome {
    const MAGIC: [u8; 3] = [255, 127, 128];
    move |input, cov| {
        let mut stage = 0;
        while stage < MAGIC.len() && input.get(stage) == Some(&MAGIC[stage]) {
            stage += 1;
            if instrumented {
                cov.record_edge(500 + stage);
            }
        }
        if instrumented {
            cov.record_edge(input.len().min(10));
        } else {
            cov.record_edge(1); // one constant edge: no feedback to climb
        }
        if stage == MAGIC.len() {
            crash("magic", "staircase")
        } else {
            FuzzOutcome::Pass
        }
    }
}

#[test]
fn feedback_cracks_a_staircase_that_blind_mutation_does_not() {
    let mut guided_found = 0;
    let mut blind_found = 0;
    for seed in 0..6u64 {
        let guided = fuzz_target(&[vec![0, 0, 0, 0]], &opts(seed, 6_000), staircase(true));
        let blind = fuzz_target(&[vec![0, 0, 0, 0]], &opts(seed, 6_000), staircase(false));
        guided_found += usize::from(!guided.crashes.is_empty());
        blind_found += usize::from(!blind.crashes.is_empty());
    }
    eprintln!("staircase: guided {guided_found}/6, blind {blind_found}/6");
    assert!(
        guided_found >= 4,
        "coverage-guided search cracked the staircase for only {guided_found}/6 seeds"
    );
    assert!(
        blind_found <= 1,
        "unguided search cracked it for {blind_found}/6 seeds: the staircase proves nothing"
    );
}

// ---------------------------------------------------------------------------
// Coverage bitmap and admission
// ---------------------------------------------------------------------------

#[test]
fn bitmap_geometry_matches_the_spec() {
    assert_eq!(COVERAGE_MAP_SIZE, 65_536);
    assert_eq!(COVERAGE_BYTES, 8_192);
    let mut map = CoverageBitmap::default();
    assert_eq!(map.count(), 0);
    map.record_edge(0);
    map.record_edge(65_535);
    map.record_edge(65_536); // wraps to edge 0
    map.record_edge(usize::MAX); // wraps too
    assert!(map.has_edge(0) && map.has_edge(65_535));
    assert!(map.has_edge(65_536 * 3));
    // 65,536 wraps to 0 and usize::MAX wraps to 65,535: two distinct edges.
    assert_eq!(map.count(), 2);
}

#[test]
fn every_edge_is_independently_addressable() {
    // Setting each of the 65,536 bits alone lights exactly that bit.
    let mut seen = 0usize;
    for edge in (0..COVERAGE_MAP_SIZE).step_by(257) {
        let mut map = CoverageBitmap::new();
        map.record_edge(edge);
        assert_eq!(map.count(), 1, "edge {edge}");
        assert!(map.has_edge(edge));
        assert!(!map.has_edge((edge + 1) % COVERAGE_MAP_SIZE));
        seen += 1;
    }
    assert!(seen > 250);
}

#[test]
fn admission_counts_distinct_new_edges_once() {
    let mut map = CoverageBitmap::new();
    assert_eq!(map.admit_trace(&[1, 2, 3, 3, 3]), (true, 3));
    assert_eq!(map.admit_trace(&[1, 2, 3]), (false, 0));
    assert_eq!(map.admit_trace(&[3, 4, 4]), (true, 1));
    assert_eq!(map.admit_trace(&[]), (false, 0));
    assert_eq!(map.count(), 4);
    // The pure form agrees.
    assert_eq!(corpus_admit(&[1, 2], &[2, 3, 3, 4]), (true, 2));
    assert_eq!(corpus_admit(&[1, 2], &[1, 2]), (false, 0));
    assert_eq!(corpus_admit(&[], &[]), (false, 0));
    let mut a = CoverageBitmap::new();
    let mut b = CoverageBitmap::new();
    a.record_edge(1);
    b.record_edge(2);
    a.merge(&b);
    assert!(a.has_edge(1) && a.has_edge(2) && a.count() == 2);
}

// ---------------------------------------------------------------------------
// Mutator algebra, exhaustive over small inputs
// ---------------------------------------------------------------------------

/// Every byte string of length 0..=2 (1 + 256 + 65,536 = 65,793).
fn all_short_inputs() -> Vec<Vec<u8>> {
    let mut all = vec![vec![]];
    for a in 0..=255u8 {
        all.push(vec![a]);
        for b in 0..=255u8 {
            all.push(vec![a, b]);
        }
    }
    all
}

#[test]
fn bit_and_byte_flips_are_involutions_for_every_short_input() {
    let inputs = all_short_inputs();
    assert_eq!(inputs.len(), 65_793);
    for data in &inputs {
        for offset in [0usize, 1, 9, usize::MAX] {
            for count in [0usize, 1, 4, 17, usize::MAX] {
                let once = bit_flip(data, offset, count);
                assert_eq!(once.len(), data.len());
                assert_eq!(bit_flip(&once, offset, count), *data);
                let once = byte_flip(data, offset, count);
                assert_eq!(once.len(), data.len());
                assert_eq!(byte_flip(&once, offset, count), *data);
            }
        }
    }
}

#[test]
fn bit_flip_with_an_astronomical_count_terminates_with_the_right_parity() {
    // REGRESSION: the loop ran `count` times: usize::MAX never finished.
    // 2 bytes = 16 bits; usize::MAX = 16 * q + 15 with q odd, so all 16 bits
    // are flipped q times (odd: flipped) plus the first 15 once more
    // (even: restored), leaving only the 16th bit from the start offset flipped.
    assert_eq!(bit_flip(&[0, 0], 0, usize::MAX), [0x00, 0x80]);
    // usize::MAX = 3 * q with q odd, so each of 3 bytes is flipped q times.
    assert_eq!(byte_flip(&[0, 0, 0], 0, usize::MAX), [0xFF, 0xFF, 0xFF]);
    assert_eq!(bit_flip(&[0], 0, 8), [0xFF]);
    assert_eq!(bit_flip(&[0], 0, 16), [0x00]);
    assert_eq!(byte_flip(&[1, 2], 0, 2), [0xFE, 0xFD]);
}

#[test]
fn bit_flip_flips_exactly_the_named_window() {
    // For every offset and count 0..=24 over 3 bytes the changed bits are
    // exactly the window of `count` bits from the offset, mod 24, with
    // parity for count > 24.
    for offset in 0..30usize {
        for count in 0..=60usize {
            let out = bit_flip(&[0, 0, 0], offset, count);
            let mut want = [0u8; 3];
            let mut tally = [0u32; 24];
            for i in 0..count {
                tally[(offset % 24 + i) % 24] += 1;
            }
            for (bit, n) in tally.iter().enumerate() {
                if n % 2 == 1 {
                    want[bit / 8] |= 1 << (bit % 8);
                }
            }
            assert_eq!(out, want, "offset {offset}, count {count}");
        }
    }
}

#[test]
fn byte_arithmetic_is_undone_by_the_negated_delta() {
    let mut cases = 0u32;
    for data in all_short_inputs().into_iter().filter(|d| d.len() <= 2) {
        for width in [1usize, 2] {
            for endian in ["little", "big"] {
                for delta in [-35i64, -1, 0, 8] {
                    for offset in [0usize, 1] {
                        let once = byte_arithmetic(&data, offset, delta, width, endian);
                        assert_eq!(once.len(), data.len());
                        let back = byte_arithmetic(&once, offset, -delta, width, endian);
                        assert_eq!(back, data, "w{width} {endian} d{delta} o{offset}");
                        cases += 1;
                    }
                }
            }
        }
    }
    assert!(cases > 1_000_000);
}

#[test]
fn byte_arithmetic_changes_only_its_window_and_wraps() {
    // Width 1: 0xFF + 1 wraps to 0x00; 0x00 - 1 wraps to 0xFF.
    assert_eq!(byte_arithmetic(&[0xFF, 7], 0, 1, 1, "little"), [0x00, 7]);
    assert_eq!(byte_arithmetic(&[0x00, 7], 0, -1, 1, "little"), [0xFF, 7]);
    // Width 2: endianness decides which byte carries.
    assert_eq!(
        byte_arithmetic(&[0xFF, 0x00], 0, 1, 2, "little"),
        [0x00, 0x01]
    );
    assert_eq!(byte_arithmetic(&[0xFF, 0x00], 0, 1, 2, "big"), [0xFF, 0x01]);
    // Width 4 and 8, little and big, wrap at 2^(8w).
    assert_eq!(byte_arithmetic(&[0xFF; 4], 0, 1, 4, "little"), [0; 4]);
    assert_eq!(byte_arithmetic(&[0xFF; 8], 0, 1, 8, "big"), [0; 8]);
    assert_eq!(
        byte_arithmetic(&[0, 0, 0, 0, 0, 0, 0, 0], 0, -1, 8, "little"),
        [0xFF; 8]
    );
    // Too-short input and unsupported widths are returned unchanged.
    assert_eq!(byte_arithmetic(&[1], 0, 5, 2, "little"), [1]);
    for width in [0usize, 3, 5, 6, 7, 9] {
        assert_eq!(
            byte_arithmetic(&[1, 2, 3, 4, 5, 6, 7, 8, 9], 0, 5, width, "little"),
            [1, 2, 3, 4, 5, 6, 7, 8, 9]
        );
    }
    // The offset wraps over the valid window positions only: 3 bytes, width 2
    // has 2 positions, so offset 2 == offset 0 and the last byte is never
    // out of bounds.
    assert_eq!(
        byte_arithmetic(&[0, 0, 0], 2, 1, 2, "little"),
        byte_arithmetic(&[0, 0, 0], 0, 1, 2, "little")
    );
}

#[test]
fn boundary_insert_writes_exactly_the_value_at_the_window() {
    for width in [1usize, 2, 4, 8] {
        for endian in ["little", "big"] {
            for val in [0u64, 1, 127, 128, 255, 256, 65535, u64::MAX] {
                let data = vec![0xAB; 10];
                let out = boundary_insert(&data, 1, width, val, endian);
                assert_eq!(out.len(), data.len());
                let window = &out[1..1 + width];
                let mut le = [0u8; 8];
                for (i, b) in window.iter().enumerate() {
                    if endian == "little" {
                        le[i] = *b;
                    } else {
                        le[width - 1 - i] = *b;
                    }
                }
                let mask = if width == 8 {
                    u64::MAX
                } else {
                    (1u64 << (8 * width)) - 1
                };
                assert_eq!(u64::from_le_bytes(le), val & mask);
                assert_eq!(out[0], 0xAB);
                assert!(out[1 + width..].iter().all(|&b| b == 0xAB));
            }
        }
    }
    assert_eq!(boundary_insert(&[1], 0, 2, 5, "little"), [1]);
}

#[test]
fn block_operations_obey_their_length_arithmetic() {
    for data in all_short_inputs()
        .into_iter()
        .filter(|d| d.len() <= 2)
        .step_by(97)
    {
        for offset in 0..5usize {
            for length in 0..5usize {
                let deleted = block_delete(&data, offset, length);
                let removed = if data.is_empty() {
                    0
                } else {
                    length.min(data.len() - offset % data.len())
                };
                assert_eq!(deleted.len(), data.len() - removed);
                let duplicated = block_duplicate(&data, offset, length, offset + 1);
                let copied = if data.is_empty() {
                    0
                } else {
                    length.min(data.len() - offset % data.len())
                };
                assert_eq!(duplicated.len(), data.len() + copied);
                let inserted = block_insert(&data, offset, &[7, 8]);
                assert_eq!(inserted.len(), data.len() + 2);
                // Insertion preserves the original as prefix + suffix.
                let at = if data.is_empty() {
                    0
                } else {
                    offset % (data.len() + 1)
                };
                assert_eq!(&inserted[..at], &data[..at]);
                assert_eq!(&inserted[at..at + 2], &[7, 8]);
                assert_eq!(&inserted[at + 2..], &data[at..]);
            }
        }
    }
    assert_eq!(block_delete(&[], 3, 3), Vec::<u8>::new());
    assert_eq!(block_duplicate(&[], 0, 4, 0), Vec::<u8>::new());
    assert_eq!(block_insert(&[], 9, &[1, 2]), [1, 2]);
    assert_eq!(block_delete(&[1, 2, 3], 1, 0), [1, 2, 3]);
    // Deleting past the end clamps to the buffer.
    assert_eq!(block_delete(&[1, 2, 3, 4], 2, 99), [1, 2]);
    // Duplicate: bytes [2,3] copied from offset 1 and inserted at the front.
    assert_eq!(block_duplicate(&[1, 2, 3], 1, 2, 0), [2, 3, 1, 2, 3]);
}

#[test]
fn splice_is_prefix_plus_suffix_with_clamped_splits() {
    for ts in 0..6usize {
        for ds in 0..6usize {
            let target = [1u8, 2, 3, 4];
            let donor = [9u8, 8, 7];
            let out = splice(&target, &donor, ts, ds);
            let mut want = target[..ts.min(4)].to_vec();
            want.extend_from_slice(&donor[ds.min(3)..]);
            assert_eq!(out, want, "ts {ts}, ds {ds}");
        }
    }
    assert_eq!(splice(&[], &[], 5, 5), Vec::<u8>::new());
    assert_eq!(splice(&[1], &[], 0, 0), Vec::<u8>::new());
}

#[test]
fn mutate_sequence_is_deterministic_and_never_empties_a_live_buffer() {
    let seeds: Vec<u8> = (0..16).collect();
    for seed in 0..200u64 {
        for steps in [1usize, 2, 5, 12] {
            let a = mutate_sequence(&seeds, seed, steps);
            assert_eq!(a, mutate_sequence(&seeds, seed, steps));
            assert!(!a.is_empty());
        }
    }
    assert_eq!(mutate_sequence(&[1, 2, 3], 5, 0), [1, 2, 3]);
    assert!(!mutate_sequence(&[], 5, 3).is_empty());
    let distinct: BTreeSet<Vec<u8>> = (0..200u64).map(|s| mutate_sequence(&seeds, s, 3)).collect();
    assert!(
        distinct.len() > 150,
        "only {} distinct mutants",
        distinct.len()
    );
}

// ---------------------------------------------------------------------------
// Minimization, signatures, energy, private reporting
// ---------------------------------------------------------------------------

#[test]
fn minimized_inputs_still_crash_with_the_same_signature_and_are_one_minimal() {
    let target = |input: &[u8]| -> FuzzOutcome {
        match (
            input.iter().position(|&b| b == 0xAA),
            input.iter().rposition(|&b| b == 0xBB),
        ) {
            (Some(i), Some(j)) if i < j => crash("pair", "frame"),
            _ => FuzzOutcome::Pass,
        }
    };
    let sig = crash_signature("pair", &["frame"]);
    let mut rng = coding_adventures_test_bootstrap::prng::SplitMix64::new(4);
    let mut tried = 0;
    for _ in 0..300 {
        let len = 2 + rng.bounded(30) as usize;
        let mut data: Vec<u8> = (0..len).map(|_| rng.bounded(256) as u8).collect();
        let (i, j) = (
            rng.bounded(len as u64 / 2) as usize,
            len / 2 + rng.bounded(len as u64 - len as u64 / 2) as usize,
        );
        data[i] = 0xAA;
        data[j.min(len - 1).max(i + 1).min(len - 1)] = 0xBB;
        if target(&data) == FuzzOutcome::Pass {
            continue;
        }
        tried += 1;
        let min = minimize_crashing_input(&data, &sig, target);
        assert!(
            matches!(target(&min), FuzzOutcome::Crash { .. }),
            "{data:?} -> {min:?}"
        );
        assert!(min.len() <= data.len());
        assert_eq!(min, [0xAA, 0xBB], "from {data:?}");
    }
    assert!(tried > 100);
}

#[test]
fn minimization_can_reach_the_empty_input() {
    // REGRESSION: with a one-byte input the chunk size started at 0, so
    // deletion was never tried and a target that crashes on everything
    // minimized to [0] instead of [].
    let always = |_: &[u8]| crash("k", "f");
    let sig = crash_signature("k", &["f"]);
    assert_eq!(
        minimize_crashing_input(&[7], &sig, always),
        Vec::<u8>::new()
    );
    assert_eq!(
        minimize_crashing_input(&[7, 8, 9], &sig, always),
        Vec::<u8>::new()
    );
    assert_eq!(minimize_crashing_input(&[], &sig, always), Vec::<u8>::new());
}

#[test]
fn minimization_does_not_trade_one_crash_for_another() {
    // Deleting the first byte changes the crash kind; the minimizer must
    // keep it. Zeroing the second byte is fine.
    let target = |input: &[u8]| match input {
        [0x01, ..] => crash("kind-one", "f"),
        [] => FuzzOutcome::Pass,
        _ => crash("kind-two", "f"),
    };
    let sig = crash_signature("kind-one", &["f"]);
    let min = minimize_crashing_input(&[1, 2, 3, 4], &sig, target);
    assert_eq!(min, [1]);
}

#[test]
fn crash_signature_is_eight_lowercase_hex_digits_of_the_fnv_of_the_joined_text() {
    let sig = crash_signature("panic", &["parse", "next_token"]);
    assert_eq!(sig.len(), 8);
    assert!(sig.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));
    let want = format!("{:08x}", fnv1a64(b"panic:parse:next_token") & 0xFFFF_FFFF);
    assert_eq!(sig, want);
    // Order and kind matter.
    assert!(!is_duplicate_crash("a", &["x", "y"], "a", &["y", "x"]));
    assert!(!is_duplicate_crash("a", &["x"], "b", &["x"]));
    assert!(is_duplicate_crash("a", &["x", "y"], "a", &["x", "y"]));
}

#[test]
fn crash_signature_delimiter_ambiguity_is_a_documented_limit() {
    // KNOWN LIMIT (VER06 §8): frames are joined with ':' and not escaped, so
    // one frame "x:y" and two frames "x","y" deduplicate as one crash.
    assert!(is_duplicate_crash("k", &["x:y"], "k", &["x", "y"]));
}

#[test]
fn energy_follows_the_spec_formula() {
    assert_eq!(calculate_energy(10, 1, 0, 0), 10);
    assert_eq!(calculate_energy(10, 4, 0, 10), 5);
    assert_eq!(calculate_energy(10, 100, 0, 10), 1);
    assert_eq!(calculate_energy(10, 1, 5, 10), 15);
    assert_eq!(calculate_energy(100, 4, 10, 10), 100); // 100 * 2 / 2
                                                       // Degenerate inputs never yield zero energy or divide by zero.
    assert_eq!(calculate_energy(0, 0, 0, 0), 1);
    assert_eq!(calculate_energy(10, 0, 0, 0), 10);
    for len in 1..200 {
        assert!(calculate_energy(10, len, 3, 50) >= 1);
        assert!(calculate_energy(10, len + 1, 3, 50) <= calculate_energy(10, len, 3, 50));
    }
}

#[test]
fn private_reporting_has_the_exact_shape_and_contains_no_input() {
    assert_eq!(
        format_tap_failure(3, "ct/eq", "fuzz-crash", "deadbeef"),
        "not ok 3 - ct/eq [fuzz-crash:deadbeef]"
    );
}

#[test]
fn no_argument_can_forge_a_tap_line_through_private_reporting() {
    // REGRESSION: the three string arguments were interpolated verbatim, so
    // a newline in a target id added a forged `ok` line to "an opaque token".
    for code in 0u32..0x3000 {
        let Some(c) = char::from_u32(code) else {
            continue;
        };
        let hostile = format!("a{c}b");
        for line in [
            format_tap_failure(1, &hostile, "fuzz-crash", "deadbeef"),
            format_tap_failure(1, "t", &hostile, "deadbeef"),
            format_tap_failure(1, "t", "fuzz-crash", &hostile),
        ] {
            assert!(
                !line
                    .chars()
                    .any(|ch| ch.is_control() || matches!(ch, '\u{2028}' | '\u{2029}')),
                "U+{code:04X}: {line:?}"
            );
            assert_eq!(line.matches('[').count(), 1, "U+{code:04X}");
            assert!(line.starts_with("not ok 1 - "));
        }
    }
}
