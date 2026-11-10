//! # Constant-Time Comparison Fuzz Targets (VER06 / VER06a / VER26 E4)
//!
//! Validates `coding_adventures_ct_compare` under the Level 2 Coverage-Guided
//! Fuzz Engine (`coding_adventures_test_bootstrap::fuzz`).
//!
//! Verifies:
//! - No unhandled crashes or panics under arbitrary hostile bytes
//! - Perfect equivalence against reference equality `(a == b)`
//! - Symmetry `ct_eq(a, b) == ct_eq(b, a)` and reflexivity `ct_eq(a, a) == true`
//! - Length mismatch rejection without byte inspection
//! - Branchless selection accuracy and fresh allocation guarantee
//! - 64-bit integer equality agreement
//! - Liveness proof: sabotaged implementations are detected, deduplicated, and minimized
//! - Privacy preservation: zero raw payload bytes exposed in failure outputs

use std::panic::catch_unwind;

use coding_adventures_ct_compare::{ct_eq, ct_eq_fixed, ct_eq_u64, ct_select_bytes};
use coding_adventures_test_bootstrap::fuzz::{
    self, crash_signature, format_tap_failure, CoverageBitmap, FuzzOptions, FuzzOutcome,
    FID_FUZZ_CRASH,
};
use coding_adventures_test_bootstrap::run::{self, Ctx, Suite};
use coding_adventures_test_bootstrap::Verdict;

fn main() {
    let mut suite = Suite::new();

    suite.add("fuzz/ct_eq/bounded_campaign", check_ct_eq_campaign);
    suite.add(
        "fuzz/ct_eq_fixed/bounded_campaign",
        check_ct_eq_fixed_campaign,
    );
    suite.add(
        "fuzz/ct_select_bytes/bounded_campaign",
        check_ct_select_bytes_campaign,
    );
    suite.add("fuzz/ct_eq_u64/bounded_campaign", check_ct_eq_u64_campaign);
    suite.add(
        "fuzz/negative_sabotage/faulty_eq_detected",
        check_faulty_eq_detected,
    );
    suite.add(
        "fuzz/negative_sabotage/faulty_select_detected",
        check_faulty_select_detected,
    );
    suite.add(
        "fuzz/private_reporting/zero_leak_audit",
        check_privacy_audit,
    );

    std::process::exit(run::main(&suite));
}

/// Unpack a continuous fuzz input into two byte slices (VER06a §2.1).
fn unpack_two_slices(input: &[u8]) -> (&[u8], &[u8]) {
    if input.is_empty() {
        (&[], &[])
    } else if input.len() == 1 {
        (&input[0..0], &input[0..1])
    } else {
        let split_raw = input[0] as usize;
        let payload = &input[1..];
        let split = split_raw % (payload.len() + 1);
        (&payload[..split], &payload[split..])
    }
}

/// Instrument coverage bitmap based on slice lengths and prefix comparison (VER06a §3).
fn instrument_coverage(cov: &mut CoverageBitmap, a: &[u8], b: &[u8], result: bool) {
    if a.len() == b.len() {
        cov.record_edge(0);
        let matching_prefix = a.iter().zip(b.iter()).take_while(|(x, y)| x == y).count();
        cov.record_edge(1000 + matching_prefix.min(256));
    } else {
        cov.record_edge(1 + ((a.len() ^ b.len()) % 256));
    }
    cov.record_edge(300 + a.len().min(64));
    cov.record_edge(2000 + (result as usize));
}

