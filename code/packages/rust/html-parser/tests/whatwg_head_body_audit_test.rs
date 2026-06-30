mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_HEAD_BODY_AUDIT: &str = include_str!("fixtures/whatwg-head-body-audit.json");
const POST_PARSE_REPAIR_EVIDENCE: &[(&str, &str)] = &[
    ("scripted-adoption01-dat-1", "head-text-mode"),
    ("tests26-dat-1251", "body-boundary"),
    ("tricky01-dat-3", "body-boundary"),
];

#[derive(Debug, Deserialize)]
struct HeadBodyAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<HeadBodyAuditCase>,
}

#[derive(Debug, Deserialize)]
struct HeadBodyAuditCase {
    id: String,
    source: String,
    axis: String,
    reason: String,
}

#[test]
fn whatwg_head_body_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-head-body-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 1000);
    assert_axis_count(&suite, "head-text-mode", 475);
    assert_axis_count(&suite, "head-metadata-boundary", 50);
    assert_axis_count(&suite, "body-frameset-transition", 150);
    assert_axis_count(&suite, "body-boundary", 275);
    assert_axis_count(&suite, "head-boundary", 45);
    assert_axis_count(&suite, "html-boundary", 70);
}

#[test]
fn whatwg_head_body_audit_cases_match_parser_dom_dump() {
    let suite = load_suite();
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for case in &suite.cases {
        assert!(
            !case.id.is_empty() && !case.axis.is_empty() && !case.reason.is_empty(),
            "case `{}` should carry audit metadata",
            case.source
        );
        let source_case = smoke_cases
            .get(&case.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", case.source));
        let actual = actual_dom_dump_for_tree_case(source_case).unwrap_or_else(|error| {
            panic!("case `{}` ({}) parse failed: {error}", case.id, case.axis)
        });

        assert_eq!(
            actual, source_case.document,
            "case `{}` ({}) failed for input {:?}",
            case.id, case.axis, source_case.data
        );
    }
}

#[test]
fn whatwg_head_body_audit_tracks_post_parse_repair_evidence() {
    let suite = load_suite();
    let audit_cases = suite
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<HashMap<_, _>>();
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for (case_id, expected_axis) in POST_PARSE_REPAIR_EVIDENCE {
        let audit_case = audit_cases.get(case_id).unwrap_or_else(|| {
            panic!("post-parse repair evidence case `{case_id}` should be audited")
        });
        assert_eq!(
            audit_case.axis, *expected_axis,
            "post-parse repair evidence case `{case_id}` should stay on its focused audit axis"
        );

        let source_case = smoke_cases
            .get(&audit_case.source)
            .unwrap_or_else(|| panic!("case `{case_id}` should exist in smoke fixture"));
        let actual = actual_dom_dump_for_tree_case(source_case).unwrap_or_else(|error| {
            panic!(
                "case `{}` ({}) parse failed: {error}",
                audit_case.id, audit_case.axis
            )
        });

        assert_eq!(
            actual, source_case.document,
            "post-parse repair evidence case `{}` ({}) failed for input {:?}",
            audit_case.id, audit_case.axis, source_case.data
        );
    }
}

fn load_suite() -> HeadBodyAuditSuite {
    serde_json::from_str(WHATWG_HEAD_BODY_AUDIT)
        .expect("WHATWG head/body audit fixture should parse")
}

fn assert_axis_count(suite: &HeadBodyAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
