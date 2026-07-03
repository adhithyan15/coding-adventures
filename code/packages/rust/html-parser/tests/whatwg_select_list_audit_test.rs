mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_DOCUMENT_SHELL_AUDIT: &str = include_str!("fixtures/whatwg-document-shell-audit.json");
const WHATWG_FOREIGN_AUDIT: &str = include_str!("fixtures/whatwg-foreign-audit.json");
const WHATWG_FORMATTING_AUDIT: &str = include_str!("fixtures/whatwg-formatting-audit.json");
const WHATWG_FORM_INTERACTIVE_AUDIT: &str =
    include_str!("fixtures/whatwg-form-interactive-audit.json");
const WHATWG_HEAD_BODY_AUDIT: &str = include_str!("fixtures/whatwg-head-body-audit.json");
const WHATWG_PARAGRAPH_AUDIT: &str = include_str!("fixtures/whatwg-paragraph-audit.json");
const WHATWG_SELECT_LIST_AUDIT: &str = include_str!("fixtures/whatwg-select-list-audit.json");
const WHATWG_TABLE_AUDIT: &str = include_str!("fixtures/whatwg-table-audit.json");
const WHATWG_TEMPLATE_AUDIT: &str = include_str!("fixtures/whatwg-template-audit.json");
const WHATWG_TREE_INSERTION_AUDIT: &str = include_str!("fixtures/whatwg-tree-insertion-audit.json");

struct SelectInTableCrossAxisCase {
    id: &'static str,
    data_snippet: &'static str,
    suites: &'static [(&'static str, &'static str, &'static str)],
}

const POST_PARSE_REPAIR_EVIDENCE: &[(&str, &str)] = &[
    ("tables01-dat-442", "select-in-table"),
    ("template-dat-475", "option-implied-end"),
    ("tests1-dat-600", "optgroup-boundary"),
    ("tests1-dat-675", "stray-select-end-tags"),
    ("tests-innerhtml-1-dat-75", "select-fragment-context"),
    ("tests2-dat-40", "select-shell"),
];
const TABLE_SELECT_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
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
const TEMPLATE_SELECT_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "head-element-boundary",
    ),
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "select-option",
    ),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "body-boundary"),
    ("select-list", WHATWG_SELECT_LIST_AUDIT, "select-in-table"),
    ("table", WHATWG_TABLE_AUDIT, "select-in-table"),
    ("template", WHATWG_TEMPLATE_AUDIT, "select-template"),
    (
        "tree-insertion",
        WHATWG_TREE_INSERTION_AUDIT,
        "template-insertion",
    ),
];
const FOREIGN_SELECT_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
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
    ("formatting", WHATWG_FORMATTING_AUDIT, "paragraph-boundary"),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "body-boundary"),
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-table-boundary",
    ),
    ("select-list", WHATWG_SELECT_LIST_AUDIT, "select-in-table"),
    ("table", WHATWG_TABLE_AUDIT, "select-in-table"),
];
const SELECT_IN_TABLE_CROSS_AXIS_CASES: &[SelectInTableCrossAxisCase] = &[
    SelectInTableCrossAxisCase {
        id: "tables01-dat-442",
        data_snippet: "<table><select><option>3</select></table>",
        suites: TABLE_SELECT_CROSS_AXIS_SUITES,
    },
    SelectInTableCrossAxisCase {
        id: "template-dat-556",
        data_snippet: "<body><table><tr><td><select><template>Foo</template><caption>A</table>",
        suites: TEMPLATE_SELECT_CROSS_AXIS_SUITES,
    },
    SelectInTableCrossAxisCase {
        id: "tests10-dat-694",
        data_snippet: "<table><tr><td><select><svg><g>foo</g><g>bar</g><p>baz</table>",
        suites: FOREIGN_SELECT_CROSS_AXIS_SUITES,
    },
];

#[derive(Debug, Deserialize)]
struct SelectListAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<SelectListAuditCase>,
}

#[derive(Debug, Deserialize)]
struct SelectListAuditCase {
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
fn whatwg_select_list_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-select-list-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 130);
    assert_axis_count(&suite, "select-shell", 50);
    assert_axis_count(&suite, "option-implied-end", 12);
    assert_axis_count(&suite, "optgroup-boundary", 10);
    assert_axis_count(&suite, "select-in-table", 30);
    assert_axis_count(&suite, "select-fragment-context", 5);
    assert_axis_count(&suite, "stray-select-end-tags", 4);
}

#[test]
fn whatwg_select_list_audit_cases_match_parser_dom_dump() {
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
fn whatwg_select_list_audit_keeps_select_in_table_cases_cross_axis() {
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for evidence in SELECT_IN_TABLE_CROSS_AXIS_CASES {
        let mut shared_source = None;

        for (suite_name, fixture, expected_axis) in evidence.suites {
            let audit_case = generic_audit_case(fixture, suite_name, evidence.id);
            assert_eq!(
                audit_case.axis, *expected_axis,
                "`{suite_name}` should keep select-in-table row `{}` on its focused axis",
                evidence.id
            );
            assert!(
                !audit_case.reason.is_empty(),
                "`{suite_name}` should keep a reason for select-in-table row `{}`",
                evidence.id
            );

            if let Some(source) = &shared_source {
                assert_eq!(
                    audit_case.source, *source,
                    "`{suite_name}` should point select-in-table row `{}` at the same html5lib row as the other audit axes",
                    evidence.id
                );
            } else {
                shared_source = Some(audit_case.source);
            }
        }

        let shared_source =
            shared_source.expect("select-in-table evidence should include at least one suite");
        let source_case = smoke_cases
            .get(&shared_source)
            .unwrap_or_else(|| panic!("case `{shared_source}` should exist in smoke fixture"));
        assert!(
            source_case.data.contains(evidence.data_snippet),
            "select-in-table evidence row `{}` should stay tied to its html5lib input",
            evidence.id
        );

        let actual = actual_dom_dump_for_tree_case(source_case)
            .unwrap_or_else(|error| panic!("case `{shared_source}` parse failed: {error}"));
        assert_eq!(
            actual, source_case.document,
            "cross-axis select-in-table evidence case `{shared_source}` failed for input {:?}",
            source_case.data
        );
    }
}

#[test]
fn whatwg_select_list_audit_tracks_post_parse_repair_evidence() {
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

fn load_suite() -> SelectListAuditSuite {
    serde_json::from_str(WHATWG_SELECT_LIST_AUDIT)
        .expect("WHATWG select/list audit fixture should parse")
}

fn generic_audit_case(fixture: &str, suite_name: &str, case_id: &str) -> GenericAuditCase {
    let suite = serde_json::from_str::<GenericAuditSuite>(fixture)
        .unwrap_or_else(|error| panic!("`{suite_name}` audit fixture should parse: {error}"));
    suite
        .cases
        .into_iter()
        .find(|case| case.id == case_id)
        .unwrap_or_else(|| panic!("`{suite_name}` should audit select-in-table case `{case_id}`"))
}

fn assert_axis_count(suite: &SelectListAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
