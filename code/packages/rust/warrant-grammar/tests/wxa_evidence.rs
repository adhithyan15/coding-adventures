//! VER04j WX-A evidence generation, tested the way VER04h tests the v0
//! generator: by *compiling and running* what it emits.  The generated harness
//! (`tests/generated/wxa_evidence.rs`, checked in and drift-tested by
//! `tests/wxa_regen.rs`) is included as a module and run against a tiny subject
//! (`wxa_subject`) whose exports implement the LEB128-shaped, record, variant
//! and tuple operations of `tests/fixtures/wxa.warrant`.
//!
//! Sections: the whole suite and every negative control; domains of composite
//! types (sizes, orders, constructors); generators, relation bias and
//! shrinkers; the unreachable-by-type count; the differential test of the
//! ground evaluator against the generated model code on every composite
//! domain within budget.

use std::collections::BTreeSet;

use coding_adventures_test_bootstrap::run::Options;
use coding_adventures_test_bootstrap::Verdict;

#[allow(dead_code, unused_variables, unused_parens, unused_mut, clippy::all)]
mod harness {
    /// The subject under test: honest implementations of the fixture's
    /// operations, shaped like the real Rust APIs WX-A is meant for.
    pub mod wxa_subject {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct Span {
            pub start: u8,
            pub len: u8,
        }
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Shape {
            Circle { r: u8 },
            Rect { w: u8, h: u8 },
            Empty,
        }
        #[derive(Debug)]
        pub enum LebErr {
            Truncated,
            Overlong,
        }
        /// The first one or two bytes of a stream as an unsigned LEB128 value:
        /// the value and how many bytes it used.
        pub fn decode2(b0: u8, b1: u8) -> Result<(u64, usize), LebErr> {
            if b0 < 0x80 {
                Ok((u64::from(b0), 1))
            } else if b1 < 0x80 {
                Ok((u64::from(b0 & 0x7f) | (u64::from(b1) << 7), 2))
            } else {
                Err(LebErr::Overlong)
            }
        }
        pub fn normalize(a: u8, b: u8) -> Span {
            Span {
                start: a.min(b),
                len: a.max(b) - a.min(b),
            }
        }
        pub fn classify(w: u8, h: u8) -> Shape {
            match (w, h) {
                (0, 0) => Shape::Empty,
                (w, h) if w == h => Shape::Circle { r: w },
                (w, h) => Shape::Rect { w, h },
            }
        }
        pub fn area(s: Shape) -> (bool, u64) {
            match s {
                Shape::Circle { r } => (false, u64::from(r) * u64::from(r)),
                Shape::Rect { w, h } => (false, u64::from(w) * u64::from(h)),
                Shape::Empty => (true, 0),
            }
        }
        pub fn order(a: u8, b: u8) -> (u8, u8) {
            (a.min(b), a.max(b))
        }
        pub fn split(a: u8) -> Option<(u8, u8)> {
            (a != 0).then_some((a >> 4, a & 15))
        }
        /// Panics on 255 (the binding normalises the panic to `OVERLONG`).
        pub fn try_pair(a: u8) -> (u8, u8) {
            (a, a.checked_add(1).expect("255 has no successor"))
        }
        pub fn sh(a: u8, n: u8) -> (u8, u8) {
            (a.checked_shl(u32::from(n)).unwrap_or(0), n)
        }
    }
    include!("generated/wxa_evidence.rs");
}
use harness::{Fault, Ty, F, V};

fn is_fail(v: &Verdict, id: &str) -> bool {
    matches!(v, Verdict::Fail { failure_id, .. } if *failure_id == id)
}

// ---------------------------------------------------------------------------
// The whole generated suite, including every negative control
// ---------------------------------------------------------------------------

#[test]
fn the_full_generated_suite_passes_and_every_fault_is_caught() {
    // The subject panics on purpose (try_pair 255); the generated harness
    // silences the hook in `run_main`, so do the same here.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| ()));
    let suite = harness::build_suite(Fault::None);
    let mut tap = Vec::new();
    let (summary, outcomes) = suite
        .run_recorded(&Options::default(), &mut tap)
        .expect("run");
    std::panic::set_hook(previous);
    let tap = String::from_utf8(tap).unwrap();
    assert!(
        summary.failed == 0 && !summary.bailed,
        "failures:\n{}",
        tap.lines()
            .filter(|l| l.starts_with("not ok"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let negctl: Vec<_> = outcomes
        .iter()
        .filter(|o| o.id.contains("/negctl/"))
        .collect();
    assert!(!negctl.is_empty() && negctl.iter().all(|o| o.passed()));
    // Every WX-A fault is bound somewhere and is caught.
    let faults: BTreeSet<&str> = negctl
        .iter()
        .map(|o| o.id.rsplit('/').next().unwrap())
        .collect();
    for f in [
        "drop-consumed",
        "swap-components",
        "field-zero",
        "wrong-ctor",
        "ok-as-err",
        "off-by-one-consumed",
        "missing-error",
    ] {
        assert!(faults.contains(f), "{f} is not exercised: {faults:?}");
    }
}
