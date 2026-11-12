//! VER04j section 6.7, the pilot: a decompressor that sizes `Vec::with_capacity`
//! from a raw header field (CWE-789, the bug the lz* survey found by reading)
//! *observably fails* the generated evidence, and the capped fix passes
//! (`tests/wxh_evidence.rs`).  The failure is a failed instance with the
//! observer's note and the measured size, not a process abort: this binary
//! survives a request for four gigabytes.

#[allow(dead_code, unused_variables, unused_parens, unused_mut, clippy::all)]
mod harness {
    pub mod wxh_subject {
        include!("support/wxh_subject_vulnerable.rs");
    }
    include!("generated/wxh_evidence.rs");
}

include!("support/wxh_runner.rs");

#[test]
fn the_bomb_examples_fail_with_the_observer_note_and_the_process_survives() {
    let run = run_once();
    assert!(run.failed > 0, "the vulnerable subject must fail the evidence");
    for id in [
        "example/tiny_bomb_is_rejected",
        "example/tiny_bomb_under_a_huge_limit_is_corrupt",
    ] {
        let o = outcome(id);
        assert!(!o.passed(), "{id} passed");
        let m = message(o);
        assert!(m.contains("bound-heap-exceeded"), "{id}: {m}");
        assert!(m.contains("asked for 4294967295 bytes"), "{id}: {m}");
    }
}

#[test]
fn the_answers_are_right_so_only_the_allocation_is_reported() {
    // The honest examples (nothing asks for more than a few bytes) still pass.
    for id in [
        "example/three_bytes_within_the_limit",
        "example/one_byte_over_the_limit",
        "example/short_header_is_corrupt",
    ] {
        let o = outcome(id);
        assert!(o.passed(), "{id}: {}", message(o));
    }
}

#[test]
fn a_claim_that_reaches_the_bomb_through_a_limit_fails_too() {
    // `success_survives_a_larger_limit` calls the operation with the maximum
    // limit and a hostile header; so does the output-bound property whenever its
    // generator draws one.  At least one generated suite catches the bomb.
    let caught = run_once()
        .outcomes
        .iter()
        .filter(|o| o.id.contains("/exhaustive/") || o.id.contains("/property/") || o.id.contains("/fuzz/"))
        .filter(|o| !o.passed())
        .count();
    assert!(caught > 0, "no generated suite noticed the bomb");
    let manifest = &run_once().manifest;
    assert!(manifest.contains("\"verdict\": \"fail\""), "{manifest}");
}
