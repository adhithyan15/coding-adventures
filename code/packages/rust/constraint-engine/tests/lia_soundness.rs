//! # The LIA tactic never answers wrongly (LANG24 soundness contract)
//!
//! The LIA tactic is incomplete: it may say `Unknown`. It must never say
//! `Unsat` for a satisfiable problem (a false proof) or `Sat` with a model
//! that breaks an assertion. These tests hold it to that against brute
//! force, on random problems over two `Int` and two `Bool` variables.
//!
//! Every problem asserts a box `−20 ≤ x, y ≤ 20`, so brute force over the
//! box *is* the full truth. With the box the tactic's search is exhaustive
//! (and its interval propagation sees the box), so many answers are real
//! proofs; the test also checks that most problems get decided at all.

use constraint_core::{Predicate, Sort};
use constraint_engine::{Engine, SolverResult, Value};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn b(p: Predicate) -> Box<Predicate> {
    Box::new(p)
}

fn var(n: &str) -> Predicate {
    Predicate::Var(n.into())
}

/// A random integer term: variables, small constants, + − and scaling.
fn int_term(rng: &mut Rng, depth: u32) -> Predicate {
    if depth == 0 || rng.below(3) == 0 {
        return match rng.below(3) {
            0 => Predicate::Int(rng.below(41) as i128 - 20),
            1 => var("x"),
            _ => var("y"),
        };
    }
    match rng.below(4) {
        0 => Predicate::Add(vec![int_term(rng, depth - 1), int_term(rng, depth - 1)]),
        1 => Predicate::Sub(b(int_term(rng, depth - 1)), b(int_term(rng, depth - 1))),
        2 => Predicate::Mul { coef: rng.below(7) as i128 - 3, term: b(int_term(rng, depth - 1)) },
        _ => Predicate::Ite(b(bool_term(rng, depth - 1)), b(int_term(rng, depth - 1)), b(int_term(rng, depth - 1))),
    }
}

/// A random Boolean term: comparisons, Bool variables, connectives.
fn bool_term(rng: &mut Rng, depth: u32) -> Predicate {
    if depth == 0 || rng.below(3) == 0 {
        return match rng.below(8) {
            0 => var("p"),
            1 => var("q"),
            k => {
                let (l, r) = (b(int_term(rng, 1)), b(int_term(rng, 1)));
                match k {
                    2 => Predicate::Le(l, r),
                    3 => Predicate::Lt(l, r),
                    4 => Predicate::Ge(l, r),
                    5 => Predicate::Gt(l, r),
                    6 => Predicate::Eq(l, r),
                    _ => Predicate::NEq(l, r),
                }
            }
        };
    }
    match rng.below(6) {
        0 => Predicate::Not(b(bool_term(rng, depth - 1))),
        1 => Predicate::And(vec![bool_term(rng, depth - 1), bool_term(rng, depth - 1)]),
        2 => Predicate::Or(vec![bool_term(rng, depth - 1), bool_term(rng, depth - 1)]),
        3 => Predicate::Implies(b(bool_term(rng, depth - 1)), b(bool_term(rng, depth - 1))),
        4 => Predicate::Eq(b(var("p")), b(bool_term(rng, depth - 1))),
        _ => Predicate::NEq(b(var("q")), b(bool_term(rng, depth - 1))),
    }
}

// --- An independent evaluator: exact arithmetic, `None` on overflow. ---

#[derive(Clone, Copy, PartialEq, Debug)]
enum V {
    I(i128),
    B(bool),
}

