//! VER04j section 14.4 (WX-N) evidence generation, tested the way the other
//! slices are: by *compiling and running* what the generator emits.  The
//! generated harness (`tests/generated/wxn_evidence.rs`, checked in and
//! drift-tested by `tests/wxn_regen.rs`) is included as a module and run against
//! a tiny subject whose exports take and return `[u8; N]`.
//!
//! Sections: the whole suite and every negative control; the manifest's honest
//! domain labels; the two length checks that fail closed; wrapping `U8`
//! arithmetic (WD-013) through the generated model code.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};

use coding_adventures_test_bootstrap::run::Options;

/// Set by a test to make `head2` return the wrong number of bytes.
static HEAD2_SHORT: AtomicBool = AtomicBool::new(false);

#[allow(dead_code, unused_variables, unused_parens, unused_mut, clippy::all)]
mod harness {
    /// The subject under test: honest implementations shaped like the real
    /// crypto APIs WX-N is meant for (`&[u8; N]` in, `[u8; N]` out).
    pub mod wxn_subject {
        use std::sync::atomic::Ordering;

        pub fn flip4(a: &[u8; 4]) -> [u8; 4] {
            [a[3], a[2], a[1], a[0]]
        }
        /// Returns a `Vec` on purpose: the Warrant type says `Bytes<2>`, so the
        /// harness must check the length itself.
        pub fn head2(a: &[u8; 4]) -> Vec<u8> {
            if super::super::HEAD2_SHORT.load(Ordering::SeqCst) {
                return vec![a[0]];
            }
            vec![a[0], a[1]]
        }
        pub fn pair(a: &[u8; 2], b: &[u8; 2]) -> Result<[u8; 4], &'static str> {
            if a == b {
                return Err("equal halves");
            }
            Ok([a[0], a[1], b[0], b[1]])
        }
        pub fn bump1(a: &[u8; 1], k: u8) -> [u8; 1] {
            [a[0].wrapping_add(k)]
        }
        pub fn scale1(a: &[u8; 1], k: u8) -> [u8; 1] {
            [a[0].wrapping_mul(k)]
        }
        pub fn shl1(a: &[u8; 1], n: u32) -> [u8; 1] {
            [if n >= 8 { 0 } else { a[0] << n }]
        }
    }
    include!("generated/wxn_evidence.rs");

    pub fn call(name: &str, args: &[V]) -> V {
        match name {
            "flip4" => op_flip4(&[], args, Fault::None),
            "head2" => op_head2(&[], args, Fault::None),
            "pair" => op_pair(&[], args, Fault::None),
            "bump1" => op_bump1(&[], args, Fault::None),
            "scale1" => op_scale1(&[], args, Fault::None),
            "shl1" => op_shl1(&[], args, Fault::None),
            _ => panic!("no such op"),
        }
    }

    pub fn model(name: &str, args: &[V]) -> V {
        match name {
            "bump1_model" => model_bump1_model(args[0].clone(), args[1].clone()),
            "scale1_model" => model_scale1_model(args[0].clone(), args[1].clone()),
            "shl1_model" => model_shl1_model(args[0].clone(), args[1].clone()),
            _ => panic!("no such model"),
        }
    }
}

use harness::{Fault, V};

fn bytes(b: &[u8]) -> V {
    V::Bytes(b.to_vec())
}

fn run() -> (coding_adventures_test_bootstrap::run::Summary, Vec<coding_adventures_test_bootstrap::run::Outcome>, String) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| ()));
    let suite = harness::build_suite(Fault::None);
    let mut tap = Vec::new();
    let (summary, outcomes) = suite.run_recorded(&Options::default(), &mut tap).expect("run");
    std::panic::set_hook(previous);
    (summary, outcomes, String::from_utf8(tap).unwrap())
}

// ---------------------------------------------------------------------------
// The whole generated suite, including every negative control
// ---------------------------------------------------------------------------

#[test]
fn the_full_generated_suite_passes_and_every_fault_is_caught() {
    let (summary, outcomes, tap) = run();
    assert!(
        summary.failed == 0 && !summary.bailed,
        "failures:\n{}",
        tap.lines().filter(|l| l.starts_with("not ok")).collect::<Vec<_>>().join("\n")
    );
    let negctl: Vec<_> = outcomes.iter().filter(|o| o.id.contains("/negctl/")).collect();
    assert!(!negctl.is_empty() && negctl.iter().all(|o| o.passed()));
    for id in [
        "negctl/example/flip4/flip-result-bit",
        "negctl/exhaustive/flip4/drop-last-result-byte",
        "negctl/exhaustive/pair/missing-error",
        "negctl/exhaustive/bump1/flip-result-bit",
    ] {
        assert!(negctl.iter().any(|o| o.id.ends_with(id)), "no negative control {id}");
    }
}

// ---------------------------------------------------------------------------
// Domains are labelled for what they are
// ---------------------------------------------------------------------------

