mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_BLOCK_BOUNDARY_AUDIT: &str = include_str!("fixtures/whatwg-block-boundary-audit.json");
const WHATWG_DOCUMENT_SHELL_AUDIT: &str = include_str!("fixtures/whatwg-document-shell-audit.json");
const WHATWG_FORMATTING_AUDIT: &str = include_str!("fixtures/whatwg-formatting-audit.json");
const WHATWG_FORM_INTERACTIVE_AUDIT: &str =
    include_str!("fixtures/whatwg-form-interactive-audit.json");
const WHATWG_FRAMESET_AUDIT: &str = include_str!("fixtures/whatwg-frameset-audit.json");
const WHATWG_HEAD_BODY_AUDIT: &str = include_str!("fixtures/whatwg-head-body-audit.json");
const WHATWG_LIST_ITEM_AUDIT: &str = include_str!("fixtures/whatwg-list-item-audit.json");
const WHATWG_NOSCRIPT_AUDIT: &str = include_str!("fixtures/whatwg-noscript-audit.json");
const WHATWG_PARAGRAPH_AUDIT: &str = include_str!("fixtures/whatwg-paragraph-audit.json");
const WHATWG_SELECT_LIST_AUDIT: &str = include_str!("fixtures/whatwg-select-list-audit.json");
const WHATWG_TABLE_AUDIT: &str = include_str!("fixtures/whatwg-table-audit.json");
const WHATWG_TEXT_CONTROL_AUDIT: &str = include_str!("fixtures/whatwg-text-control-audit.json");
const WHATWG_VOID_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-void-element-audit.json");

struct StrayNoscriptEndTagCase {
    id: &'static str,
    source: &'static str,
    data_snippet: &'static str,
    suites: &'static [(&'static str, &'static str, &'static str)],
}

const POST_PARSE_REPAIR_EVIDENCE: &[(&str, &str)] = &[
    ("noscript01-dat-327", "head-noscript-disabled"),
    ("tests16-dat-851", "comment-boundary"),
    ("tests16-dat-855", "textmode-descendant"),
    ("tests1-dat-675", "stray-noscript-end-tag"),
    ("webkit02-dat-2", "paragraph-noscript"),
];
const STRAY_NOSCRIPT_BODY_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-table-boundary",
    ),
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "html-element-boundary",
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
    ("frameset", WHATWG_FRAMESET_AUDIT, "noframes-content"),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "head-text-mode"),
    ("list-item", WHATWG_LIST_ITEM_AUDIT, "list-with-formatting"),
    ("noscript", WHATWG_NOSCRIPT_AUDIT, "stray-noscript-end-tag"),
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-formatting-boundary",
    ),
    (
        "select-list",
        WHATWG_SELECT_LIST_AUDIT,
        "stray-select-end-tags",
    ),
    ("table", WHATWG_TABLE_AUDIT, "caption-colgroup"),
    (
        "text-control",
        WHATWG_TEXT_CONTROL_AUDIT,
        "stray-text-control-end-tags",
    ),
    (
        "void-element",
        WHATWG_VOID_ELEMENT_AUDIT,
        "stray-void-end-tags",
    ),
];
const STRAY_NOSCRIPT_TABLE_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-table-boundary",
    ),
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "html-element-boundary",
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
    ("frameset", WHATWG_FRAMESET_AUDIT, "noframes-content"),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "head-text-mode"),
    ("list-item", WHATWG_LIST_ITEM_AUDIT, "list-in-table"),
    ("noscript", WHATWG_NOSCRIPT_AUDIT, "stray-noscript-end-tag"),
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
    (
        "text-control",
        WHATWG_TEXT_CONTROL_AUDIT,
        "stray-text-control-end-tags",
    ),
    ("void-element", WHATWG_VOID_ELEMENT_AUDIT, "void-in-table"),
];
const STRAY_NOSCRIPT_END_TAG_CROSS_AXIS_CASES: &[StrayNoscriptEndTagCase] = &[
    StrayNoscriptEndTagCase {
        id: "tests1-dat-110",
        source: "tests1.dat:110",
        data_snippet: "</strong></b></em></i>",
        suites: STRAY_NOSCRIPT_BODY_CROSS_AXIS_SUITES,
    },
    StrayNoscriptEndTagCase {
        id: "tests1-dat-111",
        source: "tests1.dat:111",
        data_snippet: "<table><tr></strong></b></em>",
        suites: STRAY_NOSCRIPT_TABLE_CROSS_AXIS_SUITES,
    },
    StrayNoscriptEndTagCase {
        id: "tests1-dat-675",
        source: "tests1.dat:675",
        data_snippet: "</strong></b></em></i>",
        suites: STRAY_NOSCRIPT_BODY_CROSS_AXIS_SUITES,
    },
    StrayNoscriptEndTagCase {
        id: "tests1-dat-676",
        source: "tests1.dat:676",
        data_snippet: "<table><tr></strong></b></em>",
        suites: STRAY_NOSCRIPT_TABLE_CROSS_AXIS_SUITES,
    },
];

