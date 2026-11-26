//! The S3b acceptance tests: the evaluator, the enumeration and the digests.
//!
//! Part 1 is the gate of VER04r 1.2: ct-compare compiled by this generator reproduces **all
//! eighteen** v1 digests (`inputs_sha256` and `expected_sha256` of the nine suites) of the
//! checked-in `ct-compare-warrant-v1/corpus.json`, with the same `cases`, `discarded` and
//! `expected_calls` (572,109 cases in all).  The v1 corpus was produced by an *independent*
//! program (the v1 compiler in `warrant-grammar`), and the check below is made three ways:
//!
//! * against **pinned constants**, which a test also confirms equal the checked-in corpus
//!   bytes;
//! * against a **hand-written reproduction** in this file: its own descriptor lists written
//!   from VER04i 2.6, its own stream encoder, its own reading of the ct-compare models
//!   (`eq` is `==`, `select_picks` is `Ok(if choice then a else b)`), enumerating with
//!   `warrant_grammar::domain::ProductIter` and nothing else of this crate;
//! * against the **`warrant-grammar` `Evaluator`** (the AST evaluator the v1 compiler used)
//!   on sampled cases, as a differential for the new evaluator.
//!
//! Part 2 holds the structural properties (chunks, prefix sums, chunk independence).  Part 3
//! are the seeded-wrong negative controls: a wrong evaluator op, a reordered enumeration, an
//! off-by-one discard and a wrong wrap must each change a digest, so each would fail part 1.
//! Part 4 pins hand-computed numbers on the synthetic document, and part 5 the refusals.

mod support;

use std::collections::HashMap;
use std::sync::OnceLock;

use coding_adventures_sha256::{sha256, sha256_hex};
use coding_adventures_warrant_corpus_gen::analyse;
use coding_adventures_warrant_corpus_gen::domains::resolve_claim;
use coding_adventures_warrant_corpus_gen::enumerate::Enumeration;
use coding_adventures_warrant_corpus_gen::enumerate_document;
use coding_adventures_warrant_corpus_gen::error::{code, GenError};
use coding_adventures_warrant_corpus_gen::eval::{Program, Sabotage, Stats};
use coding_adventures_warrant_corpus_gen::ir::{Cmp, Ir};
use coding_adventures_warrant_corpus_gen::lower::{LoweredModel, LoweredSuite, SuiteKind};
use coding_adventures_warrant_corpus_gen::profile::{parse_profile, Profile};
use coding_adventures_warrant_corpus_gen::stream::{fnv1a64, Header};
use coding_adventures_warrant_corpus_gen::suite::{SuiteOptions, SuitePlan, SuiteResult};
use coding_adventures_warrant_corpus_gen::value::Value;
use coding_adventures_warrant_grammar::codegen::{CodegenLimits, EvalValue, Evaluator};
use coding_adventures_warrant_grammar::domain::{DomainEnv, ProductIter};
use coding_adventures_warrant_grammar::parser::{Declaration, Expression, WarrantFile};
use support::*;

const V1_CORPUS: &str =
    include_str!("../../../../specs/fixtures/ct-compare-warrant-v1/corpus.json");

/// `(claim, cases, discarded, expected_calls, inputs_sha256, expected_sha256)` of the nine
/// suites of `ct-compare-warrant-v1/corpus.json`.
const NINE: [(&str, u64, u64, u64, &str, &str); 9] = [
    (
        "eq_agrees",
        120409,
        0,
        120409,
        "73b983e125ba39dc8c311f407423a8f40cfcf2738e65151cdf177e36c384bc5b",
        "a243a089a7ad008af8f29fca5504a92c042db32cca3e70953d91e749baf9d856",
    ),
    (
        "eq_fixed_agrees_w0",
        1,
        0,
        1,
        "a7fcfe4d1e50746bac8d869473984a2d9a8e225cae85d3d7ef6444cb106353e7",
        "df473b27b8177a3a6fd686387c8bf2df8a4b38a73cec144545d797b154b6a86b",
    ),
    (
        "eq_fixed_agrees_w1",
        65536,
        0,
        65536,
        "84321254b2aa5362fb62c7706be1a3226ea2320c87cdb39d9b09731d89da22fc",
        "cea8d7a523ba9d4aca3db279c043b91c80bb63a9b8d27d5dc5c6c1fecc2c3e7c",
    ),
    (
        "eq_fixed_agrees_w2",
        256,
        0,
        256,
        "ac971579e63dfed711937587c204454a2a67155ecd299c77144881d9c54e2ffc",
        "df90a479a9ccf237e6f63e442563504b1cbbf81188bf6817832be5cbbf1dfb9f",
    ),
    (
        "eq_fixed_agrees_w4",
        256,
        0,
        256,
        "32ae5516698a95773ef0a77c580421c75f843b37f0588ad0b4a2e69fbbce955d",
        "19322d780e674a3d7d93170f7392631fd71d101bcd9be9084eb0c518a0643c6a",
    ),
    (
        "eq_fixed_agrees_w16",
        2704,
        0,
        2704,
        "ca7fa7a0045143bb88e88dc7b55748497a8562256d82733a1b2100cd3f21d3da",
        "0d491f50b120ef82d24b640151ede4dc7c7815e2803cc3a76ffcbde2abe548e9",
    ),
    (
        "select_picks",
        139818,
        101000,
        139818,
        "34cd91027287399c80d4f32829e4db758e32fb6d1136b8e4e4a0ff91e6ec37f5",
        "5ad94e796d12126f89ec944f80b39969eb0f647c554a5d054fb26c03de4663c5",
    ),
    (
        "select_rejects_mismatch",
        101000,
        139818,
        101000,
        "556b55cc8ee97ecbead08e69a1cfd1e447e81f60fc46985f777bd41bff838bff",
        "1944a5875c5c2f61768abb0a60e7320d91e100145ff845c12be1c6d1683c46c6",
    ),
    (
        "eq_u64_agrees",
        142129,
        0,
        142129,
        "4570ec1645c72514cb7af9123dc709f0beba9bd35e70108207eb72bd042f7aab",
        "f209973592654c02a8cafae36388ac7a03d34dce9d9c644b045e9ea248399f56",
    ),
];

fn id(claim: &str) -> String {
    format!("warrant/ct_compare/exhaustive/{claim}")
}

/// ct-compare enumerated once and shared by every test of this binary.
fn ct() -> &'static Enumeration {
    static CT: OnceLock<Enumeration> = OnceLock::new();
    CT.get_or_init(|| {
        with_file(CT_COMPARE, |f| {
            enumerate_document(f, &Profile::default()).unwrap()
        })
    })
}

fn hex_of(raw: &[u8]) -> String {
    raw.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap();
    }
    out
}

// ---------------------------------------------------------------------------
// Part 1: the 18 digests
// ---------------------------------------------------------------------------

/// The checked-in v1 corpus value of `inputs_sha256` for suite `claim`.
fn v1_file_inputs(claim: &str) -> String {
    let key = format!("\"id\":\"{}\",\"inputs_sha256\":\"", id(claim));
    let at = V1_CORPUS
        .find(&key)
        .unwrap_or_else(|| panic!("suite {claim}"))
        + key.len();
    V1_CORPUS[at..at + 64].to_string()
}

/// The checked-in v1 corpus value of `expected_sha256` for suite `claim` (the key sits just
/// before `"id"` in the sorted-key line).
fn v1_file_expected(claim: &str) -> String {
    let at = V1_CORPUS
        .find(&format!("\"id\":\"{}\"", id(claim)))
        .unwrap();
    let before = &V1_CORPUS[..at];
    assert!(before.ends_with("\","), "{claim}");
    let hex = &before[before.len() - 2 - 64..before.len() - 2];
    assert!(
        before[..before.len() - 2 - 64].ends_with("\"expected_sha256\":\""),
        "{claim}"
    );
    hex.to_string()
}

fn v1_file_count(claim: &str, key: &str) -> u64 {
    // `"cases":N,"claim":"<claim>"` and friends: find the suite object start, read the key
    let at = V1_CORPUS.find(&format!("\"claim\":\"{claim}\"")).unwrap();
    let start = V1_CORPUS[..at].rfind("{\"admit\"").unwrap();
    let obj = &V1_CORPUS[start..];
    let k = format!("\"{key}\":");
    let p = obj.find(&k).unwrap() + k.len();
    obj[p..].split([',', '}']).next().unwrap().parse().unwrap()
}

