mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_DOCUMENT_SHELL_AUDIT: &str = include_str!("fixtures/whatwg-document-shell-audit.json");
const WHATWG_FOREIGN_AUDIT: &str = include_str!("fixtures/whatwg-foreign-audit.json");
const WHATWG_FORM_INTERACTIVE_AUDIT: &str =
    include_str!("fixtures/whatwg-form-interactive-audit.json");
const WHATWG_FORMATTING_AUDIT: &str = include_str!("fixtures/whatwg-formatting-audit.json");
const WHATWG_FRAMESET_AUDIT: &str = include_str!("fixtures/whatwg-frameset-audit.json");
const WHATWG_HEAD_BODY_AUDIT: &str = include_str!("fixtures/whatwg-head-body-audit.json");
const WHATWG_LEGACY_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-legacy-element-audit.json");
const WHATWG_NOSCRIPT_AUDIT: &str = include_str!("fixtures/whatwg-noscript-audit.json");
const WHATWG_PARAGRAPH_AUDIT: &str = include_str!("fixtures/whatwg-paragraph-audit.json");
const WHATWG_SELECT_LIST_AUDIT: &str = include_str!("fixtures/whatwg-select-list-audit.json");
const WHATWG_TABLE_AUDIT: &str = include_str!("fixtures/whatwg-table-audit.json");
const WHATWG_TEMPLATE_AUDIT: &str = include_str!("fixtures/whatwg-template-audit.json");
const WHATWG_TEXT_CONTROL_AUDIT: &str = include_str!("fixtures/whatwg-text-control-audit.json");
const WHATWG_TREE_INSERTION_AUDIT: &str = include_str!("fixtures/whatwg-tree-insertion-audit.json");
const WHATWG_VOID_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-void-element-audit.json");

struct HeadBodyCrossAxisCase {
    id: &'static str,
    head_body_axis: &'static str,
    data_snippet: &'static str,
    suites: &'static [(&'static str, &'static str, &'static str)],
}

