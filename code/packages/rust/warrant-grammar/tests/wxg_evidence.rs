//! VER04k section 4.6 to 4.8, tested the way VER04h tests the generator: by
//! *compiling and running* what it emits.  The generated harness
//! (`tests/generated/wxg_evidence.rs`, checked in and drift-tested by
//! `tests/wxg_regen.rs`) is included as a module and run against the honest
//! subject `tests/support/wxg_subject.rs`; `tests/wxg_broken.rs` runs it against
//! a subject with a bug for every operation.
//!
//! Sections: the whole suite and every negative control; the post suites and the
//! manifest fields; pre-state isolation and read-back; each fault as a function;
//! the monitor.

#[allow(dead_code, unused_variables, unused_parens, unused_mut, clippy::all)]
mod harness {
    pub mod wxg_subject {
        include!("support/wxg_subject.rs");
    }
    include!("generated/wxg_evidence.rs");

    /// The generated adapter of `op`, as a plain call (a test hook).
    pub fn call(op: &str, args: &[V], fault: Fault) -> V {
        match op {
            "wipe" => op_wipe(&[], args, fault),
            "wipe4" => op_wipe4(&[], args, fault),
            "bump" => op_bump(&[], args, fault),
            "toggle" => op_toggle(&[], args, fault),
            "set_on" => op_set_on(&[], args, fault),
            "push_byte" => op_push_byte(&[], args, fault),
            "clear_nonempty" => op_clear_nonempty(&[], args, fault),
            "encode" => op_encode(&[], args, fault),
            other => panic!("no operation {other}"),
        }
    }

    /// The adapter without its monitor (`op_<name>_core`): what the subject and the
    /// faults produce, before the `ensures` rows judge it.
    pub fn core(op: &str, args: &[V], fault: Fault) -> V {
        match op {
            "wipe" => op_wipe_core(&[], args, fault),
            "wipe4" => op_wipe4_core(&[], args, fault),
            "bump" => op_bump_core(&[], args, fault),
            "toggle" => op_toggle_core(&[], args, fault),
            "set_on" => op_set_on_core(&[], args, fault),
            "push_byte" => op_push_byte_core(&[], args, fault),
            "clear_nonempty" => op_clear_nonempty_core(&[], args, fault),
            "encode" => op_encode_core(&[], args, fault),
            other => panic!("no operation {other}"),
        }
    }

    /// The same call, with a postcondition violation as an `Err` message.
    pub fn monitored(op: &str, args: &[V], fault: Fault) -> Result<V, String> {
        match catch_unwind(AssertUnwindSafe(|| call(op, args, fault))) {
            Ok(v) => Ok(v),
            Err(p) => match p.downcast::<PostViolation>() {
                Ok(v) => Err(v.0),
                Err(other) => std::panic::resume_unwind(other),
            },
        }
    }
}

include!("support/wxg_runner.rs");

use harness::{Fault, V};

fn bytes(b: &[u8]) -> V {
    V::Bytes(b.to_vec())
}

#[test]
fn the_whole_suite_passes_against_the_honest_subject() {
    let run = run_once();
    let failed: Vec<String> = run
        .outcomes
        .iter()
        .filter(|o| !o.passed())
        .map(|o| format!("{}: {}", o.id, message(o)))
        .collect();
    assert!(failed.is_empty(), "{failed:#?}");
    assert_eq!(run.failed, 0);
    // 11 examples, 10 claims, 8 post suites (x3 kinds), and the negative controls.
    let n = run.outcomes.len();
    assert!(n > 100, "{n} tests");
}

#[test]
fn every_negative_control_catches_its_fault() {
    let run = run_once();
    let controls: Vec<&_> = run.outcomes.iter().filter(|o| o.id.contains("/negctl/")).collect();
    assert!(!controls.is_empty());
    for o in &controls {
        assert!(o.passed(), "{}: {}", o.id, message(o));
    }
    // The faults of 4.8 that the fixture binds, each at least once.
    for fault in ["stale-post", "skip-last-write", "drop-last-push", "clobber-prefix", "cursor-off-by-one", "missing-error"] {
        assert!(
            controls.iter().any(|o| o.id.ends_with(&format!("/{fault}"))),
            "no negative control for {fault}"
        );
    }
}

