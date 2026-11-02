//! # QF_BV through the Constraint-VM, against independent oracles (VER10)
//!
//! `.smt2` problems are read by `smt-lib-format`, executed by this crate's
//! [`Vm`], and decided by `constraint-engine`'s BV tactic (bit-blasting plus
//! CDCL). The tactic only reports UNSAT after the trusted kernel
//! `drat-check` has accepted its proof. This test adds checks that share no
//! code with the blaster, the encoder or the solver:
//!
//! - **SAT:** the model is plugged into a concrete evaluator (below, plain
//!   `u128` arithmetic) and every assertion in scope must come out true.
//! - **UNSAT:** where the declared variables total 16 bits or fewer, brute
//!   force over every assignment must find no model either.
//!
//! Two sources of problems:
//!
//! 1. **The corpus** in `tests/qfbv/*.smt2`. Each file states its expected
//!    answers in a `; expect:` line. Those expectations were derived
//!    in-house, from identities that hold by construction or by brute force.
//!    No external solver was involved (VER10 §6).
//! 2. **Random problems**, small enough to brute-force completely.

use std::collections::HashMap;

use constraint_core::{Predicate, Sort};
use constraint_engine::{SolverResult, Value};
use constraint_instructions::{ConstraintInstr, Program};
use constraint_vm::Vm;

// ---------------------------------------------------------------------------
// The concrete evaluator: SMT-LIB QF_BV semantics on u128.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum C {
    B(bool),
    V(u32, u128), // (width, value)
}

fn mask(w: u32) -> u128 {
    if w >= 128 {
        u128::MAX
    } else {
        (1 << w) - 1
    }
}

fn signed(x: u128, w: u32) -> i128 {
    if x >> (w - 1) & 1 == 1 {
        x as i128 - (1i128 << w)
    } else {
        x as i128
    }
}

