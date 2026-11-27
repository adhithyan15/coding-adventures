//! VER33 ADL-002: the gate model `code/specs/logic-gates-model.warrant` against the
//! LG02 corpus `code/specs/logic-gates-basic.warrant`.
//!
//! The Warrant checker types the model but never RUNS it, so this file carries a
//! small evaluator over the parsed AST (the real guarded parser reads both
//! documents; nothing here parses text).  It is a test oracle, not a Warrant
//! implementation: it handles exactly the forms the gate model uses.
//!
//! What it pins, in order:
//!
//! 1. the model document is `profile algorithm`, checks clean, and its claim names are
//!    exactly the 26 LG02 claim names plus the eight documented additions;
//! 2. every model claim holds: `Bool` binders exhaustively, `Int` binders over a fixed
//!    sample that includes negatives, 2, 255, 256 and large values; and every rejection
//!    claim's hypothesis is non-vacuous over that sample;
//! 3. **the oracle-versus-LG02 guard**: every LG02 `example` (26 truth-table rows and 13
//!    rejections) is replayed against the model function of the same name with the same
//!    integer arguments and must agree; the full truth tables are also restated from
//!    literals here and must agree on every `{0,1}` input;
//! 4. every gate rejects -1, 2, 255 and 2^40 in every argument position;
//! 5. negative controls: a model whose guard is `a > 1` (accepts -1) and a model with a
//!    broken NAND construction both FAIL the claims, so the claims have teeth.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::ParserLimits;
use coding_adventures_warrant_grammar::parser::{
    parse_document, BinaryOp, ClaimDecl, Declaration, Expression, Literal, ModelDecl, Pattern,
    Profile, TypeExpr, UnaryOp, WarrantFile,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

fn specs_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("repository root")
        .join("code/specs")
}