#[derive(Debug, Deserialize)]
struct NoscriptAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<NoscriptAuditCase>,
}

#[derive(Debug, Deserialize)]
struct NoscriptAuditCase {
    id: String,
    source: String,
    axis: String,
    scripting: String,
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
fn whatwg_noscript_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-noscript-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 45);
    assert_axis_count(&suite, "head-noscript-disabled", 15);
    assert_axis_count(&suite, "comment-boundary", 18);
    assert_axis_count(&suite, "textmode-descendant", 10);
    assert_axis_count(&suite, "stray-noscript-end-tag", 4);
    assert_axis_count(&suite, "paragraph-noscript", 2);
    assert_axis_count(&suite, "processing-instruction", 1);
}

#[test]
fn whatwg_noscript_audit_cases_match_parser_dom_dump() {
    let suite = load_suite();
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for case in &suite.cases {
        assert!(
            !case.id.is_empty()
                && !case.axis.is_empty()
                && !case.reason.is_empty()
                && !case.scripting.is_empty(),
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
fn whatwg_noscript_audit_keeps_stray_end_tag_cases_cross_axis() {
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

    for evidence in STRAY_NOSCRIPT_END_TAG_CROSS_AXIS_CASES {
        let noscript_case = audit_cases.get(evidence.id).unwrap_or_else(|| {
            panic!(
                "stray noscript end-tag evidence case `{}` should be audited",
                evidence.id
            )
        });
        assert_eq!(
            noscript_case.source, evidence.source,
            "stray noscript end-tag case `{}` should stay tied to its smoke fixture row",
            evidence.id
        );
        assert_eq!(
            noscript_case.axis, "stray-noscript-end-tag",
            "stray noscript end-tag case `{}` should stay on its focused audit axis",
            evidence.id
        );
        assert!(
            !noscript_case.scripting.is_empty() && !noscript_case.reason.is_empty(),
            "stray noscript end-tag case `{}` should keep noscript audit metadata",
            evidence.id
        );

        let source_case = smoke_cases
            .get(evidence.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", evidence.source));
        assert!(
            source_case.data.contains(evidence.data_snippet),
            "stray noscript end-tag case `{}` should stay tied to its html5lib input",
            evidence.id
        );
        let actual = actual_dom_dump_for_tree_case(source_case).unwrap_or_else(|error| {
            panic!(
                "case `{}` ({}) parse failed: {error}",
                noscript_case.id, noscript_case.axis
            )
        });

        assert_eq!(
            actual, source_case.document,
            "stray noscript end-tag evidence case `{}` ({}) failed for input {:?}",
            noscript_case.id, noscript_case.axis, source_case.data
        );

        for (suite_name, suite_json, expected_axis) in evidence.suites {
            let generic_suite: GenericAuditSuite = serde_json::from_str(suite_json)
                .unwrap_or_else(|error| panic!("{suite_name} audit fixture should parse: {error}"));
            let generic_case = generic_suite
                .cases
                .iter()
                .find(|case| case.id == evidence.id)
                .unwrap_or_else(|| {
                    panic!(
                        "{suite_name} audit should include stray noscript end-tag case `{}`",
                        evidence.id
                    )
                });

            assert_eq!(
                generic_case.axis, *expected_axis,
                "{suite_name} audit should keep `{}` on its focused axis",
                evidence.id
            );
            assert_eq!(
                generic_case.source, evidence.source,
                "{suite_name} audit case `{}` should point at the same WHATWG source row",
                evidence.id
            );
            assert!(
                !generic_case.reason.is_empty(),
                "{suite_name} audit case `{}` should keep a fixture reason",
                evidence.id
            );
        }
    }
}

#[test]
fn whatwg_noscript_audit_tracks_post_parse_repair_evidence() {
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
        assert!(
            !audit_case.scripting.is_empty(),
            "post-parse repair evidence case `{case_id}` should keep its scripting mode"
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

fn load_suite() -> NoscriptAuditSuite {
    serde_json::from_str(WHATWG_NOSCRIPT_AUDIT).expect("WHATWG noscript audit fixture should parse")
}

fn assert_axis_count(suite: &NoscriptAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
