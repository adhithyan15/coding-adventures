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
const WHATWG_HEAD_BODY_AUDIT: &str = include_str!("fixtures/whatwg-head-body-audit.json");
const WHATWG_LIST_ITEM_AUDIT: &str = include_str!("fixtures/whatwg-list-item-audit.json");
const WHATWG_PARAGRAPH_AUDIT: &str = include_str!("fixtures/whatwg-paragraph-audit.json");
const WHATWG_RUBY_AUDIT: &str = include_str!("fixtures/whatwg-ruby-audit.json");
const WHATWG_SELECT_LIST_AUDIT: &str = include_str!("fixtures/whatwg-select-list-audit.json");
const WHATWG_TABLE_AUDIT: &str = include_str!("fixtures/whatwg-table-audit.json");
const WHATWG_TEMPLATE_AUDIT: &str = include_str!("fixtures/whatwg-template-audit.json");
const WHATWG_TREE_INSERTION_AUDIT: &str = include_str!("fixtures/whatwg-tree-insertion-audit.json");
const WHATWG_VOID_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-void-element-audit.json");
struct FormattingCrossAxisCase {
    id: &'static str,
    formatting_axis: &'static str,
    data_snippet: &'static str,
    suites: &'static [(&'static str, &'static str, &'static str)],
}

const ADOPTION_FORMATTING_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-table-boundary",
    ),
    ("foreign", WHATWG_FOREIGN_AUDIT, "html-integration-point"),
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "select-option",
    ),
    ("select-list", WHATWG_SELECT_LIST_AUDIT, "select-in-table"),
    ("table", WHATWG_TABLE_AUDIT, "select-in-table"),
    (
        "tree-insertion",
        WHATWG_TREE_INSERTION_AUDIT,
        "table-insertion",
    ),
];
const INTERACTIVE_FORMATTING_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    ("foreign", WHATWG_FOREIGN_AUDIT, "table-foreign-boundary"),
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "form-control",
    ),
    ("table", WHATWG_TABLE_AUDIT, "row-group-boundary"),
    (
        "tree-insertion",
        WHATWG_TREE_INSERTION_AUDIT,
        "adoption-agency",
    ),
    ("void-element", WHATWG_VOID_ELEMENT_AUDIT, "void-in-table"),
];
const PARAGRAPH_FORMATTING_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "body-frameset-boundary",
    ),
    ("foreign", WHATWG_FOREIGN_AUDIT, "table-foreign-boundary"),
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "select-option",
    ),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "body-boundary"),
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-table-boundary",
    ),
    ("select-list", WHATWG_SELECT_LIST_AUDIT, "select-in-table"),
    ("table", WHATWG_TABLE_AUDIT, "select-in-table"),
];
const LIST_DEFINITION_FORMATTING_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-list-container-boundary",
    ),
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "html-element-boundary",
    ),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "body-boundary"),
    ("list-item", WHATWG_LIST_ITEM_AUDIT, "list-with-paragraph"),
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-block-boundary",
    ),
];
const RUBY_FORMATTING_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-ruby-boundary",
    ),
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "html-element-boundary",
    ),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "body-boundary"),
    ("ruby", WHATWG_RUBY_AUDIT, "block-in-ruby"),
];
const HEADING_FORMATTING_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-foreign-boundary",
    ),
    ("foreign", WHATWG_FOREIGN_AUDIT, "mathml-boundary"),
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-heading-boundary",
    ),
];
const RECONSTRUCTION_FORMATTING_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "head-element-boundary",
    ),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "body-boundary"),
    ("template", WHATWG_TEMPLATE_AUDIT, "nested-template"),
    (
        "tree-insertion",
        WHATWG_TREE_INSERTION_AUDIT,
        "template-insertion",
    ),
];
const FORMATTING_CROSS_AXIS_CASES: &[FormattingCrossAxisCase] = &[
    FormattingCrossAxisCase {
        id: "tables01-dat-453",
        formatting_axis: "adoption-agency-formatting",
        data_snippet: "<div><table><svg><foreignObject><select><table><s>",
        suites: ADOPTION_FORMATTING_CROSS_AXIS_SUITES,
    },
    FormattingCrossAxisCase {
        id: "adoption01-dat-13",
        formatting_axis: "interactive-formatting-boundary",
        data_snippet: "<a><svg><tr><input></a>",
        suites: INTERACTIVE_FORMATTING_CROSS_AXIS_SUITES,
    },
    FormattingCrossAxisCase {
        id: "tests10-dat-17",
        formatting_axis: "paragraph-boundary",
        data_snippet: "<!DOCTYPE html><body><table><tr><td><select><svg><g>foo</g><g>bar</g><p>baz</table><p>quux",
        suites: PARAGRAPH_FORMATTING_CROSS_AXIS_SUITES,
    },
    FormattingCrossAxisCase {
        id: "tests3-dat-20",
        formatting_axis: "list-definition-boundary",
        data_snippet: "<!DOCTYPE html><html><head></head><body><ul><li><div><p><li></ul></body></html>",
        suites: LIST_DEFINITION_FORMATTING_CROSS_AXIS_SUITES,
    },
    FormattingCrossAxisCase {
        id: "webkit01-dat-29",
        formatting_axis: "ruby-implied-end-tags",
        data_snippet: "<html><body><ruby><div><rp>xx</rp></div></ruby></body></html>",
        suites: RUBY_FORMATTING_CROSS_AXIS_SUITES,
    },
    FormattingCrossAxisCase {
        id: "tests19-dat-1044",
        formatting_axis: "heading-boundary",
        data_snippet: "<!doctype html><p><math><mi><p><h1>",
        suites: HEADING_FORMATTING_CROSS_AXIS_SUITES,
    },
    FormattingCrossAxisCase {
        id: "template-dat-524",
        formatting_axis: "formatting-reconstruction",
        data_snippet: "<body><template><template><b><template></template></template>text</template>",
        suites: RECONSTRUCTION_FORMATTING_CROSS_AXIS_SUITES,
    },
];
const POST_PARSE_REPAIR_EVIDENCE: &[(&str, &str)] = &[
    ("adoption01-dat-16", "formatting-reconstruction"),
    ("scripted-adoption01-dat-1", "adoption-agency-formatting"),
    ("scripted-ark-dat-1", "adoption-agency-formatting"),
    ("tests26-dat-4", "interactive-formatting-boundary"),
    ("tricky01-dat-1", "adoption-agency-formatting"),
    ("tricky01-dat-3", "adoption-agency-formatting"),
    ("tricky01-dat-7", "interactive-formatting-boundary"),
    ("tricky01-dat-8", "interactive-formatting-boundary"),
    ("tricky01-dat-9", "interactive-formatting-boundary"),
    ("tests26-dat-1251", "interactive-formatting-boundary"),
];

