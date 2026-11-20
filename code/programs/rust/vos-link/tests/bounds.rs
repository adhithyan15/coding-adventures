//! CC09's pure comparison (VOS-039b-2b): the derived operation-to-ceiling mapping and the
//! judgement, on the REAL scheduler contract and hand-built bound facts.  The end-to-end
//! negative controls (a fixture crate broken one way at a time) are in `component.rs`.

use std::collections::BTreeSet;

use vos_link::bounds::{ceiling_op, judge, MAP_SUFFIXES};
use vos_manifest::contract::parse_contract;
use vos_manifest::warrant::{BoundFact, BoundFacts};

fn contract() -> vos_manifest::Contract {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../packages/rust/vos-sched-contract/contract.vos");
    parse_contract(&std::fs::read(path).unwrap()).expect("the real contract parses")
}

fn facts(ops: &[&str], rows: &[(&str, &str, Option<u64>)]) -> BoundFacts {
    BoundFacts {
        ops: ops.iter().map(|s| s.to_string()).collect(),
        rows: rows
            .iter()
            .map(|(op, metric, value)| BoundFact {
                op: op.to_string(),
                metric: metric.to_string(),
                value: *value,
            })
            .collect(),
    }
}

/// Every mapped operation with its `steps` and `heap` rows at the ceiling.
fn full() -> BoundFacts {
    let mut ops = Vec::new();
    let mut rows = Vec::new();
    for (op, steps) in [
        ("init", 80),
        ("insert_out", 160),
        ("insert_img", 160),
        ("remove_out", 160),
        ("remove_img", 160),
        ("dispatch_out", 160),
        ("dispatch_img", 160),
    ] {
        ops.push(op);
        rows.push((op, "steps", Some(steps)));
        rows.push((op, "heap", Some(0)));
    }
    facts(&ops, &rows)
}

#[test]
fn the_mapping_is_x_itself_else_one_suffix_stripped_else_none() {
    let set: BTreeSet<&str> = ["init", "insert", "remove", "dispatch"].into();
    let m = |op: &str| ceiling_op(op, &set);
    assert_eq!(m("init").as_deref(), Some("init"));
    assert_eq!(m("insert").as_deref(), Some("insert"));
    assert_eq!(m("insert_out").as_deref(), Some("insert"));
    assert_eq!(m("dispatch_img").as_deref(), Some("dispatch"));
    // Ghost-only operations map to nothing.
    assert_eq!(m("alpha_w"), None);
    assert_eq!(m("cfg_n"), None);
    // ONE suffix: `insert_out_img` strips to `insert_out`, which is no ceiling operation.
    assert_eq!(m("insert_out_img"), None);
    // A suffix on its own, or an unknown stem, is nothing.
    assert_eq!(m("_out"), None);
    assert_eq!(m("pick_out"), None);
    // X itself wins over a suffix reading.
    let set: BTreeSet<&str> = ["x", "x_out"].into();
    assert_eq!(ceiling_op("x_out", &set).as_deref(), Some("x_out"));
}

#[test]
fn the_suffix_vocabulary_is_the_one_the_spec_states() {
    let spec = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../specs/VOS04-component-catalogue.md"),
    )
    .unwrap();
    for s in MAP_SUFFIXES {
        assert!(spec.contains(&format!("`{s}`")), "VOS04 must state {s}");
    }
    assert!(
        spec.contains("`MAP_SUFFIXES`") && spec.contains("ONE suffix `_out` or `_img`"),
        "VOS04 5.6 must state the derived mapping"
    );
    assert_eq!(MAP_SUFFIXES, ["_out", "_img"]);
}

#[test]
fn the_real_contract_ceilings_are_met_exactly_by_the_schedulers_numbers() {
    let v = judge(true, &contract(), &full());
    assert_eq!(v.findings, Vec::<String>::new());
    // depth is a note, never a finding, when absent.
    assert_eq!(v.notes.len(), 1);
    assert!(v.notes[0].contains("`depth` is not evidenced"));
}

#[test]
fn a_ghost_row_is_noted_and_never_compared_even_far_above_every_ceiling() {
    let mut f = full();
    f.ops.push("alpha_w".to_string());
    f.rows.push(BoundFact {
        op: "alpha_w".to_string(),
        metric: "steps".to_string(),
        value: Some(u64::MAX),
    });
    // A `bound` for an operation the document does not even declare is a ghost row too.
    f.rows.push(BoundFact {
        op: "cfg_n".to_string(),
        metric: "heap".to_string(),
        value: None,
    });
    let v = judge(true, &contract(), &f);
    assert!(v.findings.is_empty(), "{:?}", v.findings);
    assert!(v
        .notes
        .iter()
        .any(|n| n.contains("\"alpha_w\", \"cfg_n\" map to no contract ceiling")));
}

#[test]
fn a_metric_the_contract_does_not_ceil_is_not_compared() {
    let mut f = full();
    f.rows.push(BoundFact {
        op: "init".to_string(),
        metric: "output".to_string(),
        value: Some(1_000_000),
    });
    assert!(judge(true, &contract(), &f).findings.is_empty());
}

#[test]
fn every_ceiling_operation_must_be_realised_by_some_operation() {
    let mut f = full();
    f.ops.retain(|o| !o.starts_with("dispatch"));
    f.rows.retain(|r| !r.op.starts_with("dispatch"));
    let v = judge(true, &contract(), &f);
    assert_eq!(v.findings.len(), 1, "{:?}", v.findings);
    assert!(v.findings[0]
        .contains("operation \"dispatch\" but no operation of the document maps to it"));
    // The reference kind is held to this too.
    assert_eq!(judge(false, &contract(), &f).findings.len(), 1);
}

#[test]
fn a_duplicate_row_cannot_hide_a_large_one_behind_a_small_one() {
    let mut f = full();
    f.rows.push(BoundFact {
        op: "insert_out".to_string(),
        metric: "steps".to_string(),
        value: Some(161),
    });
    let v = judge(true, &contract(), &f);
    assert_eq!(v.findings.len(), 1, "{:?}", v.findings);
    assert!(v.findings[0].contains("<= 161 is above the ceiling 160"));
}