#[test]
fn the_manifest_labels_a_sampled_bytes_n_domain_as_sampled_and_a_full_one_as_full() {
    let (summary, outcomes, _) = run();
    assert_eq!(summary.failed, 0);
    let out = std::env::temp_dir().join(format!("wxn_manifest_{}.json", std::process::id()));
    std::env::set_var("WARRANT_EVIDENCE_OUT", &out);
    harness::write_manifest_for(&outcomes);
    std::env::remove_var("WARRANT_EVIDENCE_OUT");
    let m = std::fs::read_to_string(&out).expect("manifest written");
    let _ = std::fs::remove_file(&out);
    let has = |needle: &str| assert!(m.contains(needle), "manifest lacks {needle:?}\n{m}");
    // Bytes<4>: 3N + 4 = 16 of 2^32 values -- a sample, and it says so.
    has("a: BytesN(4) sample(16)");
    // Bytes<2>: 4^2 = 16 of 65,536.
    has("a: BytesN(2) sample(16); b: BytesN(2) sample(16)");
    // Bytes<1> is every value: 256 of 256.
    has("a: BytesN(1) full(256); k: U8 full(256)");
    // ...and a Bytes<N> domain is never called a set or exhaustive-by-size.
    assert!(!m.contains("BytesN(4) set("), "{m}");
}

// ---------------------------------------------------------------------------
// Length checks fail closed
// ---------------------------------------------------------------------------

#[test]
fn a_model_value_of_the_wrong_length_is_unreachable_by_type_never_padded() {
    let r = catch_unwind(AssertUnwindSafe(|| harness::call("flip4", &[bytes(&[1, 2, 3])])));
    let p = r.expect_err("a 3-byte value for a Bytes<4> parameter must not reach the export");
    assert!(p.downcast_ref::<harness::Unreachable>().is_some(), "payload is Unreachable");
    let r = catch_unwind(AssertUnwindSafe(|| harness::call("flip4", &[bytes(&[1, 2, 3, 4, 5])])));
    assert!(r.is_err(), "a 5-byte value is not truncated to 4");
}

#[test]
fn a_host_result_of_the_wrong_length_is_a_finding_not_a_value() {
    // Honest: two bytes.
    assert_eq!(harness::call("head2", &[bytes(&[1, 2, 3, 4])]), bytes(&[1, 2]));
    HEAD2_SHORT.store(true, Ordering::SeqCst);
    let r = catch_unwind(AssertUnwindSafe(|| harness::call("head2", &[bytes(&[1, 2, 3, 4])])));
    HEAD2_SHORT.store(false, Ordering::SeqCst);
    let p = r.expect_err("a one-byte result for a Bytes<2> must fail the harness");
    let msg = p
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
        .expect("a message payload");
    assert!(msg.contains("head2") && msg.contains("2 bytes"), "{msg}");
}

// ---------------------------------------------------------------------------
// WD-013: U8 arithmetic wraps
// ---------------------------------------------------------------------------

#[test]
fn u8_arithmetic_wraps_in_the_generated_model_code() {
    use harness::F;
    let u8v = |n: i128| V::Fixed(F::U8, n);
    // 200 + 100 = 300 = 44 (mod 256); a non-wrapping evaluator says 300.
    assert_eq!(harness::model("bump1_model", &[bytes(&[200]), u8v(100)]), bytes(&[44]));
    assert_eq!(harness::model("bump1_model", &[bytes(&[255]), u8v(1)]), bytes(&[0]));
    // The subject wraps the same way, so every one of the 65,536 cases agrees.
    assert_eq!(harness::call("bump1", &[bytes(&[200]), u8v(100)]), bytes(&[44]));
}

/// VER04 section 3: `U8` multiplication wraps and `shl` drops the high bits
/// (`n >= 8` gives 0), in the generated model code and in the subject.
#[test]
fn u8_multiplication_and_shifts_wrap_in_the_generated_model_code() {
    use harness::F;
    let u8v = |n: i128| V::Fixed(F::U8, n);
    // 200 * 3 = 600 = 88 (mod 256); a non-wrapping evaluator says 600.
    assert_eq!(harness::model("scale1_model", &[bytes(&[200]), u8v(3)]), bytes(&[88]));
    assert_eq!(harness::model("scale1_model", &[bytes(&[16]), u8v(16)]), bytes(&[0]));
    assert_eq!(harness::call("scale1", &[bytes(&[200]), u8v(3)]), bytes(&[88]));
    // shl(0x81, 1) = 0x02 (the top bit is dropped), shl(1, 7) = 128, shl(1, 8) = 0.
    assert_eq!(harness::model("shl1_model", &[bytes(&[0x81]), V::Int(1)]), bytes(&[0x02]));
    assert_eq!(harness::model("shl1_model", &[bytes(&[1]), V::Int(7)]), bytes(&[128]));
    assert_eq!(harness::model("shl1_model", &[bytes(&[1]), V::Int(8)]), bytes(&[0]));
    assert_eq!(harness::call("shl1", &[bytes(&[0x81]), V::Int(1)]), bytes(&[0x02]));
}
