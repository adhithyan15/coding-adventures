//! # The oracle-derived differential runner (SIR21 §P1)
//!
//! The [golden corpus](conformance.rs) pairs each Ruby program with a
//! **hand-typed** expected string. That is fine for structural programs, but for
//! *integer arithmetic* the expected value should not be typed by a human at all
//! — it should be **computed by the reference oracle**
//! ([`sir_conformance::oracle`], SIR21 §P2) and every backend measured against
//! *that*. This file is that differential runner: for each arithmetic case it
//!
//!   1. asks the oracle for the answer (`eval(op, lhs, rhs, arbitrary)`),
//!   2. generates the equivalent Ruby source (`puts(lhs op rhs)`),
//!   3. runs it through **every** backend's real toolchain, and
//!   4. asserts each backend's stdout equals the oracle's answer, byte for byte.
//!
//! No expected value is written by hand: the oracle is the single source of
//! truth, so a backend that disagrees is localised as `(case, backend)` and
//! cannot hide behind a second backend that shares its bug.
//!
//! ## Phase-1 net: the range every backend represents exactly *today*
//!
//! These cases deliberately stay inside the integer range all four current
//! backends reproduce exactly — Python is arbitrary-precision, but JavaScript
//! `Number` is an `f64` (exact only to 2⁵³) and Go/Rust use 64-bit integers, so
//! the *common* exact range is what "codify today's behaviour" means for
//! Phase 1. Values and results here stay well under 2⁵³.
//!
//! ## The frontier this runner made visible
//!
//! Probing beyond that range surfaced a **real, confirmed faithfulness gap**:
//! `10¹² * 10¹²` (= 10²⁴) prints `1000000000000000000000000` on Python
//! (correct — Ruby integers are arbitrary precision) but `1e+24` on JavaScript
//! (f64 precision loss) and `2003764205206896640` on Go and Rust (64-bit
//! wraparound). Only Python currently honours Ruby's arbitrary-precision
//! semantics. That divergence is exactly the "the type is the semantics"
//! problem SIR21 exists to fix, and closing it is the per-backend **`Bignum`**
//! lowering work (T4–T8). Until then this runner asserts only the range where
//! the backends already agree, so it stays green while telling the truth about
//! where they don't (see [`frontier_large_arbitrary_diverges`], `#[ignore]`d).

use semantic_ir::IntSpec;
use sir_conformance::oracle::{self, IntOp, Outcome};
use sir_conformance::{run_source, RunOutcome, Target};

/// One arithmetic case: two operands and the op joining them. The expected
/// output is **not** stored — it is derived from the oracle at test time.
struct ArithCase {
    name: &'static str,
    op: IntOp,
    lhs: i128,
    rhs: i128,
}

/// The Ruby (and every backend's) infix symbol for an op.
fn op_symbol(op: IntOp) -> &'static str {
    match op {
        IntOp::Add => "+",
        IntOp::Sub => "-",
        IntOp::Mul => "*",
    }
}

/// Cases whose operands *and* results stay inside the range every backend
/// represents exactly today (comfortably under 2⁵³). Operands are non-negative
/// literals so the generated Ruby source is unambiguous; negative *results* are
/// reached via subtraction (`5 - 8`), which every backend prints as `-3`.
const CASES: &[ArithCase] = &[
    ArithCase { name: "add_small", op: IntOp::Add, lhs: 2, rhs: 3 },
    ArithCase { name: "add_carry", op: IntOp::Add, lhs: 999_999, rhs: 1 },
    ArithCase { name: "add_big", op: IntOp::Add, lhs: 1_000_000_000_000, rhs: 1_000_000_000_000 },
    ArithCase { name: "sub_to_negative", op: IntOp::Sub, lhs: 5, rhs: 8 },
    ArithCase { name: "sub_to_zero", op: IntOp::Sub, lhs: 42, rhs: 42 },
    ArithCase { name: "sub_big", op: IntOp::Sub, lhs: 1_000_000_000_000, rhs: 1 },
    ArithCase { name: "mul_small", op: IntOp::Mul, lhs: 7, rhs: 6 },
    ArithCase { name: "mul_med", op: IntOp::Mul, lhs: 1_000_000, rhs: 1_000_000 },
    ArithCase { name: "mul_by_zero", op: IntOp::Mul, lhs: 123_456, rhs: 0 },
];