#[test]
fn the_eighteen_v1_digests_and_the_counts_are_reproduced() {
    let e = ct();
    assert_eq!(e.suites.len(), 9);
    let (mut cases, mut calls) = (0, 0);
    for (claim, n, disc, exp_calls, inputs, expected) in NINE {
        let s = e.suite(claim).unwrap_or_else(|| panic!("suite {claim}"));
        assert_eq!(s.id, id(claim));
        assert_eq!(
            s.v1_inputs_sha256.as_deref(),
            Some(inputs),
            "{claim} inputs"
        );
        assert_eq!(
            s.v1_expected_sha256.as_deref(),
            Some(expected),
            "{claim} expected"
        );
        assert_eq!(
            (s.cases, s.discarded, s.expected_calls),
            (n, disc, exp_calls),
            "{claim}"
        );
        assert_eq!(s.header, Header::V1, "{claim}: ct-compare is all v1 types");
        assert_eq!(s.wraps, 0, "{claim}: ct-compare has no model arithmetic");
        cases += s.cases;
        calls += s.expected_calls;
    }
    assert_eq!(cases, 572_109);
    assert_eq!(calls, 572_109);
    assert_eq!(e.cases(), 572_109);
    assert_eq!(e.discarded(), 240_818);
}

#[test]
fn the_pinned_constants_are_the_bytes_of_the_checked_in_v1_corpus() {
    for (claim, n, disc, calls, inputs, expected) in NINE {
        assert_eq!(v1_file_inputs(claim), inputs, "{claim}");
        assert_eq!(v1_file_expected(claim), expected, "{claim}");
        assert_eq!(v1_file_count(claim, "cases"), n, "{claim}");
        assert_eq!(v1_file_count(claim, "discarded"), disc, "{claim}");
        assert_eq!(v1_file_count(claim, "expected_calls"), calls, "{claim}");
    }
}

// -- the hand-written reproduction -------------------------------------------------------

const A: [u8; 4] = [0x00, 0x01, 0x80, 0xff];

fn strings(n: usize) -> Vec<Vec<u8>> {
    if n == 0 {
        return vec![Vec::new()];
    }
    let mut out = Vec::new();
    for s in strings(n - 1) {
        for b in A {
            let mut t = s.clone();
            t.push(b);
            out.push(t);
        }
    }
    out
}

fn bytes_d1() -> Vec<Vec<u8>> {
    let mut v = vec![Vec::new()];
    v.extend((0..=255u8).map(|b| vec![b]));
    v.extend(strings(2));
    v.extend(strings(3));
    for n in 4..=8 {
        v.push(vec![0x00; n]);
        v.push(vec![0xa5; n]);
    }
    v
}

fn bytes_n(n: usize) -> Vec<Vec<u8>> {
    match n {
        0 => vec![Vec::new()],
        1 => (0..=255u8).map(|b| vec![b]).collect(),
        2 | 3 => strings(n),
        _ => {
            let mut v: Vec<Vec<u8>> = A.iter().map(|b| vec![*b; n]).collect();
            for pos in 0..n {
                for s in [0x01u8, 0x80, 0xff] {
                    let mut x = vec![0u8; n];
                    x[pos] = s;
                    v.push(x);
                }
            }
            v
        }
    }
}

fn u64_edge() -> Vec<u64> {
    let mut v: Vec<u64> = (0..=255).collect();
    v.extend((8..64).map(|i| 1u64 << i));
    v.push(u64::MAX);
    v.extend((0..64).map(|i| u64::MAX ^ (1u64 << i)));
    v
}

#[derive(Clone)]
enum V {
    B(Vec<u8>),
    N(u64),
    T(bool),
}

fn put(out: &mut Vec<u8>, v: &V) {
    match v {
        V::T(b) => out.push(u8::from(*b)),
        V::N(n) => {
            out.push(0x10);
            out.extend_from_slice(&n.to_le_bytes());
        }
        V::B(b) => {
            out.push(0x20);
            out.extend_from_slice(&(b.len() as u32).to_le_bytes());
            out.extend_from_slice(b);
        }
    }
}

fn put_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u32).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

/// Independent reproduction of one v1 suite: enumerate the product by index arithmetic
/// (last binder fastest), admit by `admit`, encode both streams with the v1 encoding and
/// hash them whole.  Returns `(inputs, expected, cases, calls)`.
fn independent(
    claim: &str,
    types: &[&str],
    result: &str,
    cols: &[Vec<V>],
    admit: impl Fn(&[V]) -> bool,
    expected: impl Fn(&[V]) -> Vec<u8>,
) -> (String, String, u64) {
    let mut head = b"warrant-stream-v1\0".to_vec();
    put_str(&mut head, &id(claim));
    head.extend_from_slice(&(types.len() as u32).to_le_bytes());
    for t in types {
        put_str(&mut head, t);
    }
    put_str(&mut head, result);
    let (mut ins, mut exs) = (head.clone(), head);
    let total: usize = cols.iter().map(Vec::len).product();
    let mut cases = 0;
    for p in 0..total {
        let mut rest = p;
        let mut row: Vec<V> = Vec::new();
        for c in cols.iter().rev() {
            row.push(c[rest % c.len()].clone());
            rest /= c.len();
        }
        row.reverse();
        if admit(&row) {
            cases += 1;
            row.iter().for_each(|v| put(&mut ins, v));
            exs.extend(expected(&row));
        }
    }
    (sha256_hex(&ins), sha256_hex(&exs), cases)
}

fn blen(v: &V) -> usize {
    match v {
        V::B(b) => b.len(),
        _ => unreachable!(),
    }
}

#[test]
fn a_hand_written_reproduction_agrees_with_the_generator_on_all_nine_suites() {
    let e = ct();
    let eq = |r: &[V]| {
        let (V::B(a), V::B(b)) = (&r[0], &r[1]) else {
            unreachable!()
        };
        vec![u8::from(a == b)]
    };
    let to_cols = |d: Vec<Vec<u8>>| -> Vec<V> { d.into_iter().map(V::B).collect() };
    let mut results: Vec<(&str, (String, String, u64))> = Vec::new();

    let d1 = to_cols(bytes_d1());
    results.push((
        "eq_agrees",
        independent(
            "eq_agrees",
            &["Bytes", "Bytes"],
            "Bool",
            &[d1.clone(), d1.clone()],
            |_| true,
            eq,
        ),
    ));
    for n in [0usize, 1, 2, 4, 16] {
        let col = to_cols(bytes_n(n));
        let ty = format!("Bytes<{n}>");
        let name = format!("eq_fixed_agrees_w{n}");
        let r = independent(
            &name,
            &[&ty, &ty],
            "Bool",
            &[col.clone(), col],
            |_| true,
            eq,
        );
        results.push((NINE.iter().find(|x| x.0 == name).unwrap().0, r));
    }
    let choice = vec![V::T(false), V::T(true)];
    let select_ty = "Result<Bytes,SelectError>";
    let picks = |r: &[V]| {
        let (V::B(a), V::B(b), V::T(c)) = (&r[0], &r[1], &r[2]) else {
            unreachable!()
        };
        let mut out = vec![0x50, 0x20];
        let pick = if *c { a } else { b };
        out.extend_from_slice(&(pick.len() as u32).to_le_bytes());
        out.extend_from_slice(pick);
        out
    };
    let rejects = |_: &[V]| {
        let mut out = vec![0x51];
        put_str(&mut out, "CT_SELECT_LENGTH_MISMATCH");
        out
    };
    results.push((
        "select_picks",
        independent(
            "select_picks",
            &["Bytes", "Bytes", "Bool"],
            select_ty,
            &[d1.clone(), d1.clone(), choice.clone()],
            |r| blen(&r[0]) == blen(&r[1]),
            picks,
        ),
    ));
    results.push((
        "select_rejects_mismatch",
        independent(
            "select_rejects_mismatch",
            &["Bytes", "Bytes", "Bool"],
            select_ty,
            &[d1.clone(), d1, choice],
            |r| blen(&r[0]) != blen(&r[1]),
            rejects,
        ),
    ));
    let edge: Vec<V> = u64_edge().into_iter().map(V::N).collect();
    results.push((
        "eq_u64_agrees",
        independent(
            "eq_u64_agrees",
            &["U64", "U64"],
            "Bool",
            &[edge.clone(), edge],
            |_| true,
            |r| {
                let (V::N(a), V::N(b)) = (&r[0], &r[1]) else {
                    unreachable!()
                };
                vec![u8::from(a == b)]
            },
        ),
    ));
    assert_eq!(results.len(), 9);
    for (claim, (inputs, expected, cases)) in results {
        let s = e.suite(claim).unwrap();
        assert_eq!(
            s.v1_inputs_sha256.as_deref(),
            Some(inputs.as_str()),
            "{claim}"
        );
        assert_eq!(
            s.v1_expected_sha256.as_deref(),
            Some(expected.as_str()),
            "{claim}"
        );
        assert_eq!(s.cases, cases, "{claim}");
    }
}

