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
const WHATWG_FRAMESET_AUDIT: &str = include_str!("fixtures/whatwg-frameset-audit.json");
const WHATWG_HEAD_BODY_AUDIT: &str = include_str!("fixtures/whatwg-head-body-audit.json");
const WHATWG_LEGACY_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-legacy-element-audit.json");
const WHATWG_LIST_ITEM_AUDIT: &str = include_str!("fixtures/whatwg-list-item-audit.json");
const WHATWG_NOSCRIPT_AUDIT: &str = include_str!("fixtures/whatwg-noscript-audit.json");
const WHATWG_PARAGRAPH_AUDIT: &str = include_str!("fixtures/whatwg-paragraph-audit.json");
const WHATWG_TABLE_AUDIT: &str = include_str!("fixtures/whatwg-table-audit.json");
const WHATWG_TEXT_CONTROL_AUDIT: &str = include_str!("fixtures/whatwg-text-control-audit.json");
const WHATWG_TREE_INSERTION_AUDIT: &str = include_str!("fixtures/whatwg-tree-insertion-audit.json");
struct ParagraphCrossAxisCase {
    id: &'static str,
    paragraph_axis: &'static str,
    data_snippet: &'static str,
    fragment_context: Option<&'static str>,
    suites: &'static [(&'static str, &'static str, &'static str)],
}

const PARAGRAPH_FORMATTING_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
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
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "body-boundary"),
    (
        "legacy-element",
        WHATWG_LEGACY_ELEMENT_AUDIT,
        "tricky-parser-recovery",
    ),
];
const PARAGRAPH_BLOCK_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-list-container-boundary",
    ),
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "list-definition-boundary",
    ),
    ("list-item", WHATWG_LIST_ITEM_AUDIT, "list-with-paragraph"),
];
const PARAGRAPH_BASIC_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "html-element-boundary",
    ),
    ("formatting", WHATWG_FORMATTING_AUDIT, "paragraph-boundary"),
    ("frameset", WHATWG_FRAMESET_AUDIT, "body-compatibility"),
    (
        "head-body",
        WHATWG_HEAD_BODY_AUDIT,
        "body-frameset-transition",
    ),
];
const PARAGRAPH_TABLE_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
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
    ("table", WHATWG_TABLE_AUDIT, "foster-parenting"),
    (
        "tree-insertion",
        WHATWG_TREE_INSERTION_AUDIT,
        "adoption-agency",
    ),
];
const PARAGRAPH_TEXT_MODE_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "textarea-rawtext",
    ),
    ("formatting", WHATWG_FORMATTING_AUDIT, "paragraph-boundary"),
    ("text-control", WHATWG_TEXT_CONTROL_AUDIT, "rcdata-controls"),
];
const PARAGRAPH_SPECIAL_END_TAG_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "head-element-boundary",
    ),
    ("formatting", WHATWG_FORMATTING_AUDIT, "paragraph-boundary"),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "head-boundary"),
    ("noscript", WHATWG_NOSCRIPT_AUDIT, "head-noscript-disabled"),
    (
        "text-control",
        WHATWG_TEXT_CONTROL_AUDIT,
        "noscript-scripting",
    ),
];
const PARAGRAPH_HEADING_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-foreign-boundary",
    ),
    ("foreign", WHATWG_FOREIGN_AUDIT, "mathml-boundary"),
    ("formatting", WHATWG_FORMATTING_AUDIT, "heading-boundary"),
];
const PARAGRAPH_FRAGMENT_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
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
const PARAGRAPH_FORM_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-form-boundary",
    ),
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "button-boundary",
    ),
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "interactive-formatting-boundary",
    ),
];
const PARAGRAPH_CROSS_AXIS_CASES: &[ParagraphCrossAxisCase] = &[
    ParagraphCrossAxisCase {
        id: "tricky01-dat-3",
        paragraph_axis: "paragraph-formatting-boundary",
        data_snippet: "<p><font size=\"7\">First paragraph.</p>",
        fragment_context: None,
        suites: PARAGRAPH_FORMATTING_CROSS_AXIS_SUITES,
    },
    ParagraphCrossAxisCase {
        id: "blocks-dat-38",
        paragraph_axis: "paragraph-block-boundary",
        data_snippet: "<!doctype html><p>foo<dl>bar<p>baz",
        fragment_context: None,
        suites: PARAGRAPH_BLOCK_CROSS_AXIS_SUITES,
    },
    ParagraphCrossAxisCase {
        id: "tests19-dat-42",
        paragraph_axis: "paragraph-basic-boundary",
        data_snippet: "<!doctype html><html><frameset></frameset></html><p>",
        fragment_context: None,
        suites: PARAGRAPH_BASIC_CROSS_AXIS_SUITES,
    },
    ParagraphCrossAxisCase {
        id: "adoption01-dat-6",
        paragraph_axis: "paragraph-table-boundary",
        data_snippet: "<table><a>1<p>2</a>3</p>",
        fragment_context: None,
        suites: PARAGRAPH_TABLE_CROSS_AXIS_SUITES,
    },
    ParagraphCrossAxisCase {
        id: "tests1-dat-654",
        paragraph_axis: "paragraph-text-mode-boundary",
        data_snippet: "<textarea><p></textarea>",
        fragment_context: None,
        suites: PARAGRAPH_TEXT_MODE_CROSS_AXIS_SUITES,
    },
    ParagraphCrossAxisCase {
        id: "noscript01-dat-341",
        paragraph_axis: "paragraph-special-end-tag",
        data_snippet: "<head><noscript></p><!--foo--></noscript>",
        fragment_context: None,
        suites: PARAGRAPH_SPECIAL_END_TAG_CROSS_AXIS_SUITES,
    },
    ParagraphCrossAxisCase {
        id: "tests19-dat-31",
        paragraph_axis: "paragraph-heading-boundary",
        data_snippet: "<!doctype html><p><math><mi><p><h1>",
        fragment_context: None,
        suites: PARAGRAPH_HEADING_CROSS_AXIS_SUITES,
    },
    ParagraphCrossAxisCase {
        id: "foreign-fragment-dat-58",
        paragraph_axis: "paragraph-fragment-context",
        data_snippet: "<svg><p>",
        fragment_context: Some("div"),
        suites: PARAGRAPH_FRAGMENT_CROSS_AXIS_SUITES,
    },
    ParagraphCrossAxisCase {
        id: "tests20-dat-2",
        paragraph_axis: "paragraph-form-boundary",
        data_snippet: "<!doctype html><p><button><address>",
        fragment_context: None,
        suites: PARAGRAPH_FORM_CROSS_AXIS_SUITES,
    },
];
const POST_PARSE_REPAIR_EVIDENCE: &[(&str, &str)] = &[
    ("scripted-adoption01-dat-1", "paragraph-text-mode-boundary"),
    ("scripted-ark-dat-1", "paragraph-text-mode-boundary"),
    ("tricky01-dat-3", "paragraph-formatting-boundary"),
    ("tricky01-dat-7", "paragraph-table-boundary"),
    ("tricky01-dat-8", "paragraph-table-boundary"),
];

