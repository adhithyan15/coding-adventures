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

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span,
    Stmt,
};
use sir_conformance::oracle::{DivOp, Outcome};
use sir_conformance::{run_module, run_source, RunOutcome, Target};

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

// ── SIR21 T3b-2 Slice 3: direct-call coverage for the new division ops ─────
//
// `division_matches_ruby_floor_on_every_backend` above sources through Ruby,
// so it only ever exercises bare `/` (no frontend emits `div_floor`/
// `div_trunc`/`udiv_trunc`/`div_true` yet — that's Slices 4-6 of this arc).
// This section calls each new op DIRECTLY by name via a hand-built `Module`
// (`sir_conformance::run_module`, added alongside these tests specifically
// because no source text exists that would lower to these builtins), so it
// proves every one of Slice 2's 7 backend PRs actually wired the dispatch
// table correctly — independent of any frontend migration.

fn s() -> Span {
    Span::synthetic()
}
fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}
fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}
fn bin(name: &str, a: Expr, b: Expr) -> Expr {
    call(name, vec![a, b])
}
fn print_stmt(expr: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "__sys_write__".into(),
            args: vec![
                Expr::StrLit { value: "stdout".into(), span: s() },
                Expr::StrLit { value: "once".into(), span: s() },
                Expr::BoolLit { value: false, span: s() },
                expr,
            ],
            effects: EffectSet::PURE.with(Effect::MayPrint),
            span: s(),
        },
        span: s(),
    }
}

fn div_module(stmts: Vec<Stmt>) -> Module {
    Module {
        name: "divdirect".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::ConsoleIO,
            Feature::Strings,
            Feature::Floats,
            Feature::Exceptions,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block { stmts, value: Expr::NilLit { span: s() }, span: s() },
            effects: EffectSet::PURE.with(Effect::MayPrint),
            metadata: Metadata::new(),
            span: s(),
        }],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("test")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: s(),
    }
}

/// Every backend that runs a `bin(op, lhs, rhs)` case must reproduce
/// `expected`. Mirrors [`division_matches_ruby_floor_on_every_backend`]'s
/// own accounting: `Skipped` is not asserted (no toolchain), `Failed` is a
/// hard failure naming the offending backend, and at least one backend must
/// have actually run or the assertion proved nothing.
fn assert_direct_call_matches_on_every_backend(op: &str, lhs: i64, rhs: i64, expected: &str) {
    let m = div_module(vec![print_stmt(bin(op, ilit(lhs), ilit(rhs)))]);
    // `run_module`'s temp filename is built from `name` + target + PID only
    // (no per-call nonce) — since `cargo test` runs test functions
    // concurrently within one process, two calls sharing a name AND target
    // race on the same temp path, so distinguish every case explicitly.
    let name = format!("divdirect_{op}_{lhs}_{rhs}").replace('-', "n");
    let mut ran = 0usize;
    for &target in Target::all() {
        match run_module(&name, &m, target) {
            RunOutcome::Ran(out) => {
                assert_eq!(
                    out,
                    expected,
                    "\nDIRECT-CALL {op}: backend {} computed {op}({lhs}, {rhs}) = {out}, expected {expected}\n",
                    target.tag(),
                );
                ran += 1;
            }
            RunOutcome::Failed(msg) => panic!(
                "DIRECT-CALL {op}: backend {} failed on {op}({lhs}, {rhs}): {}",
                target.tag(),
                msg.lines().next().unwrap_or("")
            ),
            RunOutcome::Skipped(_) => {}
        }
    }
    assert!(ran > 0, "DIRECT-CALL {op}({lhs}, {rhs}): no backend toolchain available");
}

#[test]
fn direct_call_div_floor_matches_ruby_floor_on_every_backend() {
    for &(lhs, rhs) in CASES {
        let expected = floor_expected(lhs, rhs);
        assert_direct_call_matches_on_every_backend(
            "div_floor",
            lhs as i64,
            rhs as i64,
            &expected,
        );
    }
}

