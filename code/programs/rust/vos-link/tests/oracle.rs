//! CC11's pure judge ([`vos_link::oracle`], VOS-039c): the cases a fixture crate cannot reach, because
//! the document analysis only returns a `BuildShape` that is a constructor (or not) and the other
//! shapes (a budget given up, a wide model) are decided by the walk's own limits.

use std::collections::{BTreeMap, BTreeSet};

use vos_link::build_gen::PlanEntry;
use vos_link::oracle::{is_numeric_type, judge, load, successor_ops, Facts, ORACLE_FAULT};
use vos_manifest::warrant::{BuildShape, OpFact};

fn entry(claim: &str) -> PlanEntry {
    PlanEntry {
        tier: "exhaustive".into(),
        name: claim.into(),
        fault: ORACLE_FAULT.into(),
        attack: None,
        line: 1,
    }
}

/// The claims the contract owns in the fixtures: `c1`.
fn owned() -> BTreeSet<String> {
    ["c1".to_string()].into_iter().collect()
}

fn faults(pairs: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
    pairs
        .iter()
        .map(|(o, f)| (o.to_string(), f.iter().map(|x| x.to_string()).collect()))
        .collect()
}

/// A compliant crate's facts: the build shape, the successors, their faults, the plan and the calls.
type GoodFacts = (
    BuildShape,
    BTreeSet<String>,
    BTreeMap<String, Vec<String>>,
    Vec<PlanEntry>,
    Vec<(String, Vec<String>)>,
);

fn good_facts() -> GoodFacts {
    (
        BuildShape::Constructor {
            in_cone: vec![true, false],
        },
        ["ins".to_string()].into_iter().collect(),
        faults(&[("ins", &["field-zero", ORACLE_FAULT])]),
        vec![entry("c1")],
        vec![("c1".to_string(), vec!["ins".to_string()])],
    )
}

#[test]
fn a_compliant_crate_has_no_finding() {
    let (b, s, f, p, c) = good_facts();
    assert!(judge(&Facts {
        build: &b,
        successors: &s,
        last_field: Some("U8"),
        unclassified: &[],
        owned: &owned(),
        faults: &f,
        plan: &p,
        calls: &c
    })
    .is_empty());
}

#[test]
fn a_cone_walk_that_cannot_decide_fails_closed() {
    let (_, s, f, p, c) = good_facts();
    for (shape, word) in [
        (BuildShape::Missing, "is missing"),
        (BuildShape::NotConstructor, "not an `Impl(..)` constructor"),
        (BuildShape::BudgetExhausted, "could not decide"),
        (BuildShape::WideModel, "could not decide"),
    ] {
        let out = judge(&Facts {
            build: &shape,
            successors: &s,
            last_field: Some("U8"),
            unclassified: &[],
            owned: &owned(),
            faults: &f,
            plan: &p,
            calls: &c,
        });
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(out[0].contains(word), "{out:?}");
    }
}

#[test]
fn a_constructor_with_no_field_is_refused_not_a_panic() {
    // `Impl()` has no last field: the subtraction that finds it must not underflow.
    let (_, s, f, p, c) = good_facts();
    let shape = BuildShape::Constructor { in_cone: vec![] };
    let out = judge(&Facts {
        build: &shape,
        successors: &s,
        last_field: Some("U8"),
        unclassified: &[],
        owned: &owned(),
        faults: &f,
        plan: &p,
        calls: &c,
    });
    assert_eq!(out.len(), 1, "{out:?}");
    assert!(out[0].contains("are none;"), "{out:?}");
}

#[test]
fn a_flip_fault_that_is_not_the_oracle_fault_does_not_count() {
    // `field-flip` on the claim is another fault: only `field-flip-last` reaches the last field.
    let (b, s, f, _, c) = good_facts();
    let mut e = entry("c1");
    e.fault = "field-flip".into();
    let out = judge(&Facts {
        build: &b,
        successors: &s,
        last_field: Some("U8"),
        unclassified: &[],
        owned: &owned(),
        faults: &f,
        plan: &[e],
        calls: &c,
    });
    assert_eq!(out.len(), 1, "{out:?}");
    assert!(
        out[0].contains("no `exhaustive/<claim> field-flip-last` entry"),
        "{out:?}"
    );
}

fn op(name: &str, returns_state: bool, state_params: usize, inout_state: bool) -> OpFact {
    OpFact {
        name: name.into(),
        returns_state,
        state_params,
        inout_state,
    }
}

#[test]
fn successors_are_the_state_to_state_operations_of_the_document() {
    let ops = [
        op("init", true, 0, false),     // no state argument
        op("ins", true, 1, false),      // the state-to-state shape
        op("ins_out", false, 1, false), // a projection
        op("pair", true, 2, false),
        op("opt", true, 1, false), // `Option<Impl>` mentions Impl too: a state
        op("mutate", false, 1, true), // an `inout Impl`: mutated in place
        op("cfg", false, 0, false),
    ];
    let s: Vec<String> = successor_ops(&ops).into_iter().collect();
    assert_eq!(s, ["ins", "mutate", "opt", "pair"]);
}

