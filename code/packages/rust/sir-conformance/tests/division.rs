//! # Division conformance — the confirmed floor-vs-trunc frontier (SIR21 §E3)
//!
//! Ruby's integer `/` **floors** (rounds toward −∞): `7 / 2 == 3`,
//! `-7 / 2 == -4`. That is what the reference oracle
//! ([`sir_conformance::oracle::DivOp::Floor`]) prescribes, and what a faithful
//! transpile of Ruby must reproduce on every backend.
//!
//! It does **not**, today. Probing the current pipeline surfaced a real,
//! multi-way divergence — the textbook "one overloaded `divide` that does the
//! Ruby thing on Tuesdays" bug SIR21 §E3 exists to kill:
//!
//! | `-7 / 2` (Ruby floor = **−4**) | backend today |
//! |-------------------------------|---------------|
//! | Python `-3`                   | truncates toward 0 (its runtime `div` is `int(a/b)`) |
//! | JavaScript `-3.5`             | true (float) division, not integer at all |
//! | Go / Rust — **crash**         | native `/` panics / errors on the path |
//!
//! Even *positive* division diverges: `7 / 2` prints `3.5` on JavaScript.
//!
//! There is also a **deliberate conflict** to resolve, not just bugs to fix: the
//! Python runtime's `div` is documented and unit-tested as *truncating* ("to
//! match SIR semantics"), while the SIR21 oracle (and Ruby) say *floor*. The two
//! disagree on what a bare `/` means — which is precisely why §E3 splits it into
//! explicit `div_floor` / `div_trunc`. Resolving that (flip vs. split, and the
//! `Integer#/` floors but `Float#/` true-divides polymorphism) is a design
//! decision tracked outside this test.
//!
//! This file **captures** the frontier so it is tracked and oracle-judged, the
//! way the `10²⁴` bignum frontier is captured in `arithmetic.rs`. The
//! cross-backend assertion is `#[ignore]`d so the suite stays green; it flips to
//! a passing assertion the day division is made floor-faithful everywhere. The
//! toolchain-free control below always runs.

use sir_conformance::oracle::{DivOp, Outcome};
use sir_conformance::{run_source, RunOutcome, Target};

/// `(lhs, rhs)` pairs covering every sign combination plus exact division.
/// Ruby evaluates each with **floor** `/`.
const CASES: &[(i128, i128)] = &[
    (7, 2),   // 3
    (-7, 2),  // -4  (floor; trunc would be -3)
    (7, -2),  // -4
    (-7, -2), // 3
    (6, 3),   // 2   (exact — floor == trunc)
    (-6, 2),  // -3  (exact)
];

/// The Ruby-floor expected output for a case, straight from the oracle.
fn floor_expected(lhs: i128, rhs: i128) -> String {
    match DivOp::Floor.eval(lhs, rhs) {
        Outcome::Value(v) => v.to_string(),
        other => panic!("division case {lhs}/{rhs}: oracle gave {other:?}, expected a Value"),
    }
}

/// Toolchain-free control: the oracle's floor expectations really are Ruby's.
/// This always runs — it is the ground truth the frontier is measured against.
#[test]
fn oracle_floor_matches_ruby_integer_division() {
    assert_eq!(floor_expected(7, 2), "3");
    assert_eq!(floor_expected(-7, 2), "-4");
    assert_eq!(floor_expected(7, -2), "-4");
    assert_eq!(floor_expected(-7, -2), "3");
    assert_eq!(floor_expected(6, 3), "2");
    assert_eq!(floor_expected(-6, 2), "-3");
}

/// The frontier itself: every backend must reproduce Ruby's floor `/`. Ignored
/// until division is made floor-faithful (Python truncates, JS true-divides,
/// Go/Rust crash on the negative path) — run with `cargo test -- --ignored` to
/// watch it, and it becomes a passing assertion once the gap is closed.
#[test]
#[ignore = "division frontier (SIR21 §E3): backends don't implement Ruby floor `/` — \
            Python truncates, JS true-divides, Go/Rust crash on negatives. Flips green \
            when division is made floor-faithful. Semantics decision tracked separately."]
fn division_matches_ruby_floor_on_every_backend() {
    let mut ran = 0usize;
    for &(lhs, rhs) in CASES {
        let expected = floor_expected(lhs, rhs);
        let ruby = format!("puts({} / {})\n", lhs, rhs);
        for &target in Target::all() {
            match run_source("division", &ruby, target) {
                RunOutcome::Ran(out) => {
                    assert_eq!(
                        out,
                        expected,
                        "\nDIVISION FRONTIER: backend {} computed {}/{} = {out}, Ruby floor = {expected}\n",
                        target.tag(),
                        lhs,
                        rhs,
                    );
                    ran += 1;
                }
                // A crash (Go/Rust today) is a Failed outcome — surface it as a
                // frontier failure too when this test is un-ignored.
                RunOutcome::Failed(msg) => panic!(
                    "DIVISION FRONTIER: backend {} failed on {}/{}: {}",
                    target.tag(),
                    lhs,
                    rhs,
                    msg.lines().next().unwrap_or("")
                ),
                RunOutcome::Skipped(_) => {}
            }
        }
    }
    assert!(ran > 0, "no backend toolchain available — the frontier proved nothing");
}