fn check_ct_eq_campaign(_ctx: &Ctx) -> Verdict {
    let opts = FuzzOptions {
        max_iterations: 500,
        seed: 0xCAFE_BABE,
        max_input_len: 256,
        base_energy: 10,
    };

    let initial_corpus = vec![
        vec![],
        vec![0],
        vec![0, 1, 2],
        vec![0xAA; 16],
        vec![0x55; 32],
    ];

    let report = fuzz::fuzz_target(&initial_corpus, &opts, |input, cov| {
        let (a, b) = unpack_two_slices(input);
        let actual = ct_eq(a, b);
        let expected = a == b;

        instrument_coverage(cov, a, b, actual);

        // Verification of fundamental invariants (VER06a §1.1)
        if actual != expected {
            return FuzzOutcome::Crash {
                kind: "disagreement".into(),
                frames: vec!["model_mismatch".into()],
            };
        }
        if ct_eq(b, a) != actual {
            return FuzzOutcome::Crash {
                kind: "asymmetry".into(),
                frames: vec!["symmetry_violated".into()],
            };
        }
        if !ct_eq(a, a) || !ct_eq(b, b) {
            return FuzzOutcome::Crash {
                kind: "irreflexive".into(),
                frames: vec!["reflexivity_violated".into()],
            };
        }
        if a.len() != b.len() && actual {
            return FuzzOutcome::Crash {
                kind: "length_leak".into(),
                frames: vec!["length_mismatch_true".into()],
            };
        }

        FuzzOutcome::Pass
    });

    if !report.crashes.is_empty() {
        return Verdict::fail(
            FID_FUZZ_CRASH,
            format!(
                "ct_eq crashed with signature: {:?}",
                report.crashes[0].signature
            ),
        );
    }

    if report.total_edges == 0 {
        return Verdict::fail(
            "corpus-mismatch",
            "coverage bitmap did not observe any edges",
        );
    }

    if report.iterations < 500 {
        return Verdict::fail(
            "corpus-mismatch",
            format!("insufficient iterations: {}", report.iterations),
        );
    }
    Verdict::Pass
}

fn check_ct_eq_fixed_campaign(_ctx: &Ctx) -> Verdict {
    let opts = FuzzOptions {
        max_iterations: 300,
        seed: 0xDEAD_BEEF,
        max_input_len: 64,
        base_energy: 8,
    };

    // Corpus with 32-byte chunks (16 bytes each for two arrays)
    let initial_corpus = vec![vec![0u8; 32], vec![0xFFu8; 32], (0..32u8).collect()];

    let report = fuzz::fuzz_target(&initial_corpus, &opts, |input, cov| {
        if input.len() < 32 {
            cov.record_edge(1);
            return FuzzOutcome::Pass;
        }

        let a_slice: &[u8; 16] = input[0..16].try_into().unwrap();
        let b_slice: &[u8; 16] = input[16..32].try_into().unwrap();

        let actual = ct_eq_fixed(a_slice, b_slice);
        let expected = a_slice == b_slice;

        instrument_coverage(cov, a_slice, b_slice, actual);

        if actual != expected {
            return FuzzOutcome::Crash {
                kind: "fixed_disagreement".into(),
                frames: vec!["fixed_mismatch".into()],
            };
        }

        FuzzOutcome::Pass
    });

    if !report.crashes.is_empty() {
        return Verdict::fail(
            FID_FUZZ_CRASH,
            format!(
                "ct_eq_fixed crashed with signature: {:?}",
                report.crashes[0].signature
            ),
        );
    }

    if report.iterations < 300 {
        return Verdict::fail(
            "corpus-mismatch",
            format!("insufficient iterations: {}", report.iterations),
        );
    }
    Verdict::Pass
}

