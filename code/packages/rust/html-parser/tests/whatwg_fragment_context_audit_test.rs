mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_FRAGMENT_CONTEXT_AUDIT: &str =
    include_str!("fixtures/whatwg-fragment-context-audit.json");
const POST_PARSE_REPAIR_EVIDENCE: &[(&str, &str, &str)] = &[
    ("tests4-dat-1", "div", "fragment-basic-context"),
    ("tests4-dat-2", "textarea", "fragment-text-mode-context"),
    ("tests4-dat-6", "html", "fragment-shell-context"),
    ("tests6-dat-7", "div", "fragment-block-context"),
    ("tests6-dat-18", "caption", "fragment-table-context"),
    (
        "tests-innerhtml-1-dat-75",
        "select",
        "fragment-select-context",
    ),
    (
        "foreign-fragment-dat-1",
        "svg path",
        "fragment-foreign-context",
    ),
    ("template-dat-109", "template", "fragment-template-context"),
];

#[derive(Debug, Deserialize)]
struct FragmentContextAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<FragmentContextAuditCase>,
}

#[derive(Debug, Deserialize)]
struct FragmentContextAuditCase {
    id: String,
    source: String,
    context: String,
    axis: String,
    reason: String,
}

#[test]
fn whatwg_fragment_context_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-fragment-context-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 190);
    assert_axis_count(&suite, "fragment-table-context", 90);
    assert_axis_count(&suite, "fragment-foreign-context", 60);
    assert_axis_count(&suite, "fragment-shell-context", 13);
    assert_axis_count(&suite, "fragment-text-mode-context", 6);
    assert_axis_count(&suite, "fragment-block-context", 5);
    assert_axis_count(&suite, "fragment-select-context", 5);
    assert_axis_count(&suite, "fragment-basic-context", 3);
    assert_axis_count(&suite, "fragment-template-context", 1);
}

#[test]
fn whatwg_fragment_context_audit_cases_match_parser_dom_dump() {
    let suite = load_suite();
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for case in &suite.cases {
        assert!(
            !case.id.is_empty()
                && !case.context.is_empty()
                && !case.axis.is_empty()
                && !case.reason.is_empty(),
            "case `{}` should carry audit metadata",
            case.source
        );
        let source_case = smoke_cases
            .get(&case.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", case.source));
        assert_eq!(
            source_case.fragment_context.as_deref(),
            Some(case.context.as_str()),
            "case `{}` should keep its fragment context",
            case.source
        );
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
fn whatwg_fragment_context_audit_tracks_post_parse_repair_evidence() {
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

    for (case_id, expected_context, expected_axis) in POST_PARSE_REPAIR_EVIDENCE {
        let audit_case = audit_cases.get(case_id).unwrap_or_else(|| {
            panic!("post-parse repair evidence case `{case_id}` should be audited")
        });
        assert_eq!(
            audit_case.context, *expected_context,
            "post-parse repair evidence case `{case_id}` should keep its focused fragment context"
        );
        assert_eq!(
            audit_case.axis, *expected_axis,
            "post-parse repair evidence case `{case_id}` should stay on its focused audit axis"
        );

        let source_case = smoke_cases
            .get(&audit_case.source)
            .unwrap_or_else(|| panic!("case `{case_id}` should exist in smoke fixture"));
        assert_eq!(
            source_case.fragment_context.as_deref(),
            Some(*expected_context),
            "post-parse repair evidence case `{case_id}` should keep its smoke fragment context"
        );
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

fn load_suite() -> FragmentContextAuditSuite {
    serde_json::from_str(WHATWG_FRAGMENT_CONTEXT_AUDIT)
        .expect("WHATWG fragment-context audit fixture should parse")
}

fn assert_axis_count(suite: &FragmentContextAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
