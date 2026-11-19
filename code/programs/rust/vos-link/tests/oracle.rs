//! CC11's pure judge ([`vos_link::oracle`], VOS-039c): the cases a fixture crate cannot reach, because
//! the document analysis only returns a `BuildShape` that is a constructor (or not) and the other
//! shapes (a budget given up, a wide model) are decided by the walk's own limits.

use std::collections::{BTreeMap, BTreeSet};

use coding_adventures_warrant_grammar::binding::parse_evidence_bindings;
use vos_link::build_gen::PlanEntry;
use vos_link::oracle::{judge, load, successor_ops, Facts, ORACLE_FAULT};
use vos_manifest::warrant::BuildShape;

fn entry(claim: &str) -> PlanEntry {
    PlanEntry {
        tier: "exhaustive".into(),
        name: claim.into(),
        fault: ORACLE_FAULT.into(),
        attack: None,
        line: 1,
    }
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

#[test]
fn successors_are_the_state_to_state_operations_of_the_bindings() {
    let text = "package p\nrevision 0.1.0\nspec code/specs/p.warrant\noutput code/packages/rust/p/tests/generated/warrant_evidence.rs\nseed 0x1\nproperty_cases 4096\nfuzz_iterations 5000\nnegctl_fuzz_iterations 1000\nexhaustive_budget 400000\nstruct Impl path=p::Impl fields=m:u8,c:u8\nop init export=a ret=Impl faults=field-flip\nop ins export=b args=Impl,u8 ret=Impl faults=field-zero\nop ins_out export=c args=Impl,u8 ret=u8 faults=field-zero\nop pair export=d args=Impl,Impl ret=Impl faults=field-zero\n";
    let b = parse_evidence_bindings(text).expect("bindings");
    let s: Vec<String> = successor_ops(&b.hosts).into_iter().collect();
    // `init` has no state argument and `ins_out` returns a projection: neither is a successor.
    assert_eq!(s, ["ins", "pair"]);
}

#[test]
fn unreadable_bindings_and_documents_are_refusals_with_a_message() {
    let doc = "warrant t v1\n  record M(a: Bool)\n  record Impl(x: U8)\n  model build(m: M) -> Impl = (Impl(if m.a then 1u8 else 0u8))\n".to_string();
    let bad = load("not a bindings file\n", doc.clone()).expect("a verdict");
    assert!(bad.unwrap_err().starts_with("evidence.bindings:"));
    let good_bindings = "package p\nrevision 0.1.0\nspec code/specs/p.warrant\noutput code/packages/rust/p/tests/generated/warrant_evidence.rs\nseed 0x1\nproperty_cases 4096\nfuzz_iterations 5000\nnegctl_fuzz_iterations 1000\nexhaustive_budget 400000\nop f export=vos_ghost_f args=u8 ret=u8 faults=field-zero\n";
    let bad_doc = load(
        good_bindings,
        "warrant t v1\n  claim c:\n    forall v: Bool. zz(v)\n".to_string(),
    )
    .expect("a verdict");
    assert!(bad_doc.unwrap_err().starts_with("the document:"));
    let (shape, succ) = load(good_bindings, doc).expect("a verdict").expect("loads");
    assert_eq!(
        shape,
        BuildShape::Constructor {
            in_cone: vec![true]
        }
    );
    assert!(succ.is_empty());
}