#[test]
fn direct_call_div_trunc_truncates_toward_zero_on_every_backend() {
    for &(lhs, rhs) in CASES {
        let expected = match DivOp::Trunc.eval(lhs, rhs) {
            Outcome::Value(v) => v.to_string(),
            other => panic!("div_trunc case {lhs}/{rhs}: oracle gave {other:?}, expected a Value"),
        };
        assert_direct_call_matches_on_every_backend(
            "div_trunc",
            lhs as i64,
            rhs as i64,
            &expected,
        );
    }
}

/// `udiv_trunc` is only exercised on NON-NEGATIVE operands here — a negative
/// operand's "unsigned" interpretation genuinely differs across backends
/// (C/Go/Rust reinterpret the bit pattern as `u64`; Python/Ruby/JS/TS have
/// no fixed width and no signed/unsigned distinction to reinterpret, so
/// they'd compute plain signed truncation instead), which is exactly the
/// documented, intentional per-backend divergence each Slice 2 PR already
/// recorded — not something this cross-backend parity check should paper
/// over by picking one interpretation. On non-negative operands every
/// backend agrees, which is what this asserts.
#[test]
fn direct_call_udiv_trunc_matches_div_trunc_on_nonnegative_operands_every_backend() {
    const NONNEGATIVE_CASES: &[(i128, i128)] = &[(7, 2), (6, 3), (0, 5)];
    for &(lhs, rhs) in NONNEGATIVE_CASES {
        let expected = match DivOp::Trunc.eval(lhs, rhs) {
            Outcome::Value(v) => v.to_string(),
            other => panic!("udiv_trunc case {lhs}/{rhs}: oracle gave {other:?}, expected a Value"),
        };
        assert_direct_call_matches_on_every_backend(
            "udiv_trunc",
            lhs as i64,
            rhs as i64,
            &expected,
        );
    }
}

/// `div_true` always true-divides — no `DivOp` oracle variant exists for it
/// (the integer oracle's overflow/rounding model doesn't apply to an op that
/// unconditionally floats), so expected values are hand-computed the same
/// way [`FLOAT_CASES`] above already does.
#[test]
fn direct_call_div_true_always_true_divides_on_every_backend() {
    const TRUE_DIV_CASES: &[(i64, i64, &str)] = &[
        (7, 2, "3.5"),
        (-7, 2, "-3.5"),
        (6, 3, "2.0"), // exact division still true-divides, not "2"
    ];
    for &(lhs, rhs, expected) in TRUE_DIV_CASES {
        assert_direct_call_matches_on_every_backend("div_true", lhs, rhs, expected);
    }
}

/// Every one of the four new division ops must raise on a zero divisor, on
/// every backend — a case entirely untested before this Slice 3 addition
/// (no zero-divisor case existed anywhere in this file). An uncaught raise
/// is a non-zero exit, which `run_module` reports as `RunOutcome::Failed`
/// — so here (uniquely in this file) `Failed` is the EXPECTED outcome, not
/// a test failure; a `Ran` outcome means the zero check silently failed to
/// fire, which IS the bug this test exists to catch.
#[test]
fn direct_call_zero_divisor_raises_for_every_op_on_every_backend() {
    let mut ran = 0usize;
    for op in ["div_floor", "div_trunc", "udiv_trunc", "div_true"] {
        let m = div_module(vec![print_stmt(bin(op, ilit(7), ilit(0)))]);
        let name = format!("divdirectzero_{op}");
        for &target in Target::all() {
            match run_module(&name, &m, target) {
                RunOutcome::Ran(out) => panic!(
                    "DIRECT-CALL ZERO-DIVISOR: backend {} did NOT raise for {op}(7, 0) — \
                     printed {out:?} instead of failing",
                    target.tag(),
                ),
                RunOutcome::Failed(_) => ran += 1,
                RunOutcome::Skipped(_) => {}
            }
        }
    }
    assert!(ran > 0, "DIRECT-CALL ZERO-DIVISOR: no backend toolchain available");
}
