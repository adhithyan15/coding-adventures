//! # Division conformance — the confirmed floor-vs-trunc frontier (SIR21 §E3)
//!
//! Ruby's integer `/` **floors** (rounds toward −∞): `7 / 2 == 3`,
//! `-7 / 2 == -4`. That is what the reference oracle
//! ([`sir_conformance::oracle::DivOp::Floor`]) prescribes, and what a faithful
//! transpile of Ruby must reproduce on every backend.
//!
//! Probing the pipeline once surfaced a real, multi-way divergence — the
//! textbook "one overloaded `divide` that does the Ruby thing on Tuesdays" bug
//! SIR21 §E3 exists to kill. **It is now closed:** every backend that runs the
//! negative cases reproduces Ruby's floor.
//!
//! | `-7 / 2` (Ruby floor = **−4**) | was | now |
//! |-------------------------------|-----|-----|
//! | Python     | `-3` (truncated)   | ✅ `-4` — `//` on two ints; `Float#/` true-divides |
//! | Rust       | `-3` (truncated)   | ✅ `-4` — floored int path in `__sir::divide` |
//! | Go         | `-3` (truncated)   | ✅ `-4` — floored int path in `_sir_divide` |
//! | JavaScript | `-3.5` (float `/`) | ✅ `-4` — `Math.floor` when both operands integral |
//! | Ruby       | (native)           | ✅ `-4` — emits Ruby, whose `/` floors natively |
//! | C          | (crashed)          | ✅ `-4` — runtime already floored; emitter now lowers `neg` |
//!
//! The int-path fixes live in each backend's runtime `divide` helper and mirror
//! the oracle's [`DivOp::Floor`]: the floored quotient is the truncated one
//! minus one exactly when the remainder is non-zero and its sign differs from
//! the divisor's. Python, Rust, Go and C carry tagged values, so they dispatch
//! `Integer#/` (floor) vs `Float#/` (true-divide) faithfully; the C runtime
//! already floored (`_sir_ifloordiv`). JavaScript numbers are all `f64`, so it
//! floors when both operands are integer-valued (`Number.isInteger`) — correct
//! for every integer-division case the frontier asserts; a Ruby `Float` that is
//! integral (`7.0`) still needs the typed pipeline to true-divide, tracked
//! separately. Ruby transpiles to Ruby, so its `/` is already floor.
//!
//! **A gap this frontier forced to the surface — now fixed everywhere:** Ruby
//! lowers unary minus (`-x`) to a `neg` builtin, and the Go and Rust *runtimes*
//! had no `neg` — so every negative literal (not just division) crashed with
//! `unknown builtin: neg`. That is the "Go/Rust crash on negatives" this doc
//! long recorded; it was never a division bug. Both runtimes now implement it,
//! and the **C backend's emitter** now lowers `neg` too (to its single-argument
//! `_sir_minus`, which negates tag-preservingly), so C's negative cases run and
//! are asserted rather than skipped. Ruby needs the `ruby` toolchain present to
//! run; absent it, it skips.
//!
//! The resolution of the original "deliberate conflict" (flip vs. split) went
//! the additive way: the oracle already carries *both* honest ops (`div_floor`,
//! `div_trunc`), and each backend maps `Integer#/` onto floor — no overloaded
//! runtime `divide`. [`division_matches_ruby_floor_on_every_backend`] is now a
//! live (non-ignored) assertion; [`python_division_is_ruby_floor_faithful`]
//! remains as a granular per-backend guard. The toolchain-free control below
//! always runs.

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

/// The frontier itself, now **closed and live**: every backend that *runs* a
/// case must reproduce Ruby's floor `/` for it. Each backend's runtime `divide`
/// floors integer division (SIR21 §E3), so this is no longer `#[ignore]`d — it
/// is a first-class conformance assertion. It asserts on `Ran` outcomes and
/// fails (naming the offending backend) the day one regresses to truncation or
/// true-division on integer operands; `Skipped` cases are not asserted (Ruby
/// skips without a `ruby` toolchain, and any backend skips without its
/// compiler). Verified locally across all six backends — Python, JavaScript,
/// Go, Rust, Ruby and C — flooring every sign combination.
#[test]
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

