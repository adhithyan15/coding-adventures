//! VER04k section 4.1 and 4.11: a subject with one bug per operation (an
//! off-by-one loop, a mutation lost on a copy, a skipped write, an append that
//! overwrites, a state-dependent write, a missing refusal, a half-written
//! encoding) *observably fails* the generated evidence, and the failure names the
//! violated postcondition where a row catches it.

#[allow(dead_code, unused_variables, unused_parens, unused_mut, clippy::all)]
mod harness {
    pub mod wxg_subject {
        include!("support/wxg_subject_broken.rs");
    }
    include!("generated/wxg_evidence.rs");
}

include!("support/wxg_runner.rs");

/// The failed outcomes whose id contains `part`, with their messages.
fn failures(part: &str) -> Vec<String> {
    run_once()
        .outcomes
        .iter()
        .filter(|o| !o.passed() && o.id.contains(part) && !o.id.contains("/negctl/"))
        .map(|o| format!("{}: {}", o.id, message(o)))
        .collect()
}

#[test]
fn every_operation_of_the_broken_subject_fails_some_suite() {
    assert!(run_once().failed > 0);
    for op in ["wipe", "wipe4", "bump", "toggle", "set_on", "push_byte", "clear_nonempty", "encode"] {
        let f = failures(&format!("/{op}"));
        let by_example_or_claim = run_once().outcomes.iter().any(|o| !o.passed() && o.id.contains(op));
        assert!(by_example_or_claim, "nothing failed for {op}");
        assert!(!f.is_empty() || by_example_or_claim, "{op}");
    }
}

#[test]
fn a_failing_post_suite_names_the_violated_row() {
    // `wipe` leaves the last byte: row 2 (`result == zeros(len(buf))`) is false.
    let f = failures("exhaustive/wipe/post");
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(f[0].contains("postcondition-violated: wipe row 2"), "{f:?}");
    // `bump` and `toggle` do not write: row 1 (`result != x`) is false.
    assert!(failures("exhaustive/bump/post")[0].contains("postcondition-violated: bump row 1"));
    assert!(failures("exhaustive/toggle/post")[0].contains("postcondition-violated: toggle row 1"));
    // `set_on` only for a non-zero byte: the cell with a == 0 violates row 1.
    assert!(failures("exhaustive/set_on/post")[0].contains("postcondition-violated: set_on row 1"));
    // `clear_nonempty` does not refuse an empty buffer.
    assert!(failures("exhaustive/clear_nonempty/post")[0].contains("postcondition-violated: clear_nonempty row 1"));
}

#[test]
fn claims_catch_what_no_row_states() {
    // The stale `wipe4`, the clobbering `push_byte` and the half-written `encode`
    // satisfy their (weak) rows; the written claims are what fail.
    for claim in ["wipe4_zeroes", "push_byte_appends_the_byte", "encode_writes_the_complement"] {
        assert!(!failures(claim).is_empty(), "{claim} passed against a broken subject");
    }
}

#[test]
fn the_examples_fail_too() {
    for ex in ["example/wipe_three_bytes", "example/wipe4_ones", "example/bump_wraps", "example/toggle_true", "example/push_byte_appends"] {
        assert!(!failures(ex).is_empty(), "{ex} passed against a broken subject");
    }
}