/// Run `f` on the parsed (and, for the model, checked) document read from `text`.
fn with_file<R>(text: &str, f: impl FnOnce(&WarrantFile<'_>) -> R) -> R {
    let doc = SourceDocument::admit(text.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    check_warrant(&file).expect("the document checks");
    f(&file)
}

fn read(name: &str) -> String {
    std::fs::read_to_string(specs_dir().join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

// ---------------------------------------------------------------------------
// A tiny evaluator over the AST.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum V {
    Int(i128),
    Bool(bool),
    Absent,
    Some(Box<V>),
    Ok(Box<V>),
    Err(Box<V>),
    Variant(String),
}

struct Machine<'f, 'a> {
    models: HashMap<&'a str, &'f ModelDecl<'a>>,
}

impl<'f, 'a> Machine<'f, 'a> {
    fn new(file: &'f WarrantFile<'a>) -> Self {
        let mut models = HashMap::new();
        for d in &file.declarations {
            if let Declaration::Model(m) = d {
                models.insert(m.name, m);
            }
        }
        Machine { models }
    }

    fn call(&self, name: &str, args: Vec<V>, depth: usize) -> V {
        assert!(depth < 64, "model call depth");
        let m = self
            .models
            .get(name)
            .unwrap_or_else(|| panic!("no model `{name}`"));
        assert_eq!(m.params.len(), args.len(), "arity of `{name}`");
        let mut env = HashMap::new();
        for (p, a) in m.params.iter().zip(args) {
            env.insert(p.name.to_string(), a);
        }
        self.eval(m.body.as_ref().expect("a model body"), &env, depth + 1)
    }

    fn eval(&self, e: &Expression<'_>, env: &HashMap<String, V>, depth: usize) -> V {
        match e {
            Expression::Grouped { inner, .. } => self.eval(inner, env, depth),
            Expression::Literal { lit, .. } => match lit {
                Literal::Bool(b) => V::Bool(*b),
                Literal::Int(n) | Literal::U64(n) => V::Int(i128::from(*n)),
                Literal::U8(n) => V::Int(i128::from(*n)),
                Literal::Fixed(_, n) => V::Int(*n),
                other => panic!("unsupported literal {other:?}"),
            },
            Expression::Unary { op, operand, .. } => match (op, self.eval(operand, env, depth)) {
                (UnaryOp::Not, V::Bool(b)) => V::Bool(!b),
                (UnaryOp::Neg, V::Int(n)) => V::Int(-n),
                (op, v) => panic!("bad unary {op:?} {v:?}"),
            },
            Expression::Binary {
                op, left, right, ..
            } => {
                // `and`, `or` and `implies` short-circuit.
                if matches!(op, BinaryOp::And | BinaryOp::Or | BinaryOp::Implies) {
                    let l = self.eval(left, env, depth);
                    let V::Bool(l) = l else {
                        panic!("non-Bool operand")
                    };
                    let short = match op {
                        BinaryOp::And => !l,
                        BinaryOp::Or => l,
                        _ => !l,
                    };
                    if short {
                        return V::Bool(!matches!(op, BinaryOp::And));
                    }
                    return self.eval(right, env, depth);
                }
                let (l, r) = (self.eval(left, env, depth), self.eval(right, env, depth));
                match (op, &l, &r) {
                    (BinaryOp::Eq, _, _) => V::Bool(l == r),
                    (BinaryOp::NotEq, _, _) => V::Bool(l != r),
                    (BinaryOp::Add, V::Int(a), V::Int(b)) => V::Int(a + b),
                    (BinaryOp::Sub, V::Int(a), V::Int(b)) => V::Int(a - b),
                    (BinaryOp::Mul, V::Int(a), V::Int(b)) => V::Int(a * b),
                    (BinaryOp::Lt, V::Int(a), V::Int(b)) => V::Bool(a < b),
                    (BinaryOp::Gt, V::Int(a), V::Int(b)) => V::Bool(a > b),
                    (BinaryOp::Le, V::Int(a), V::Int(b)) => V::Bool(a <= b),
                    (BinaryOp::Ge, V::Int(a), V::Int(b)) => V::Bool(a >= b),
                    _ => panic!("bad binary {op:?} {l:?} {r:?}"),
                }
            }
            Expression::Conditional {
                condition,
                then_branch,
                else_branch,
                ..
            } => match self.eval(condition, env, depth) {
                V::Bool(true) => self.eval(then_branch, env, depth),
                V::Bool(false) => self.eval(else_branch, env, depth),
                v => panic!("non-Bool condition {v:?}"),
            },
            Expression::Match { target, arms, .. } => {
                let v = self.eval(target, env, depth);
                for arm in arms {
                    let mut inner = env.clone();
                    if bind(&arm.pattern, &v, &mut inner) {
                        return self.eval(&arm.body, &inner, depth);
                    }
                }
                panic!("non-exhaustive match on {v:?}");
            }
            Expression::Reference { name, args, .. } => {
                let segs = &name.segments;
                match (segs.as_slice(), args) {
                    (["None"], None) => V::Absent,
                    (["Some"], Some(a)) => V::Some(Box::new(self.eval(&a[0], env, depth))),
                    (["Ok"], Some(a)) => V::Ok(Box::new(self.eval(&a[0], env, depth))),
                    (["Err"], Some(a)) => V::Err(Box::new(self.eval(&a[0], env, depth))),
                    ([n], None) if env.contains_key(*n) => env[*n].clone(),
                    ([ty, ctor], None) => V::Variant(format!("{ty}.{ctor}")),
                    ([f], Some(a)) => {
                        let vals = a.iter().map(|x| self.eval(x, env, depth)).collect();
                        self.call(f, vals, depth)
                    }
                    _ => panic!("unsupported reference {segs:?}"),
                }
            }
            other => panic!("unsupported expression {other:?}"),
        }
    }

    /// Evaluate a claim over its binders.  `Bool` binders are exhaustive, `Int`
    /// binders range over `ints`.  Returns `(cases that passed the hypothesis,
    /// failing assignments)`.
    fn claim(&self, c: &ClaimDecl<'_>, ints: &[i128]) -> (usize, Vec<Vec<V>>) {
        let Expression::Quantified {
            is_forall: true,
            binders,
            body,
            ..
        } = &c.body
        else {
            panic!("claim `{}` is not a forall", c.name)
        };
        let domains: Vec<Vec<V>> = binders
            .iter()
            .map(|b| match &b.param_type {
                TypeExpr::Named { name: "Bool", .. } => vec![V::Bool(false), V::Bool(true)],
                TypeExpr::Named { name: "Int", .. } => ints.iter().map(|n| V::Int(*n)).collect(),
                other => panic!("claim `{}`: unsupported binder type {other:?}", c.name),
            })
            .collect();
        let mut admitted = 0;
        let mut failures = Vec::new();
        let mut idx = vec![0usize; domains.len()];
        loop {
            let mut env = HashMap::new();
            let tuple: Vec<V> = idx
                .iter()
                .zip(&domains)
                .map(|(i, d)| d[*i].clone())
                .collect();
            for (b, v) in binders.iter().zip(&tuple) {
                env.insert(b.name.to_string(), v.clone());
            }
            let hypothesis = binders.iter().all(|b| match &b.refinement {
                Some(r) => self.eval(r, &env, 0) == V::Bool(true),
                None => true,
            });
            if hypothesis {
                admitted += 1;
                if self.eval(body, &env, 0) != V::Bool(true) {
                    failures.push(tuple);
                }
            }
            // odometer
            let mut k = 0;
            loop {
                if k == idx.len() {
                    return (admitted, failures);
                }
                idx[k] += 1;
                if idx[k] < domains[k].len() {
                    break;
                }
                idx[k] = 0;
                k += 1;
            }
        }
    }
}

fn bind(p: &Pattern<'_>, v: &V, env: &mut HashMap<String, V>) -> bool {
    match (p, v) {
        (Pattern::None, V::Absent) => true,
        (Pattern::Some(q), V::Some(x))
        | (Pattern::Ok(q), V::Ok(x))
        | (Pattern::Err(q), V::Err(x)) => bind(q, x, env),
        (Pattern::Wild, _) => true,
        (Pattern::Bool(b), V::Bool(x)) => b == x,
        (Pattern::Ctor { name, args }, _) if args.is_empty() && name.segments.len() == 1 => {
            env.insert(name.segments[0].to_string(), v.clone());
            true
        }
        _ => false,
    }
}

/// Samples for `Int` binders: both signs, the byte boundary, a wide value.
const INTS: &[i128] = &[
    -(1 << 40),
    -256,
    -255,
    -2,
    -1,
    0,
    1,
    2,
    3,
    127,
    128,
    254,
    255,
    256,
    1 << 40,
];

const LG02_CLAIMS: [&str; 26] = [
    "not_truth_table",
    "not_rejects_non_bit",
    "and_truth_table",
    "and_rejects_non_bit",
    "or_truth_table",
    "or_rejects_non_bit",
    "xor_truth_table",
    "xor_rejects_non_bit",
    "nand_truth_table",
    "nand_rejects_non_bit",
    "nor_truth_table",
    "nor_rejects_non_bit",
    "xnor_truth_table",
    "xnor_rejects_non_bit",
    "nand_not_truth_table",
    "nand_not_rejects_non_bit",
    "nand_and_truth_table",
    "nand_and_rejects_non_bit",
    "nand_or_truth_table",
    "nand_or_rejects_non_bit",
    "nand_xor_truth_table",
    "nand_xor_rejects_non_bit",
    "nand_not_equals_not",
    "nand_and_equals_and",
    "nand_or_equals_or",
    "nand_xor_equals_xor",
];

const ADDITIONS: [&str; 8] = [
    "nand_not_equals_not_over_int",
    "nand_and_equals_and_over_int",
    "nand_or_equals_or_over_int",
    "nand_xor_equals_xor_over_int",
    "nand_not_matches_ref",
    "nand_and_matches_ref",
    "nand_or_matches_ref",
    "nand_xor_matches_ref",
];

const GATES: [(&str, usize); 11] = [
    ("not_gate", 1),
    ("and_gate", 2),
    ("or_gate", 2),
    ("xor_gate", 2),
    ("nand_gate", 2),
    ("nor_gate", 2),
    ("xnor_gate", 2),
    ("nand_not", 1),
    ("nand_and", 2),
    ("nand_or", 2),
    ("nand_xor", 2),
];

fn claim_names(file: &WarrantFile<'_>) -> Vec<String> {
    file.declarations
        .iter()
        .filter_map(|d| match d {
            Declaration::Claim(c) => Some(c.name.to_string()),
            _ => None,
        })
        .collect()
}

fn ok(n: i128) -> V {
    V::Ok(Box::new(V::Int(n)))
}

fn reject() -> V {
    V::Err(Box::new(V::Variant("GateError.INVALID_INPUT".into())))
}

#[test]
fn the_model_is_an_algorithm_profile_document_with_the_expected_claims() {
    with_file(&read("logic-gates-model.warrant"), |file| {
        assert_eq!(file.profile, Profile::Algorithm);
        let mut got = claim_names(file);
        got.sort();
        let mut want: Vec<String> = LG02_CLAIMS
            .iter()
            .chain(ADDITIONS.iter())
            .map(|s| s.to_string())
            .collect();
        want.sort();
        assert_eq!(got, want);
        // Neither reserved word of VER33 5.1 names a claim or a module.
        assert!(got.iter().all(|n| n != "claim" && n != "gen"));
        assert_ne!(file.name, "claim");
        assert_ne!(file.name, "gen");
        // All eleven gate model functions exist with the LG02 arities.
        let m = Machine::new(file);
        for (g, arity) in GATES {
            assert_eq!(m.models[g].params.len(), arity, "{g}");
        }
    });
}

#[test]
fn the_model_claim_names_are_exactly_the_lg02_claim_names_plus_additions() {
    let lg02 = with_file(&read("logic-gates-basic.warrant"), claim_names);
    let mut lg02 = lg02;
    lg02.sort();
    let mut want: Vec<String> = LG02_CLAIMS.iter().map(|s| s.to_string()).collect();
    want.sort();
    assert_eq!(lg02, want, "LG02 changed: re-derive the model's claim list");
}

#[test]
fn every_model_claim_holds_and_no_hypothesis_is_vacuous() {
    with_file(&read("logic-gates-model.warrant"), |file| {
        let m = Machine::new(file);
        let mut seen = 0;
        for d in &file.declarations {
            if let Declaration::Claim(c) = d {
                let (admitted, failures) = m.claim(c, INTS);
                assert!(
                    failures.is_empty(),
                    "claim {} fails at {:?}",
                    c.name,
                    failures
                );
                assert!(admitted > 0, "claim {} is vacuous over the sample", c.name);
                if c.name.ends_with("_rejects_non_bit") {
                    // The rejection hypothesis must admit negatives too.
                    assert!(admitted >= 6, "claim {} admits only {admitted}", c.name);
                }
                seen += 1;
            }
        }
        assert_eq!(seen, 34);
    });
}

/// The LG02 examples, replayed against the model: the oracle-versus-LG02 guard.
#[test]
fn every_lg02_example_agrees_with_the_model() {
    let lg02_text = read("logic-gates-basic.warrant");
    let model_text = read("logic-gates-model.warrant");
    with_file(&lg02_text, |lg02| {
        with_file(&model_text, |model| {
            let lg = Machine::new(lg02);
            let m = Machine::new(model);
            let mut replayed = 0;
            for d in &lg02.declarations {
                let Declaration::Example(ex) = d else {
                    continue;
                };
                let Expression::Binary {
                    op: BinaryOp::Eq,
                    left,
                    right,
                    ..
                } = &ex.body
                else {
                    panic!("example {} is not `call == expected`", ex.name)
                };
                let Expression::Reference {
                    name,
                    args: Some(args),
                    ..
                } = &**left
                else {
                    panic!("example {}: the left side is not a call", ex.name)
                };
                let empty = HashMap::new();
                let vals: Vec<V> = args.iter().map(|a| lg.eval(a, &empty, 0)).collect();
                let want = lg.eval(right, &empty, 0);
                let got = m.call(name.segments[0], vals.clone(), 0);
                assert_eq!(got, want, "example {} ({:?})", ex.name, vals);
                replayed += 1;
            }
            // 26 truth-table rows (2 for not, 4 for each of six gates) and 13 rejections.
            assert_eq!(replayed, 39, "LG02's example count changed");
        })
    });
}

/// The truth tables, restated from literals (not from any Warrant operator), against
/// every gate on every `{0,1}` input.
#[test]
fn the_truth_tables_hold_on_every_bit_input() {
    with_file(&read("logic-gates-model.warrant"), |file| {
        let m = Machine::new(file);
        type BitFn = fn(i128, i128) -> i128;
        let f2: [(&str, BitFn); 9] = [
            ("and_gate", |a, b| a & b),
            ("or_gate", |a, b| a | b),
            ("xor_gate", |a, b| a ^ b),
            ("nand_gate", |a, b| 1 - (a & b)),
            ("nor_gate", |a, b| 1 - (a | b)),
            ("xnor_gate", |a, b| 1 - (a ^ b)),
            ("nand_and", |a, b| a & b),
            ("nand_or", |a, b| a | b),
            ("nand_xor", |a, b| a ^ b),
        ];
        for (g, f) in f2 {
            for a in 0..=1 {
                for b in 0..=1 {
                    assert_eq!(
                        m.call(g, vec![V::Int(a), V::Int(b)], 0),
                        ok(f(a, b)),
                        "{g}({a},{b})"
                    );
                }
            }
        }
        for g in ["not_gate", "nand_not"] {
            assert_eq!(m.call(g, vec![V::Int(0)], 0), ok(1), "{g}(0)");
            assert_eq!(m.call(g, vec![V::Int(1)], 0), ok(0), "{g}(1)");
        }
    });
}

#[test]
fn every_gate_rejects_other_inputs_in_every_position() {
    with_file(&read("logic-gates-model.warrant"), |file| {
        let m = Machine::new(file);
        for (g, arity) in GATES {
            for bad in [-1i128, 2, 255, 1 << 40, -(1 << 40)] {
                for pos in 0..arity {
                    for good in [0i128, 1] {
                        let mut args = vec![V::Int(good); arity];
                        args[pos] = V::Int(bad);
                        assert_eq!(m.call(g, args.clone(), 0), reject(), "{g}{args:?}");
                    }
                }
            }
        }
    });
}

/// A copy of the model text with one definition replaced, for the negative controls.
fn mutate(from: &str, to: &str) -> String {
    let text = read("logic-gates-model.warrant");
    assert!(text.contains(from), "mutation anchor missing: {from}");
    text.replacen(from, to, 1)
}

fn failing_claims(text: &str) -> Vec<String> {
    with_file(text, |file| {
        let m = Machine::new(file);
        let mut out = Vec::new();
        for d in &file.declarations {
            if let Declaration::Claim(c) = d {
                if !m.claim(c, INTS).1.is_empty() {
                    out.push(c.name.to_string());
                }
            }
        }
        out
    })
}

#[test]
fn a_gt_one_guard_accepts_minus_one_and_the_claims_catch_it() {
    // The wrong guard VER32 9.2 warns about: `x > 1` rejects 2 and 255 but not -1.
    let bad = mutate(
        "model to_bit(x: Int) -> Option<Bool> = if x == 0 then Some(false) else (if x == 1 then Some(true) else None)",
        "model to_bit(x: Int) -> Option<Bool> = if x > 1 then None else Some(x == 1)",
    );
    let failed = failing_claims(&bad);
    assert!(
        failed.contains(&"not_rejects_non_bit".to_string()),
        "{failed:?}"
    );
    assert!(
        failed.contains(&"and_rejects_non_bit".to_string()),
        "{failed:?}"
    );
}

#[test]
fn a_broken_nand_construction_is_caught_by_the_equivalence_claims() {
    // nand_or built from (nand_not a) and b instead of (nand_not b).
    let bad = mutate("Ok(nb) -> nand_gate(na, nb)", "Ok(nb) -> nand_gate(na, b)");
    let failed = failing_claims(&bad);
    assert!(
        failed.contains(&"nand_or_equals_or".to_string()),
        "{failed:?}"
    );
    assert!(
        failed.contains(&"nand_or_equals_or_over_int".to_string()),
        "{failed:?}"
    );
    assert!(
        failed.contains(&"nand_or_matches_ref".to_string()),
        "{failed:?}"
    );
    // The T3 claim over every Int also catches a construction that is right on bits
    // and wrong on errors: swallow the first NAND's error in nand_and.
    let bad = mutate(
        "Err(e) -> Err(e)\n    | Ok(n) -> nand_not(n))",
        "Err(e) -> Ok(0)\n    | Ok(n) -> nand_not(n))",
    );
    let failed = failing_claims(&bad);
    assert!(
        failed.contains(&"nand_and_rejects_non_bit".to_string()),
        "{failed:?}"
    );
    assert!(
        failed.contains(&"nand_and_equals_and_over_int".to_string()),
        "{failed:?}"
    );
    // ... but LG02's own bit-domain equivalence cannot see that: it is the reason the
    // `_over_int` claims exist.
    assert!(
        !failed.contains(&"nand_and_equals_and".to_string()),
        "{failed:?}"
    );
}
