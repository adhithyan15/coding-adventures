mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_BLOCK_BOUNDARY_AUDIT: &str = include_str!("fixtures/whatwg-block-boundary-audit.json");
const WHATWG_DOCUMENT_SHELL_AUDIT: &str = include_str!("fixtures/whatwg-document-shell-audit.json");
const WHATWG_FOREIGN_AUDIT: &str = include_str!("fixtures/whatwg-foreign-audit.json");
const WHATWG_FORM_INTERACTIVE_AUDIT: &str =
    include_str!("fixtures/whatwg-form-interactive-audit.json");
const WHATWG_FORMATTING_AUDIT: &str = include_str!("fixtures/whatwg-formatting-audit.json");
const WHATWG_FRAGMENT_CONTEXT_AUDIT: &str =
    include_str!("fixtures/whatwg-fragment-context-audit.json");
const WHATWG_HEAD_BODY_AUDIT: &str = include_str!("fixtures/whatwg-head-body-audit.json");
const WHATWG_LEGACY_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-legacy-element-audit.json");
const WHATWG_LIST_ITEM_AUDIT: &str = include_str!("fixtures/whatwg-list-item-audit.json");
const WHATWG_PARAGRAPH_AUDIT: &str = include_str!("fixtures/whatwg-paragraph-audit.json");
const WHATWG_RUBY_AUDIT: &str = include_str!("fixtures/whatwg-ruby-audit.json");
const WHATWG_SELECT_LIST_AUDIT: &str = include_str!("fixtures/whatwg-select-list-audit.json");
const WHATWG_TABLE_AUDIT: &str = include_str!("fixtures/whatwg-table-audit.json");
const WHATWG_TEXT_CONTROL_AUDIT: &str = include_str!("fixtures/whatwg-text-control-audit.json");
const WHATWG_TREE_INSERTION_AUDIT: &str = include_str!("fixtures/whatwg-tree-insertion-audit.json");

struct BlockBoundaryCrossAxisCase {
    id: &'static str,
    block_axis: &'static str,
    data_snippet: &'static str,
    fragment_context: Option<&'static str>,
    suites: &'static [(&'static str, &'static str, &'static str)],
}

const BLOCK_LIST_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "list-definition-boundary",
    ),
    ("list-item", WHATWG_LIST_ITEM_AUDIT, "list-with-paragraph"),
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-block-boundary",
    ),
];
const BLOCK_FORM_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "button-boundary",
    ),
    ("formatting", WHATWG_FORMATTING_AUDIT, "paragraph-boundary"),
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-form-boundary",
    ),
];
const BLOCK_TABLE_CROSS_AXIS_SUITES: &[(&str, &str, &str)] =
    &[("table", WHATWG_TABLE_AUDIT, "caption-colgroup")];
const BLOCK_FORMATTING_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "html-element-boundary",
    ),
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
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "body-boundary"),
    (
        "legacy-element",
        WHATWG_LEGACY_ELEMENT_AUDIT,
        "tricky-parser-recovery",
    ),
];
const BLOCK_RUBY_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "html-element-boundary",
    ),
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "ruby-implied-end-tags",
    ),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "body-boundary"),
    ("ruby", WHATWG_RUBY_AUDIT, "block-in-ruby"),
];
const BLOCK_SELECT_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
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
    ("select-list", WHATWG_SELECT_LIST_AUDIT, "select-shell"),
];
const BLOCK_TEXT_MODE_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "html-element-boundary",
    ),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "body-boundary"),
    (
        "text-control",
        WHATWG_TEXT_CONTROL_AUDIT,
        "pre-listing-newline",
    ),
];
const BLOCK_FRAGMENT_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    ("foreign", WHATWG_FOREIGN_AUDIT, "foreign-fragment"),
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
const BLOCK_BOUNDARY_CROSS_AXIS_CASES: &[BlockBoundaryCrossAxisCase] = &[
    BlockBoundaryCrossAxisCase {
        id: "blocks-dat-38",
        block_axis: "block-list-container-boundary",
        data_snippet: "<!doctype html><p>foo<dl>bar<p>baz",
        fragment_context: None,
        suites: BLOCK_LIST_CROSS_AXIS_SUITES,
    },
    BlockBoundaryCrossAxisCase {
        id: "tests20-dat-6",
        block_axis: "block-form-boundary",
        data_snippet: "<!doctype html><p><button><center>",
        fragment_context: None,
        suites: BLOCK_FORM_CROSS_AXIS_SUITES,
    },
    BlockBoundaryCrossAxisCase {
        id: "tests6-dat-17",
        block_axis: "block-table-boundary",
        data_snippet: "<table><caption><div>",
        fragment_context: None,
        suites: BLOCK_TABLE_CROSS_AXIS_SUITES,
    },
    BlockBoundaryCrossAxisCase {
        id: "tricky01-dat-5",
        block_axis: "block-formatting-boundary",
        data_snippet: "<label><a><div>Hello",
        fragment_context: None,
        suites: BLOCK_FORMATTING_CROSS_AXIS_SUITES,
    },
    BlockBoundaryCrossAxisCase {
        id: "webkit01-dat-29",
        block_axis: "block-ruby-boundary",
        data_snippet: "<ruby><div><rp>xx",
        fragment_context: None,
        suites: BLOCK_RUBY_CROSS_AXIS_SUITES,
    },
    BlockBoundaryCrossAxisCase {
        id: "webkit02-dat-36",
        block_axis: "block-select-boundary",
        data_snippet: "<select><div><i></div><option>option",
        fragment_context: None,
        suites: BLOCK_SELECT_CROSS_AXIS_SUITES,
    },
    BlockBoundaryCrossAxisCase {
        id: "tests3-dat-11",
        block_axis: "block-text-mode-boundary",
        data_snippet: "<pre>x<div>\ny</pre>",
        fragment_context: None,
        suites: BLOCK_TEXT_MODE_CROSS_AXIS_SUITES,
    },
    BlockBoundaryCrossAxisCase {
        id: "foreign-fragment-dat-21",
        block_axis: "block-fragment-context",
        data_snippet: "<div></div>",
        fragment_context: Some("math ms"),
        suites: BLOCK_FRAGMENT_CROSS_AXIS_SUITES,
    },
];
const POST_PARSE_REPAIR_EVIDENCE: &[(&str, &str)] = &[
    ("tricky01-dat-5", "block-formatting-boundary"),
    ("tricky01-dat-6", "block-table-boundary"),
    ("tricky01-dat-8", "block-table-boundary"),
    ("tricky01-dat-9", "block-text-mode-boundary"),
];