fn eval(p: &Predicate, x: i128, y: i128, pv: bool, qv: bool) -> Option<V> {
    let e = |q: &Predicate| eval(q, x, y, pv, qv);
    let i = |q: &Predicate| match e(q)? {
        V::I(n) => Some(n),
        V::B(_) => None,
    };
    let t = |q: &Predicate| match e(q)? {
        V::B(v) => Some(v),
        V::I(_) => None,
    };
    Some(match p {
        Predicate::Int(n) => V::I(*n),
        Predicate::Bool(v) => V::B(*v),
        Predicate::Var(n) => match n.as_str() {
            "x" => V::I(x),
            "y" => V::I(y),
            "p" => V::B(pv),
            _ => V::B(qv),
        },
        Predicate::Add(ps) => V::I(ps.iter().try_fold(0i128, |acc, q| acc.checked_add(i(q)?))?),
        Predicate::Sub(a, c) => V::I(i(a)?.checked_sub(i(c)?)?),
        Predicate::Mul { coef, term } => V::I(coef.checked_mul(i(term)?)?),
        Predicate::Ite(c, a, d) => {
            if t(c)? {
                e(a)?
            } else {
                e(d)?
            }
        }
        Predicate::Le(a, c) => V::B(i(a)? <= i(c)?),
        Predicate::Lt(a, c) => V::B(i(a)? < i(c)?),
        Predicate::Ge(a, c) => V::B(i(a)? >= i(c)?),
        Predicate::Gt(a, c) => V::B(i(a)? > i(c)?),
        Predicate::Eq(a, c) => V::B(e(a)? == e(c)?),
        Predicate::NEq(a, c) => V::B(e(a)? != e(c)?),
        Predicate::Not(a) => V::B(!t(a)?),
        Predicate::And(ps) => V::B(ps.iter().map(&t).collect::<Option<Vec<_>>>()?.into_iter().all(|v| v)),
        Predicate::Or(ps) => V::B(ps.iter().map(&t).collect::<Option<Vec<_>>>()?.into_iter().any(|v| v)),
        Predicate::Implies(a, c) => V::B(!t(a)? || t(c)?),
        other => panic!("unexpected {other:?}"),
    })
}

fn holds(asserts: &[Predicate], x: i128, y: i128, p: bool, q: bool) -> bool {
    asserts.iter().all(|a| eval(a, x, y, p, q) == Some(V::B(true)))
}

fn engine(asserts: &[Predicate]) -> Engine {
    let mut e = Engine::new();
    for (n, s) in [("x", Sort::Int), ("y", Sort::Int), ("p", Sort::Bool), ("q", Sort::Bool)] {
        e.declare_var(n, s);
    }
    for a in asserts {
        e.assert(a.clone());
    }
    e
}

fn the_box() -> Vec<Predicate> {
    ["x", "y"]
        .iter()
        .flat_map(|v| {
            [
                Predicate::Ge(b(var(v)), b(Predicate::Int(-20))),
                Predicate::Le(b(var(v)), b(Predicate::Int(20))),
            ]
        })
        .collect()
}

#[test]
fn boxed_problems_are_never_answered_wrongly() {
    let (mut sat, mut unsat, mut unknown) = (0, 0, 0);
    for seed in 1..=3u64 {
        let mut rng = Rng(0x00B0_C5ED_0000_0000 + seed);
        for round in 0..400 {
            let mut asserts = the_box();
            for _ in 0..1 + rng.below(3) {
                asserts.push(bool_term(&mut rng, 3));
            }
            let truth = (-20..=20).any(|x| {
                (-20..=20).any(|y| [false, true].iter().any(|&p| [false, true].iter().any(|&q| holds(&asserts, x, y, p, q))))
            });
            match engine(&asserts).check_sat() {
                SolverResult::Sat(m) => {
                    let gi = |n: &str| m.get(n).and_then(Value::as_int).unwrap();
                    let gb = |n: &str| m.get(n).and_then(Value::as_bool).unwrap();
                    assert!(
                        holds(&asserts, gi("x"), gi("y"), gb("p"), gb("q")),
                        "seed {seed} round {round}: Sat model violates {asserts:?}"
                    );
                    sat += 1;
                }
                SolverResult::Unsat => {
                    assert!(!truth, "seed {seed} round {round}: false UNSAT for {asserts:?}");
                    unsat += 1;
                }
                SolverResult::Unknown(_) => unknown += 1,
            }
        }
    }
    eprintln!("boxed: sat={sat} unsat={unsat} unknown={unknown}");
    // The box makes the search exhaustive, so most problems get decided.
    assert!(sat > 300 && unsat > 100, "sat={sat} unsat={unsat} unknown={unknown}");
}

#[test]
fn unboxed_problems_are_never_answered_wrongly() {
    // Without the box nothing bounds the search; every Unsat must still be
    // right. Truth is checked over a wide window: an Unsat must have no
    // model there (a real proof has none anywhere).
    let mut rng = Rng(0x0000_0B0F_F0C5_0001);
    for round in 0..100 {
        let asserts: Vec<Predicate> = (0..1 + rng.below(3)).map(|_| bool_term(&mut rng, 3)).collect();
        match engine(&asserts).check_sat() {
            SolverResult::Sat(m) => {
                let gi = |n: &str| m.get(n).and_then(Value::as_int).unwrap();
                let gb = |n: &str| m.get(n).and_then(Value::as_bool).unwrap();
                assert!(holds(&asserts, gi("x"), gi("y"), gb("p"), gb("q")), "round {round}: {asserts:?}");
            }
            SolverResult::Unsat => {
                let found = (-40..=40).any(|x| {
                    (-40..=40).any(|y| [false, true].iter().any(|&p| [false, true].iter().any(|&q| holds(&asserts, x, y, p, q))))
                });
                assert!(!found, "round {round}: false UNSAT for {asserts:?}");
            }
            SolverResult::Unknown(_) => {}
        }
    }
}

