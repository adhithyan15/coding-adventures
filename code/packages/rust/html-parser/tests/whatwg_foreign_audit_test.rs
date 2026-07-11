mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_BLOCK_BOUNDARY_AUDIT: &str = include_str!("fixtures/whatwg-block-boundary-audit.json");
const WHATWG_FOREIGN_AUDIT: &str = include_str!("fixtures/whatwg-foreign-audit.json");
const WHATWG_FORM_INTERACTIVE_AUDIT: &str =
    include_str!("fixtures/whatwg-form-interactive-audit.json");
const WHATWG_FORMATTING_AUDIT: &str = include_str!("fixtures/whatwg-formatting-audit.json");
const WHATWG_FRAGMENT_CONTEXT_AUDIT: &str =
    include_str!("fixtures/whatwg-fragment-context-audit.json");
const WHATWG_LEGACY_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-legacy-element-audit.json");
const WHATWG_PARAGRAPH_AUDIT: &str = include_str!("fixtures/whatwg-paragraph-audit.json");
const WHATWG_SELECT_LIST_AUDIT: &str = include_str!("fixtures/whatwg-select-list-audit.json");
const WHATWG_TABLE_AUDIT: &str = include_str!("fixtures/whatwg-table-audit.json");
const WHATWG_TREE_INSERTION_AUDIT: &str = include_str!("fixtures/whatwg-tree-insertion-audit.json");
const WHATWG_VOID_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-void-element-audit.json");
struct ForeignCrossAxisCase {
    id: &'static str,
    foreign_axis: &'static str,
    data_snippet: &'static str,
    fragment_context: Option<&'static str>,
    suites: &'static [(&'static str, &'static str, &'static str)],
}

const SVG_BOUNDARY_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-foreign-boundary",
    ),
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "interactive-formatting",
    ),
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "interactive-formatting-boundary",
    ),
    (
        "legacy-element",
        WHATWG_LEGACY_ELEMENT_AUDIT,
        "main-element-boundary",
    ),
];
const MATHML_BOUNDARY_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-foreign-boundary",
    ),
    ("formatting", WHATWG_FORMATTING_AUDIT, "heading-boundary"),
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-heading-boundary",
    ),
];
const HTML_INTEGRATION_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-table-boundary",
    ),
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "select-option",
    ),
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "adoption-agency-formatting",
    ),
    ("select-list", WHATWG_SELECT_LIST_AUDIT, "select-in-table"),
    ("table", WHATWG_TABLE_AUDIT, "select-in-table"),
    (
        "tree-insertion",
        WHATWG_TREE_INSERTION_AUDIT,
        "table-insertion",
    ),
];
const TABLE_FOREIGN_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "form-control",
    ),
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "interactive-formatting-boundary",
    ),
    ("table", WHATWG_TABLE_AUDIT, "row-group-boundary"),
    (
        "tree-insertion",
        WHATWG_TREE_INSERTION_AUDIT,
        "adoption-agency",
    ),
    ("void-element", WHATWG_VOID_ELEMENT_AUDIT, "void-in-table"),
];
const FOREIGN_FRAGMENT_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-fragment-context",
    ),
    (
        "fragment-context",
        WHATWG_FRAGMENT_CONTEXT_AUDIT,
        "fragment-foreign-context",
    ),
    (
        "tree-insertion",
        WHATWG_TREE_INSERTION_AUDIT,
        "foreign-fragment",
    ),
];
const FOREIGN_CROSS_AXIS_CASES: &[ForeignCrossAxisCase] = &[
    ForeignCrossAxisCase {
        id: "main-element-dat-305",
        foreign_axis: "svg-boundary",
        data_snippet: "<!DOCTYPE html>xxx<svg><x><g><a><main><b>",
        fragment_context: None,
        suites: SVG_BOUNDARY_CROSS_AXIS_SUITES,
    },
    ForeignCrossAxisCase {
        id: "tests19-dat-1044",
        foreign_axis: "mathml-boundary",
        data_snippet: "<!doctype html><p><math><mi><p><h1>",
        fragment_context: None,
        suites: MATHML_BOUNDARY_CROSS_AXIS_SUITES,
    },
    ForeignCrossAxisCase {
        id: "tables01-dat-453",
        foreign_axis: "html-integration-point",
        data_snippet: "<div><table><svg><foreignObject><select><table><s>",
        fragment_context: None,
        suites: HTML_INTEGRATION_CROSS_AXIS_SUITES,
    },
    ForeignCrossAxisCase {
        id: "adoption01-dat-13",
        foreign_axis: "table-foreign-boundary",
        data_snippet: "<a><svg><tr><input></a>",
        fragment_context: None,
        suites: TABLE_FOREIGN_CROSS_AXIS_SUITES,
    },
    ForeignCrossAxisCase {
        id: "foreign-fragment-dat-21",
        foreign_axis: "foreign-fragment",
        data_snippet: "<div></div>",
        fragment_context: Some("math ms"),
        suites: FOREIGN_FRAGMENT_CROSS_AXIS_SUITES,
    },
];
const POST_PARSE_REPAIR_EVIDENCE: &[(&str, &str)] = &[
    ("domjs-unsafe-dat-121", "svg-boundary"),
    ("html5test-com-dat-294", "mathml-boundary"),
    ("namespace-sensitivity-dat-326", "html-integration-point"),
    ("adoption01-dat-13", "table-foreign-boundary"),
    ("foreign-fragment-dat-1", "foreign-fragment"),
];

