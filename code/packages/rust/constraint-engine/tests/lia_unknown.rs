//! # Incomplete LIA searches answer `Unknown`, never `Unsat` (VER10 §8.3-4)
//!
//! The LIA tactic is a bounded search. When the search stops before it has
//! covered every value the variables could take, "nothing found" is not a
//! proof, and the tactic must say `Unknown`. The earlier false-`Unsat`
//! family (security review, #18808) all had one shape: a search that was
//! cut short (a sampled window, an exhausted budget, an unevaluable term,
//! a stale assignment) was read as exhaustive.
//!
//! These tests build that family from the other side. Each problem has a
//! known truth (satisfiable, or really unsatisfiable) and the test checks
//! the answer is *consistent* with it:
//!
//! * satisfiable → `Sat` with a model that really satisfies the assertions,
//!   or `Unknown`; never `Unsat`;
//! * unsatisfiable → `Unsat` or `Unknown`; never `Sat`.
//!
//! Cases tagged "must be Unknown" additionally pin the answer: the search
//! provably cannot finish, so any other answer would be a guess.

use constraint_core::{Predicate, Sort};
use constraint_engine::{Engine, SolverResult, Value};

fn b(p: Predicate) -> Box<Predicate> {
    Box::new(p)
}

fn var(n: &str) -> Predicate {
    Predicate::Var(n.into())
}

fn int(k: i128) -> Predicate {
    Predicate::Int(k)
}

fn engine(ints: &[&str], asserts: &[Predicate]) -> Engine {
    let mut e = Engine::new();
    for n in ints {
        e.declare_var(*n, Sort::Int);
    }
    for a in asserts {
        e.assert(a.clone());
    }
    e
}

fn cmp(f: fn(Box<Predicate>, Box<Predicate>) -> Predicate, v: &str, k: i128) -> Predicate {
    f(b(var(v)), b(int(k)))
}

fn is_unknown(r: &SolverResult) -> bool {
    matches!(r, SolverResult::Unknown(_))
}

/// Check the answer's consistency with a known truth. `holds` evaluates the
/// assertions on a model's integer values.
fn consistent(r: &SolverResult, satisfiable: bool, ints: &[&str], holds: impl Fn(&[i128]) -> bool, what: &str) {
    match r {
        SolverResult::Unsat => assert!(!satisfiable, "{what}: FALSE UNSAT on a satisfiable problem"),
        SolverResult::Sat(m) => {
            let vals: Vec<i128> = ints.iter().map(|n| m.get(n).and_then(Value::as_int).expect("model value")).collect();
            assert!(holds(&vals), "{what}: Sat model {vals:?} violates the assertions");
        }
        SolverResult::Unknown(_) => {}
    }
}

#[test]
fn a_fully_excluded_sample_window_is_unknown() {
    // x ≥ 0 with x ≠ 0, 1, …, 1000: the search samples [0, 1000] and every
    // sampled value is excluded; the model (1001, …) lies outside.
    let mut a = vec![cmp(Predicate::Ge, "x", 0)];
    a.extend((0..=1000).map(|k| cmp(Predicate::NEq, "x", k)));
    let r = engine(&["x"], &a).check_sat();
    assert!(is_unknown(&r), "must be Unknown, got {r:?}");
    // The same with an upper bound only, and with neither bound.
    let mut a = vec![cmp(Predicate::Le, "x", 7)];
    a.extend((-1000..=7).map(|k| cmp(Predicate::NEq, "x", k)));
    assert!(is_unknown(&engine(&["x"], &a).check_sat()));
    let a: Vec<Predicate> = (-500..=500).map(|k| cmp(Predicate::NEq, "x", k)).collect();
    assert!(is_unknown(&engine(&["x"], &a).check_sat()));
}

#[test]
fn a_model_beyond_a_wide_box_sample_is_unknown() {
    // Box [0, 10^6] (far wider than the 1000-value window), the model hidden
    // behind a disjunction propagation cannot read.
    let a = [
        cmp(Predicate::Ge, "x", 0),
        cmp(Predicate::Le, "x", 1_000_000),
        Predicate::Or(vec![cmp(Predicate::Eq, "x", 777_777), cmp(Predicate::Eq, "x", 888_888)]),
    ];
    let r = engine(&["x"], &a).check_sat();
    assert!(is_unknown(&r), "must be Unknown, got {r:?}");
    // And with the whole disjunction false: really UNSAT, but unprovable by
    // sampling, so Unknown (or Unsat); never Sat.
    let a = [
        cmp(Predicate::Ge, "x", 0),
        cmp(Predicate::Le, "x", 1_000_000),
        Predicate::Or(vec![cmp(Predicate::Eq, "x", -1), cmp(Predicate::Eq, "x", -2)]),
    ];
    let r = engine(&["x"], &a).check_sat();
    assert!(!r.is_sat(), "{r:?}");
}

