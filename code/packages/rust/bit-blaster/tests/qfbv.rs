//! # The QF_BV front end, end to end, against independent oracles
//!
//! Every answer from `bit_blaster::smt` is checked by something that shares
//! no code with the blaster, the encoder or the solver:
//!
//! - **SAT:** the model is plugged into a concrete evaluator (below, plain
//!   `u128` arithmetic) and every assertion in scope must come out true.
//! - **UNSAT:** the attached DRAT proof must pass `drat-check`, the trusted
//!   kernel. Where the declared variables total 16 bits or fewer, brute
//!   force over every assignment must also find no model.
//!
//! Two sources of problems:
//!
//! 1. **The corpus** in `tests/qfbv/*.smt2`, read by `smt-lib-format`. Each
//!    file states its expected answers in a `; expect:` line. Those
//!    expectations were derived in-house, either from identities that hold
//!    by construction or by brute force. No external solver was involved
//!    (VER10 §6).
//! 2. **Random problems**, small enough to brute-force completely.

use std::collections::HashMap;

use bit_blaster::smt::{check_program, check_program_with, Answer, Limits, Value};
use cdcl_sat::write_dimacs;
use constraint_core::{Predicate, Sort};
use constraint_instructions::{ConstraintInstr, Program};

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
    let answers = check_program(program).unwrap_or_else(|e| panic!("{name}: {e}"));
    // Replay the commands to know the declarations and assertions in
    // scope at each check-sat.
    let mut decls: Vec<(String, Sort)> = Vec::new();
    let mut asserts: Vec<Predicate> = Vec::new();
    let mut scopes: Vec<(usize, usize)> = Vec::new();
    let mut verdicts = Vec::new();
    let mut next = answers.iter();
    for instr in program.instructions() {
        match instr {
            ConstraintInstr::DeclareVar { name, sort } => decls.push((name.clone(), sort.clone())),
            ConstraintInstr::Assert { pred } => asserts.push(pred.clone()),
            ConstraintInstr::PushScope => scopes.push((decls.len(), asserts.len())),
            ConstraintInstr::PopScope => {
                let (d, a) = scopes.pop().unwrap();
                decls.truncate(d);
                asserts.truncate(a);
            }
            ConstraintInstr::CheckSat => match next.next().expect("one answer per check-sat") {
                Answer::Sat(model) => {
                    let env: HashMap<String, C> = decls
                        .iter()
                        .map(|(n, s)| {
                            let v = match (s, &model[n]) {
                                (Sort::Bool, Value::Bool(x)) => C::B(*x),
                                (Sort::BitVec(w), v @ Value::BitVec(_)) => C::V(*w, v.as_u128().unwrap()),
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
                Answer::Unsat { cnf, proof } => {
                    drat_check::check(&write_dimacs(cnf), &proof.to_drat_text())
                        .unwrap_or_else(|e| panic!("{name}: certificate rejected: {e}"));
                    if let Some(found) = brute_force(&decls, &asserts) {
                        panic!("{name}: UNSAT, but brute force found {found:?}");
                    }
                    verdicts.push("unsat");
                }
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

fn refuse(text: &str, needle: &str) {
    let program = smt_lib_format::read(text).unwrap_or_else(|e| panic!("{e:?}"));
    match check_program(&program) {
        Err(e) => assert!(e.to_string().contains(needle), "{text}: `{e}` lacks `{needle}`"),
        Ok(a) => panic!("{text}: expected a refusal, got {a:?}"),
    }
}

#[test]
fn unsupported_input_is_refused_not_guessed() {
    let d = "(declare-const x (_ BitVec 4))(declare-const p Bool)";
    refuse("(declare-const n Int)", "sort Int");
    refuse("(declare-const x (_ BitVec 0))", "width 0");
    refuse("(declare-const x (_ BitVec 513))", "width 513");
    refuse("(declare-fun f ((_ BitVec 4)) (_ BitVec 4))", "uninterpreted function");
    refuse(&format!("{d}(assert x)"), "asserted a bit-vector");
    refuse(&format!("{d}(assert (bvult p x))"), "Boolean where a bit-vector");
    refuse(&format!("{d}(assert (not x))"), "bit-vector where a Boolean");
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
    refuse(&format!("{d}(assert (= x #b))"), "malformed");
    refuse(&format!("{d}(assert (= x #b102))"), "malformed");
    refuse(&format!("{d}(assert (= x (_ bv16 4)))"), "does not fit");
    refuse(&format!("{d}(assert (= x (_ bvx 4)))"), "indexed term");
    refuse(&format!("{d}(assert (= x (_ foo 4)))"), "indexed term");
    refuse(&format!("{d}(assert (= x (_ bv1 4 5)))"), "indexed term");
    refuse(&format!("{d}(assert (= x (_ bv1 -4)))"), "width -4");
    refuse(&format!("{d}(assert (= x (_ bv1 0)))"), "width 0");
    refuse(&format!("{d}(assert (= x (_ bv340282366920938463463374607431768211456 4)))"), "beyond 128");
    refuse(&format!("{d}(assert (< 1 2))"), "term");
    let wide = format!("(assert (= #x{} #x{}))", "f".repeat(200), "f".repeat(200));
    refuse(&wide, "width 800");
}

#[test]
fn limits_bound_the_work() {
    let text = "(declare-const a (_ BitVec 64))(declare-const b (_ BitVec 64))\
                (assert (= (bvmul a b) (bvmul b a)))(check-sat)";
    let program = smt_lib_format::read(text).unwrap();
    let small = Limits { max_width: 512, max_nodes: 1000 };
    let e = check_program_with(&program, small).unwrap_err();
    assert!(e.to_string().contains("1000 AIG nodes"), "{e}");
    assert_eq!(Limits::default(), Limits { max_width: 512, max_nodes: 4_000_000 });
}

#[test]
fn deep_predicates_and_unbalanced_programs_are_refused() {
    // Deeper than constraint-core's MAX_PREDICATE_DEPTH, built directly.
    let deep = (0..600).fold(Predicate::Bool(true), |acc, _| Predicate::Not(Box::new(acc)));
    let program = Program::new_unchecked(vec![ConstraintInstr::Assert { pred: deep }]);
    assert!(check_program(&program).unwrap_err().to_string().contains("deeper than"));
    let pop = Program::new_unchecked(vec![ConstraintInstr::PopScope]);
    assert!(check_program(&pop).unwrap_err().to_string().contains("pop without push"));
}

#[test]
fn reset_and_shadowing() {
    let text = "(declare-const x (_ BitVec 2))(assert (= x #b11))(check-sat)\
                (reset)(declare-const x Bool)(assert (not x))(check-sat)\
                (push 1)(declare-const x (_ BitVec 3))(assert (= x #b101))(check-sat)(pop 1)(check-sat)";
    let program = smt_lib_format::read(text).unwrap();
    let answers = check_program(&program).unwrap();
    let models: Vec<_> = answers
        .iter()
        .map(|a| match a {
            Answer::Sat(m) => m["x"].clone(),
            Answer::Unsat { .. } => panic!("all four are satisfiable"),
        })
        .collect();
    assert_eq!(models[0].as_u128(), Some(3));
    assert_eq!(models[1], Value::Bool(false));
    assert_eq!(models[2].as_u128(), Some(5));
    assert_eq!(models[3], Value::Bool(false));
    assert_eq!(Value::Bool(true).as_u128(), None);
    assert_eq!(Value::BitVec(vec![true; 129]).as_u128(), None);
}

#[test]
fn boolean_connectives() {
    // Boolean `=`, Boolean `ite` and constants, through the reader.
    let text = "(declare-const p Bool)(declare-const q Bool)(declare-const c Bool)\
                (assert (= p (ite c q true)))(assert (not p))(assert (or false c))(check-sat)";
    let program = smt_lib_format::read(text).unwrap();
    assert_eq!(run_and_verify("bool connectives", &program), vec!["sat"]);
    // `Iff` never comes from the SMT-LIB reader (it reads `=`), so build it.
    let var = |n: &str| Predicate::Var(n.into());
    let iff = Predicate::Iff(Box::new(var("p")), Box::new(Predicate::Not(Box::new(var("p")))));
    let program = Program::new_unchecked(vec![
        ConstraintInstr::DeclareVar { name: "p".into(), sort: Sort::Bool },
        ConstraintInstr::Assert { pred: iff },
        ConstraintInstr::CheckSat,
    ]);
    assert_eq!(run_and_verify("p iff not p", &program), vec!["unsat"]);
}
