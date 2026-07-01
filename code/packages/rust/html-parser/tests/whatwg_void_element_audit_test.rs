mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_VOID_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-void-element-audit.json");
const POST_PARSE_REPAIR_EVIDENCE: &[(&str, &str)] = &[
    ("adoption01-dat-13", "void-in-table"),
    ("noscript01-dat-332", "metadata-void-elements"),
    ("menuitem-element-dat-311", "body-void-elements"),
    ("noscript01-dat-338", "stray-void-end-tags"),
    ("tests19-dat-1086", "void-foreign-boundary"),
    ("tests7-dat-17", "void-in-select"),
    ("tests6-dat-27", "void-fragment-context"),
    ("template-dat-494", "legacy-void-elements"),
];

#[derive(Debug, Deserialize)]
struct VoidElementAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<VoidElementAuditCase>,
}

#[derive(Debug, Deserialize)]
struct VoidElementAuditCase {
    id: String,
    source: String,
    axis: String,
    reason: String,
}

#[test]
fn whatwg_void_element_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-void-element-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 225);
    assert_axis_count(&suite, "void-in-table", 50);
    assert_axis_count(&suite, "metadata-void-elements", 50);
    assert_axis_count(&suite, "body-void-elements", 60);
    assert_axis_count(&suite, "stray-void-end-tags", 5);
    assert_axis_count(&suite, "void-foreign-boundary", 14);
    assert_axis_count(&suite, "void-in-select", 8);
    assert_axis_count(&suite, "void-fragment-context", 15);
    assert_axis_count(&suite, "legacy-void-elements", 15);
}

#[test]
fn whatwg_void_element_audit_cases_match_parser_dom_dump() {
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
fn whatwg_void_element_audit_tracks_post_parse_repair_evidence() {
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

fn load_suite() -> VoidElementAuditSuite {
    serde_json::from_str(WHATWG_VOID_ELEMENT_AUDIT)
        .expect("WHATWG void-element audit fixture should parse")
}

fn assert_axis_count(suite: &VoidElementAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