/// Float-division cases: the OTHER half of Ruby's polymorphic `/`. `Float#/`
/// TRUE-divides (never floors), and an integral-valued float result still
/// displays with a trailing `.0` (`6.0 / 2 == 3.0`, not `3`). Expected strings
/// are Ruby's `Float#to_s`, which every backend must reproduce — the same
/// cross-backend parity the integer frontier locks, on the float side. The JS
/// backend gained this with the tagged-float substrate (its numbers are all
/// f64, so an integral Float like `6.0` was previously indistinguishable from
/// Integer `6` and wrongly floored); the tagged-value backends (Rust/Go/C) and
/// Python/Ruby already carried the distinction.
const FLOAT_CASES: &[(&str, &str)] = &[
    ("7.0 / 2", "3.5"),   // Float / Int  → Float
    ("6.0 / 2", "3.0"),   // integral result still prints `.0`
    ("6.0 / 3.0", "2.0"), // Float / Float
    ("7 / 2.0", "3.5"),   // Int / Float  → Float (promotes)
    ("-7.0 / 2", "-3.5"), // sign preserved (true-divide, not floor)
];

#[test]
fn float_division_true_divides_on_every_backend() {
    let mut ran = 0usize;
    for &(expr, expected) in FLOAT_CASES {
        let ruby = format!("puts({})\n", expr);
        for &target in Target::all() {
            match run_source("floatdiv", &ruby, target) {
                RunOutcome::Ran(out) => {
                    assert_eq!(
                        out,
                        expected,
                        "\nFLOAT DIVISION: backend {} computed `{}` = {out}, Ruby = {expected}\n",
                        target.tag(),
                        expr,
                    );
                    ran += 1;
                }
                RunOutcome::Failed(msg) => panic!(
                    "FLOAT DIVISION: backend {} failed on `{}`: {}",
                    target.tag(),
                    expr,
                    msg.lines().next().unwrap_or("")
                ),
                RunOutcome::Skipped(_) => {}
            }
        }
    }
    assert!(ran > 0, "no backend toolchain available — float division proved nothing");
}

/// The Python arm of the frontier, **closed** and guarded. The Python backend's
/// `Integer#/` now floors (SIR21 §E3 `DivOp::Floor`), so it reproduces Ruby's
/// `/` on every sign combination end-to-end — emit, run, compare to the oracle.
///
/// Unlike the all-backend test above this is **not** `#[ignore]`d: it is a live
/// regression guard that would fail the day someone reverts the Python `div`
/// helper to truncation. It runs whenever `python3` and the `sir-runtime-*`
/// packages are present; if they are not it is inert (prints a note) rather than
/// falsely green — the same convention the all-backend frontier uses.
#[test]
fn python_division_is_ruby_floor_faithful() {
    let mut ran = 0usize;
    for &(lhs, rhs) in CASES {
        let expected = floor_expected(lhs, rhs);
        let ruby = format!("puts({} / {})\n", lhs, rhs);
        match run_source("division_py", &ruby, Target::Python) {
            RunOutcome::Ran(out) => {
                assert_eq!(
                    out, expected,
                    "\nPython division not floor-faithful: {lhs}/{rhs} = {out}, Ruby floor = {expected}\n",
                );
                ran += 1;
            }
            RunOutcome::Failed(msg) => panic!(
                "Python division errored on {}/{}: {}",
                lhs,
                rhs,
                msg.lines().next().unwrap_or("")
            ),
            RunOutcome::Skipped(_) => {}
        }
    }
    if ran == 0 {
        eprintln!(
            "python_division_is_ruby_floor_faithful: python3 / sir-runtime-* unavailable — skipped"
        );
    }
}