fn eval(p: &Predicate, env: &HashMap<String, C>) -> C {
    let bv = |q: &Predicate| match eval(q, env) {
        C::V(w, v) => (w, v),
        C::B(_) => panic!("expected a bit-vector: {q:?}"),
    };
    let b = |q: &Predicate| match eval(q, env) {
        C::B(x) => x,
        C::V(..) => panic!("expected a Boolean: {q:?}"),
    };
    match p {
        Predicate::Bool(x) => C::B(*x),
        Predicate::Var(n) if n.starts_with("#b") => {
            C::V(n.len() as u32 - 2, u128::from_str_radix(&n[2..], 2).unwrap())
        }
        Predicate::Var(n) if n.starts_with("#x") => {
            C::V(4 * (n.len() as u32 - 2), u128::from_str_radix(&n[2..], 16).unwrap())
        }
        Predicate::Var(n) => env[n],
        Predicate::Not(a) => C::B(!b(a)),
        Predicate::And(ps) => C::B(ps.iter().all(&b)),
        Predicate::Or(ps) => C::B(ps.iter().any(&b)),
        Predicate::Implies(x, y) => C::B(!b(x) || b(y)),
        Predicate::Iff(x, y) => C::B(b(x) == b(y)),
        Predicate::Eq(x, y) => C::B(eval(x, env) == eval(y, env)),
        Predicate::NEq(x, y) => C::B(eval(x, env) != eval(y, env)),
        Predicate::Ite(c, t, e) => {
            if b(c) {
                eval(t, env)
            } else {
                eval(e, env)
            }
        }
        Predicate::Apply { f, args } if f == "_" => match (&args[0], &args[1]) {
            (Predicate::Var(n), Predicate::Int(w)) => C::V(*w as u32, n[2..].parse().unwrap()),
            _ => panic!("bad indexed literal"),
        },
        Predicate::Apply { f, args } => {
            let (w, x) = bv(&args[0]);
            let m = mask(w);
            let rest: Vec<u128> = args[1..].iter().map(|q| bv(q).1).collect();
            let fold = |op: fn(u128, u128) -> u128| C::V(w, rest.iter().fold(x, |acc, &y| op(acc, y) & m));
            let y = rest.first().copied().unwrap_or(0);
            let shift = |y: u128| (y < w as u128).then_some(y as u32);
            match f.as_str() {
                "bvnot" => C::V(w, !x & m),
                "bvneg" => C::V(w, x.wrapping_neg() & m),
                "bvand" => fold(|a, b| a & b),
                "bvor" => fold(|a, b| a | b),
                "bvxor" => fold(|a, b| a ^ b),
                "bvadd" => fold(u128::wrapping_add),
                "bvsub" => fold(u128::wrapping_sub),
                "bvmul" => fold(u128::wrapping_mul),
                "bvshl" => C::V(w, shift(y).map_or(0, |k| (x << k) & m)),
                "bvlshr" => C::V(w, shift(y).map_or(0, |k| x >> k)),
                "bvashr" => C::V(w, (signed(x, w) >> shift(y).unwrap_or(w - 1)) as u128 & m),
                "bvult" => C::B(x < y),
                "bvule" => C::B(x <= y),
                "bvugt" => C::B(x > y),
                "bvuge" => C::B(x >= y),
                "bvslt" => C::B(signed(x, w) < signed(y, w)),
                "bvsle" => C::B(signed(x, w) <= signed(y, w)),
                "bvsgt" => C::B(signed(x, w) > signed(y, w)),
                "bvsge" => C::B(signed(x, w) >= signed(y, w)),
                other => panic!("evaluator: unknown {other}"),
            }
        }
        other => panic!("evaluator: unsupported {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Replaying a program and checking every answer.
// ---------------------------------------------------------------------------

/// Run `program`, check each answer against the oracles, and return the
/// answers as "sat"/"unsat".
fn run_and_verify(name: &str, program: &Program) -> Vec<&'static str> {
    let answers = Vm::new().execute(program).unwrap_or_else(|e| panic!("{name}: {e}")).sat_results;
    // Replay the commands to know the declarations and assertions in
    // scope at each check-sat.
    let mut decls: Vec<(String, Sort)> = Vec::new();
    let mut asserts: Vec<Predicate> = Vec::new();
    let mut scopes: Vec<(usize, usize)> = Vec::new();
    let mut verdicts = Vec::new();
    let mut next = answers.iter();
    for instr in program.instructions() {
        match instr {
            ConstraintInstr::DeclareVar { name, sort } => {
                decls.retain(|(n, _)| n != name); // a redeclaration replaces
                decls.push((name.clone(), sort.clone()));
            }
            ConstraintInstr::Assert { pred } => asserts.push(pred.clone()),
            ConstraintInstr::PushScope => scopes.push((decls.len(), asserts.len())),
            ConstraintInstr::PopScope => {
                let (d, a) = scopes.pop().unwrap();
                decls.truncate(d);
                asserts.truncate(a);
            }
            ConstraintInstr::CheckSat => match next.next().expect("one answer per check-sat") {
                SolverResult::Sat(model) => {
                    let env: HashMap<String, C> = decls
                        .iter()
                        .map(|(n, s)| {
                            let v = match (s, model.get(n).expect("every declared variable has a value")) {
                                (Sort::Bool, Value::Bool(x)) => C::B(*x),
                                (Sort::BitVec(w), v @ Value::BitVec(_)) => C::V(*w, v.as_bv_u128().unwrap()),
                                other => panic!("{name}: model sort mismatch {other:?}"),
                            };
                            (n.clone(), v)
                        })
                        .collect();
                    for a in &asserts {
                        assert_eq!(eval(a, &env), C::B(true), "{name}: model violates {a:?}");
                    }
                    verdicts.push("sat");
                }
                SolverResult::Unsat => {
                    if let Some(found) = brute_force(&decls, &asserts) {
                        panic!("{name}: UNSAT, but brute force found {found:?}");
                    }
                    verdicts.push("unsat");
                }
                SolverResult::Unknown(why) => panic!("{name}: Unknown: {why}"),
            },
            _ => {}
        }
    }
    verdicts
}

/// Search every assignment when the declarations total ≤ 16 bits. Returns
/// a model if one exists; `None` if none does or the space is too big.
fn brute_force(decls: &[(String, Sort)], asserts: &[Predicate]) -> Option<HashMap<String, C>> {
    let widths: Vec<u32> = decls
        .iter()
        .map(|(_, s)| match s {
            Sort::Bool => 1,
            Sort::BitVec(w) => *w,
            _ => unreachable!(),
        })
        .collect();
    let total: u32 = widths.iter().sum();
    if total > 16 {
        return None;
    }
    (0u128..1 << total).find_map(|bits| {
        let mut env = HashMap::new();
        let mut off = 0;
        for ((n, s), &w) in decls.iter().zip(&widths) {
            let v = bits >> off & mask(w);
            off += w;
            env.insert(n.clone(), if *s == Sort::Bool { C::B(v == 1) } else { C::V(w, v) });
        }
        asserts.iter().all(|a| eval(a, &env) == C::B(true)).then_some(env)
    })
}

#[test]
fn corpus_answers_match_expectations_and_are_verified() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/qfbv");
    let mut files: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    files.sort();
    let (mut sat, mut unsat) = (0, 0);
    for path in &files {
        let text = std::fs::read_to_string(path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let expected: Vec<&str> = text
            .lines()
            .find_map(|l| l.strip_prefix("; expect:"))
            .unwrap_or_else(|| panic!("{name}: no `; expect:` line"))
            .split_whitespace()
            .collect();
        let program = smt_lib_format::read(&text).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let got = run_and_verify(&name, &program);
        assert_eq!(got, expected, "{name}");
        sat += got.iter().filter(|v| **v == "sat").count();
        unsat += got.iter().filter(|v| **v == "unsat").count();
    }
    assert!(files.len() >= 15 && sat >= 8 && unsat >= 12, "{} files, {sat} sat, {unsat} unsat", files.len());
}

// ---------------------------------------------------------------------------
// Random problems, brute-forced completely.
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const VEC_OPS: [&str; 11] =
    ["bvadd", "bvsub", "bvmul", "bvand", "bvor", "bvxor", "bvshl", "bvlshr", "bvashr", "bvnot", "bvneg"];
const CMP_OPS: [&str; 9] = ["bvult", "bvule", "bvugt", "bvuge", "bvslt", "bvsle", "bvsgt", "bvsge", "="];

fn random_bv(rng: &mut Rng, w: u32, vars: &[&str], depth: u32) -> String {
    if depth == 0 || rng.below(3) == 0 {
        return if rng.below(3) == 0 {
            format!("(_ bv{} {w})", rng.next() as u128 & mask(w))
        } else {
            vars[rng.below(vars.len())].to_string()
        };
    }
    let op = VEC_OPS[rng.below(VEC_OPS.len())];
    let a = random_bv(rng, w, vars, depth - 1);
    if op == "bvnot" || op == "bvneg" {
        return format!("({op} {a})");
    }
    let b = random_bv(rng, w, vars, depth - 1);
    format!("({op} {a} {b})")
}

fn random_bool(rng: &mut Rng, w: u32, vars: &[&str], depth: u32) -> String {
    match rng.below(5) {
        0 if depth > 0 => format!("(not {})", random_bool(rng, w, vars, depth - 1)),
        1 if depth > 0 => {
            let (a, b) = (random_bool(rng, w, vars, depth - 1), random_bool(rng, w, vars, depth - 1));
            format!("({} {a} {b})", ["and", "or", "=>"][rng.below(3)])
        }
        _ => {
            let op = CMP_OPS[rng.below(CMP_OPS.len())];
            let (a, b) = (random_bv(rng, w, vars, 2), random_bv(rng, w, vars, 2));
            format!("({op} {a} {b})")
        }
    }
}

#[test]
fn random_problems_agree_with_brute_force() {
    let mut rng = Rng(0x0B17_B1A5_7E55);
    let vars = ["x", "y", "z"];
    let (mut sat, mut unsat) = (0, 0);
    for round in 0..400 {
        let w = 1 + rng.below(4) as u32;
        let mut text = String::from("(set-logic QF_BV)\n");
        for v in vars {
            text.push_str(&format!("(declare-const {v} (_ BitVec {w}))\n"));
        }
        for _ in 0..1 + rng.below(3) {
            text.push_str(&format!("(assert {})\n", random_bool(&mut rng, w, &vars, 2)));
        }
        text.push_str("(check-sat)\n");
        let program = smt_lib_format::read(&text).unwrap();
        // run_and_verify checks models by evaluation and UNSAT by DRAT and
        // brute force (3 × 4 bits ≤ 16).
        for v in run_and_verify(&format!("round {round}:\n{text}"), &program) {
            if v == "sat" {
                sat += 1
            } else {
                unsat += 1
            }
        }
    }
    assert!(sat > 50 && unsat > 50, "sat={sat} unsat={unsat}");
}

// ---------------------------------------------------------------------------
// Everything outside the supported fragment is refused, with a reason.
// ---------------------------------------------------------------------------

/// Run `text` and require its single check-sat to be Unknown, with a
/// reason containing `needle`.
fn refuse(text: &str, needle: &str) {
    let program = smt_lib_format::read(&format!("{text}(check-sat)")).unwrap_or_else(|e| panic!("{e:?}"));
    match Vm::new().execute(&program).unwrap().sat_results.as_slice() {
        [SolverResult::Unknown(why)] => assert!(why.contains(needle), "{text}: `{why}` lacks `{needle}`"),
        other => panic!("{text}: expected Unknown, got {other:?}"),
    }
}

#[test]
fn unsupported_input_is_refused_not_guessed() {
    let d = "(declare-const x (_ BitVec 4))(declare-const p Bool)";
    refuse("(declare-const x (_ BitVec 0))(assert true)", "not yet supported");
    refuse("(declare-const x (_ BitVec 513))(assert true)", "not yet supported");
    refuse("(declare-const x (_ BitVec 4))(declare-const n Int)(assert true)", "BitVec mixed with Int");
    refuse(&format!("{d}(assert x)"), "bit-vector where a Boolean");
    refuse(&format!("{d}(assert (bvult p x))"), "Boolean where a bit-vector");
    refuse(&format!("{d}(assert (= x p))"), "different sorts");
    refuse(&format!("{d}(assert (= x #b101))"), "different sorts or widths");
    refuse(&format!("{d}(assert (= x (ite p x #b101)))"), "`ite` arms");
    refuse(&format!("{d}(assert (= x (bvadd x #b101)))"), "`bvadd` on widths 4 and 3");
    refuse(&format!("{d}(assert (bvult x #b101))"), "`bvult` on widths");
    refuse(&format!("{d}(assert (bvult x))"), "takes 2 arguments");
    refuse(&format!("{d}(assert (= x (bvnot x x)))"), "takes 1 argument");
    refuse(&format!("{d}(assert (= x (bvadd x)))"), "at least 2");
    refuse(&format!("{d}(assert (= x (bvudiv x x)))"), "function `bvudiv`");
    refuse(&format!("{d}(assert (= x y))"), "undeclared `y`");
    refuse(&format!("{d}(assert (= x #q1))"), "malformed");
    refuse(&format!("{d}(assert (= x (_ bv16 4)))"), "does not fit");
    refuse(&format!("{d}(assert (< 1 2))"), "not supported");
}

#[test]
fn a_huge_unused_declaration_is_refused_without_allocating() {
    // No assertions: the engine's trivial-model fast path must not build a
    // four-billion-bit value.
    let program = smt_lib_format::read("(declare-const x (_ BitVec 4000000000))(check-sat)").unwrap();
    let out = Vm::new().execute(&program).unwrap();
    let model = out.sat_results[0].model().expect("an empty conjunction is SAT");
    assert!(model.get("x").is_none());
}

#[test]
fn boolean_connectives() {
    // Boolean `=`, Boolean `ite` and constants, through the reader, in QF_BV.
    let text = "(set-logic QF_BV)(declare-const p Bool)(declare-const q Bool)(declare-const c Bool)\
                (assert (= p (ite c q true)))(assert (not p))(assert (or false c))(check-sat)";
    let program = smt_lib_format::read(text).unwrap();
    assert_eq!(run_and_verify("bool connectives", &program), vec!["sat"]);
    // `Iff` never comes from the SMT-LIB reader (it reads `=`), so build it.
    let var = |n: &str| Predicate::Var(n.into());
    let iff = Predicate::Iff(Box::new(var("p")), Box::new(Predicate::Not(Box::new(var("p")))));
    let program = Program::new_unchecked(vec![
        ConstraintInstr::SetLogic { logic: constraint_core::Logic::QF_BV },
        ConstraintInstr::DeclareVar { name: "p".into(), sort: Sort::Bool },
        ConstraintInstr::Assert { pred: iff },
        ConstraintInstr::CheckSat,
    ]);
    assert_eq!(run_and_verify("p iff not p", &program), vec!["unsat"]);
}

#[test]
fn reset_and_redeclaration() {
    let text = "(declare-const x (_ BitVec 2))(assert (= x #b11))(check-sat)\
                (reset)(declare-const x Bool)(assert (not x))(check-sat)\
                (push 1)(declare-const y (_ BitVec 3))(assert (= y #b101))(check-sat)(pop 1)(check-sat)";
    let program = smt_lib_format::read(text).unwrap();
    let results = Vm::new().execute(&program).unwrap().sat_results;
    let value = |i: usize, n: &str| results[i].model().unwrap().get(n).cloned().unwrap();
    assert_eq!(value(0, "x").as_bv_u128(), Some(3));
    assert_eq!(value(1, "x"), Value::Bool(false));
    assert_eq!(value(2, "y").as_bv_u128(), Some(5));
    assert!(results[3].model().unwrap().get("y").is_none(), "y went out of scope");
    assert_eq!(Value::Bool(true).as_bv_u128(), None);
    assert_eq!(Value::BitVec(vec![true; 129]).as_bv_u128(), None);
    assert_eq!(value(2, "y").to_string(), "#b101");
}

/// Security review (VER10, round 1) wrong-answer inputs. Each was once
/// answered wrongly: some with a certified false UNSAT, some with a false
/// SAT. All must now be refused.
#[test]
fn review_regressions_are_refused() {
    // A declared variable named like a literal was read as the literal.
    refuse("(declare-const #b1 (_ BitVec 1))(assert (= #b1 #b0))", "reserved for bit-vector literals");
    // Bit-vector syntax with only Int variables declared reached the LIA
    // tactic, which reads what it cannot evaluate as false.
    refuse("(declare-const n Int)(assert (= n 1))(assert (= (bvadd #b01 #b01) #b10))", "BitVec mixed with Int");
    refuse("(declare-const n Int)(assert (= #b01 #b01))", "BitVec mixed with Int");
    refuse("(declare-const n Int)(assert (= n 1))(assert (not (= #b0001 #x1)))", "BitVec mixed with Int");
    // The same LIA flaw without any bit-vectors: an undeclared name.
    refuse("(declare-const n Int)(assert (= m m))", "outside the LIA fragment");
    refuse("(declare-const n Int)(assert (not (f n)))", "outside the LIA fragment");
    refuse("(declare-const n Int)(declare-const p Bool)(assert (= n p))", "outside the LIA fragment");
    // Bit-vector syntax with nothing declared goes to the BV tactic and is
    // decided, not misread.
    let program = smt_lib_format::read("(assert (= (bvadd #b01 #b01) #b10))(check-sat)").unwrap();
    assert_eq!(run_and_verify("ground BV", &program), vec!["sat"]);
}

/// Security review (VER10, round 2): Bool variables next to Int variables
/// were grounded as integers and compared with Booleans as "unknown", read
/// as false, and every one of these satisfiable problems came back UNSAT.
#[test]
fn bool_and_int_together_are_decided_correctly() {
    let d = "(declare-const n Int)(declare-const p Bool)";
    for (body, want_p) in [
        ("(assert (= p true))", true),
        ("(assert (= n 3))(assert (distinct p false))", true),
        ("(assert (= n 3))(assert (= p (> n 0)))", true),
        ("(assert (= n 3))(assert (= (ite p p p) true))", true),
        ("(assert (= n 3))(assert (= p (> n 5)))", false),
    ] {
        let program = smt_lib_format::read(&format!("{d}{body}(check-sat)")).unwrap();
        let out = Vm::new().execute(&program).unwrap();
        let model = out.sat_results[0].model().unwrap_or_else(|| panic!("{body}: {:?}", out.sat_results[0]));
        assert_eq!(model.get("p"), Some(&Value::Bool(want_p)), "{body}");
    }
    // Arithmetic near the i128 edge once saturated and turned a true
    // comparison false; such literals are now refused outright.
    refuse(
        "(declare-const n Int)(assert (= n 5))\
         (assert (> (+ n 170141183460469231731687303715884105727) (+ n 170141183460469231731687303715884105726)))",
        "beyond ±2^62",
    );
    refuse("(declare-const n Int)(assert (> (* 4611686018427387905 n) 0))", "beyond ±2^62");
    // Arithmetic over Booleans is ill-sorted even though `infer_sort`
    // accepts it.
    refuse("(declare-const n Int)(declare-const p Bool)(declare-const q Bool)(assert (= (+ p q) p))", "arithmetic on a Bool");
    refuse("(declare-const n Int)(declare-const p Bool)(assert (> p n))", "arithmetic on a Bool");
}