// -- the warrant-grammar differential ------------------------------------------------------

/// The reference side (right of the top-level `==`) of the claim named `claim`.
fn reference_ast<'f, 'a>(file: &'f WarrantFile<'a>, claim: &str) -> &'f Expression<'a> {
    let Some(Declaration::Claim(c)) = file
        .declarations
        .iter()
        .find(|d| matches!(d, Declaration::Claim(c) if c.name == claim))
    else {
        panic!("claim {claim}")
    };
    let mut body = &c.body;
    while let Expression::Grouped { inner, .. } = body {
        body = inner;
    }
    if let Expression::Quantified { body: inner, .. } = body {
        body = inner;
    }
    let Expression::Binary { right, .. } = body else {
        panic!("{claim}: not a comparison")
    };
    right
}

#[test]
fn the_grammar_evaluator_agrees_with_this_evaluator_on_sampled_ct_compare_cases() {
    with_file(CT_COMPARE, |file| {
        let an = analyse(file, &Profile::default()).unwrap();
        let models = an.lowered.models.clone();
        let program = Program::new(&models);
        let env = DomainEnv::new(file);
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        let mut checked = 0u64;
        for suite in &an.lowered.suites {
            let SuiteKind::Reference { expected, .. } = &suite.kind else {
                panic!()
            };
            let doms = resolve_claim(file, &env, suite, 1 << 24).unwrap();
            let ast = reference_ast(file, &suite.claim);
            let domains: Vec<_> = doms.binders.iter().map(|b| b.domain.clone()).collect();
            for case in ProductIter::new(&domains, 0).unwrap().step_by(997) {
                let mut names: HashMap<&str, EvalValue> = HashMap::new();
                let mut vars: Vec<(&str, Value)> = Vec::new();
                for (b, v) in suite.binders.iter().zip(&case.values) {
                    names.insert(b.0.as_str(), v.clone());
                    vars.push((b.0.as_str(), Value::from_eval(v).unwrap()));
                }
                // admission is checked separately; evaluate the reference for every sample
                let mut st = Stats::default();
                let mine = program.eval(expected, &vars, &mut st).unwrap();
                ev.reset_work();
                let theirs = ev.eval(ast, &names).unwrap();
                assert_eq!(
                    mine,
                    Value::from_eval(&theirs).unwrap(),
                    "{} @ {}",
                    suite.claim,
                    case.ordinal
                );
                checked += 1;
            }
        }
        assert!(checked > 500, "{checked}");
    });
}

#[test]
fn the_grammar_evaluator_agrees_on_the_wrapping_v1_synthetic_document_and_differs_in_v0() {
    // v1: the AST evaluator wraps fixed-width integers too, so the two agree on every
    // sampled case of the wrapping claims.
    with_file(ENUM_V1, |file| {
        let p = parse_profile(b"chunk_size 1024\n").unwrap();
        let an = analyse(file, &p).unwrap();
        let models = an.lowered.models.clone();
        let program = Program::new(&models);
        let env = DomainEnv::new(file);
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        for claim in [
            "add_wraps",
            "sub_wraps",
            "mul_wraps",
            "neg_min",
            "twice_wraps",
        ] {
            let suite = an.lowered.suites.iter().find(|s| s.claim == claim).unwrap();
            let SuiteKind::Reference { expected, .. } = &suite.kind else {
                panic!()
            };
            let doms = resolve_claim(file, &env, suite, 1 << 24).unwrap();
            let domains: Vec<_> = doms.binders.iter().map(|b| b.domain.clone()).collect();
            let ast = reference_ast(file, claim);
            for case in ProductIter::new(&domains, 0).unwrap().step_by(37) {
                let mut names: HashMap<&str, EvalValue> = HashMap::new();
                let mut vars: Vec<(&str, Value)> = Vec::new();
                for (b, v) in suite.binders.iter().zip(&case.values) {
                    names.insert(b.0.as_str(), v.clone());
                    vars.push((b.0.as_str(), Value::from_eval(v).unwrap()));
                }
                let mine = program
                    .eval(expected, &vars, &mut Stats::default())
                    .unwrap();
                ev.reset_work();
                let theirs = Value::from_eval(&ev.eval(ast, &names).unwrap()).unwrap();
                assert_eq!(mine, theirs, "{claim} @ {}", case.ordinal);
            }
        }
    });
    // v0 (WD-013): the Rust v0 harness binds a `U8` as a `Nat`, i.e. as an exact integer.  The
    // AST `Evaluator` wraps an `EvalValue::U8` (so it agrees with this evaluator, below) but
    // does the exact sum when it is handed the value as a `Nat`: `inc(255)` is 256 there and
    // 0 here.  That is the difference the v0 guard (VER04r 3.3) exists for, so it is pinned.
    with_file(ENUM_V0, |file| {
        let an = analyse(file, &Profile::default()).unwrap();
        let models = an.lowered.models.clone();
        let program = Program::new(&models);
        let call = Ir::Model {
            name: "inc".into(),
            args: vec![Ir::Var("x".into())],
        };
        let vars = [("x", Value::Int(255))];
        let mut st = Stats::default();
        assert_eq!(program.eval(&call, &vars, &mut st).unwrap(), Value::Int(0));
        assert_eq!(st.wraps, 1);
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        let model = file
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Model(m) if m.name == "inc" => m.body.as_ref(),
                _ => None,
            })
            .unwrap();
        let as_nat: HashMap<&str, EvalValue> = HashMap::from([("x", EvalValue::Nat(255))]);
        let theirs = ev.eval(model, &as_nat).unwrap();
        assert_eq!(Value::from_eval(&theirs).unwrap(), Value::Int(256));
        let as_u8: HashMap<&str, EvalValue> = HashMap::from([("x", EvalValue::U8(255))]);
        let theirs = ev.eval(model, &as_u8).unwrap();
        assert_eq!(Value::from_eval(&theirs).unwrap(), Value::Int(0));
        // fed U8 values the two agree even in v0: U8 + U8 wraps in both
        let add = Ir::Model {
            name: "add8".into(),
            args: vec![Ir::Var("a".into()), Ir::Var("b".into())],
        };
        let two = [("a", Value::Int(255)), ("b", Value::Int(1))];
        assert_eq!(
            program.eval(&add, &two, &mut Stats::default()).unwrap(),
            Value::Int(0)
        );
        let add_ast = file
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Model(m) if m.name == "add8" => m.body.as_ref(),
                _ => None,
            })
            .unwrap();
        let env: HashMap<&str, EvalValue> =
            HashMap::from([("a", EvalValue::U8(255)), ("b", EvalValue::U8(1))]);
        assert_eq!(
            Value::from_eval(&ev.eval(add_ast, &env).unwrap()).unwrap(),
            Value::Int(0)
        );
    });
}

// ---------------------------------------------------------------------------
// Part 2: structure
// ---------------------------------------------------------------------------

