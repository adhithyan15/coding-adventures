mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_TEXT_CONTROL_AUDIT: &str =
    include_str!("fixtures/whatwg-text-control-audit.json");

#[derive(Debug, Deserialize)]
struct TextControlAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<TextControlAuditCase>,
}

#[derive(Debug, Deserialize)]
struct TextControlAuditCase {
    id: String,
    source: String,
    axis: String,
    reason: String,
}

#[test]
fn whatwg_text_control_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-text-control-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 690);
    assert_axis_count(&suite, "script-rawtext", 360);
    assert_axis_count(&suite, "rcdata-controls", 80);
    assert_axis_count(&suite, "rawtext-elements", 95);
    assert_axis_count(&suite, "noscript-scripting", 45);
    assert_axis_count(&suite, "plaintext-recovery", 55);
    assert_axis_count(&suite, "pre-listing-newline", 25);
    assert_axis_count(&suite, "fragment-context", 6);
    assert_axis_count(&suite, "stray-text-control-end-tags", 6);
}

#[test]
fn whatwg_text_control_audit_cases_match_parser_dom_dump() {
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

fn load_suite() -> TextControlAuditSuite {
    serde_json::from_str(WHATWG_TEXT_CONTROL_AUDIT)
        .expect("WHATWG text-control audit fixture should parse")
}

fn assert_axis_count(suite: &TextControlAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