/// Derive the expected stdout for a case from the oracle. On the Phase-1 net
/// every case is arbitrary-precision and in the oracle's `i128` range, so the
/// outcome is always a definite `Value`; anything else is a corpus bug.
fn expected_for(c: &ArithCase) -> String {
    match oracle::eval(c.op, c.lhs, c.rhs, IntSpec::arbitrary()) {
        Outcome::Value(v) => v.to_string(),
        other => panic!(
            "arithmetic case `{}` is malformed: oracle returned {other:?}, expected a Value \
             (Phase-1 cases must be arbitrary-precision and within the oracle's range)",
            c.name
        ),
    }
}

#[test]
fn arithmetic_matches_oracle_on_every_backend() {
    let mut ran = 0usize;
    let mut skipped = 0usize;

    for c in CASES {
        let expected = expected_for(c);
        let ruby = format!("puts({} {} {})\n", c.lhs, op_symbol(c.op), c.rhs);

        for &target in Target::all() {
            match run_source(c.name, &ruby, target) {
                RunOutcome::Ran(out) => {
                    assert_eq!(
                        out,
                        expected,
                        "\nORACLE-DIFFERENTIAL FAILURE\n  case:    {} ({} {} {})\n  backend: {}\n  \
                         oracle:  {expected}\n  backend: {out}\n",
                        c.name,
                        c.lhs,
                        op_symbol(c.op),
                        c.rhs,
                        target.tag(),
                    );
                    ran += 1;
                }
                RunOutcome::Skipped(why) => {
                    eprintln!("skip {}/{}: {why}", c.name, target.tag());
                    skipped += 1;
                }
                RunOutcome::Failed(msg) => panic!(
                    "\nORACLE-DIFFERENTIAL ERROR\n  case:    {}\n  backend: {}\n  {msg}\n",
                    c.name,
                    target.tag(),
                ),
            }
        }
    }

    eprintln!(
        "oracle-differential: {} cases x {} backends = {ran} ran, {skipped} skipped",
        CASES.len(),
        Target::all().len(),
    );
    assert!(
        ran > 0,
        "no backend toolchain was available — the differential runner proved nothing"
    );
}

/// Every generated case's expected value comes from the oracle and is a plain
/// decimal string — a fast, toolchain-free guard that the corpus is well-formed
/// (runs even on a host with no backends at all).
#[test]
fn every_case_has_a_well_formed_oracle_expectation() {
    for c in CASES {
        let expected = expected_for(c);
        assert!(
            expected.chars().all(|ch| ch.is_ascii_digit() || ch == '-'),
            "case `{}` produced a non-numeric expectation `{expected}`",
            c.name
        );
        // Cross-check the oracle against Rust's own i128 math for these small,
        // in-range operands (an independent second opinion on the oracle).
        let native = match c.op {
            IntOp::Add => c.lhs + c.rhs,
            IntOp::Sub => c.lhs - c.rhs,
            IntOp::Mul => c.lhs * c.rhs,
        };
        assert_eq!(expected, native.to_string(), "oracle disagreed with native i128 on `{}`", c.name);
    }
}

/// The confirmed frontier: large arbitrary-precision integers do **not** yet
/// agree across backends (Python is correct; JS/Go/Rust lose precision or wrap).
/// Ignored so the suite stays green, but kept as executable documentation — run
/// it with `cargo test -- --ignored` to watch the divergence, and it becomes a
/// passing assertion once the `Bignum` per-backend lowering (T4–T8) lands.
#[test]
#[ignore = "bignum frontier: JS/Go/Rust lose arbitrary-precision beyond native range (T4–T8)"]
fn frontier_large_arbitrary_diverges() {
    // 10^12 * 10^12 = 10^24 — within the oracle's i128 range, beyond every
    // backend's native integer.
    let expected = expected_for(&ArithCase {
        name: "mul_10e24",
        op: IntOp::Mul,
        lhs: 1_000_000_000_000,
        rhs: 1_000_000_000_000,
    });
    assert_eq!(expected, "1000000000000000000000000");

    let ruby = "puts(1000000000000 * 1000000000000)\n";
    for &target in Target::all() {
        if let RunOutcome::Ran(out) = run_source("mul_10e24", ruby, target) {
            // Today only Python matches; when this assertion holds for *all*
            // targets, the bignum frontier is closed.
            assert_eq!(out, expected, "backend {} still diverges from the oracle", target.tag());
        }
    }
}

