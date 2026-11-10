//! VER04j WX-A evidence generation, tested the way VER04h tests the v0
//! generator: by *compiling and running* what it emits.  The generated harness
//! (`tests/generated/wxa_evidence.rs`, checked in and drift-tested by
//! `tests/wxa_regen.rs`) is included as a module and run against a tiny subject
//! (`wxa_subject`) whose exports implement the LEB128-shaped, record, variant
//! and tuple operations of `tests/fixtures/wxa.warrant`.
//!
//! Sections: the whole suite and every negative control; domains of composite
//! types (sizes, orders, constructors); generators, relation bias and
//! shrinkers; the unreachable-by-type count; the differential test of the
//! ground evaluator against the generated model code on every composite
//! domain within budget.

use std::collections::{BTreeSet, HashMap};

use coding_adventures_test_bootstrap::run::Options;
use coding_adventures_test_bootstrap::Verdict;
use coding_adventures_warrant_grammar::codegen::{CodegenLimits, EvalValue, Evaluator};
use coding_adventures_warrant_grammar::fixed::FixedTy;
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, Declaration, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

#[allow(dead_code, unused_variables, unused_parens, unused_mut, clippy::all)]
mod harness {
    /// The subject under test: honest implementations of the fixture's
    /// operations, shaped like the real Rust APIs WX-A is meant for.
    pub mod wxa_subject {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct Span {
            pub start: u8,
            pub len: u8,
        }
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Shape {
            Circle { r: u8 },
            Rect { w: u8, h: u8 },
            Empty,
        }
        #[derive(Debug)]
        pub enum LebErr {
            Truncated,
            Overlong,
        }
        /// The first one or two bytes of a stream as an unsigned LEB128 value:
        /// the value and how many bytes it used.
        pub fn decode2(b0: u8, b1: u8) -> Result<(u64, usize), LebErr> {
            if b0 < 0x80 {
                Ok((u64::from(b0), 1))
            } else if b1 < 0x80 {
                Ok((u64::from(b0 & 0x7f) | (u64::from(b1) << 7), 2))
            } else {
                Err(LebErr::Overlong)
            }
        }
        pub fn normalize(a: u8, b: u8) -> Span {
            Span {
                start: a.min(b),
                len: a.max(b) - a.min(b),
            }
        }
        pub fn classify(w: u8, h: u8) -> Shape {
            match (w, h) {
                (0, 0) => Shape::Empty,
                (w, h) if w == h => Shape::Circle { r: w },
                (w, h) => Shape::Rect { w, h },
            }
        }
        pub fn area(s: Shape) -> (bool, u64) {
            match s {
                Shape::Circle { r } => (false, u64::from(r) * u64::from(r)),
                Shape::Rect { w, h } => (false, u64::from(w) * u64::from(h)),
                Shape::Empty => (true, 0),
            }
        }
        pub fn order(a: u8, b: u8) -> (u8, u8) {
            (a.min(b), a.max(b))
        }
        pub fn split(a: u8) -> Option<(u8, u8)> {
            (a != 0).then_some((a >> 4, a & 15))
        }
        /// Panics on 255 (the binding normalises the panic to `OVERLONG`).
        pub fn try_pair(a: u8) -> (u8, u8) {
            (a, a.checked_add(1).expect("255 has no successor"))
        }
        pub fn sh(a: u8, n: u8) -> (u8, u8) {
            (a.checked_shl(u32::from(n)).unwrap_or(0), n)
        }
    }
    include!("generated/wxa_evidence.rs");

    // ----- test hooks: the generated module's private machinery ----------------

    /// The generated model function `name` applied to `args`.
    pub fn model(name: &str, args: &[V]) -> V {
        let a = |i: usize| args[i].clone();
        match name {
            "low7" => model_low7(a(0)),
            "dec2" => model_dec2(a(0), a(1)),
            "span_model" => model_span_model(a(0), a(1)),
            "classify_model" => model_classify_model(a(0), a(1)),
            "area_model" => model_area_model(a(0)),
            "order_model" => model_order_model(a(0), a(1)),
            "split_model" => model_split_model(a(0)),
            "try_pair_model" => model_try_pair_model(a(0)),
            other => panic!("no model {other}"),
        }
    }

    /// `n` environments drawn by the relation-biased generator in `mode`
    /// (0-1 fresh, 2-3 equal, 4-7 related by a one-leaf change).
    pub fn gen_values(tys: &[Ty], seed: u64, mode: u8, n: usize) -> Vec<Vec<V>> {
        let mut rng = SplitMix64::new(seed);
        (0..n)
            .map(|_| build(tys, &mut RngSrc(&mut rng), mode))
            .collect()
    }

    /// The composite-aware shrinker (`true` from `prop` means the property holds).
    pub fn shrink_env(
        tys: &[Ty],
        cand: Vec<V>,
        prop: &dyn Fn(&Vec<V>) -> bool,
    ) -> (Vec<V>, usize) {
        shrink_comp(tys, cand, prop)
    }