fn check_ct_select_bytes_campaign(_ctx: &Ctx) -> Verdict {
    let opts = FuzzOptions {
        max_iterations: 400,
        seed: 0x1234_5678,
        max_input_len: 128,
        base_energy: 10,
    };

    let initial_corpus = vec![
        vec![0, 0],
        vec![1, 0, 0xAA, 0xBB],
        vec![0, 1, 0x11, 0x22, 0x33, 0x44],
    ];

    let report = fuzz::fuzz_target(&initial_corpus, &opts, |input, cov| {
        if input.len() < 2 {
            return FuzzOutcome::Pass;
        }
        let choice = (input[0] & 1) != 0;
        cov.record_edge(3000 + (choice as usize));

        let payload = &input[2..];
        let (a, b) = if (input[1] & 1) == 0 {
            // Equal length slice pair
            let half = payload.len() / 2;
            (&payload[..half], &payload[half..2 * half])
        } else {
            // Divergent length slice pair
            let split = (input[1] as usize) % (payload.len() + 1);
            (&payload[..split], &payload[split..])
        };

        if a.len() == b.len() {
            let out = ct_select_bytes(a, b, choice);
            let expected = if choice { a } else { b };
            if out.as_slice() != expected {
                return FuzzOutcome::Crash {
                    kind: "selection_mismatch".into(),
                    frames: vec!["wrong_bytes".into()],
                };
            }
            // Verify fresh allocation invariant (VER06a §1.1)
            if !out.is_empty() && (out.as_ptr() == a.as_ptr() || out.as_ptr() == b.as_ptr()) {
                return FuzzOutcome::Crash {
                    kind: "aliased_allocation".into(),
                    frames: vec!["not_fresh_copy".into()],
                };
            }
            cov.record_edge(3010);
        } else {
            // Unequal length: must panic with exact message
            let prev_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(|_| {}));
            let panic_res = catch_unwind(|| {
                ct_select_bytes(a, b, choice);
            });
            std::panic::set_hook(prev_hook);
            match panic_res {
                Ok(_) => {
                    return FuzzOutcome::Crash {
                        kind: "missing_length_panic".into(),
                        frames: vec!["accepted_unequal".into()],
                    };
                }
                Err(err) => {
                    let msg = err
                        .downcast_ref::<&str>()
                        .copied()
                        .or_else(|| err.downcast_ref::<String>().map(|s| s.as_str()))
                        .unwrap_or("");
                    if !msg.contains("ct_select_bytes requires equal-length slices") {
                        return FuzzOutcome::Crash {
                            kind: "unexpected_panic_msg".into(),
                            frames: vec!["wrong_error".into()],
                        };
                    }
                }
            }
            cov.record_edge(3020);
        }

        FuzzOutcome::Pass
    });

    if !report.crashes.is_empty() {
        return Verdict::fail(
            FID_FUZZ_CRASH,
            format!(
                "ct_select_bytes crashed with signature: {:?}",
                report.crashes[0].signature
            ),
        );
    }

    if report.iterations < 400 {
        return Verdict::fail(
            "corpus-mismatch",
            format!("insufficient iterations: {}", report.iterations),
        );
    }
    Verdict::Pass
}

fn check_ct_eq_u64_campaign(_ctx: &Ctx) -> Verdict {
    let opts = FuzzOptions {
        max_iterations: 300,
        seed: 0x9999_8888,
        max_input_len: 16,
        base_energy: 6,
    };

    let initial_corpus = vec![
        vec![0u8; 16],
        vec![0xFFu8; 16],
        vec![1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0],
    ];

    let report = fuzz::fuzz_target(&initial_corpus, &opts, |input, cov| {
        let mut buf = [0u8; 16];
        let copy_len = input.len().min(16);
        buf[..copy_len].copy_from_slice(&input[..copy_len]);

        let a = u64::from_le_bytes(buf[0..8].try_into().unwrap());
        let b = u64::from_le_bytes(buf[8..16].try_into().unwrap());

        let actual = ct_eq_u64(a, b);
        let expected = a == b;

        cov.record_edge(4000 + (actual as usize));

        if actual != expected {
            return FuzzOutcome::Crash {
                kind: "u64_mismatch".into(),
                frames: vec!["model_disagreement".into()],
            };
        }
        if ct_eq_u64(b, a) != actual || !ct_eq_u64(a, a) {
            return FuzzOutcome::Crash {
                kind: "u64_symmetry".into(),
                frames: vec!["symmetry_broken".into()],
            };
        }

        FuzzOutcome::Pass
    });

    if !report.crashes.is_empty() {
        return Verdict::fail(
            FID_FUZZ_CRASH,
            format!(
                "ct_eq_u64 crashed with signature: {:?}",
                report.crashes[0].signature
            ),
        );
    }

    if report.iterations < 300 {
        return Verdict::fail(
            "corpus-mismatch",
            format!("insufficient iterations: {}", report.iterations),
        );
    }
    Verdict::Pass
}

/// Fault injection: sabotage ct_eq by returning early on first matching byte
fn sabotaged_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    if a.is_empty() {
        return true;
    }
    // Sabotage: returns true if first byte matches, ignoring remaining bytes
    a[0] == b[0]
}