#[derive(Debug, Deserialize)]
struct BlockBoundaryAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<BlockBoundaryAuditCase>,
}

#[derive(Debug, Deserialize)]
struct BlockBoundaryAuditCase {
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
fn whatwg_block_boundary_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-block-boundary-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 425);
    assert_axis_count(&suite, "block-grouping-boundary", 85);
    assert_axis_count(&suite, "block-formatting-boundary", 65);
    assert_axis_count(&suite, "block-foreign-boundary", 55);
    assert_axis_count(&suite, "block-form-boundary", 50);
    assert_axis_count(&suite, "block-table-boundary", 40);
    assert_axis_count(&suite, "block-fragment-context", 30);
    assert_axis_count(&suite, "block-sectioning-boundary", 20);
    assert_axis_count(&suite, "block-list-container-boundary", 18);
    assert_axis_count(&suite, "block-template-boundary", 15);
    assert_axis_count(&suite, "block-text-mode-boundary", 13);
    assert_axis_count(&suite, "block-heading-boundary", 12);
    assert_axis_count(&suite, "block-ruby-boundary", 10);
    assert_axis_count(&suite, "block-select-boundary", 5);
}

#[test]
fn whatwg_block_boundary_audit_cases_match_parser_dom_dump() {
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
fn whatwg_block_boundary_audit_keeps_block_axis_cases_cross_axis() {
    let suite = load_suite();
    let block_cases = suite
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<HashMap<_, _>>();
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for evidence in BLOCK_BOUNDARY_CROSS_AXIS_CASES {
        let block_case = block_cases
            .get(evidence.id)
            .unwrap_or_else(|| panic!("block-boundary audit should include `{}`", evidence.id));
        assert_eq!(
            block_case.axis, evidence.block_axis,
            "block-boundary row `{}` should stay on its focused audit axis",
            evidence.id
        );
        assert!(
            !block_case.reason.is_empty(),
            "block-boundary row `{}` should keep a fixture reason",
            evidence.id
        );

        for (suite_name, fixture, expected_axis) in evidence.suites {
            let audit_case = generic_audit_case(fixture, suite_name, evidence.id);
            assert_eq!(
                audit_case.source, block_case.source,
                "`{suite_name}` should point block-boundary row `{}` at the same html5lib row as the block-boundary audit",
                evidence.id
            );
            assert_eq!(
                audit_case.axis, *expected_axis,
                "`{suite_name}` should keep block-boundary row `{}` on its focused axis",
                evidence.id
            );
            assert!(
                !audit_case.reason.is_empty(),
                "`{suite_name}` should keep a reason for block-boundary row `{}`",
                evidence.id
            );
        }

        let source_case = smoke_cases
            .get(&block_case.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", evidence.id));
        assert!(
            source_case.data.contains(evidence.data_snippet),
            "block-boundary row `{}` should stay tied to its html5lib input",
            evidence.id
        );
        if let Some(fragment_context) = evidence.fragment_context {
            assert_eq!(
                source_case.fragment_context.as_deref(),
                Some(fragment_context),
                "block-boundary row `{}` should keep its html5lib fragment context",
                evidence.id
            );
        }

        let actual = actual_dom_dump_for_tree_case(source_case).unwrap_or_else(|error| {
            panic!(
                "case `{}` ({}) parse failed: {error}",
                block_case.id, block_case.axis
            )
        });

        assert_eq!(
            actual, source_case.document,
            "cross-axis block-boundary evidence case `{}` ({}) failed for input {:?}",
            block_case.id, block_case.axis, source_case.data
        );
    }
}

#[test]
fn whatwg_block_boundary_audit_tracks_post_parse_repair_evidence() {
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

fn load_suite() -> BlockBoundaryAuditSuite {
    serde_json::from_str(WHATWG_BLOCK_BOUNDARY_AUDIT)
        .expect("WHATWG block-boundary audit fixture should parse")
}

fn generic_audit_case(fixture: &str, suite_name: &str, case_id: &str) -> GenericAuditCase {
    let suite = serde_json::from_str::<GenericAuditSuite>(fixture)
        .unwrap_or_else(|error| panic!("`{suite_name}` audit fixture should parse: {error}"));
    suite
        .cases
        .into_iter()
        .find(|case| case.id == case_id)
        .unwrap_or_else(|| panic!("`{suite_name}` should audit block-boundary case `{case_id}`"))
}

fn assert_axis_count(suite: &BlockBoundaryAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