    /// The faults as plain functions on results.
    pub fn off_by_one(v: V) -> V {
        fault_off_by_one_consumed(v)
    }
    pub fn wrong_ctor(v: V, ty: Ty) -> V {
        fault_wrong_ctor(v, ty)
    }
    pub fn field_zero(v: V) -> V {
        fault_field_zero(v)
    }
    pub fn drop_consumed(v: V) -> V {
        fault_drop_consumed(v)
    }
    pub fn swap_components(v: V) -> V {
        fault_swap_components(v)
    }
    pub fn ok_as_err(v: V, case: &str) -> V {
        fault_ok_as_err(v, case)
    }

    /// The exact text the manifest records for a claim's binders.
    pub fn describe_binders(tys: &[Ty], names: &[&str]) -> String {
        describe(tys, names)
    }
    pub fn claim_tys(claim: &str) -> (&'static [Ty], &'static [&'static str]) {
        match claim {
            "composite_binder_domains" => (
                CLAIM_COMPOSITE_BINDER_DOMAINS_TYS,
                CLAIM_COMPOSITE_BINDER_DOMAINS_NAMES,
            ),
            "area_matches_the_model" => (CLAIM_AREA_MATCHES_THE_MODEL_TYS, CLAIM_AREA_MATCHES_THE_MODEL_NAMES),
            "order_on_a_tuple_binder" => (CLAIM_ORDER_ON_A_TUPLE_BINDER_TYS, CLAIM_ORDER_ON_A_TUPLE_BINDER_NAMES),
            other => panic!("no claim {other}"),
        }
    }
}
use harness::{Fault, Ty, F, V};

fn is_fail(v: &Verdict, id: &str) -> bool {
    matches!(v, Verdict::Fail { failure_id, .. } if *failure_id == id)
}

fn fixed_of(f: F) -> FixedTy {
    match f {
        F::U8 => FixedTy::U8,
        F::U16 => FixedTy::U16,
        F::U32 => FixedTy::U32,
        F::U64 => FixedTy::U64,
        F::I8 => FixedTy::I8,
        F::I16 => FixedTy::I16,
        F::I32 => FixedTy::I32,
        F::I64 => FixedTy::I64,
    }
}

const U8T: Ty = Ty::Fx(F::U8);
const U16E: Ty = Ty::FxEdge(F::U16);
const SHAPE: Ty = Ty::Data(
    "Shape",
    &[
        ("Circle", &[U8T]),
        ("Rect", &[U8T, U8T]),
        ("Empty", &[]),
    ],
);

// ---------------------------------------------------------------------------
// The whole generated suite, including every negative control
// ---------------------------------------------------------------------------

#[test]
fn the_full_generated_suite_passes_and_every_fault_is_caught() {
    // The subject panics on purpose (try_pair 255); the generated harness
    // silences the hook in `run_main`, so do the same here.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| ()));
    let suite = harness::build_suite(Fault::None);
    let mut tap = Vec::new();
    let (summary, outcomes) = suite
        .run_recorded(&Options::default(), &mut tap)
        .expect("run");
    std::panic::set_hook(previous);
    let tap = String::from_utf8(tap).unwrap();
    assert!(
        summary.failed == 0 && !summary.bailed,
        "failures:\n{}",
        tap.lines()
            .filter(|l| l.starts_with("not ok"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let negctl: Vec<_> = outcomes
        .iter()
        .filter(|o| o.id.contains("/negctl/"))
        .collect();
    assert!(!negctl.is_empty() && negctl.iter().all(|o| o.passed()));
    // Every WX-A fault is bound somewhere and is caught.
    let faults: BTreeSet<&str> = negctl
        .iter()
        .map(|o| o.id.rsplit('/').next().unwrap())
        .collect();
    for f in [
        "drop-consumed",
        "swap-components",
        "field-zero",
        "wrong-ctor",
        "ok-as-err",
        "off-by-one-consumed",
        "missing-error",
    ] {
        assert!(faults.contains(f), "{f} is not exercised: {faults:?}");
    }
}

// ---------------------------------------------------------------------------
// The manifest of the run: domains, demotion, unreachable-by-type counts
// ---------------------------------------------------------------------------

fn run_suite_and_manifest() -> String {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| ()));
    let suite = harness::build_suite(Fault::None);
    let mut tap = Vec::new();
    let (summary, outcomes) = suite
        .run_recorded(&Options::default(), &mut tap)
        .expect("run");
    std::panic::set_hook(previous);
    assert_eq!(summary.failed, 0);
    let out = std::env::temp_dir().join(format!("wxa_manifest_{}.json", std::process::id()));
    std::env::set_var("WARRANT_EVIDENCE_OUT", &out);
    harness::write_manifest_for(&outcomes);
    std::env::remove_var("WARRANT_EVIDENCE_OUT");
    let text = std::fs::read_to_string(&out).expect("manifest written");
    let _ = std::fs::remove_file(&out);
    text
}

#[test]
fn the_manifest_records_composite_domains_and_unreachable_counts() {
    let m = run_suite_and_manifest();
    assert!(m.contains("\"warrant_version\":\"v1\"") || m.contains("\"warrant_version\": \"v1\""));
    // Per-binder domain text: tuple(...), record Name(...), variant Name[k ctors].
    let has = |needle: &str| assert!(m.contains(needle), "manifest lacks {needle:?}\n{m}");
    has("p: tuple(U8 full(256), Bool set(2)) size(512)");
    has("s: variant Shape[3 ctors] size(65793)");
    has("r: Result<Bool set(2), 2 cases> size(4); o: Option<Bool set(2)> size(3); l: List<Bool set(2)> size(15) [len 0..=2 over the first 16, len 3 over the first 4]; a: U8 full(256)");
    // `sh`'s count flows into a shift: the NatCount domain has three values a
    // `u8` host cannot hold (256, 65535, 65536); with 256 values of `a` that is
    // 768 cases counted as unreachable by type, never narrowed.
    let entry = |id_part: &str| -> String {
        let at = m
            .find(id_part)
            .unwrap_or_else(|| panic!("no entry {id_part}"));
        let rest = &m[at..];
        rest[..rest.find('}').unwrap()].to_string()
    };
    let sh = entry("exhaustive/sh_matches_the_model");
    assert!(
        sh.contains("\"unreachable_by_type\":768") || sh.contains("\"unreachable_by_type\": 768"),
        "{sh}"
    );
    let dec = entry("exhaustive/decode2_matches_the_model");
    assert!(
        dec.contains("\"unreachable_by_type\":0") || dec.contains("\"unreachable_by_type\": 0"),
        "{dec}"
    );
    // Nothing is demoted here: every claim fits the reference budget.
    assert!(!m.contains("\"demoted\":[\"") && !m.contains("\"demoted\": [\""));
}

// ---------------------------------------------------------------------------
// Domains (section 4.7): sizes, orders, constructors
// ---------------------------------------------------------------------------

fn u64_edge_len() -> u64 {
    256 + 56 + 1 + 64
}

#[test]
fn domain_sizes_equal_the_formulas() {
    // (U64, Nat) is 377 x 65 = 24,505 elements (the example of section 4.7).
    let pair = Ty::Tuple(&[Ty::Fx(F::U64), Ty::Nat]);
    assert_eq!(u64_edge_len(), 377);
    assert_eq!(harness::domain_size(pair), Some(24_505));
    let d = harness::domain(pair);
    assert_eq!(d.len(), 24_505);
    // First and last element of the lexicographic order: the last component
    // varies fastest.
    assert_eq!(d[0], V::Tup(vec![V::Fixed(F::U64, 0), V::Int(0)]));
    assert_eq!(d[1], V::Tup(vec![V::Fixed(F::U64, 0), V::Int(1)]));
    assert_eq!(d[65], V::Tup(vec![V::Fixed(F::U64, 1), V::Int(0)]));
    let last_first = i128::from(u64::MAX ^ (1u64 << 63));
    assert_eq!(
        d[24_504],
        V::Tup(vec![V::Fixed(F::U64, last_first), V::Int(64)])
    );
    // A variant is the concatenation of its constructors' products.
    let v = Ty::Data(
        "S",
        &[
            ("A", &[Ty::Bool]),
            ("B", &[Ty::Bool, Ty::Bool]),
            ("C", &[]),
        ],
    );
    assert_eq!(harness::domain_size(v), Some(2 + 4 + 1));
    let d = harness::domain(v);
    let names: Vec<(usize, Vec<bool>)> = d
        .iter()
        .map(|x| match x {
            V::Con("S", i, _, fs) => (
                *i,
                fs.iter().map(|f| matches!(f, V::Bool(true))).collect(),
            ),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        names,
        vec![
            (0, vec![false]),
            (0, vec![true]),
            (1, vec![false, false]),
            (1, vec![false, true]),
            (1, vec![true, false]),
            (1, vec![true, true]),
            (2, vec![]),
        ]
    );
    // Every constructor appears (and a record is its single constructor).
    let ctors: BTreeSet<usize> = names.iter().map(|(i, _)| *i).collect();
    assert_eq!(ctors, BTreeSet::from([0, 1, 2]));
    let rec = Ty::Data("Rec", &[("Rec", &[U8T, Ty::Bool])]);
    assert_eq!(harness::domain_size(rec), Some(512));
    // Option<T>: None, then Some(x); size 1 + |T|.
    let o = Ty::Opt(&Ty::Bool);
    assert_eq!(
        harness::domain(o),
        vec![
            V::Opt(None),
            V::Opt(Some(Box::new(V::Bool(false)))),
            V::Opt(Some(Box::new(V::Bool(true)))),
        ]
    );
    // Result<T, E>: Ok(x) for x in T, then each error case; size |T| + |E|.
    let r = Ty::Res(&Ty::Bool, &["X", "Y"]);
    assert_eq!(
        harness::domain(r),
        vec![
            V::Ok(Box::new(V::Bool(false))),
            V::Ok(Box::new(V::Bool(true))),
            V::Err("X".into()),
            V::Err("Y".into()),
        ]
    );
    // List<T>: lengths 0..=2 over the first 16 elements, length 3 over the
    // first 4: 1 + d + d^2 + e^3.
    let l = Ty::List(&Ty::Bool);
    assert_eq!(harness::domain_size(l), Some(1 + 2 + 4 + 8));
    let d = harness::domain(l);
    assert_eq!(d[0], V::List(vec![]));
    assert_eq!(d[1], V::List(vec![V::Bool(false)]));
    assert_eq!(d[3], V::List(vec![V::Bool(false), V::Bool(false)]));
    assert_eq!(d[14], V::List(vec![V::Bool(true); 3]));
    let wide = Ty::List(&U8T);
    assert_eq!(harness::domain_size(wide), Some(1 + 16 + 256 + 64));
    let d = harness::domain(wide);
    assert_eq!(d.len(), 337);
    // Elements come from the first 16 (length <= 2) and first 4 (length 3).
    let max_elem = |len: usize| -> i128 {
        d.iter()
            .filter_map(|x| match x {
                V::List(vs) if vs.len() == len => vs.iter().map(|e| e.int()).max(),
                _ => None,
            })
            .max()
            .unwrap()
    };
    assert_eq!((max_elem(1), max_elem(2), max_elem(3)), (15, 15, 3));
    // Nesting multiplies.
    let nested = Ty::Tuple(&[Ty::Opt(&Ty::Bool), Ty::Res(&Ty::Bool, &["X"])]);
    assert_eq!(harness::domain_size(nested), Some(3 * 3));
    assert_eq!(harness::domain_size(Ty::Opt(&Ty::Opt(&Ty::Bool))), Some(4));
}

#[test]
fn the_closed_form_matches_the_materialised_domain_for_every_type() {
    let types: Vec<Ty> = vec![
        Ty::Bool,
        U8T,
        Ty::Nat,
        Ty::NatCount,
        Ty::Fx(F::U64),
        Ty::Bytes,
        Ty::BytesN(3),
        Ty::Tuple(&[Ty::Bool, U8T]),
        Ty::Tuple(&[Ty::Opt(&U8T), Ty::Bool, Ty::Nat]),
        SHAPE,
        Ty::Data("One", &[("One", &[Ty::Bool])]),
        Ty::Opt(&SHAPE),
        Ty::Res(&Ty::Tuple(&[Ty::Bool, Ty::Bool]), &["E1", "E2", "E3"]),
        Ty::Res(&Ty::Bool, &[]),
        Ty::List(&Ty::Bool),
        Ty::List(&Ty::Opt(&Ty::Bool)),
        Ty::List(&U16E),
        Ty::Tuple(&[U16E, Ty::Bool]),
    ];
    for t in types {
        let size = harness::domain_size(t).expect("sized");
        assert_eq!(harness::domain(t).len() as u64, size, "{t:?}");
        // domain_take is a prefix of the domain, for any k.
        let full = harness::domain(t);
        for k in [
            0usize,
            1,
            2,
            3,
            7,
            64,
            65,
            size as usize / 2,
            size as usize,
            size as usize + 5,
        ] {
            let got = harness::domain_take(t, k);
            assert_eq!(got.len(), k.min(full.len()), "{t:?} take {k}");
            assert_eq!(got[..], full[..got.len()], "{t:?} take {k}");
        }
    }
}

#[test]
fn byte_budgets_multiply_size_by_the_per_element_cost() {
    // A node costs VALUE_OVERHEAD (32); a Bytes payload at most 8 (BytesN: n).
    let o = harness::VALUE_OVERHEAD;
    assert_eq!(o, 32);
    assert_eq!(harness::domain_bytes(Ty::Bool), Some(2 * o));
    assert_eq!(harness::domain_bytes(Ty::Bytes), Some(347 * (o + 8)));
    // A tuple element is its own node plus its components.
    assert_eq!(
        harness::domain_bytes(Ty::Tuple(&[Ty::Bool, Ty::Bool])),
        Some(4 * (3 * o))
    );
    // A record with a Bytes field costs the payload per element.
    let rec = Ty::Data("R", &[("R", &[Ty::Bytes, Ty::Bool])]);
    assert_eq!(harness::domain_bytes(rec), Some(347 * 2 * (o + (o + 8) + o)));
    // The budget check refuses a product that is over either budget before
    // anything is materialised, including a composite whose size overflows.
    assert!(harness::within_budget(&[Ty::Tuple(&[U8T, U8T])]));
    assert!(!harness::within_budget(&[Ty::Tuple(&[
        Ty::Fx(F::U16),
        Ty::Fx(F::U16)
    ])]));
    let huge = Ty::Tuple(&[Ty::Fx(F::U16); 8]);
    assert_eq!(harness::domain_size(huge), None, "2^128 does not fit a u64");
    assert!(!harness::within_budget(&[huge]));
}

// ---------------------------------------------------------------------------
// Generators, relation bias and shrinkers (section 4.7)
// ---------------------------------------------------------------------------

#[test]
fn generators_reach_every_constructor() {
    let envs = harness::gen_values(&[SHAPE], 7, 0, 600);
    let mut seen = [0usize; 3];
    for e in &envs {
        if let V::Con(_, i, _, fs) = &e[0] {
            seen[*i] += 1;
            assert_eq!(fs.len(), [1, 2, 0][*i]);
        } else {
            panic!("{:?}", e[0]);
        }
    }
    // Constructors are drawn uniformly: each gets about a third.
    assert!(seen.iter().all(|n| (140..=260).contains(n)), "{seen:?}");
    // Option, Result and List: every shape is reached.
    let o = harness::gen_values(&[Ty::Opt(&Ty::Bool)], 3, 0, 200);
    assert!(o.iter().any(|e| e[0] == V::Opt(None)));
    assert!(o.iter().any(|e| matches!(e[0], V::Opt(Some(_)))));
    let r = harness::gen_values(&[Ty::Res(&Ty::Bool, &["X", "Y"])], 3, 0, 400);
    assert!(r.iter().any(|e| matches!(e[0], V::Ok(_))));
    assert!(r.iter().any(|e| e[0] == V::Err("X".into())));
    assert!(r.iter().any(|e| e[0] == V::Err("Y".into())));
    let l = harness::gen_values(&[Ty::List(&Ty::Bool)], 3, 0, 400);
    let lens: BTreeSet<usize> = l
        .iter()
        .map(|e| match &e[0] {
            V::List(v) => v.len(),
            _ => panic!(),
        })
        .collect();
    assert_eq!(lens, BTreeSet::from([0, 1, 2, 3]));
    // Tuples draw their components independently.
    let t = harness::gen_values(&[Ty::Tuple(&[Ty::Bool, Ty::Bool])], 5, 0, 200);
    let combos: BTreeSet<(bool, bool)> = t
        .iter()
        .map(|e| match &e[0] {
            V::Tup(v) => (v[0] == V::Bool(true), v[1] == V::Bool(true)),
            _ => panic!(),
        })
        .collect();
    assert_eq!(combos.len(), 4);
    // Deterministic: the same seed draws the same values.
    assert_eq!(
        harness::gen_values(&[SHAPE], 11, 1, 50),
        harness::gen_values(&[SHAPE], 11, 1, 50)
    );
}

#[test]
fn relation_bias_changes_one_leaf_and_keeps_the_constructor() {
    // Two binders of one tuple type: modes 4-7 perturb a single component of
    // the second, modes 2-3 copy the first.
    let ty = Ty::Tuple(&[U8T, U8T, Ty::Fx(F::U32)]);
    for mode in 4..=7u8 {
        let envs = harness::gen_values(&[ty, ty], 21, mode, 300);
        let mut changed = 0;
        for e in &envs {
            let (V::Tup(a), V::Tup(b)) = (&e[0], &e[1]) else {
                panic!()
            };
            let diff = a.iter().zip(b).filter(|(x, y)| x != y).count();
            assert!(diff <= 1, "mode {mode}: {diff} components changed");
            changed += diff;
        }
        assert!(changed > 100, "mode {mode} rarely perturbs ({changed}/300)");
    }
    for mode in [2u8, 3] {
        for e in harness::gen_values(&[ty, ty], 22, mode, 50) {
            assert_eq!(e[0], e[1], "mode {mode} copies");
        }
    }
    // A variant keeps its constructor under relation bias; an Option keeps
    // its shape; a list keeps its length.
    for e in harness::gen_values(&[SHAPE, SHAPE], 23, 5, 200) {
        let (V::Con(_, i, _, _), V::Con(_, j, _, _)) = (&e[0], &e[1]) else {
            panic!()
        };
        assert_eq!(i, j);
    }
    let opt = Ty::Opt(&U8T);
    for e in harness::gen_values(&[opt, opt], 24, 6, 200) {
        assert_eq!(
            matches!(e[0], V::Opt(Some(_))),
            matches!(e[1], V::Opt(Some(_)))
        );
    }
    let list = Ty::List(&U8T);
    for e in harness::gen_values(&[list, list], 25, 7, 200) {
        let (V::List(a), V::List(b)) = (&e[0], &e[1]) else {
            panic!()
        };
        assert_eq!(a.len(), b.len());
    }
    // Result: the Ok payload is perturbed in place, an Err is copied.
    let res = Ty::Res(&U8T, &["X"]);
    for e in harness::gen_values(&[res, res], 26, 4, 200) {
        assert_eq!(matches!(e[0], V::Ok(_)), matches!(e[1], V::Ok(_)));
    }
}

#[test]
fn shrinkers_work_componentwise_and_prefer_earlier_constructors() {
    let two = Ty::Tuple(&[U8T, U8T]);
    // Fails (property false) iff a >= 10: shrinks to the boundary in the first
    // component and to zero in the second.
    let prop = |env: &Vec<V>| match &env[0] {
        V::Tup(c) => c[0].int() < 10,
        _ => true,
    };
    let (shrunk, steps) = harness::shrink_env(
        &[two],
        vec![V::Tup(vec![V::Fixed(F::U8, 200), V::Fixed(F::U8, 77)])],
        &prop,
    );
    assert_eq!(
        shrunk,
        vec![V::Tup(vec![V::Fixed(F::U8, 10), V::Fixed(F::U8, 0)])]
    );
    assert!(steps > 0);
    // A variant first tries an earlier constructor with minimal fields: when
    // every non-Empty value fails, Rect(200, 9) shrinks to Circle(0).
    let fails_unless_empty = |env: &Vec<V>| matches!(&env[0], V::Con(_, 2, _, _));
    let rect = V::Con(
        "Shape",
        1,
        "Rect",
        vec![V::Fixed(F::U8, 200), V::Fixed(F::U8, 9)],
    );
    let (shrunk, _) = harness::shrink_env(&[SHAPE], vec![rect], &fails_unless_empty);
    assert_eq!(
        shrunk,
        vec![V::Con("Shape", 0, "Circle", vec![V::Fixed(F::U8, 0)])]
    );
    // When the earlier constructor does not fail, the fields shrink instead:
    // fails iff Rect with w >= 3.
    let rect_w = |env: &Vec<V>| match &env[0] {
        V::Con(_, 1, _, fs) => fs[0].int() < 3,
        _ => true,
    };
    let rect = V::Con(
        "Shape",
        1,
        "Rect",
        vec![V::Fixed(F::U8, 200), V::Fixed(F::U8, 9)],
    );
    let (shrunk, _) = harness::shrink_env(&[SHAPE], vec![rect], &rect_w);
    assert_eq!(
        shrunk,
        vec![V::Con(
            "Shape",
            1,
            "Rect",
            vec![V::Fixed(F::U8, 3), V::Fixed(F::U8, 0)]
        )]
    );
    // Option shrinks toward None when None still fails, else shrinks inside.
    let opt = Ty::Opt(&U8T);
    let some100 = V::Opt(Some(Box::new(V::Fixed(F::U8, 100))));
    let (shrunk, _) = harness::shrink_env(&[opt], vec![some100.clone()], &|_: &Vec<V>| false);
    assert_eq!(shrunk, vec![V::Opt(None)]);
    let ge5 = |env: &Vec<V>| match &env[0] {
        V::Opt(Some(x)) => x.int() < 5,
        _ => true,
    };
    let (shrunk, _) = harness::shrink_env(&[opt], vec![some100], &ge5);
    assert_eq!(shrunk, vec![V::Opt(Some(Box::new(V::Fixed(F::U8, 5))))]);
    // Result: the Ok payload shrinks.
    let res = Ty::Res(&U8T, &["X"]);
    let ok9 = V::Ok(Box::new(V::Fixed(F::U8, 9)));
    let (shrunk, _) = harness::shrink_env(&[res], vec![ok9], &|e: &Vec<V>| {
        !matches!(&e[0], V::Ok(x) if x.int() >= 2)
    });
    assert_eq!(shrunk, vec![V::Ok(Box::new(V::Fixed(F::U8, 2)))]);
    // A list shrinks toward [], then loses elements, then shrinks them.
    let list = Ty::List(&U8T);
    let nine = |n: usize| V::List(vec![V::Fixed(F::U8, 9); n]);
    let long_fails = |env: &Vec<V>| match &env[0] {
        V::List(v) => v.len() < 2,
        _ => true,
    };
    let (shrunk, _) = harness::shrink_env(&[list], vec![nine(4)], &long_fails);
    assert_eq!(shrunk, vec![V::List(vec![V::Fixed(F::U8, 0); 2])]);
    let (shrunk, _) = harness::shrink_env(&[list], vec![nine(3)], &|_: &Vec<V>| false);
    assert_eq!(shrunk, vec![V::List(vec![])]);
    // Scalars still shrink as in v1 (a Fixed binder next to a composite).
    let (shrunk, _) = harness::shrink_env(
        &[U8T, two],
        vec![
            V::Fixed(F::U8, 200),
            V::Tup(vec![V::Fixed(F::U8, 1), V::Fixed(F::U8, 1)]),
        ],
        &|e: &Vec<V>| e[0].int() < 17,
    );
    assert_eq!(shrunk[0], V::Fixed(F::U8, 17));
    assert_eq!(
        shrunk[1],
        V::Tup(vec![V::Fixed(F::U8, 0), V::Fixed(F::U8, 0)])
    );
}

// ---------------------------------------------------------------------------
// The faults of section 4.8, as rewrites of a result
// ---------------------------------------------------------------------------

#[test]
fn each_fault_rewrites_exactly_what_the_specification_says() {
    let fx = |n: i128| V::Fixed(F::U8, n);
    let pair = |a: V, b: V| V::Tup(vec![a, b]);
    // drop-consumed: the second component becomes 0 (inside Ok / Some too).
    assert_eq!(
        harness::drop_consumed(pair(fx(5), V::Int(9))),
        pair(fx(5), V::Int(0))
    );
    assert_eq!(
        harness::drop_consumed(V::Ok(Box::new(pair(fx(5), fx(9))))),
        V::Ok(Box::new(pair(fx(5), fx(0))))
    );
    assert_eq!(
        harness::drop_consumed(V::Opt(Some(Box::new(pair(fx(5), fx(9)))))),
        V::Opt(Some(Box::new(pair(fx(5), fx(0)))))
    );
    assert_eq!(harness::drop_consumed(V::Err("X".into())), V::Err("X".into()));
    // off-by-one-consumed: +1 on an even second component, -1 on an odd one.
    assert_eq!(
        harness::off_by_one(pair(fx(5), V::Int(0))),
        pair(fx(5), V::Int(1))
    );
    assert_eq!(
        harness::off_by_one(pair(fx(5), V::Int(3))),
        pair(fx(5), V::Int(2))
    );
    assert_eq!(
        harness::off_by_one(pair(fx(5), fx(254))),
        pair(fx(5), fx(255))
    );
    assert_eq!(harness::off_by_one(pair(fx(5), fx(1))), pair(fx(5), fx(0)));
    assert_eq!(
        harness::off_by_one(pair(fx(5), V::Bool(true))),
        pair(fx(5), V::Bool(true))
    );
    // swap-components reverses the tuple (exchanges a pair).
    assert_eq!(
        harness::swap_components(pair(fx(1), fx(2))),
        pair(fx(2), fx(1))
    );
    assert_eq!(
        harness::swap_components(V::Tup(vec![fx(1), fx(2), fx(3)])),
        V::Tup(vec![fx(3), fx(2), fx(1)])
    );
    // field-zero: the first numeric field of a record becomes 0.
    let rec = V::Con("Span", 0, "Span", vec![V::Bool(true), fx(4), fx(7)]);
    assert_eq!(
        harness::field_zero(rec),
        V::Con("Span", 0, "Span", vec![V::Bool(true), fx(0), fx(7)])
    );
    // wrong-ctor: the previous constructor (the last for the first), with the
    // first element of each field's domain as fields.
    let rect = V::Con("Shape", 1, "Rect", vec![fx(8), fx(9)]);
    assert_eq!(
        harness::wrong_ctor(rect, SHAPE),
        V::Con("Shape", 0, "Circle", vec![fx(0)])
    );
    let circle = V::Con("Shape", 0, "Circle", vec![fx(8)]);
    assert_eq!(
        harness::wrong_ctor(circle, SHAPE),
        V::Con("Shape", 2, "Empty", vec![])
    );
    // ok-as-err reports the first error case.
    assert_eq!(
        harness::ok_as_err(V::Ok(Box::new(fx(1))), "TRUNCATED"),
        V::Err("TRUNCATED".into())
    );
    assert_eq!(
        harness::ok_as_err(V::Err("X".into()), "TRUNCATED"),
        V::Err("X".into())
    );
}

// ---------------------------------------------------------------------------
// Differential test: the ground evaluator against the generated model code on
// every composite domain within budget (VER04j sections 8.1 and 10.3)
// ---------------------------------------------------------------------------

fn to_eval(v: &V) -> EvalValue {
    match v {
        V::Bool(b) => EvalValue::Bool(*b),
        V::Int(n) => EvalValue::Int(*n),
        V::Fixed(f, n) => EvalValue::from_fixed(fixed_of(*f), *n),
        V::Bytes(b) => EvalValue::Bytes(b.clone()),
        V::Tup(vs) => EvalValue::Tuple(vs.iter().map(to_eval).collect()),
        V::List(vs) => EvalValue::List(vs.iter().map(to_eval).collect()),
        V::Opt(None) => EvalValue::Option(None),
        V::Opt(Some(x)) => EvalValue::Option(Some(Box::new(to_eval(x)))),
        V::Con(ty, i, c, fs) => EvalValue::Con {
            ty: ty.to_string(),
            index: *i,
            ctor: c.to_string(),
            record: *ty == "Span",
            fields: fs.iter().map(to_eval).collect(),
        },
        V::Ok(x) => EvalValue::Result(Ok(Box::new(to_eval(x)))),
        V::Err(n) | V::Name(n) => EvalValue::Result(Err(("LebError".into(), n.clone()))),
    }
}

/// Do the ground value and the generated model's value denote the same thing?
fn same(e: &EvalValue, v: &V) -> bool {
    match (e, v) {
        (EvalValue::Bool(a), V::Bool(b)) => a == b,
        (EvalValue::Bytes(a), V::Bytes(b)) => a == b,
        (EvalValue::Tuple(a), V::Tup(b)) | (EvalValue::List(a), V::List(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| same(x, y))
        }
        (EvalValue::Option(None), V::Opt(None)) => true,
        (EvalValue::Option(Some(a)), V::Opt(Some(b))) => same(a, b),
        (EvalValue::Result(Ok(a)), V::Ok(b)) => same(a, b),
        (EvalValue::Result(Err((_, c))), V::Err(d)) => c == d,
        (
            EvalValue::Con {
                index,
                ctor,
                fields,
                ..
            },
            V::Con(_, j, c, fs),
        ) => {
            index == j
                && ctor == c
                && fields.len() == fs.len()
                && fields.iter().zip(fs).all(|(x, y)| same(x, y))
        }
        (a, V::Int(n)) | (a, V::Fixed(_, n)) => a.as_i128() == Some(*n),
        _ => false,
    }
}

const FIXTURE: &str = include_str!("fixtures/wxa.warrant");

/// Every model of the fixture with the type of each parameter.
fn models() -> Vec<(&'static str, Vec<Ty>)> {
    vec![
        ("low7", vec![U8T]),
        ("dec2", vec![U8T, U8T]),
        ("span_model", vec![U8T, U8T]),
        ("classify_model", vec![U8T, U8T]),
        ("area_model", vec![SHAPE]),
        ("order_model", vec![U8T, U8T]),
        ("split_model", vec![U8T]),
        ("try_pair_model", vec![U8T]),
    ]
}

/// All argument vectors of a model: the Cartesian product of its parameters'
/// domains.
fn argument_vectors(tys: &[Ty]) -> Vec<Vec<V>> {
    assert!(harness::within_budget(tys), "the domain is within budget");
    let mut out: Vec<Vec<V>> = vec![Vec::new()];
    for t in tys {
        let d = harness::domain(*t);
        let mut next = Vec::with_capacity(out.len() * d.len());
        for prefix in &out {
            for x in &d {
                let mut p = prefix.clone();
                p.push(x.clone());
                next.push(p);
            }
        }
        out = next;
    }
    out
}

#[test]
fn ground_evaluator_and_generated_model_agree_on_every_composite_domain() {
    let doc = SourceDocument::admit(FIXTURE.as_bytes(), SourceLimits::default()).unwrap();
    let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
    let file = parse_document(&layout, &doc, ParserLimits::default()).unwrap();
    let mut ev = Evaluator::new(&file, CodegenLimits::default());
    let mut total = 0usize;
    for (name, tys) in models() {
        let m = file
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Model(m) if m.name == name => Some(m),
                _ => None,
            })
            .unwrap_or_else(|| panic!("model {name}"));
        let body = m.body.as_ref().expect("a model with a body");
        let mut compared = 0usize;
        for args in argument_vectors(&tys) {
            let generated = harness::model(name, &args);
            let mut env: HashMap<&str, EvalValue> = HashMap::new();
            for (p, a) in m.params.iter().zip(&args) {
                env.insert(p.name, to_eval(a));
            }
            ev.reset_work();
            let ground = ev
                .eval(body, &env)
                .unwrap_or_else(|e| panic!("{name}{args:?}: {e}"));
            assert!(
                same(&ground, &generated),
                "{name}{args:?}: ground {ground:?} vs generated {generated:?}"
            );
            compared += 1;
        }
        let expected: u64 = tys
            .iter()
            .map(|t| harness::domain_size(*t).unwrap())
            .product();
        assert_eq!(
            compared as u64, expected,
            "{name}: every element of the domain"
        );
        total += compared;
    }
    // Pairs of bytes (x4), a variant domain, and single bytes.
    assert_eq!(total, 4 * 65_536 + 65_793 + 3 * 256);
    // Nothing was vacuous: both Ok and Err results occur for dec2, and every
    // constructor of Shape is produced by classify_model.
    let outcomes: BTreeSet<&str> = argument_vectors(&[U8T, U8T])
        .iter()
        .map(|a| match harness::model("dec2", a) {
            V::Ok(_) => "ok",
            V::Err(_) => "err",
            _ => "?",
        })
        .collect();
    assert_eq!(outcomes, BTreeSet::from(["ok", "err"]));
    let ctors: BTreeSet<usize> = argument_vectors(&[U8T, U8T])
        .iter()
        .map(|a| match harness::model("classify_model", a) {
            V::Con(_, i, _, _) => i,
            _ => 99,
        })
        .collect();
    assert_eq!(ctors, BTreeSet::from([0, 1, 2]));
}

#[test]
fn the_differential_oracle_notices_a_perturbed_model() {
    // Self-test: a model result rewritten by a fault is *not* the ground
    // value somewhere on the domain, so an equally broken generated model
    // could not slip through the comparison above.
    let doc = SourceDocument::admit(FIXTURE.as_bytes(), SourceLimits::default()).unwrap();
    let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
    let file = parse_document(&layout, &doc, ParserLimits::default()).unwrap();
    let mut ev = Evaluator::new(&file, CodegenLimits::default());
    let m = file
        .declarations
        .iter()
        .find_map(|d| match d {
            Declaration::Model(m) if m.name == "dec2" => Some(m),
            _ => None,
        })
        .unwrap();
    let body = m.body.as_ref().unwrap();
    let mut mismatches = 0usize;
    for args in argument_vectors(&[U8T, U8T]) {
        let wrong = harness::off_by_one(harness::model("dec2", &args));
        let mut env: HashMap<&str, EvalValue> = HashMap::new();
        for (p, a) in m.params.iter().zip(&args) {
            env.insert(p.name, to_eval(a));
        }
        ev.reset_work();
        let ground = ev.eval(body, &env).unwrap();
        if !same(&ground, &wrong) {
            mismatches += 1;
        }
    }
    // Every Ok result (0..=127 one byte, 128..=255 x 0..=127 two bytes) is off.
    assert_eq!(mismatches, 128 * 256 + 128 * 128);
}

#[test]
fn the_generated_suite_notices_every_fault_it_names() {
    // The suite run with an injected fault must fail, per fault (the BUILD
    // sabotage modes use the same variable).
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| ()));
    for fault in [
        "drop-consumed",
        "swap-components",
        "field-zero",
        "wrong-ctor",
        "ok-as-err",
        "off-by-one-consumed",
        "missing-error",
    ] {
        let f = Fault::parse(fault).expect(fault);
        let suite = harness::build_suite(f);
        let mut tap = Vec::new();
        let (summary, outcomes) = suite
            .run_recorded(&Options::default(), &mut tap)
            .expect("run");
        let failing: Vec<&str> = outcomes
            .iter()
            .filter(|o| !o.passed() && !o.id.contains("/negctl/"))
            .map(|o| o.id.as_str())
            .collect();
        assert!(
            summary.failed > 0 && !failing.is_empty(),
            "{fault} went unnoticed"
        );
        assert!(
            failing.iter().any(|id| id.contains("/exhaustive/")),
            "{fault}: the exhaustive suites catch it ({failing:?})"
        );
    }
    std::panic::set_hook(previous);
    assert!(Fault::parse("not-a-fault").is_none());
    assert!(is_fail(&Verdict::fail("x", "y"), "x"));
}