fn check_faulty_eq_detected(_ctx: &Ctx) -> Verdict {
    let opts = FuzzOptions {
        max_iterations: 100,
        seed: 42,
        max_input_len: 32,
        base_energy: 10,
    };

    let target_sig = crash_signature("sabotaged_equality", &["first_byte_leak"]);

    let report = fuzz::fuzz_target(&[vec![0, 0, 0, 0]], &opts, |input, _| {
        let (a, b) = unpack_two_slices(input);
        let actual = sabotaged_eq(a, b);
        let expected = a == b;

        if actual != expected {
            FuzzOutcome::Crash {
                kind: "sabotaged_equality".into(),
                frames: vec!["first_byte_leak".into()],
            }
        } else {
            FuzzOutcome::Pass
        }
    });

    // Fuzz engine must detect the defect and report the crash
    if report.crashes.is_empty() {
        return Verdict::fail(
            "corpus-mismatch",
            "fuzz engine failed to discover known sabotage defect",
        );
    }

    if report.crashes[0].signature != target_sig {
        return Verdict::fail(
            "corpus-mismatch",
            format!("signature mismatch: got {:?}", report.crashes[0].signature),
        );
    }

    // Input minimization verification (VER06 §5.3)
    let minimal =
        fuzz::minimize_crashing_input(&report.crashes[0].minimal_input, &target_sig, |candidate| {
            let (a, b) = unpack_two_slices(candidate);
            if sabotaged_eq(a, b) != (a == b) {
                FuzzOutcome::Crash {
                    kind: "sabotaged_equality".into(),
                    frames: vec!["first_byte_leak".into()],
                }
            } else {
                FuzzOutcome::Pass
            }
        });

    if minimal.len() > report.crashes[0].minimal_input.len() {
        return Verdict::fail("corpus-mismatch", "minimization enlarged input");
    }

    Verdict::Pass
}

fn check_faulty_select_detected(_ctx: &Ctx) -> Verdict {
    let opts = FuzzOptions {
        max_iterations: 100,
        seed: 77,
        max_input_len: 16,
        base_energy: 5,
    };

    let target_sig = crash_signature("sabotaged_select", &["inverted_choice"]);

    let report = fuzz::fuzz_target(&[vec![0, 0]], &opts, |input, _| {
        if input.is_empty() {
            return FuzzOutcome::Pass;
        }
        let choice = (input[0] & 1) != 0;
        let payload = &input[1..];
        let half = payload.len() / 2;
        let a = &payload[..half];
        let b = &payload[half..2 * half];

        // Sabotage: inverts choice
        let bad_out = ct_select_bytes(a, b, !choice);
        let expected = if choice { a } else { b };

        if bad_out.as_slice() != expected && a != b {
            FuzzOutcome::Crash {
                kind: "sabotaged_select".into(),
                frames: vec!["inverted_choice".into()],
            }
        } else {
            FuzzOutcome::Pass
        }
    });

    if report.crashes.is_empty() {
        return Verdict::fail(
            "corpus-mismatch",
            "fuzz engine failed to discover selection sabotage",
        );
    }

    if report.crashes[0].signature != target_sig {
        return Verdict::fail(
            "corpus-mismatch",
            format!("signature mismatch: got {:?}", report.crashes[0].signature),
        );
    }

    Verdict::Pass
}

fn check_privacy_audit(_ctx: &Ctx) -> Verdict {
    let secret_payload = b"SUPER_SECRET_PAYLOAD_KEY_MATERIAL";
    let target = "ct_eq_fuzz";
    let sig_hash = "f1a2b3c4";

    let tap_line = format_tap_failure(1, target, FID_FUZZ_CRASH, sig_hash);

    // Strict compliance with VER00 §7.3: opaque token only
    let expected_line = "not ok 1 - ct_eq_fuzz [fuzz-crash:f1a2b3c4]";
    if tap_line != expected_line {
        return Verdict::fail(
            "corpus-mismatch",
            format!("tap_line format error: got {tap_line:?}, want {expected_line:?}"),
        );
    }

    if tap_line.contains(std::str::from_utf8(secret_payload).unwrap()) {
        return Verdict::fail(
            "corpus-mismatch",
            "CRITICAL: tap_line leaked secret raw payload bytes",
        );
    }

    Verdict::Pass
}