#[test]
fn an_exhausted_budget_is_unknown_not_unsat() {
    // Three variables boxed in [0, 1000] each: 10^9 combinations, far beyond
    // the 2·10^6-call budget. The tactic may find the model (149 or 150 in
    // every variable) or give up; it must not call it UNSAT.
    let pick = |v: &str| Predicate::Or(vec![cmp(Predicate::Eq, v, 149), cmp(Predicate::Eq, v, 150)]);
    let mut a = vec![];
    for v in ["x", "y", "z"] {
        a.push(cmp(Predicate::Ge, v, 0));
        a.push(cmp(Predicate::Le, v, 1000));
        a.push(pick(v));
    }
    let r = engine(&["x", "y", "z"], &a).check_sat();
    let holds = |v: &[i128]| v.iter().all(|&x| x == 149 || x == 150);
    consistent(&r, true, &["x", "y", "z"], holds, "three boxed disjunctions");

    // Really unsatisfiable (no value is 149 or 150 AND below 100): the
    // search cannot prove it within budget either.
    let mut a = vec![];
    for v in ["x", "y", "z"] {
        a.push(cmp(Predicate::Ge, v, 0));
        a.push(cmp(Predicate::Le, v, 1000));
        a.push(Predicate::Or(vec![cmp(Predicate::Eq, v, -1), cmp(Predicate::Eq, v, -2)]));
    }
    let r = engine(&["x", "y", "z"], &a).check_sat();
    assert!(!r.is_sat(), "an unsatisfiable problem must never get a model: {r:?}");
}

#[test]
fn unevaluable_and_overflowing_terms_are_unknown_not_unsat() {
    // 2^100 · x + 2^100 · x overflows i128 when x is large; the search must
    // read "cannot evaluate" as Unknown, not as a failed constraint.
    let big = 1i128 << 100;
    let a = [
        cmp(Predicate::Ge, "x", 0),
        cmp(Predicate::Le, "x", 3),
        Predicate::Or(vec![
            Predicate::Eq(b(Predicate::Mul { coef: big, term: b(var("x")) }), b(int(big * 2))),
            cmp(Predicate::Eq, "x", 99),
        ]),
    ];
    let r = engine(&["x"], &a).check_sat();
    // Satisfiable at x = 2.
    consistent(&r, true, &["x"], |v| v[0] == 2, "scaled equality");
    // A product that overflows for every candidate: no model can be
    // verified, so never Unsat.
    let huge = i128::MAX / 2;
    let a = [
        cmp(Predicate::Ge, "x", 2),
        cmp(Predicate::Le, "x", 9),
        Predicate::Or(vec![
            Predicate::Eq(b(Predicate::Mul { coef: huge, term: b(var("x")) }), b(int(0))),
            cmp(Predicate::Eq, "x", 3),
        ]),
    ];
    let r = engine(&["x"], &a).check_sat();
    // x = 3 satisfies the second disjunct (the first is unevaluable there).
    assert!(!r.is_unsat(), "false UNSAT on a satisfiable problem: {r:?}");
}

#[test]
fn a_failed_branch_leaves_no_stale_assignment() {
    // x ∈ [0, 3], y ∈ [0, 3]; the constraint set rejects every y for x = 0
    // and x = 1, and accepts (2, 3). A stale value of y left behind by the
    // failed x = 0 branch would make a later branch judge constraints ground
    // under the wrong y and answer Unsat.
    let a = [
        cmp(Predicate::Ge, "x", 0),
        cmp(Predicate::Le, "x", 3),
        cmp(Predicate::Ge, "y", 0),
        cmp(Predicate::Le, "y", 3),
        Predicate::Or(vec![
            Predicate::And(vec![cmp(Predicate::Eq, "x", 2), cmp(Predicate::Eq, "y", 3)]),
            Predicate::And(vec![cmp(Predicate::Eq, "x", 7), cmp(Predicate::Eq, "y", 7)]),
        ]),
    ];
    let r = engine(&["x", "y"], &a).check_sat();
    consistent(&r, true, &["x", "y"], |v| v == [2, 3], "stale assignment");
    assert!(r.is_sat(), "a boxed two-variable problem is within budget and must be decided: {r:?}");
}

