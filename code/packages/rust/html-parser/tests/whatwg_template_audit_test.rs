mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_TEMPLATE_AUDIT: &str = include_str!("fixtures/whatwg-template-audit.json");

#[derive(Debug, Deserialize)]
struct TemplateAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<TemplateAuditCase>,
}

#[derive(Debug, Deserialize)]
struct TemplateAuditCase {
    id: String,
    source: String,
    axis: String,
    reason: String,
}

#[test]
fn whatwg_template_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-template-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 115);
    assert_axis_count(&suite, "template-shell", 4);
    assert_axis_count(&suite, "table-template", 15);
    assert_axis_count(&suite, "select-template", 5);
    assert_axis_count(&suite, "frameset-template", 3);
    assert_axis_count(&suite, "nested-template", 20);
    assert_axis_count(&suite, "eof-template", 15);
    assert_axis_count(&suite, "head-body-template", 8);
    assert_axis_count(&suite, "rawtext-template", 4);
    assert_axis_count(&suite, "foreign-content-template", 2);
    assert_axis_count(&suite, "template-fragment-context", 1);
}

#[test]
fn whatwg_template_audit_cases_match_parser_dom_dump() {
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

fn load_suite() -> TemplateAuditSuite {
    serde_json::from_str(WHATWG_TEMPLATE_AUDIT)
        .expect("WHATWG template audit fixture should parse")
}

fn assert_axis_count(suite: &TemplateAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