// ── The coverage gate (SIR21 §P5) ─────────────────────────────────────────
//
// The differential runner proves the cases it *has*. The coverage gate proves
// there are no *gaps*: for every operation the oracle can evaluate (`IntOp::ALL`,
// the set a frontend could emit), at least one conformance case must exist and
// pass on every backend that accepts it. This is the structural fix for the
// "a construct is emittable but a backend never implemented it, and no test
// noticed" class of bug — the same shape as the `case_eq` gap. An op that grows
// the oracle but gains no case fails CI as a *coverage* error, not silently.
//
// This is the arithmetic slice of the gate (op × backend). Extending it to the
// full `SirType`/feature surface of the golden corpus is a later slice.

/// Toolchain-free: every op the oracle supports is exercised by ≥1 case. Runs
/// even on a host with no backends, so a new op with no case fails immediately.
#[test]
fn coverage_gate_every_op_has_a_case() {
    for &op in IntOp::ALL {
        assert!(
            CASES.iter().any(|c| c.op == op),
            "COVERAGE GAP: op `{}` is in IntOp::ALL but no arithmetic case exercises it \
             — add a case to CASES (SIR21 §P5)",
            op.tag()
        );
    }
}

/// Toolchain-gated: every `(op, backend)` cell is actually *proven* — for each
/// op, a representative case runs on every available backend and matches the
/// oracle. A backend that accepts an op (its toolchain is present and the
/// program runs) but produces the wrong answer, or cannot run it at all, is a
/// coverage failure localised to `(op, backend)`.
#[test]
fn coverage_gate_every_op_backend_cell_is_proven() {
    let mut proven: std::collections::HashSet<(&str, &str)> = std::collections::HashSet::new();

    for &target in Target::all() {
        for &op in IntOp::ALL {
            // First case that exercises this op is the representative.
            let case = CASES.iter().find(|c| c.op == op).unwrap_or_else(|| {
                panic!("no case for op `{}` (see coverage_gate_every_op_has_a_case)", op.tag())
            });
            let expected = expected_for(case);
            let ruby = format!("puts({} {} {})\n", case.lhs, op_symbol(op), case.rhs);
            match run_source(case.name, &ruby, target) {
                RunOutcome::Ran(out) => {
                    assert_eq!(
                        out,
                        expected,
                        "COVERAGE FAILURE at ({}, {}): backend disagreed with the oracle",
                        op.tag(),
                        target.tag(),
                    );
                    proven.insert((op.tag(), target.tag()));
                }
                RunOutcome::Skipped(_) => {}
                RunOutcome::Failed(msg) => {
                    panic!("COVERAGE ERROR at ({}, {}): {msg}", op.tag(), target.tag())
                }
            }
        }
    }

    // On every backend that ran at least one op (its toolchain is present),
    // *all* ops must be proven — no accepted-but-untested cell.
    for &target in Target::all() {
        let backend_ran = IntOp::ALL.iter().any(|op| proven.contains(&(op.tag(), target.tag())));
        if backend_ran {
            for &op in IntOp::ALL {
                assert!(
                    proven.contains(&(op.tag(), target.tag())),
                    "COVERAGE GAP: backend `{}` ran some ops but not `{}` — every accepted op \
                     must have a passing case (SIR21 §P5)",
                    target.tag(),
                    op.tag()
                );
            }
        }
    }

    assert!(
        !proven.is_empty(),
        "no backend toolchain was available — the coverage gate proved nothing"
    );
}