#[test]
fn chunks_cover_the_product_and_the_counts_are_prefix_sums() {
    let e = ct();
    let m_expected = [
        ("eq_agrees", 2),
        ("eq_fixed_agrees_w0", 1),
        ("eq_fixed_agrees_w1", 1),
        ("eq_fixed_agrees_w2", 1),
        ("eq_fixed_agrees_w4", 1),
        ("eq_fixed_agrees_w16", 1),
        ("select_picks", 4),
        ("select_rejects_mismatch", 4),
        ("eq_u64_agrees", 3),
    ];
    for (claim, m) in m_expected {
        let s = e.suite(claim).unwrap();
        assert_eq!(s.chunks_total, m, "{claim}");
        assert_eq!(s.chunks.len() as u64, m);
        assert_eq!(s.chunk_size, 65_536);
        // m = ceil(P / chunk_size)
        assert_eq!(s.chunks_total, s.product.div_ceil(65_536).max(1));
        // discarded = P - sum(cases); first_admitted is the running prefix sum
        let mut running = 0;
        for (i, c) in s.chunks.iter().enumerate() {
            assert_eq!(c.index, i as u64);
            assert_eq!(c.first_admitted, running, "{claim} chunk {i}");
            running += c.cases;
        }
        assert_eq!(running, s.cases);
        assert_eq!(s.discarded, s.product - running, "{claim}");
        assert_eq!(
            s.chunks.iter().map(|c| c.calls).sum::<u64>(),
            s.expected_calls
        );
        // 2 + (..) chunks never exceed 16, so the slim set is every chunk (VER04r 5.6)
        assert_eq!(s.slim.chunks, (0..m).collect::<Vec<_>>(), "{claim}");
        assert_eq!(s.slim.inputs_sha256, s.inputs_sha256, "{claim}");
        assert_eq!(s.slim.expected_sha256, s.expected_sha256, "{claim}");
    }
}

#[test]
fn chunk_lines_are_the_section_5_5_format_and_the_suite_digest_is_the_framing_of_them() {
    for s in &ct().suites {
        let lines = s.chunk_lines();
        assert_eq!(lines.len() as u64, s.chunks_total);
        let mut in_digests = Vec::new();
        let mut ex_digests = Vec::new();
        for (i, l) in lines.iter().enumerate() {
            let f: Vec<&str> = l.split(' ').collect();
            assert_eq!(f.len(), 6, "{l}");
            assert_eq!(f[0], i.to_string());
            for hex in &f[4..] {
                assert_eq!(hex.len(), 64);
                assert!(hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));
            }
            in_digests.push(unhex(f[4]));
            ex_digests.push(unhex(f[5]));
        }
        // re-derive the suite digest from the LINES with the sha256 crate: framing is
        // "warrant-chunks-v1\0" || u32 m || raw digests
        let frame = |ds: &[[u8; 32]]| {
            let mut b = b"warrant-chunks-v1\0".to_vec();
            b.extend_from_slice(&(ds.len() as u32).to_le_bytes());
            ds.iter().for_each(|d| b.extend_from_slice(d));
            hex_of(&sha256(&b))
        };
        assert_eq!(frame(&in_digests), s.inputs_sha256, "{}", s.claim);
        assert_eq!(Some(frame(&ex_digests)), s.expected_sha256, "{}", s.claim);
    }
}