/// The security review's false-UNSAT inputs: all satisfiable, none may be
/// answered UNSAT. Where a model is in the sampled windows, it is found.
#[test]
fn review_inputs_are_not_unsat() {
    let n = || b(var("x"));
    let int = |k: i128| b(Predicate::Int(k));
    let cases = [
        vec![Predicate::Lt(n(), int(0))],
        vec![Predicate::Not(b(Predicate::Ge(n(), int(0))))],
        vec![Predicate::Eq(b(Predicate::Mul { coef: 2, term: n() }), int(-4))],
        vec![Predicate::Eq(b(Predicate::Mul { coef: 2, term: n() }), int(4000))],
        vec![Predicate::Gt(b(Predicate::Add(vec![var("x"), var("x")])), int(2500))],
        vec![Predicate::Or(vec![Predicate::Eq(n(), int(-3)), Predicate::Eq(n(), int(2000))])],
        vec![Predicate::And(vec![
            Predicate::Ge(n(), int(0)),
            Predicate::Le(n(), int(5000)),
            Predicate::Eq(b(Predicate::Mul { coef: 3, term: n() }), int(9000)),
        ])],
        vec![
            Predicate::And(vec![Predicate::Ge(n(), int(-20)), Predicate::Le(n(), int(20))]),
            Predicate::Eq(int(6), b(Predicate::Mul { coef: -2, term: n() })),
        ],
    ];
    for (i, asserts) in cases.iter().enumerate() {
        let r = engine(asserts).check_sat();
        assert!(!r.is_unsat(), "case {i}: false UNSAT for {asserts:?}");
        if let SolverResult::Sat(m) = &r {
            let x = m.get("x").and_then(Value::as_int).unwrap();
            assert!(holds(asserts, x, 0, false, false), "case {i}");
        }
    }
    // The boxed and small cases are decided, not just "not wrong".
    assert!(engine(&cases[0]).check_sat().is_sat());
    assert!(engine(&cases[2]).check_sat().is_sat());
    assert!(engine(&cases[7]).check_sat().is_sat());
}

/// Proofs that need bound propagation, not just enumeration.
#[test]
fn propagation_proves_what_enumeration_cannot() {
    let int = |k: i128| b(Predicate::Int(k));
    // x > 5 ∧ y > 5 ∧ x + y < 10: no single variable is boxed.
    let a = vec![
        Predicate::Gt(b(var("x")), int(5)),
        Predicate::Gt(b(var("y")), int(5)),
        Predicate::Lt(b(Predicate::Add(vec![var("x"), var("y")])), int(10)),
    ];
    assert!(engine(&a).check_sat().is_unsat());
    // x = y + 3 ∧ y ≥ 0 ∧ x ≤ 2 (bounds flow through an equality).
    let a = vec![
        Predicate::Eq(b(var("x")), b(Predicate::Add(vec![var("y"), Predicate::Int(3)]))),
        Predicate::Ge(b(var("y")), int(0)),
        Predicate::Le(b(var("x")), int(2)),
    ];
    assert!(engine(&a).check_sat().is_unsat());
    // ¬(x < 0) ∧ ¬(x ≥ 10) ∧ x ≠ 0..9 written as a range violation.
    let a = vec![
        Predicate::Not(b(Predicate::Lt(b(var("x")), int(0)))),
        Predicate::Not(b(Predicate::Ge(b(var("x")), int(10)))),
        Predicate::Not(b(Predicate::And(vec![
            Predicate::Ge(b(var("x")), int(0)),
            Predicate::Lt(b(var("x")), int(10)),
        ]))),
    ];
    assert!(engine(&a).check_sat().is_unsat());
    // A cycle that only tightens by one per round: stopping is sound.
    let a = vec![
        Predicate::Lt(b(var("x")), b(var("y"))),
        Predicate::Lt(b(var("y")), b(var("x"))),
    ];
    assert!(!engine(&a).check_sat().is_sat());
}