#[derive(Debug, Deserialize)]
struct ForeignAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<ForeignAuditCase>,
}

#[derive(Debug, Deserialize)]
struct ForeignAuditCase {
    id: String,
    source: String,
    axis: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct GenericAuditSuite {
    cases: Vec<GenericAuditCase>,
}

#[derive(Debug, Deserialize)]
struct GenericAuditCase {
    id: String,
    source: String,
    axis: String,
    reason: String,
}

#[test]
fn whatwg_foreign_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-foreign-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 420);
    assert_axis_count(&suite, "svg-boundary", 125);
    assert_axis_count(&suite, "mathml-boundary", 75);
    assert_axis_count(&suite, "html-integration-point", 65);
    assert_axis_count(&suite, "table-foreign-boundary", 50);
    assert_axis_count(&suite, "foreign-fragment", 60);
}

#[test]
fn whatwg_foreign_audit_cases_match_parser_dom_dump() {
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
fn whatwg_foreign_audit_keeps_foreign_axis_cases_cross_axis() {
    let suite = load_suite();
    let foreign_cases = suite
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<HashMap<_, _>>();
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for evidence in FOREIGN_CROSS_AXIS_CASES {
        let foreign_case = foreign_cases
            .get(evidence.id)
            .unwrap_or_else(|| panic!("foreign audit should include `{}`", evidence.id));
        assert_eq!(
            foreign_case.axis, evidence.foreign_axis,
            "foreign row `{}` should stay on its focused audit axis",
            evidence.id
        );
        assert!(
            !foreign_case.reason.is_empty(),
            "foreign row `{}` should keep a fixture reason",
            evidence.id
        );

        for (suite_name, fixture, expected_axis) in evidence.suites {
            let audit_case = generic_audit_case(fixture, suite_name, evidence.id);
            assert_eq!(
                audit_case.source, foreign_case.source,
                "`{suite_name}` should point foreign row `{}` at the same html5lib row as the foreign audit",
                evidence.id
            );
            assert_eq!(
                audit_case.axis, *expected_axis,
                "`{suite_name}` should keep foreign row `{}` on its focused axis",
                evidence.id
            );
            assert!(
                !audit_case.reason.is_empty(),
                "`{suite_name}` should keep a reason for foreign row `{}`",
                evidence.id
            );
        }

        let source_case = smoke_cases
            .get(&foreign_case.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", evidence.id));
        assert!(
            source_case.data.contains(evidence.data_snippet),
            "foreign row `{}` should stay tied to its html5lib input",
            evidence.id
        );
        if let Some(fragment_context) = evidence.fragment_context {
            assert_eq!(
                source_case.fragment_context.as_deref(),
                Some(fragment_context),
                "foreign row `{}` should keep its html5lib fragment context",
                evidence.id
            );
        }

        let actual = actual_dom_dump_for_tree_case(source_case).unwrap_or_else(|error| {
            panic!(
                "case `{}` ({}) parse failed: {error}",
                foreign_case.id, foreign_case.axis
            )
        });

        assert_eq!(
            actual, source_case.document,
            "cross-axis foreign evidence case `{}` ({}) failed for input {:?}",
            foreign_case.id, foreign_case.axis, source_case.data
        );
    }
}

#[test]
fn whatwg_foreign_audit_tracks_post_parse_repair_evidence() {
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

fn load_suite() -> ForeignAuditSuite {
    serde_json::from_str(WHATWG_FOREIGN_AUDIT).expect("WHATWG foreign audit fixture should parse")
}

fn assert_axis_count(suite: &ForeignAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}

fn generic_audit_case(
    raw_fixture: &str,
    suite_name: &str,
    case_id: &str,
) -> GenericAuditCase {
    let suite = serde_json::from_str::<GenericAuditSuite>(raw_fixture)
        .unwrap_or_else(|error| panic!("{suite_name} audit fixture should parse: {error}"));
    suite
        .cases
        .into_iter()
        .find(|case| case.id == case_id)
        .unwrap_or_else(|| panic!("{suite_name} audit should include `{case_id}`"))
}
