//! VOS-D063 (VER04j section 14.3 item 54): the allocation observer counts the
//! export call and nothing else.
//!
//! The generated harness (`tests/generated/wxo_evidence.rs`, drift-tested by
//! `tests/wxo_regen.rs`) has four examples over an in-test subject
//! (`tests/support/wxo_subject.rs`), every operation under a `heap <= 0` row:
//!
//! * `make` returns a record and `pick` a payload-carrying variant.  The
//!   adapter allocates a `Vec` to build the Warrant value of either (that is
//!   the harness's own marshalling).  Before the fix the observer counted it
//!   and the rows failed; now they must hold.
//! * `grow`'s EXPORT allocates a kilobyte.  The row must fail, so the observer
//!   has not gone blind: that is the negative control of the fix.

#[allow(dead_code, unused_variables, unused_parens, unused_mut, clippy::all)]
mod harness {
    pub mod wxo_subject {
        include!("support/wxo_subject.rs");
    }
    include!("generated/wxo_evidence.rs");
}

include!("support/wxh_runner.rs");

#[test]
fn marshalling_a_record_or_a_variant_is_not_charged_to_the_subject() {
    for id in [
        "example/make_is_seven",
        "example/pick_runs_three",
        "example/pick_idles_on_255",
    ] {
        let o = outcome(id);
        assert!(o.passed(), "{id}: {}", message(o));
    }
}

#[test]
fn an_allocation_inside_the_export_is_still_caught() {
    let o = outcome("example/grow_echoes");
    assert!(!o.passed(), "the allocating export must fail its heap row");
    let m = message(o);
    assert!(m.contains("bound-heap-exceeded"), "{m}");
    assert!(m.contains("1024 bytes against a heap bound of 0"), "{m}");
}

#[test]
fn the_manifest_records_zero_for_the_honest_operations_and_the_peak_for_the_allocating_one() {
    let m = &run_once().manifest;
    // One entry per example, in registration order; the honest ones peak at 0.
    for id in ["make_is_seven", "pick_runs_three", "pick_idles_on_255"] {
        let at = m.find(&format!("example/{id}")).unwrap_or_else(|| panic!("{id} in {m}"));
        let entry = &m[at..];
        let entry = &entry[..entry.find('}').unwrap()];
        assert!(entry.contains("\"max_observed_heap\": 0"), "{id}: {entry}");
    }
    let at = m.find("example/grow_echoes").expect("grow in the manifest");
    let entry = &m[at..];
    let entry = &entry[..entry.find('}').unwrap()];
    assert!(entry.contains("\"max_observed_heap\": 1024"), "{entry}");
}
