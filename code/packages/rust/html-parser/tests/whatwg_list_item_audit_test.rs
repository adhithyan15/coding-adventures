mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_BLOCK_BOUNDARY_AUDIT: &str = include_str!("fixtures/whatwg-block-boundary-audit.json");
const WHATWG_DOCUMENT_SHELL_AUDIT: &str = include_str!("fixtures/whatwg-document-shell-audit.json");
const WHATWG_FORM_INTERACTIVE_AUDIT: &str =
    include_str!("fixtures/whatwg-form-interactive-audit.json");
const WHATWG_FORMATTING_AUDIT: &str = include_str!("fixtures/whatwg-formatting-audit.json");
const WHATWG_FRAMESET_AUDIT: &str = include_str!("fixtures/whatwg-frameset-audit.json");
const WHATWG_HEAD_BODY_AUDIT: &str = include_str!("fixtures/whatwg-head-body-audit.json");
const WHATWG_LEGACY_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-legacy-element-audit.json");
const WHATWG_LIST_ITEM_AUDIT: &str = include_str!("fixtures/whatwg-list-item-audit.json");
const WHATWG_PARAGRAPH_AUDIT: &str = include_str!("fixtures/whatwg-paragraph-audit.json");
const WHATWG_SELECT_LIST_AUDIT: &str = include_str!("fixtures/whatwg-select-list-audit.json");
const WHATWG_TABLE_AUDIT: &str = include_str!("fixtures/whatwg-table-audit.json");
const WHATWG_VOID_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-void-element-audit.json");

struct ListItemCrossAxisCase {
    id: &'static str,
    list_axis: &'static str,
    data_snippet: &'static str,
    suites: &'static [(&'static str, &'static str, &'static str)],
}

const LIST_PARAGRAPH_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
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
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "list-definition-boundary",
    ),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "body-boundary"),
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-block-boundary",
    ),
];
const NESTED_LIST_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-list-container-boundary",
    ),
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "body-frameset-boundary",
    ),
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "list-definition-boundary",
    ),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "body-boundary"),
];
const LIST_FORMATTING_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-formatting-boundary",
    ),
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "html-element-boundary",
    ),
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "list-definition-boundary",
    ),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "html-boundary"),
    (
        "legacy-element",
        WHATWG_LEGACY_ELEMENT_AUDIT,
        "tricky-parser-recovery",
    ),
];
const LIST_TABLE_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-table-boundary",
    ),
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "stray-interactive-end-tags",
    ),
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "list-definition-boundary",
    ),
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-table-boundary",
    ),
    (
        "select-list",
        WHATWG_SELECT_LIST_AUDIT,
        "stray-select-end-tags",
    ),
    ("table", WHATWG_TABLE_AUDIT, "caption-colgroup"),
    ("void-element", WHATWG_VOID_ELEMENT_AUDIT, "void-in-table"),
];
const LIST_FRAMESET_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "body-frameset-boundary",
    ),
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "list-definition-boundary",
    ),
    ("frameset", WHATWG_FRAMESET_AUDIT, "body-compatibility"),
    (
        "head-body",
        WHATWG_HEAD_BODY_AUDIT,
        "body-frameset-transition",
    ),
];
const LIST_ITEM_CROSS_AXIS_CASES: &[ListItemCrossAxisCase] = &[
    ListItemCrossAxisCase {
        id: "tests3-dat-20",
        list_axis: "list-with-paragraph",
        data_snippet: "<ul><li><div><p><li>",
        suites: LIST_PARAGRAPH_CROSS_AXIS_SUITES,
    },
    ListItemCrossAxisCase {
        id: "tests1-dat-34",
        list_axis: "nested-list-boundary",
        data_snippet: "<li>hello<li>world<ul>how<li>do</ul>you",
        suites: NESTED_LIST_CROSS_AXIS_SUITES,
    },
    ListItemCrossAxisCase {
        id: "tricky01-dat-4",
        list_axis: "list-with-formatting",
        data_snippet: "<dt><b>Boo\n<dd>Goo?",
        suites: LIST_FORMATTING_CROSS_AXIS_SUITES,
    },
    ListItemCrossAxisCase {
        id: "tests1-dat-111",
        list_axis: "list-in-table",
        data_snippet: "<table><tr></strong></b></em></i>",
        suites: LIST_TABLE_CROSS_AXIS_SUITES,
    },
    ListItemCrossAxisCase {
        id: "tests19-dat-1064",
        list_axis: "li-implied-end-tag",
        data_snippet: "<!doctype html><li><frameset>",
        suites: LIST_FRAMESET_CROSS_AXIS_SUITES,
    },
    ListItemCrossAxisCase {
        id: "tests19-dat-1065",
        list_axis: "dt-dd-implied-end-tag",
        data_snippet: "<!doctype html><dd><frameset>",
        suites: LIST_FRAMESET_CROSS_AXIS_SUITES,
    },
];
const POST_PARSE_REPAIR_EVIDENCE: &[(&str, &str)] = &[
    ("tests19-dat-1037", "li-implied-end-tag"),
    ("tests19-dat-1043", "dt-dd-implied-end-tag"),
    ("tests1-dat-668", "list-with-formatting"),
    ("tests1-dat-676", "list-in-table"),
];