#[test]
fn random_unbounded_problems_never_contradict_the_truth() {
    // Random one- and two-variable problems with no box, whose truth is
    // decided over a window much wider than the sampled one for UNSAT claims
    // (a real proof has no model anywhere) and by evaluating the model for
    // SAT claims.
    let mut state = 0xC0FF_EE00_1234_5678u64;
    let mut next = move |n: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state % n
    };
    let (mut sat, mut unsat, mut unknown) = (0, 0, 0);
    for round in 0..200 {
        let mut asserts = Vec::new();
        for _ in 0..1 + next(3) {
            let (c1, c2) = (next(9) as i128 - 4, next(9) as i128 - 4);
            let k = next(61) as i128 - 30;
            let lhs = Predicate::Add(vec![
                Predicate::Mul { coef: c1, term: b(var("x")) },
                Predicate::Mul { coef: c2, term: b(var("y")) },
            ]);
            let atom = match next(7) {
                0 => Predicate::Le(b(lhs), b(int(k))),
                1 => Predicate::Lt(b(lhs), b(int(k))),
                2 => Predicate::Ge(b(lhs), b(int(k))),
                3 => Predicate::Gt(b(lhs), b(int(k))),
                4 => Predicate::Eq(b(lhs), b(int(k))),
                5 => Predicate::NEq(b(lhs), b(int(k))),
                _ => Predicate::Or(vec![Predicate::Eq(b(lhs.clone()), b(int(k))), Predicate::Gt(b(lhs), b(int(k + 5)))]),
            };
            asserts.push(atom);
        }
        let eval = |p: &Predicate, x: i128, y: i128| -> bool { holds(p, x, y) };
        let all = |x: i128, y: i128| asserts.iter().all(|p| eval(p, x, y));
        let r = engine(&["x", "y"], &asserts).check_sat();
        match &r {
            SolverResult::Sat(m) => {
                let (x, y) = (m.get("x").and_then(Value::as_int).unwrap(), m.get("y").and_then(Value::as_int).unwrap());
                assert!(all(x, y), "round {round}: model ({x}, {y}) violates {asserts:?}");
                sat += 1;
            }
            SolverResult::Unsat => {
                for x in -150..=150 {
                    for y in -150..=150 {
                        assert!(!all(x, y), "round {round}: FALSE UNSAT, ({x}, {y}) satisfies {asserts:?}");
                    }
                }
                unsat += 1;
            }
            SolverResult::Unknown(_) => unknown += 1,
        }
    }
    eprintln!("unbounded random: sat={sat} unsat={unsat} unknown={unknown}");
}

/// Evaluate a linear atom produced by the generator above on `(x, y)`.
fn holds(p: &Predicate, x: i128, y: i128) -> bool {
    let num = |q: &Predicate| -> i128 {
        fn go(q: &Predicate, x: i128, y: i128) -> i128 {
            match q {
                Predicate::Int(k) => *k,
                Predicate::Var(n) => {
                    if n == "x" {
                        x
                    } else {
                        y
                    }
                }
                Predicate::Add(ps) => ps.iter().map(|p| go(p, x, y)).sum(),
                Predicate::Mul { coef, term } => coef * go(term, x, y),
                other => panic!("unexpected {other:?}"),
            }
        }
        go(q, x, y)
    };
    match p {
        Predicate::Le(a, c) => num(a) <= num(c),
        Predicate::Lt(a, c) => num(a) < num(c),
        Predicate::Ge(a, c) => num(a) >= num(c),
        Predicate::Gt(a, c) => num(a) > num(c),
        Predicate::Eq(a, c) => num(a) == num(c),
        Predicate::NEq(a, c) => num(a) != num(c),
        Predicate::Or(ps) => ps.iter().any(|p| holds(p, x, y)),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn bit_vectors_mixed_with_ints_are_unknown() {
    // VER10 §5: a BitVec next to an Int has no combined theory.
    let mut e = Engine::new();
    e.declare_var("x", Sort::Int);
    e.declare_var("v", Sort::BitVec(4));
    e.assert(cmp(Predicate::Ge, "x", 0));
    e.assert(Predicate::Eq(b(var("v")), b(var("v"))));
    let r = e.check_sat();
    assert!(is_unknown(&r), "{r:?}");
}