const HEAD_TEXT_MODE_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "head-element-boundary",
    ),
    ("table", WHATWG_TABLE_AUDIT, "cell-boundary"),
    ("template", WHATWG_TEMPLATE_AUDIT, "rawtext-template"),
    ("text-control", WHATWG_TEXT_CONTROL_AUDIT, "script-rawtext"),
    (
        "tree-insertion",
        WHATWG_TREE_INSERTION_AUDIT,
        "template-insertion",
    ),
];
const HEAD_METADATA_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "head-element-boundary",
    ),
    ("noscript", WHATWG_NOSCRIPT_AUDIT, "head-noscript-disabled"),
    (
        "text-control",
        WHATWG_TEXT_CONTROL_AUDIT,
        "noscript-scripting",
    ),
    (
        "void-element",
        WHATWG_VOID_ELEMENT_AUDIT,
        "metadata-void-elements",
    ),
];
const BODY_FRAMESET_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "body-frameset-boundary",
    ),
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "form-control",
    ),
    ("frameset", WHATWG_FRAMESET_AUDIT, "body-compatibility"),
    (
        "void-element",
        WHATWG_VOID_ELEMENT_AUDIT,
        "body-void-elements",
    ),
];
const BODY_BOUNDARY_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
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
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-table-boundary",
    ),
    ("select-list", WHATWG_SELECT_LIST_AUDIT, "select-in-table"),
    ("table", WHATWG_TABLE_AUDIT, "select-in-table"),
];
const HTML_BOUNDARY_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "html-element-boundary",
    ),
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "adoption-agency-formatting",
    ),
    (
        "legacy-element",
        WHATWG_LEGACY_ELEMENT_AUDIT,
        "tricky-parser-recovery",
    ),
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-formatting-boundary",
    ),
];
const HEAD_BOUNDARY_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "head-element-boundary",
    ),
    ("formatting", WHATWG_FORMATTING_AUDIT, "paragraph-boundary"),
    ("noscript", WHATWG_NOSCRIPT_AUDIT, "head-noscript-disabled"),
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-special-end-tag",
    ),
    (
        "text-control",
        WHATWG_TEXT_CONTROL_AUDIT,
        "noscript-scripting",
    ),
];
const HEAD_BODY_CROSS_AXIS_CASES: &[HeadBodyCrossAxisCase] = &[
    HeadBodyCrossAxisCase {
        id: "template-dat-498",
        head_body_axis: "head-text-mode",
        data_snippet: "<body><template><script>var i = 1;</script><td></td></template>",
        suites: HEAD_TEXT_MODE_CROSS_AXIS_SUITES,
    },
    HeadBodyCrossAxisCase {
        id: "noscript01-dat-332",
        head_body_axis: "head-metadata-boundary",
        data_snippet: "<head><noscript><basefont><!--foo--></noscript>",
        suites: HEAD_METADATA_CROSS_AXIS_SUITES,
    },
    HeadBodyCrossAxisCase {
        id: "webkit01-dat-51",
        head_body_axis: "body-frameset-transition",
        data_snippet: "<!doctype html><input type=\"hidden\"><frameset>",
        suites: BODY_FRAMESET_CROSS_AXIS_SUITES,
    },
    HeadBodyCrossAxisCase {
        id: "tests10-dat-17",
        head_body_axis: "body-boundary",
        data_snippet: "<select><svg><g>foo</g><g>bar</g><p>baz</table><p>quux",
        suites: BODY_BOUNDARY_CROSS_AXIS_SUITES,
    },
    HeadBodyCrossAxisCase {
        id: "tricky01-dat-2",
        head_body_axis: "html-boundary",
        data_snippet: "<font color=red><i>Italic and Red<p>",
        suites: HTML_BOUNDARY_CROSS_AXIS_SUITES,
    },
    HeadBodyCrossAxisCase {
        id: "noscript01-dat-341",
        head_body_axis: "head-boundary",
        data_snippet: "<head><noscript></p><!--foo--></noscript>",
        suites: HEAD_BOUNDARY_CROSS_AXIS_SUITES,
    },
];
const POST_PARSE_REPAIR_EVIDENCE: &[(&str, &str)] = &[
    ("scripted-adoption01-dat-1", "head-text-mode"),
    ("scripted-webkit01-dat-1", "head-text-mode"),
    ("scripted-webkit01-dat-2", "head-text-mode"),
    ("tests26-dat-4", "body-boundary"),
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
fn whatwg_head_body_audit_keeps_shell_boundary_cases_cross_axis() {
    let suite = load_suite();
    let head_body_cases = suite
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<HashMap<_, _>>();
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for evidence in HEAD_BODY_CROSS_AXIS_CASES {
        let head_body_case = head_body_cases
            .get(evidence.id)
            .unwrap_or_else(|| panic!("head/body audit should include `{}`", evidence.id));
        assert_eq!(
            head_body_case.axis, evidence.head_body_axis,
            "head/body row `{}` should stay on its focused audit axis",
            evidence.id
        );
        assert!(
            !head_body_case.reason.is_empty(),
            "head/body row `{}` should keep a fixture reason",
            evidence.id
        );

        for (suite_name, fixture, expected_axis) in evidence.suites {
            let audit_case = generic_audit_case(fixture, suite_name, evidence.id);
            assert_eq!(
                audit_case.source, head_body_case.source,
                "`{suite_name}` should point head/body row `{}` at the same html5lib row as the head/body audit",
                evidence.id
            );
            assert_eq!(
                audit_case.axis, *expected_axis,
                "`{suite_name}` should keep head/body row `{}` on its focused axis",
                evidence.id
            );
            assert!(
                !audit_case.reason.is_empty(),
                "`{suite_name}` should keep a reason for head/body row `{}`",
                evidence.id
            );
        }

        let source_case = smoke_cases
            .get(&head_body_case.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", evidence.id));
        assert!(
            source_case.data.contains(evidence.data_snippet),
            "head/body row `{}` should stay tied to its html5lib input",
            evidence.id
        );
        let actual = actual_dom_dump_for_tree_case(source_case).unwrap_or_else(|error| {
            panic!(
                "case `{}` ({}) parse failed: {error}",
                head_body_case.id, head_body_case.axis
            )
        });

        assert_eq!(
            actual, source_case.document,
            "cross-axis head/body evidence case `{}` ({}) failed for input {:?}",
            head_body_case.id, head_body_case.axis, source_case.data
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

fn generic_audit_case(fixture: &str, suite_name: &str, case_id: &str) -> GenericAuditCase {
    let suite = serde_json::from_str::<GenericAuditSuite>(fixture)
        .unwrap_or_else(|error| panic!("`{suite_name}` audit fixture should parse: {error}"));
    suite
        .cases
        .into_iter()
        .find(|case| case.id == case_id)
        .unwrap_or_else(|| panic!("`{suite_name}` should audit head/body case `{case_id}`"))
}

fn assert_axis_count(suite: &HeadBodyAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