#[derive(Debug, Deserialize)]
struct FormattingAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<FormattingAuditCase>,
}

#[derive(Debug, Deserialize)]
struct FormattingAuditCase {
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
fn whatwg_formatting_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-formatting-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 630);
    assert_axis_count(&suite, "adoption-agency-formatting", 110);
    assert_axis_count(&suite, "interactive-formatting-boundary", 145);
    assert_axis_count(&suite, "paragraph-boundary", 210);
    assert_axis_count(&suite, "list-definition-boundary", 55);
    assert_axis_count(&suite, "ruby-implied-end-tags", 40);
    assert_axis_count(&suite, "heading-boundary", 25);
    assert_axis_count(&suite, "formatting-reconstruction", 30);
}

#[test]
fn whatwg_formatting_audit_cases_match_parser_dom_dump() {
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
fn whatwg_formatting_audit_keeps_formatting_axis_cases_cross_axis() {
    let suite = load_suite();
    let formatting_cases = suite
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<HashMap<_, _>>();
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for evidence in FORMATTING_CROSS_AXIS_CASES {
        let formatting_case = formatting_cases
            .get(evidence.id)
            .unwrap_or_else(|| panic!("formatting audit should include `{}`", evidence.id));
        assert_eq!(
            formatting_case.axis, evidence.formatting_axis,
            "formatting row `{}` should stay on its focused audit axis",
            evidence.id
        );
        assert!(
            !formatting_case.reason.is_empty(),
            "formatting row `{}` should keep a fixture reason",
            evidence.id
        );

        for (suite_name, fixture, expected_axis) in evidence.suites {
            let audit_case = generic_audit_case(fixture, suite_name, evidence.id);
            assert_eq!(
                audit_case.source, formatting_case.source,
                "`{suite_name}` should point formatting row `{}` at the same html5lib row as the formatting audit",
                evidence.id
            );
            assert_eq!(
                audit_case.axis, *expected_axis,
                "`{suite_name}` should keep formatting row `{}` on its focused axis",
                evidence.id
            );
            assert!(
                !audit_case.reason.is_empty(),
                "`{suite_name}` should keep a reason for formatting row `{}`",
                evidence.id
            );
        }

        let source_case = smoke_cases
            .get(&formatting_case.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", evidence.id));
        assert!(
            source_case.data.contains(evidence.data_snippet),
            "formatting row `{}` should stay tied to its html5lib input",
            evidence.id
        );
        let actual = actual_dom_dump_for_tree_case(source_case).unwrap_or_else(|error| {
            panic!(
                "case `{}` ({}) parse failed: {error}",
                formatting_case.id, formatting_case.axis
            )
        });

        assert_eq!(
            actual, source_case.document,
            "cross-axis formatting evidence case `{}` ({}) failed for input {:?}",
            formatting_case.id, formatting_case.axis, source_case.data
        );
    }
}

#[test]
fn whatwg_formatting_audit_tracks_post_parse_repair_evidence() {
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

fn load_suite() -> FormattingAuditSuite {
    serde_json::from_str(WHATWG_FORMATTING_AUDIT)
        .expect("WHATWG formatting audit fixture should parse")
}

fn generic_audit_case(fixture: &str, suite_name: &str, case_id: &str) -> GenericAuditCase {
    let suite = serde_json::from_str::<GenericAuditSuite>(fixture)
        .unwrap_or_else(|error| panic!("`{suite_name}` audit fixture should parse: {error}"));
    suite
        .cases
        .into_iter()
        .find(|case| case.id == case_id)
        .unwrap_or_else(|| panic!("`{suite_name}` should audit formatting case `{case_id}`"))
}

fn assert_axis_count(suite: &FormattingAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