fn judged_with<'a>(
    unclassified: &'a [String],
    owned: &'a BTreeSet<String>,
    f: impl FnOnce(&mut Facts<'a>),
) -> Vec<String> {
    let (b, s, fa, p, c) = good_facts();
    let (b, s, fa, p, c) = (
        &*Box::leak(Box::new(b)),
        &*Box::leak(Box::new(s)),
        &*Box::leak(Box::new(fa)),
        &*Box::leak(Box::new(p)),
        &*Box::leak(Box::new(c)),
    );
    let mut facts = Facts {
        build: b,
        successors: s,
        last_field: Some("U8"),
        unclassified,
        owned,
        faults: fa,
        plan: p,
        calls: c,
    };
    f(&mut facts);
    judge(&facts)
}

fn judged(f: impl FnOnce(&mut Facts<'_>)) -> Vec<String> {
    let o: &'static BTreeSet<String> = Box::leak(Box::new(owned()));
    judged_with(&[], o, f)
}

#[test]
fn an_oracle_that_is_not_a_number_is_refused() {
    // F2: `field-flip-last` flips a numeric field, so a Bool, Bytes, alias or composite oracle fails.
    for t in ["Bool", "Bytes", "Mine", ""] {
        let out = judged(|f| f.last_field = Some(t));
        assert_eq!(out.len(), 1, "{t}: {out:?}");
        assert!(out[0].contains("cannot flip"), "{out:?}");
    }
    assert_eq!(judged(|f| f.last_field = None).len(), 1);
    for t in ["U8", "U16", "U32", "U64", "I8", "I16", "I32", "I64", "Nat"] {
        assert!(judged(|f| f.last_field = Some(t)).is_empty(), "{t}");
    }
}

#[test]
fn the_numeric_predicate_is_the_generators_table() {
    // The generator's `is_numeric` accepts the checker's fixed-width types (`FixedTy`, which
    // `is_numeric_type` consults itself) and `Nat`.  This pins every fixed-width name accepted,
    // and the non-numeric names of the language refused.
    use coding_adventures_warrant_grammar::fixed::FixedTy;
    for t in FixedTy::ALL {
        assert!(is_numeric_type(t.name()), "{}", t.name());
    }
    for t in ["Bool", "Bytes", "Int", "Option", "List", "Impl", "u8"] {
        assert!(!is_numeric_type(t), "{t}");
    }
}

#[test]
fn a_binding_that_is_no_op_of_the_document_fails_closed() {
    let o = owned();
    assert!(judged_with(&[], &o, |_| {}).is_empty());
    let u = ["ghost".to_string()];
    let out = judged_with(&u, &o, |_| {});
    assert_eq!(out.len(), 1, "{out:?}");
    assert!(
        out[0].contains("no `op` of the generated document"),
        "{out:?}"
    );
}

#[test]
fn a_plan_entry_on_a_claim_the_contract_does_not_own_does_not_count() {
    // F3: `c1` is the implementation's own here (the contract owns nothing).
    let none = BTreeSet::new();
    let out = judged_with(&[], &none, |_| {});
    assert_eq!(out.len(), 1, "{out:?}");
    assert!(
        out[0].contains("claim of the contract's `any` tier"),
        "{out:?}"
    );
}

#[test]
fn unreadable_bindings_and_documents_are_refusals_with_a_message() {
    let doc = "warrant t v1\n  record M(a: Bool)\n  record Impl(x: U8)\n  op ins(s: Impl, t: U8) -> Impl\n  op shadow(s: Impl) -> U8\n  labels ins:\n    s public\n    t public\n  labels shadow:\n    s public\n  model build(m: M) -> Impl = (Impl(if m.a then 1u8 else 0u8))\n".to_string();
    let bad = load("not a bindings file\n", doc.clone()).expect("a verdict");
    assert!(bad.unwrap_err().starts_with("evidence.bindings:"));
    let good_bindings = "package p\nrevision 0.1.0\nspec code/specs/p.warrant\noutput code/packages/rust/p/tests/generated/warrant_evidence.rs\nseed 0x1\nproperty_cases 4096\nfuzz_iterations 5000\nnegctl_fuzz_iterations 1000\nexhaustive_budget 400000\nop f export=vos_ghost_f args=u8 ret=u8 faults=field-zero\nop ins export=vos_ghost_ins faults=field-zero\n";
    let bad_doc = load(
        good_bindings,
        "warrant t v1\n  claim c:\n    forall v: Bool. zz(v)\n".to_string(),
    )
    .expect("a verdict");
    assert!(bad_doc.unwrap_err().starts_with("the document:"));
    let d = load(good_bindings, doc).expect("a verdict").expect("loads");
    assert_eq!(
        d.build,
        BuildShape::Constructor {
            in_cone: vec![true]
        }
    );
    // `ins` is a successor although its binding spells neither `args=` nor `ret=` (F1); `f` is
    // bound but is no `op` of the document.
    assert_eq!(d.successors.into_iter().collect::<Vec<_>>(), ["ins"]);
    assert_eq!(d.unclassified, ["f"]);
    assert_eq!(d.last_field.as_deref(), Some("U8"));
}

#[test]
fn a_document_with_no_state_to_state_operation_fails() {
    let none = BTreeSet::new();
    let o = owned();
    let out = {
        let (b, _, fa, p, c) = good_facts();
        judge(&Facts {
            build: &b,
            successors: &none,
            last_field: Some("U8"),
            unclassified: &[],
            owned: &o,
            faults: &fa,
            plan: &p,
            calls: &c,
        })
    };
    assert_eq!(out.len(), 1, "{out:?}");
    assert!(out[0].contains("no `op` of the document returns `Impl` from an `Impl` argument"));
}