#[test]
fn each_operation_has_exhaustive_property_and_fuzz_post_suites() {
    let run = run_once();
    for op in ["wipe", "wipe4", "bump", "toggle", "set_on", "push_byte", "clear_nonempty", "encode"] {
        for kind in ["exhaustive", "property", "fuzz"] {
            let id = format!("warrant/wxg/{kind}/{op}/post");
            let o = run.outcomes.iter().find(|o| o.id == id).unwrap_or_else(|| panic!("no {id}"));
            assert!(o.passed(), "{id}");
        }
    }
}

#[test]
fn the_manifest_counts_monitored_calls_and_postcondition_rows() {
    let m = &run_once().manifest;
    assert!(m.contains("\"monitored_calls\""), "{m}");
    assert!(m.contains("\"postcondition_rows\""), "{m}");
    // `wipe` has two rows, `bump` one.
    let entry = |op: &str| -> String {
        let id = format!("warrant/wxg/exhaustive/{op}/post");
        let at = m.find(&id).expect("manifest entry");
        m[at..].chars().take(700).collect()
    };
    assert!(entry("wipe").contains("\"postcondition_rows\": 2"), "{}", entry("wipe"));
    assert!(entry("bump").contains("\"postcondition_rows\": 1"), "{}", entry("bump"));
}

#[test]
fn domains_are_labelled_honestly() {
    let m = &run_once().manifest;
    // `Bytes<4>` is a sample of 2^32 values, `Bytes` a fixed set of 347, a `U8` is full.
    assert!(m.contains("BytesN(4) sample("), "{m}");
    assert!(m.contains("Bytes set(347)"), "{m}");
    assert!(m.contains("BytesN(1) full(256)"), "{m}");
    assert!(m.contains("BytesN(2) sample("), "{m}");
}

#[test]
fn pre_state_isolation_the_subject_gets_a_copy_and_the_oracle_value_is_untouched() {
    // The adapter hands the subject a copy: the argument the rows and claims use
    // as the pre-state is exactly what it was.
    let args = [bytes(&[1, 2, 3])];
    let r = harness::core("wipe", &args, Fault::None);
    assert_eq!(r, bytes(&[0, 0, 0]));
    assert_eq!(args[0], bytes(&[1, 2, 3]), "the pre-state must be untouched");
    let cell = [V::Con("Cell", 0, "Cell", vec![V::Fixed(harness::F::U8, 7), V::Bool(false)])];
    let r = harness::core("set_on", &cell, Fault::None);
    assert_eq!(r, V::Con("Cell", 0, "Cell", vec![V::Fixed(harness::F::U8, 7), V::Bool(true)]));
    assert_eq!(cell[0], V::Con("Cell", 0, "Cell", vec![V::Fixed(harness::F::U8, 7), V::Bool(false)]));
}

#[test]
fn read_back_is_from_the_buffer_the_subject_was_given() {
    // The honest read-back sees the write; `stale-post` (the violation of the
    // rule) returns the pre-state, which a postcondition then rejects.
    let args = [bytes(&[9, 9])];
    assert_eq!(harness::core("wipe", &args, Fault::None), bytes(&[0, 0]));
    let stale = harness::monitored("wipe", &args, Fault::StalePost);
    assert_eq!(stale, Err("postcondition-violated: wipe row 2".to_string()));
}