#[test]
fn descriptors_labels_and_result_types_of_ct_compare() {
    let e = ct();
    let d = |claim: &str| {
        e.suite(claim)
            .unwrap()
            .descriptors
            .iter()
            .map(|j| j.render().unwrap().split_whitespace().collect::<String>())
            .collect::<Vec<_>>()
    };
    assert_eq!(d("eq_agrees"), [r#"{"d":"bytes_d1","size":347}"#; 2]);
    assert_eq!(
        d("eq_fixed_agrees_w4"),
        [r#"{"d":"bytes_n","n":4,"size":16}"#; 2]
    );
    assert_eq!(d("eq_u64_agrees"), [r#"{"d":"u64_edge","size":377}"#; 2]);
    assert_eq!(d("select_picks")[2], r#"{"d":"bool","size":2}"#);
    let label = |c: &str| e.suite(c).unwrap().label.name();
    assert_eq!(label("eq_agrees"), "bounded-set");
    assert_eq!(label("eq_fixed_agrees_w0"), "full");
    assert_eq!(label("eq_fixed_agrees_w1"), "full");
    // Bytes<N> for N >= 2 is a sample (VER04r 9.3): the intended visible change from v1
    for c in [
        "eq_fixed_agrees_w2",
        "eq_fixed_agrees_w4",
        "eq_fixed_agrees_w16",
    ] {
        assert_eq!(label(c), "sample", "{c}");
    }
    assert_eq!(label("eq_u64_agrees"), "bounded-set");
    assert_eq!(
        e.suite("select_picks").unwrap().result_type,
        "Result<Bytes,SelectError>"
    );
    assert_eq!(e.suite("eq_agrees").unwrap().result_type, "Bool");
    assert_eq!(
        e.suite("eq_agrees").unwrap().binder_types,
        ["Bytes", "Bytes"]
    );
    assert_eq!(
        e.suite("eq_fixed_agrees_w16").unwrap().binder_types,
        ["Bytes<16>", "Bytes<16>"]
    );
    // ir_cost: eq_agrees is 1 (admit) + body + expected (VER04r 4.2: largest expression 10)
    assert!(e
        .suites
        .iter()
        .all(|s| s.ir_cost >= 1 && s.demoted.is_empty()));
    assert_eq!(e.suite("eq_agrees").unwrap().ir_cost, 1 + 10 + 6);
}

/// Plan one claim of `src` and hand the plan to `f`.
fn with_plan<R>(
    src: &str,
    profile: &Profile,
    claim: &str,
    f: impl FnOnce(&SuitePlan<'_>, &LoweredSuite) -> R,
) -> R {
    with_file(src, |file| {
        let an = analyse(file, profile).unwrap();
        let program = Program::new(&an.lowered.models);
        let env = DomainEnv::new(file);
        let i = an
            .lowered
            .suites
            .iter()
            .position(|s| s.claim == claim)
            .unwrap();
        let suite = &an.lowered.suites[i];
        let doms = resolve_claim(file, &env, suite, profile.budget).unwrap();
        let plan = SuitePlan::new(
            &program,
            suite,
            doms,
            an.suite_costs[i].total,
            profile.chunk_size,
        )
        .unwrap();
        f(&plan, suite)
    })
}

#[test]
fn every_chunk_enumerated_alone_equals_the_same_chunk_after_its_predecessors() {
    let e = ct();
    // chunk jumping: unrank c * chunk_size and enumerate from there (VER04r 5.4); the
    // select suites have a refinement, so first_admitted comes from the admission
    // arithmetic, not from a walk over the earlier chunks
    for claim in [
        "eq_agrees",
        "select_picks",
        "select_rejects_mismatch",
        "eq_u64_agrees",
    ] {
        let s = e.suite(claim).unwrap();
        with_plan(CT_COMPARE, &Profile::default(), claim, |plan, _| {
            for c in (0..s.chunks_total).rev() {
                let alone = plan.run_chunk(c, None).unwrap();
                assert_eq!(alone, s.chunks[c as usize], "{claim} chunk {c}");
            }
            assert_eq!(plan.total_cases(), s.cases);
            // admitted_before at every chunk boundary is the running prefix sum
            for c in &s.chunks {
                assert_eq!(plan.admitted_before(c.index * 65_536), c.first_admitted);
            }
            assert_eq!(plan.admitted_before(plan.product()), s.cases);
            assert_eq!(plan.admitted_before(0), 0);
            assert!(plan.run_chunk(s.chunks_total, None).is_err());
        });
    }
}

#[test]
fn the_enumeration_walker_visits_the_same_values_as_the_s2_product_iterator() {
    // For a suite with a refinement, hash the stream of the S2 `ProductIter` filtered by the
    // refinement evaluated with this crate's evaluator, and compare with the chunk digests:
    // the walker and the iterator are two different enumerations of the same product.
    with_file(CT_COMPARE, |file| {
        let p = Profile::default();
        let an = analyse(file, &p).unwrap();
        let program = Program::new(&an.lowered.models);
        let env = DomainEnv::new(file);
        let suite = an
            .lowered
            .suites
            .iter()
            .find(|s| s.claim == "select_rejects_mismatch")
            .unwrap();
        let doms = resolve_claim(file, &env, suite, p.budget).unwrap();
        let domains: Vec<_> = doms.binders.iter().map(|b| b.domain.clone()).collect();
        let mut admitted = 0u64;
        let mut first_chunk_cases = 0u64;
        for case in ProductIter::new(&domains, 0).unwrap() {
            let vars: Vec<(&str, Value)> = suite
                .binders
                .iter()
                .zip(&case.values)
                .map(|(b, v)| (b.0.as_str(), Value::from_eval(v).unwrap()))
                .collect();
            let ok = program
                .eval(&suite.admit, &vars, &mut Stats::default())
                .unwrap();
            if ok == Value::Bool(true) {
                admitted += 1;
                if case.ordinal < 65_536 {
                    first_chunk_cases += 1;
                }
            }
        }
        let s = ct().suite("select_rejects_mismatch").unwrap();
        assert_eq!(admitted, s.cases);
        assert_eq!(first_chunk_cases, s.chunks[0].cases);
    });
}

#[test]
fn the_full_ct_compare_enumeration_is_fast_enough_for_ci_in_the_debug_profile() {
    let t = std::time::Instant::now();
    let e = with_file(CT_COMPARE, |f| {
        enumerate_document(f, &Profile::default()).unwrap()
    });
    let took = t.elapsed();
    assert_eq!(e.cases(), 572_109);
    // VER04r budget for the task: 60 s on CI.  It is a few seconds; leave room for a slow
    // shared runner, and fail loudly (not flakily) only on a real regression.
    assert!(took.as_secs() < 60, "enumeration took {took:?}");
}

// ---------------------------------------------------------------------------
// Part 3: seeded-wrong negative controls
// ---------------------------------------------------------------------------

/// Run `claim` of `src` through the pipeline after `mutate` has tampered with its lowered
/// suite or resolved domains, under a (possibly wrong) evaluator.
fn tampered_try(
    src: &str,
    profile: &Profile,
    claim: &str,
    sabotage: Sabotage,
    mutate: impl FnOnce(
        &mut Vec<LoweredModel>,
        &mut LoweredSuite,
        &mut Vec<coding_adventures_warrant_corpus_gen::domains::BinderDomain>,
    ),
) -> Result<SuiteResult, GenError> {
    with_file(src, |file| {
        let an = analyse(file, profile).unwrap();
        let env = DomainEnv::new(file);
        let i = an
            .lowered
            .suites
            .iter()
            .position(|s| s.claim == claim)
            .unwrap();
        let mut models = an.lowered.models.clone();
        let mut suite = an.lowered.suites[i].clone();
        let mut doms = resolve_claim(file, &env, &suite, profile.budget).unwrap();
        mutate(&mut models, &mut suite, &mut doms.binders);
        let program = Program::new(&models).with_sabotage(sabotage);
        let plan = SuitePlan::new(
            &program,
            &suite,
            doms,
            an.suite_costs[i].total,
            profile.chunk_size,
        )
        .unwrap();
        plan.run(&SuiteOptions::default())
    })
}

fn tampered(
    src: &str,
    profile: &Profile,
    claim: &str,
    sabotage: Sabotage,
    mutate: impl FnOnce(
        &mut Vec<LoweredModel>,
        &mut LoweredSuite,
        &mut Vec<coding_adventures_warrant_corpus_gen::domains::BinderDomain>,
    ),
) -> SuiteResult {
    tampered_try(src, profile, claim, sabotage, mutate).unwrap()
}

fn pinned(claim: &str) -> &'static (&'static str, u64, u64, u64, &'static str, &'static str) {
    NINE.iter().find(|n| n.0 == claim).unwrap()
}

#[test]
fn control_with_no_tampering_still_reproduces_the_pinned_digests() {
    // the harness itself is honest: an untouched run through `tampered` equals the pin
    let r = tampered(
        CT_COMPARE,
        &Profile::default(),
        "eq_fixed_agrees_w2",
        Sabotage::None,
        |_, _, _| {},
    );
    let p = pinned("eq_fixed_agrees_w2");
    assert_eq!(r.v1_inputs_sha256.as_deref(), Some(p.4));
    assert_eq!(r.v1_expected_sha256.as_deref(), Some(p.5));
}

#[test]
fn control_a_wrong_evaluator_op_changes_the_expected_digest_only() {
    // `eq` answers `ne`: the pure side `same(a, b) = a == b` flips on every case
    let r = tampered(
        CT_COMPARE,
        &Profile::default(),
        "eq_fixed_agrees_w2",
        Sabotage::EqIsNe,
        |_, _, _| {},
    );
    let p = pinned("eq_fixed_agrees_w2");
    assert_eq!(
        r.v1_inputs_sha256.as_deref(),
        Some(p.4),
        "the inputs do not depend on the evaluator"
    );
    assert_ne!(r.v1_expected_sha256.as_deref(), Some(p.5));
    assert_ne!(
        r.expected_sha256,
        ct().suite("eq_fixed_agrees_w2").unwrap().expected_sha256
    );
    // a wrong op in the pure side of a bigger suite too
    let r = tampered(
        CT_COMPARE,
        &Profile::default(),
        "eq_u64_agrees",
        Sabotage::EqIsNe,
        |_, _, _| {},
    );
    assert_ne!(
        r.v1_expected_sha256.as_deref(),
        Some(pinned("eq_u64_agrees").5)
    );
}

#[test]
fn control_a_reordered_enumeration_changes_the_inputs_digest() {
    let r = tampered(
        CT_COMPARE,
        &Profile::default(),
        "eq_fixed_agrees_w2",
        Sabotage::None,
        |_, _, b| {
            b[0].sabotage_swap(0, 1);
        },
    );
    let p = pinned("eq_fixed_agrees_w2");
    assert_ne!(r.v1_inputs_sha256.as_deref(), Some(p.4));
    assert_ne!(
        r.inputs_sha256,
        ct().suite("eq_fixed_agrees_w2").unwrap().inputs_sha256
    );
    // the counts do not notice: only the digests do, which is why they are the evidence
    assert_eq!(r.cases, p.1);
    // swapping the LAST binder's values reorders within every row
    let r = tampered(
        CT_COMPARE,
        &Profile::default(),
        "eq_fixed_agrees_w2",
        Sabotage::None,
        |_, _, b| {
            b[1].sabotage_swap(2, 3);
        },
    );
    assert_ne!(r.v1_inputs_sha256.as_deref(), Some(p.4));
}

#[test]
fn control_an_off_by_one_discard_changes_counts_and_digests() {
    // `len(a) == len(b)` becomes `len(a) <= len(b)`: select_picks admits too many tuples
    let r = tampered(
        CT_COMPARE,
        &Profile::default(),
        "select_picks",
        Sabotage::None,
        |_, s, _| {
            let Ir::Cmp(op, ..) = &mut s.admit else {
                panic!("the admit of select_picks is a comparison")
            };
            assert_eq!(*op, Cmp::Eq);
            *op = Cmp::Le;
        },
    );
    let p = pinned("select_picks");
    assert!(r.cases > p.1);
    assert_eq!(r.cases + r.discarded, r.product);
    assert_ne!(r.v1_inputs_sha256.as_deref(), Some(p.4));
    assert_ne!(r.v1_expected_sha256.as_deref(), Some(p.5));
    // a one-tuple shift: `len(a) == len(b)` becomes `len(a) + 1 > len(b) and ...` is not
    // needed; dropping the admission altogether is the extreme
    let r = tampered(
        CT_COMPARE,
        &Profile::default(),
        "select_rejects_mismatch",
        Sabotage::None,
        |_, s, _| {
            s.admit = Ir::Bool(true);
        },
    );
    assert_eq!(r.cases, r.product);
    assert_ne!(
        r.v1_inputs_sha256.as_deref(),
        Some(pinned("select_rejects_mismatch").4)
    );
}

#[test]
fn control_a_wrong_wrap_changes_the_digest_and_the_wraps_count() {
    let p = small_profile();
    let good = ct_v1("add_wraps");
    assert_eq!(good.wraps, 32_640);
    // an evaluator that never wraps produces 256 for a U8 sum: the stream encoder refuses a
    // value outside its type, so a wrong wrap cannot even reach a digest
    let e = tampered_try(ENUM_V1, &p, "add_wraps", Sabotage::NoWrap, |_, _, _| {}).unwrap_err();
    assert_eq!(e.code, code::W0701);
    assert!(e.message.contains("range"), "{e}");
    // wrapping the wrong way: sub behaving as add
    let good = ct_v1("sub_wraps");
    let bad = tampered(ENUM_V1, &p, "sub_wraps", Sabotage::SubIsAdd, |_, _, _| {});
    assert_ne!(bad.expected_sha256, good.expected_sha256);
    assert_ne!(bad.v1_expected_sha256, good.v1_expected_sha256);
}

#[test]
fn control_a_model_body_changed_to_not_equal_changes_expected_but_not_inputs() {
    // `same(a, b) = a == b` becomes `a != b`: every expected Bool flips, the inputs do not
    let r = tampered(
        CT_COMPARE,
        &Profile::default(),
        "eq_fixed_agrees_w2",
        Sabotage::None,
        |models, _, _| {
            let same = models.iter_mut().find(|m| m.name == "same").unwrap();
            let Ir::Cmp(op, ..) = &mut same.body else {
                panic!("same is a comparison")
            };
            assert_eq!(*op, Cmp::Eq);
            *op = Cmp::Ne;
        },
    );
    let p = pinned("eq_fixed_agrees_w2");
    assert_eq!(r.v1_inputs_sha256.as_deref(), Some(p.4));
    assert_ne!(r.v1_expected_sha256.as_deref(), Some(p.5));
    assert_eq!(r.cases, p.1);
}

#[test]
fn control_swapped_or_dropped_binders_and_refinements_change_the_inputs_digest() {
    // `choice` moved to the front: the context prefix lists the types in binder order and the
    // product order changes, so the stream is a different stream
    let r = tampered(
        CT_COMPARE,
        &Profile::default(),
        "select_picks",
        Sabotage::None,
        |_, s, b| {
            s.binders.rotate_right(1);
            b.rotate_right(1);
        },
    );
    let p = pinned("select_picks");
    assert_eq!(r.cases, p.1, "the same tuples are admitted");
    assert_ne!(r.v1_inputs_sha256.as_deref(), Some(p.4));
    assert_ne!(r.v1_expected_sha256.as_deref(), Some(p.5));
    // a refinement dropped from select_picks admits everything
    let r = tampered(
        CT_COMPARE,
        &Profile::default(),
        "select_picks",
        Sabotage::None,
        |_, s, _| s.admit = Ir::Bool(true),
    );
    assert!(r.cases > p.1);
    assert_ne!(r.v1_inputs_sha256.as_deref(), Some(p.4));
}

// ---------------------------------------------------------------------------
// Part 4: hand-computed numbers on the synthetic document
// ---------------------------------------------------------------------------

fn small_profile() -> Profile {
    parse_profile(b"chunk_size 1024\n").unwrap()
}

/// The synthetic v1 document enumerated once at `chunk_size 1024`.
fn v1() -> &'static Enumeration {
    static E: OnceLock<Enumeration> = OnceLock::new();
    E.get_or_init(|| {
        with_file(ENUM_V1, |f| {
            enumerate_document(f, &small_profile()).unwrap()
        })
    })
}

fn ct_v1(claim: &str) -> &'static SuiteResult {
    v1().suite(claim).unwrap_or_else(|| panic!("{claim}"))
}

/// Count pairs `(a, b)` of `u8` for which `pred` holds.
fn pairs(pred: impl Fn(u32, u32) -> bool) -> u64 {
    (0..256u32)
        .flat_map(|a| (0..256u32).map(move |b| (a, b)))
        .filter(|(a, b)| pred(*a, *b))
        .count() as u64
}

#[test]
fn wraps_are_counted_by_hand() {
    // add8 wraps when a + b >= 256: for each a, the b >= 256 - a: sum of a = 32,640
    assert_eq!(ct_v1("add_wraps").wraps, 32_640);
    assert_eq!(pairs(|a, b| a + b >= 256), 32_640);
    // sub8 wraps when a < b
    assert_eq!(ct_v1("sub_wraps").wraps, pairs(|a, b| a < b));
    assert_eq!(ct_v1("sub_wraps").wraps, 32_640);
    assert_eq!(ct_v1("mul_wraps").wraps, pairs(|a, b| a * b >= 256));
    // -(-128i8) wraps: exactly one of the 256 I8 values
    assert_eq!(ct_v1("neg_min").wraps, 1);
    // a subject ARGUMENT is subject-independent: op_inc(a + 1u8) counts a=255's wrap, and
    // the expected side inc(a) counts its own: 2
    assert_eq!(ct_v1("arg_wraps").wraps, 2);
    // twice(a) = inc(inc(a)): a = 254 wraps the second inc, a = 255 the first: 2
    assert_eq!(ct_v1("twice_wraps").wraps, 2);
    // no arithmetic, no wraps
    for c in [
        "assoc_calls",
        "result_claim",
        "nat_len",
        "u64_edge",
        "u32_edge",
        "u16_full",
    ] {
        assert_eq!(ct_v1(c).wraps, 0, "{c}");
    }
}

#[test]
fn admission_wraps_count_once_per_product_ordinal() {
    // `a + b < 100u8` reads both binders: admit is evaluated at every ordinal, and wraps
    // when a + b >= 256 (32,640 ordinals, admitted or not); the admitted cases' expected
    // add8(a, b) wraps on the admitted ones with a + b >= 256
    let admitted = |a: u32, b: u32| (a + b) % 256 < 100;
    let s = ct_v1("refine_wraps_both");
    assert_eq!(s.cases, pairs(admitted));
    assert_eq!(
        s.wraps,
        32_640 + pairs(|a, b| admitted(a, b) && a + b >= 256)
    );
    // `a + 1u8 > 0u8` reads only the first binder: one evaluation per prefix tuple, its
    // wraps multiplied by the 256 ordinals the prefix stands for.  a = 255 wraps (and is the
    // only one discarded): 256 admit wraps + the expected side of the admitted cases
    let p = ct_v1("refine_wraps_prefix");
    assert_eq!(p.cases, 255 * 256);
    assert_eq!(p.discarded, 256);
    assert_eq!(p.wraps, 256 + pairs(|a, b| a < 255 && a + b >= 256));
}

#[test]
fn refinements_on_a_prefix_and_on_the_last_binder_count_the_same_way() {
    let prefix = ct_v1("refine_prefix");
    assert_eq!(prefix.cases, 155 * 256); // a in 101..=255
    assert_eq!(prefix.wraps, pairs(|a, b| a > 100 && a + b >= 256));
    let last = ct_v1("refine_last");
    assert_eq!(last.cases, pairs(|a, b| a <= b));
    assert_eq!(last.cases, 256 * 257 / 2);
    assert_eq!(last.wraps, pairs(|a, b| a <= b && a + b >= 256));
    // discarded = P - cases, chunk by chunk
    for s in [prefix, last] {
        assert_eq!(s.discarded, s.product - s.cases);
        let mut run = 0;
        for c in &s.chunks {
            assert_eq!(c.first_admitted, run);
            run += c.cases;
        }
    }
}

#[test]
fn expected_calls_follow_the_pure_trace() {
    // add(add(a, b), c) == add(a, add(b, c)): four calls per case (VER04r 4.4)
    let s = ct_v1("assoc_calls");
    assert_eq!(s.cases, 4096);
    assert_eq!(s.expected_calls, 4 * 4096);
    assert_eq!(s.kind, "predicate");
    assert_eq!(s.expected_sha256, None);
    assert_eq!(s.v1_expected_sha256, None);
    // a pure guard `b != 0` selects the branch whose calls count: one call per case with
    // b != 0, none otherwise (the case is still admitted)
    let g = ct_v1("guard_branch");
    assert_eq!(g.cases, 65_536);
    assert_eq!(g.expected_calls, 255 * 256);
    assert_eq!(g.wraps, pairs(|a, b| b != 0 && a + b >= 256));
    // the guard's untaken branch contributes no wraps either
    assert_eq!(g.kind, "predicate");
    // two nested subject calls: two per case
    assert_eq!(ct_v1("twice_wraps").expected_calls, 2 * 256);
    // a reference suite has one call per admitted case
    assert_eq!(ct_v1("add_wraps").expected_calls, ct_v1("add_wraps").cases);
}

#[test]
fn the_slim_set_follows_the_stride_formula_on_a_synthetic_suite_with_more_than_16_chunks() {
    // chunk_size 1024: the 65,536-ordinal suites have m = 64 chunks (> 16): k = ceil(64/14) = 5
    for claim in ["add_wraps", "refine_prefix", "u16_full", "guard_branch"] {
        let s = ct_v1(claim);
        assert_eq!(s.chunks_total, 64, "{claim}");
        let o = fnv1a64(&s.id) % 5;
        let mut want = vec![0u64, 63];
        want.extend((o..64).step_by(5));
        want.sort_unstable();
        want.dedup();
        assert_eq!(s.slim.chunks, want, "{claim}");
        assert!(s.slim.chunks.len() <= 16);
        // the slim digest is the framing over just those chunks' digests
        let digests: Vec<[u8; 32]> = want.iter().map(|c| s.chunks[*c as usize].inputs).collect();
        let mut buf = b"warrant-chunks-v1\0".to_vec();
        buf.extend_from_slice(&(digests.len() as u32).to_le_bytes());
        digests.iter().for_each(|d| buf.extend_from_slice(d));
        assert_eq!(s.slim.inputs_sha256, sha256_hex(&buf), "{claim}");
        assert_ne!(s.slim.inputs_sha256, s.inputs_sha256);
    }
    // a suite of at most 16 chunks is slim = full
    assert_eq!(ct_v1("assoc_calls").slim.chunks, [0, 1, 2, 3]);
    assert_eq!(
        ct_v1("assoc_calls").slim.inputs_sha256,
        ct_v1("assoc_calls").inputs_sha256
    );
}

#[test]
fn stream_headers_descriptors_and_labels_of_the_synthetic_suites() {
    let h = |c: &str| ct_v1(c).header;
    assert_eq!(h("add_wraps"), Header::V1);
    assert_eq!(h("result_claim"), Header::V1);
    assert_eq!(h("nat_len"), Header::V1);
    assert_eq!(h("u64_edge"), Header::V1);
    // I8, U16 and U32 are outside the v1 type set
    for c in ["neg_min", "u16_full", "u32_edge"] {
        assert_eq!(h(c), Header::V2, "{c}");
        assert_eq!(ct_v1(c).v1_inputs_sha256, None, "{c}");
        assert_eq!(ct_v1(c).v1_expected_sha256, None, "{c}");
    }
    let d = |c: &str| {
        ct_v1(c).descriptors[0]
            .render()
            .unwrap()
            .split_whitespace()
            .collect::<String>()
    };
    assert_eq!(d("neg_min"), r#"{"d":"fx_full","size":256,"t":"I8"}"#);
    assert_eq!(d("u16_full"), r#"{"d":"fx_full","size":65536,"t":"U16"}"#);
    assert_eq!(d("u32_edge"), r#"{"d":"fx_edge","size":313,"t":"U32"}"#);
    assert_eq!(d("u64_edge"), r#"{"d":"u64_edge","size":377}"#);
    assert_eq!(d("add_wraps"), r#"{"d":"u8_all","size":256}"#);
    assert_eq!(ct_v1("result_claim").product, 2 * 347);
    assert_eq!(ct_v1("nat_len").result_type, "Nat");
    assert_eq!(ct_v1("neg_min").result_type, "I8");
    assert_eq!(ct_v1("u16_full").label.name(), "full");
    assert_eq!(ct_v1("u32_edge").label.name(), "bounded-set");
}

#[test]
fn an_i8_suite_encodes_signed_values_as_sign_extended_two_complement() {
    // neg8(-128) = -128: the encoded expected value of the minimum case is 11 80ff..ff
    let s = ct_v1("neg_min");
    assert_eq!(s.cases, 256);
    // re-derive the expected digest independently: header v2, id, 1 binder I8, result I8
    let mut stream = b"warrant-stream-v2\0".to_vec();
    put_str(&mut stream, &s.id);
    stream.extend_from_slice(&1u32.to_le_bytes());
    put_str(&mut stream, "I8");
    put_str(&mut stream, "I8");
    stream.extend_from_slice(&0u32.to_le_bytes()); // chunk index 0
    let mut inputs = stream.clone();
    let mut expected = stream;
    for a in -128i64..=127 {
        inputs.push(0x11);
        inputs.extend_from_slice(&a.to_le_bytes());
        // -a wraps to itself for the minimum
        let neg = if a == -128 { -128i64 } else { -a };
        expected.push(0x11);
        expected.extend_from_slice(&neg.to_le_bytes());
    }
    assert_eq!(hex_of(&sha256(&inputs)), hex_of(&s.chunks[0].inputs));
    assert_eq!(
        hex_of(&sha256(&expected)),
        hex_of(&s.chunks[0].expected.unwrap())
    );
}

#[test]
fn wraps_do_not_depend_on_which_suite_ran_first() {
    // enumerate one claim on its own: the count equals the whole-document run's
    for claim in ["refine_wraps_both", "refine_wraps_prefix", "arg_wraps"] {
        let alone = with_plan(ENUM_V1, &small_profile(), claim, |plan, _| {
            plan.run(&SuiteOptions::default()).unwrap().wraps
        });
        assert_eq!(alone, ct_v1(claim).wraps, "{claim}");
    }
}

// ---------------------------------------------------------------------------
// Part 5: refusals and limits
// ---------------------------------------------------------------------------

/// ENUM_V1 up to its first claim: the models, ops and contracts.
fn v1_head() -> &'static str {
    &ENUM_V1[..ENUM_V1.find("  claim ").unwrap()]
}

fn v1_with(extra_models: &str, claims: &str) -> String {
    let head = v1_head().replace(
        "  error E = BAD\n",
        &format!("  error E = BAD\n{extra_models}"),
    );
    format!("{head}{claims}")
}

fn v0_with(claims: &str) -> String {
    format!("{}{claims}", &ENUM_V0[..ENUM_V0.find("  claim ").unwrap()])
}

fn enumerate_err(src: &str, profile: &Profile) -> GenError {
    with_file(src, |f| enumerate_document(f, profile).unwrap_err())
}

#[test]
fn a_v0_document_whose_u8_arithmetic_wraps_is_refused_rule_2() {
    let src = v0_with("  claim wraps:\n    forall a: U8, b: U8. op_add(a, b) == add8(a, b)\n");
    let e = enumerate_err(&src, &Profile::default());
    assert_eq!(e.code, code::W0701);
    assert_eq!(e.at, "wraps");
    assert!(e.message.contains("v0 harness evaluates U8 as Nat"), "{e}");
    // the shipped v0 synthetic document passes: its admitted cases never wrap
    with_file(ENUM_V0, |f| {
        assert!(enumerate_document(f, &Profile::default()).is_ok())
    });
}

#[test]
fn a_v0_wrap_in_admit_alone_is_refused_because_admit_counts_per_product_ordinal() {
    // the expected side is the binder `a`: no model arithmetic.  Only `admit` wraps, and it
    // does so at every ordinal with a + b >= 256, admitted or not.
    let src = v0_with(
        "  claim admit_wraps:\n    forall a: U8, b: U8 | a + b < 200u8. op_add(a, b) == a\n",
    );
    let e = enumerate_err(&src, &Profile::default());
    assert_eq!(e.code, code::W0701);
    assert!(e.message.contains("U8 arithmetic wraps"), "{e}");
    // the same claim without a wrap on any ordinal is fine
    let ok = v0_with(
        "  claim no_wrap:\n    forall a: U8 | a < 100u8, b: U8 | b < 100u8. op_add(a, b) == a\n",
    );
    with_file(&ok, |f| {
        assert!(enumerate_document(f, &Profile::default()).is_ok())
    });
}

#[test]
fn a_suite_that_admits_nothing_is_w0706() {
    let src = v1_with(
        "",
        "  claim none:\n    forall a: U8 | a > 255u8. op_inc(a) == inc(a)\n",
    );
    let e = enumerate_err(&src, &Profile::default());
    assert_eq!(e.code, code::W0706);
    assert_eq!(e.at, "none");
    // a constant-false admit (no binder read) is the same refusal
    let src = v1_with(
        "",
        "  claim none:\n    forall a: U8 | 1u8 > 2u8. op_inc(a) == inc(a)\n",
    );
    assert_eq!(enumerate_err(&src, &Profile::default()).code, code::W0706);
}

#[test]
fn a_product_over_the_budget_is_w0706_and_over_4096_chunks_is_a_limit() {
    // 347^3 = 41,781,923 > 2^24: W0706 from the S2 resolver, never silently sampled
    let src = v0_with(
        "  claim big:\n    forall a: Bytes, b: Bytes, c: Bytes. op_eq(a, b) == same(a, c)\n",
    );
    assert_eq!(enumerate_err(&src, &Profile::default()).code, code::W0706);
    // 2^24 ordinals at chunk_size 1024 is 16,384 chunks > 4,096
    let src = v1_with(
        "",
        "  claim wide:\n    forall a: U8, b: U8, c: U8. op_inc(a) == inc(b)\n",
    );
    let e = enumerate_err(&src, &small_profile());
    assert_eq!(e.code, code::LIMIT);
    assert!(e.message.contains("4096 chunks"), "{e}");
    // a budget below the product refuses too (v0: no demotion)
    let p = parse_profile(b"budget 1000\n").unwrap();
    let src = v0_with("  claim c:\n    forall a: Bytes, b: Bytes. op_eq(a, b) == same(a, b)\n");
    assert_eq!(enumerate_err(&src, &p).code, code::W0706);
}

#[test]
fn a_demoted_16_bit_binder_is_a_described_fx_edge_since_s10c_2() {
    // two U16 binders are 2^32 ordinals: the budget demotion makes the first an edge set, the
    // 16-bit `fx_edge` of [D25] (VER04r 16.2 item 5, settled), and records the demotion
    let src = v1_with(
        "",
        "  claim demoted:\n    forall a: U16, b: U16. op_u16(a) == id16(b)\n",
    );
    let en = with_file(&src, |f| enumerate_document(f, &small_profile()).unwrap());
    let s = &en.suites[0];
    assert_eq!(s.claim, "demoted");
    assert!(!s.demoted.is_empty(), "the budget demoted a binder");
    assert_eq!(s.demoted[0], "0", "the leftmost full 16-bit leaf goes first");
    let first = s.descriptors[0].render().unwrap();
    assert!(first.contains("\"d\": \"fx_edge\"") && first.contains("\"t\": \"U16\""), "{first}");
    assert!(first.contains("\"size\": 281"), "{first}");
    assert_eq!(s.label.name(), "bounded-set");
}

#[test]
fn an_exact_intermediate_past_2_to_127_is_w0701_at_generation() {
    // len(a) * (2^64-1) * (2^64-1) is 2^128-ish for len(a) = 1: outside the window
    let src = v1_with(
        "  model huge(n: Nat) -> Int = n * 18446744073709551615 * 18446744073709551615\n",
        "  claim big:\n    forall a: Bytes. op_len(a) == huge(len(a))\n",
    );
    let e = enumerate_err(&src, &small_profile());
    assert_eq!(e.code, code::W0701);
    assert!(e.message.contains("2^127"), "{e}");
}

#[test]
fn a_negative_or_oversized_expected_value_cannot_be_encoded() {
    // Nat - Nat is an Int: the expected value of `len(a) - 3` is negative for short inputs,
    // and a stream value must have its declared type (Nat is 0..=2^64-1)
    let src = v1_with(
        "  model short(n: Nat) -> Int = n - 3\n",
        "  claim neg:\n    forall a: Bytes. op_len(a) == short(len(a))\n",
    );
    let e = enumerate_err(&src, &small_profile());
    assert_eq!(e.code, code::W0701);
    assert_eq!(e.message, "a value is outside the range of Nat (VER04r 2.3)", "{e}");
}

#[test]
fn the_ir_cost_and_work_bounds_still_refuse_before_the_first_case() {
    // a model chain that doubles its cost per level: stopped by S3a's checked cost
    let mut models = String::from("  model c0(a: U8) -> U8 = a\n");
    for i in 1..=12 {
        models += &format!("  model c{i}(a: U8) -> U8 = c{p}(c{p}(a))\n", p = i - 1);
    }
    let src = v1_with(
        &models,
        "  claim deep:\n    forall a: U8. op_inc(a) == c12(a)\n",
    );
    assert_eq!(enumerate_err(&src, &Profile::default()).code, code::LIMIT);
}

#[test]
fn profile_lines_are_checked_against_the_document() {
    let base = |text: &str| {
        let p = parse_profile(text.as_bytes()).unwrap();
        with_file(ENUM_V0, |f| enumerate_document(f, &p))
    };
    // an unknown claim, a chunk past the last chunk, a stride that is not smaller
    assert_eq!(base("slim_chunk nope 0\n").unwrap_err().code, code::PROFILE);
    assert_eq!(
        base("slim_chunk bytes_eq 1\n").unwrap_err().code,
        code::PROFILE
    );
    assert!(base("slim_chunk bytes_eq 0\n").is_ok());
    // m <= 16: the stride is irrelevant and ignored
    assert!(base("slim_stride 3\n").is_ok());
    // m = 64 (the synthetic v1 document at chunk 1024): automatic stride 5.  A larger stride
    // is clamped per suite to 5 (owner decision 2026-11-17), never an error
    let with = |text: &str| {
        let p = parse_profile(text.as_bytes()).unwrap();
        with_file(ENUM_V1, |f| enumerate_document(f, &p))
    };
    for big in [5, 9, 1000] {
        let clamped = with(&format!("chunk_size 1024\nslim_stride {big}\n")).unwrap();
        for s in &clamped.suites {
            assert_eq!(s.slim, ct_v1(&s.claim).slim, "{} stride {big}", s.claim);
        }
    }
    let denser = with("chunk_size 1024\nslim_stride 4\n").unwrap();
    assert!(
        denser.suite("add_wraps").unwrap().slim.chunks.len() > ct_v1("add_wraps").slim.chunks.len()
    );
    // a slim_chunk line only adds
    let added = with("chunk_size 1024\nslim_chunk add_wraps 1\nslim_chunk add_wraps 2\n").unwrap();
    let a = added.suite("add_wraps").unwrap();
    assert!(a.slim.chunks.contains(&1) && a.slim.chunks.contains(&2));
    assert!(a.slim.chunks.len() >= ct_v1("add_wraps").slim.chunks.len() + 2);
    assert!(a.slim.chunks.windows(2).all(|w| w[0] < w[1]));
    // the first and last chunk are always in
    assert!(a.slim.chunks.contains(&0) && a.slim.chunks.contains(&63));
    // and it never changes a suite digest
    assert_eq!(a.inputs_sha256, ct_v1("add_wraps").inputs_sha256);
}

#[test]
fn a_slim_set_over_32_chunks_is_a_limit_error() {
    let mut text = String::from("chunk_size 1024\n");
    for c in 0..40 {
        text += &format!("slim_chunk add_wraps {c}\n");
    }
    let p = parse_profile(text.as_bytes()).unwrap();
    let e = with_file(ENUM_V1, |f| enumerate_document(f, &p).unwrap_err());
    assert_eq!(e.code, code::LIMIT);
    assert!(e.message.contains("32 chunks"), "{e}");
}

#[test]
fn witness_chunks_can_be_added_to_a_result_without_rehashing() {
    // the S3c seam: `slim_set` takes the witnesses as a parameter
    let s = ct_v1("add_wraps");
    let with_w = s.slim_set(&[1, 2, 2]).unwrap();
    assert_eq!(with_w.chunks, {
        let mut c = s.slim.chunks.clone();
        c.extend([1, 2]);
        c.sort_unstable();
        c
    });
    assert_ne!(with_w.inputs_sha256, s.slim.inputs_sha256);
    assert!(s.slim_set(&[64]).is_err(), "a witness past the last chunk");
    let too_many: Vec<u64> = (0..40).collect();
    assert!(s.slim_set(&too_many).is_err());
}

#[test]
fn a_product_of_no_binders_is_one_case() {
    // a ground claim has one (empty) tuple: P = 1, one chunk, one case, one call
    let src = v0_with("  claim ground:\n    op_eq(b\"00\", b\"00\") == true\n");
    let e = with_file(&src, |f| {
        enumerate_document(f, &Profile::default()).unwrap()
    });
    let s = e.suite("ground").unwrap();
    assert_eq!(
        (s.product, s.chunks_total, s.cases, s.expected_calls),
        (1, 1, 1, 1)
    );
    assert!(s.binder_types.is_empty());
    assert_eq!(s.label.name(), "full");
    assert_eq!(s.chunks[0].first_admitted, 0);
}

#[test]
fn the_digests_are_deterministic_across_runs() {
    let again = with_file(ENUM_V0, |f| {
        enumerate_document(f, &Profile::default()).unwrap()
    });
    let twice = with_file(ENUM_V0, |f| {
        enumerate_document(f, &Profile::default()).unwrap()
    });
    for (a, b) in again.suites.iter().zip(&twice.suites) {
        assert_eq!(a.inputs_sha256, b.inputs_sha256);
        assert_eq!(a.expected_sha256, b.expected_sha256);
        assert_eq!(a.chunk_lines(), b.chunk_lines());
    }
}