#[derive(Debug, Deserialize)]
struct ListItemAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<ListItemAuditCase>,
}

#[derive(Debug, Deserialize)]
struct ListItemAuditCase {
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
fn whatwg_list_item_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-list-item-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 55);
    assert_axis_count(&suite, "list-with-paragraph", 20);
    assert_axis_count(&suite, "nested-list-boundary", 10);
    assert_axis_count(&suite, "dt-dd-implied-end-tag", 10);
    assert_axis_count(&suite, "list-with-formatting", 6);
    assert_axis_count(&suite, "li-implied-end-tag", 6);
    assert_axis_count(&suite, "list-in-table", 1);
}

#[test]
fn whatwg_list_item_audit_cases_match_parser_dom_dump() {
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
fn whatwg_list_item_audit_keeps_list_axis_cases_cross_axis() {
    let suite = load_suite();
    let list_cases = suite
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<HashMap<_, _>>();
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for evidence in LIST_ITEM_CROSS_AXIS_CASES {
        let list_case = list_cases
            .get(evidence.id)
            .unwrap_or_else(|| panic!("list-item audit should include `{}`", evidence.id));
        assert_eq!(
            list_case.axis, evidence.list_axis,
            "list-item row `{}` should stay on its focused audit axis",
            evidence.id
        );
        assert!(
            !list_case.reason.is_empty(),
            "list-item row `{}` should keep a fixture reason",
            evidence.id
        );

        for (suite_name, fixture, expected_axis) in evidence.suites {
            let audit_case = generic_audit_case(fixture, suite_name, evidence.id);
            assert_eq!(
                audit_case.source, list_case.source,
                "`{suite_name}` should point list-item row `{}` at the same html5lib row as the list-item audit",
                evidence.id
            );
            assert_eq!(
                audit_case.axis, *expected_axis,
                "`{suite_name}` should keep list-item row `{}` on its focused axis",
                evidence.id
            );
            assert!(
                !audit_case.reason.is_empty(),
                "`{suite_name}` should keep a reason for list-item row `{}`",
                evidence.id
            );
        }

        let source_case = smoke_cases
            .get(&list_case.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", evidence.id));
        assert!(
            source_case.data.contains(evidence.data_snippet),
            "list-item row `{}` should stay tied to its html5lib input",
            evidence.id
        );
        let actual = actual_dom_dump_for_tree_case(source_case).unwrap_or_else(|error| {
            panic!(
                "case `{}` ({}) parse failed: {error}",
                list_case.id, list_case.axis
            )
        });

        assert_eq!(
            actual, source_case.document,
            "cross-axis list-item evidence case `{}` ({}) failed for input {:?}",
            list_case.id, list_case.axis, source_case.data
        );
    }
}

#[test]
fn whatwg_list_item_audit_tracks_post_parse_repair_evidence() {
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

fn load_suite() -> ListItemAuditSuite {
    serde_json::from_str(WHATWG_LIST_ITEM_AUDIT)
        .expect("WHATWG list-item audit fixture should parse")
}

fn generic_audit_case(fixture: &str, suite_name: &str, case_id: &str) -> GenericAuditCase {
    let suite = serde_json::from_str::<GenericAuditSuite>(fixture)
        .unwrap_or_else(|error| panic!("`{suite_name}` audit fixture should parse: {error}"));
    suite
        .cases
        .into_iter()
        .find(|case| case.id == case_id)
        .unwrap_or_else(|| panic!("`{suite_name}` should audit list-item case `{case_id}`"))
}

fn assert_axis_count(suite: &ListItemAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