#[test]
fn each_fault_rewrites_the_read_back_as_section_4_8_says() {
    // skip-last-write: the last element is the pre-state's.
    let r = harness::core("wipe", &[bytes(&[1, 2, 3])], Fault::SkipLastWrite);
    assert_eq!(r, bytes(&[0, 0, 3]));
    // ... and is inert on an empty pre-state.
    assert_eq!(harness::core("wipe", &[bytes(&[])], Fault::SkipLastWrite), bytes(&[]));
    // drop-last-push: a longer post-state loses its last element.
    let out = [V::Fixed(harness::F::U8, 7), bytes(&[1, 2])];
    let r = harness::core("push_byte", &out, Fault::DropLastPush);
    assert_eq!(r, V::Tup(vec![V::Int(1), bytes(&[1, 2])]));
    // clobber-prefix: the first byte's low bit is flipped when the pre-state is non-empty.
    let r = harness::core("push_byte", &out, Fault::ClobberPrefix);
    assert_eq!(r, V::Tup(vec![V::Int(1), bytes(&[0, 2, 7])]));
    let r = harness::core("push_byte", &[V::Fixed(harness::F::U8, 7), bytes(&[])], Fault::ClobberPrefix);
    assert_eq!(r, V::Tup(vec![V::Int(1), bytes(&[7])]), "an empty pre-state has no prefix to clobber");
    // cursor-off-by-one: the post-state gets +1 when even and -1 when odd.
    let post = |x: i128, fault| harness::core("bump", &[V::Fixed(harness::F::U8, x)], fault);
    assert_eq!(post(4, Fault::None), V::Fixed(harness::F::U8, 5));
    assert_eq!(post(4, Fault::CursorOffByOne), V::Fixed(harness::F::U8, 4), "5 is odd, so 5 - 1");
    assert_eq!(post(5, Fault::None), V::Fixed(harness::F::U8, 6));
    assert_eq!(post(5, Fault::CursorOffByOne), V::Fixed(harness::F::U8, 7), "6 is even, so 6 + 1");
    // missing-error: an `Err` becomes an `Ok` of the minimal payload.
    let r = harness::core("clear_nonempty", &[bytes(&[])], Fault::MissingError);
    assert!(matches!(r, V::Ok(_)), "{r:?}");
    let r = harness::core("clear_nonempty", &[bytes(&[])], Fault::None);
    assert_eq!(r, V::Err("TOO_SHORT".into()));
}

#[test]
fn the_error_slots_carry_no_post_state_on_err_and_the_payload_on_ok() {
    // S2 (panic normalised, M9): Err has no post-state.
    assert_eq!(harness::core("clear_nonempty", &[bytes(&[])], Fault::None), V::Err("TOO_SHORT".into()));
    assert_eq!(
        harness::core("clear_nonempty", &[bytes(&[5, 6])], Fault::None),
        V::Ok(Box::new(bytes(&[0, 0])))
    );
    // S4 (err_any): Ok of (host count, post-state), or the one case.
    let ok = harness::core("encode", &[V::Fixed(harness::F::U8, 1), bytes(&[0xaa, 0xbb, 0xcc])], Fault::None);
    assert_eq!(ok, V::Ok(Box::new(V::Tup(vec![V::Int(2), bytes(&[1, 0xfe, 0xcc])]))));
    let err = harness::core("encode", &[V::Fixed(harness::F::U8, 1), bytes(&[0xaa])], Fault::None);
    assert_eq!(err, V::Err("TOO_SHORT".into()));
}

#[test]
fn the_monitor_reports_the_row_and_runs_only_where_the_refinements_admit() {
    // A violated row names itself.
    let r = harness::monitored("bump", &[V::Fixed(harness::F::U8, 3)], Fault::StalePost);
    assert_eq!(r, Err("postcondition-violated: bump row 1".to_string()));
    let r = harness::monitored("toggle", &[V::Bool(true)], Fault::StalePost);
    assert_eq!(r, Err("postcondition-violated: toggle row 1".to_string()));
    // An honest call passes silently.
    assert!(harness::monitored("bump", &[V::Fixed(harness::F::U8, 3)], Fault::None).is_ok());
}