#[derive(Debug, Deserialize)]
struct ParagraphAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<ParagraphAuditCase>,
}

#[derive(Debug, Deserialize)]
struct ParagraphAuditCase {
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
fn whatwg_paragraph_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-paragraph-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 380);
    assert_axis_count(&suite, "paragraph-form-boundary", 70);
    assert_axis_count(&suite, "paragraph-formatting-boundary", 70);
    assert_axis_count(&suite, "paragraph-block-boundary", 55);
    assert_axis_count(&suite, "paragraph-basic-boundary", 50);
    assert_axis_count(&suite, "paragraph-table-boundary", 45);
    assert_axis_count(&suite, "paragraph-text-mode-boundary", 30);
    assert_axis_count(&suite, "paragraph-special-end-tag", 25);
    assert_axis_count(&suite, "paragraph-heading-boundary", 10);
    assert_axis_count(&suite, "paragraph-fragment-context", 6);
}

#[test]
fn whatwg_paragraph_audit_cases_match_parser_dom_dump() {
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
fn whatwg_paragraph_audit_keeps_paragraph_axis_cases_cross_axis() {
    let suite = load_suite();
    let paragraph_cases = suite
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<HashMap<_, _>>();
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for evidence in PARAGRAPH_CROSS_AXIS_CASES {
        let paragraph_case = paragraph_cases
            .get(evidence.id)
            .unwrap_or_else(|| panic!("paragraph audit should include `{}`", evidence.id));
        assert_eq!(
            paragraph_case.axis, evidence.paragraph_axis,
            "paragraph row `{}` should stay on its focused audit axis",
            evidence.id
        );
        assert!(
            !paragraph_case.reason.is_empty(),
            "paragraph row `{}` should keep a fixture reason",
            evidence.id
        );

        for (suite_name, fixture, expected_axis) in evidence.suites {
            let audit_case = generic_audit_case(fixture, suite_name, evidence.id);
            assert_eq!(
                audit_case.source, paragraph_case.source,
                "`{suite_name}` should point paragraph row `{}` at the same html5lib row as the paragraph audit",
                evidence.id
            );
            assert_eq!(
                audit_case.axis, *expected_axis,
                "`{suite_name}` should keep paragraph row `{}` on its focused axis",
                evidence.id
            );
            assert!(
                !audit_case.reason.is_empty(),
                "`{suite_name}` should keep a reason for paragraph row `{}`",
                evidence.id
            );
        }

        let source_case = smoke_cases
            .get(&paragraph_case.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", evidence.id));
        assert!(
            source_case.data.contains(evidence.data_snippet),
            "paragraph row `{}` should stay tied to its html5lib input",
            evidence.id
        );
        if let Some(fragment_context) = evidence.fragment_context {
            assert_eq!(
                source_case.fragment_context.as_deref(),
                Some(fragment_context),
                "paragraph row `{}` should keep its html5lib fragment context",
                evidence.id
            );
        }

        let actual = actual_dom_dump_for_tree_case(source_case).unwrap_or_else(|error| {
            panic!(
                "case `{}` ({}) parse failed: {error}",
                paragraph_case.id, paragraph_case.axis
            )
        });

        assert_eq!(
            actual, source_case.document,
            "cross-axis paragraph evidence case `{}` ({}) failed for input {:?}",
            paragraph_case.id, paragraph_case.axis, source_case.data
        );
    }
}

#[test]
fn whatwg_paragraph_audit_tracks_post_parse_repair_evidence() {
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

fn load_suite() -> ParagraphAuditSuite {
    serde_json::from_str(WHATWG_PARAGRAPH_AUDIT)
        .expect("WHATWG paragraph audit fixture should parse")
}

fn assert_axis_count(suite: &ParagraphAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}

fn generic_audit_case(raw_fixture: &str, suite_name: &str, case_id: &str) -> GenericAuditCase {
    let suite = serde_json::from_str::<GenericAuditSuite>(raw_fixture)
        .unwrap_or_else(|error| panic!("{suite_name} audit fixture should parse: {error}"));
    suite
        .cases
        .into_iter()
        .find(|case| case.id == case_id)
        .unwrap_or_else(|| panic!("{suite_name} audit should include `{case_id}`"))
}
