mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_BLOCK_BOUNDARY_AUDIT: &str = include_str!("fixtures/whatwg-block-boundary-audit.json");
const WHATWG_CHARACTER_REFERENCE_AUDIT: &str =
    include_str!("fixtures/whatwg-character-reference-audit.json");
const WHATWG_DOCUMENT_SHELL_AUDIT: &str = include_str!("fixtures/whatwg-document-shell-audit.json");
const WHATWG_FOREIGN_AUDIT: &str = include_str!("fixtures/whatwg-foreign-audit.json");
const WHATWG_FORM_INTERACTIVE_AUDIT: &str =
    include_str!("fixtures/whatwg-form-interactive-audit.json");
const WHATWG_FORMATTING_AUDIT: &str = include_str!("fixtures/whatwg-formatting-audit.json");
const WHATWG_FRAGMENT_CONTEXT_AUDIT: &str =
    include_str!("fixtures/whatwg-fragment-context-audit.json");
const WHATWG_HEAD_BODY_AUDIT: &str = include_str!("fixtures/whatwg-head-body-audit.json");
const WHATWG_NOSCRIPT_AUDIT: &str = include_str!("fixtures/whatwg-noscript-audit.json");
const WHATWG_PARAGRAPH_AUDIT: &str = include_str!("fixtures/whatwg-paragraph-audit.json");
const WHATWG_SELECT_LIST_AUDIT: &str = include_str!("fixtures/whatwg-select-list-audit.json");
const WHATWG_TABLE_AUDIT: &str = include_str!("fixtures/whatwg-table-audit.json");
const WHATWG_TEMPLATE_AUDIT: &str = include_str!("fixtures/whatwg-template-audit.json");
const WHATWG_TEXT_CONTROL_AUDIT: &str = include_str!("fixtures/whatwg-text-control-audit.json");
const WHATWG_TREE_INSERTION_AUDIT: &str = include_str!("fixtures/whatwg-tree-insertion-audit.json");
const WHATWG_VOID_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-void-element-audit.json");

struct TextControlCrossAxisCase {
    id: &'static str,
    text_axis: &'static str,
    data_snippet: &'static str,
    suites: &'static [(&'static str, &'static str, &'static str)],
}

const RCDATA_CONTROL_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "character-reference",
        WHATWG_CHARACTER_REFERENCE_AUDIT,
        "character-reference-rcdata-boundary",
    ),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "head-text-mode"),
];
const TEMPLATE_RAWTEXT_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "head-element-boundary",
    ),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "head-text-mode"),
    ("table", WHATWG_TABLE_AUDIT, "cell-boundary"),
    ("template", WHATWG_TEMPLATE_AUDIT, "rawtext-template"),
    (
        "tree-insertion",
        WHATWG_TREE_INSERTION_AUDIT,
        "template-insertion",
    ),
];
const NOSCRIPT_TEXT_CONTROL_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "head-element-boundary",
    ),
    ("formatting", WHATWG_FORMATTING_AUDIT, "paragraph-boundary"),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "head-boundary"),
    ("noscript", WHATWG_NOSCRIPT_AUDIT, "head-noscript-disabled"),
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-special-end-tag",
    ),
];
const FRAGMENT_TEXT_CONTROL_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "character-reference",
        WHATWG_CHARACTER_REFERENCE_AUDIT,
        "character-reference-fragment-context",
    ),
    (
        "fragment-context",
        WHATWG_FRAGMENT_CONTEXT_AUDIT,
        "fragment-text-mode-context",
    ),
];
const STRAY_TEXT_CONTROL_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
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
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "head-text-mode"),
    ("noscript", WHATWG_NOSCRIPT_AUDIT, "stray-noscript-end-tag"),
    (
        "select-list",
        WHATWG_SELECT_LIST_AUDIT,
        "stray-select-end-tags",
    ),
    ("table", WHATWG_TABLE_AUDIT, "caption-colgroup"),
    (
        "void-element",
        WHATWG_VOID_ELEMENT_AUDIT,
        "stray-void-end-tags",
    ),
];
const PLAINTEXT_FOREIGN_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-foreign-boundary",
    ),
    ("foreign", WHATWG_FOREIGN_AUDIT, "html-integration-point"),
];
const TEXT_CONTROL_CROSS_AXIS_CASES: &[TextControlCrossAxisCase] = &[
    TextControlCrossAxisCase {
        id: "tests5-dat-13",
        text_axis: "rcdata-controls",
        data_snippet: "<title>&amp;</title>",
        suites: RCDATA_CONTROL_CROSS_AXIS_SUITES,
    },
    TextControlCrossAxisCase {
        id: "template-dat-498",
        text_axis: "script-rawtext",
        data_snippet: "<body><template><script>var i = 1;</script><td></td></template>",
        suites: TEMPLATE_RAWTEXT_CROSS_AXIS_SUITES,
    },
    TextControlCrossAxisCase {
        id: "noscript01-dat-341",
        text_axis: "noscript-scripting",
        data_snippet: "<head><noscript></p><!--foo--></noscript>",
        suites: NOSCRIPT_TEXT_CONTROL_CROSS_AXIS_SUITES,
    },
    TextControlCrossAxisCase {
        id: "tests4-dat-4",
        text_axis: "fragment-context",
        data_snippet: "this is &#x0043;DATA inside a <style> element",
        suites: FRAGMENT_TEXT_CONTROL_CROSS_AXIS_SUITES,
    },
    TextControlCrossAxisCase {
        id: "tests1-dat-110",
        text_axis: "stray-text-control-end-tags",
        data_snippet: "</strong></b></em></i></u></strike></s></blink>",
        suites: STRAY_TEXT_CONTROL_CROSS_AXIS_SUITES,
    },
    TextControlCrossAxisCase {
        id: "webkit02-dat-20",
        text_axis: "plaintext-recovery",
        data_snippet:
            "<svg><foreignObject><div>foo</div><plaintext></foreignObject></svg><div>bar</div>",
        suites: PLAINTEXT_FOREIGN_CROSS_AXIS_SUITES,
    },
];
const POST_PARSE_REPAIR_EVIDENCE: &[(&str, &str)] = &[
    ("scripted-ark-dat-1", "script-rawtext"),
    ("scripted-webkit01-dat-2", "script-rawtext"),
];

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

#[test]
fn whatwg_text_control_audit_keeps_text_mode_cases_cross_axis() {
    let suite = load_suite();
    let text_cases = suite
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<HashMap<_, _>>();
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for evidence in TEXT_CONTROL_CROSS_AXIS_CASES {
        let text_case = text_cases
            .get(evidence.id)
            .unwrap_or_else(|| panic!("text-control audit should include `{}`", evidence.id));
        assert_eq!(
            text_case.axis, evidence.text_axis,
            "text-control row `{}` should stay on its focused audit axis",
            evidence.id
        );
        assert!(
            !text_case.reason.is_empty(),
            "text-control row `{}` should keep a fixture reason",
            evidence.id
        );

        for (suite_name, fixture, expected_axis) in evidence.suites {
            let audit_case = generic_audit_case(fixture, suite_name, evidence.id);
            assert_eq!(
                audit_case.source, text_case.source,
                "`{suite_name}` should point text-control row `{}` at the same html5lib row as the text-control audit",
                evidence.id
            );
            assert_eq!(
                audit_case.axis, *expected_axis,
                "`{suite_name}` should keep text-control row `{}` on its focused axis",
                evidence.id
            );
            assert!(
                !audit_case.reason.is_empty(),
                "`{suite_name}` should keep a reason for text-control row `{}`",
                evidence.id
            );
        }

        let source_case = smoke_cases
            .get(&text_case.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", evidence.id));
        assert!(
            source_case.data.contains(evidence.data_snippet),
            "text-control row `{}` should stay tied to its html5lib input",
            evidence.id
        );
        let actual = actual_dom_dump_for_tree_case(source_case).unwrap_or_else(|error| {
            panic!(
                "case `{}` ({}) parse failed: {error}",
                text_case.id, text_case.axis
            )
        });

        assert_eq!(
            actual, source_case.document,
            "cross-axis text-control evidence case `{}` ({}) failed for input {:?}",
            text_case.id, text_case.axis, source_case.data
        );
    }
}

#[test]
fn whatwg_text_control_audit_tracks_post_parse_repair_evidence() {
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

fn load_suite() -> TextControlAuditSuite {
    serde_json::from_str(WHATWG_TEXT_CONTROL_AUDIT)
        .expect("WHATWG text-control audit fixture should parse")
}

fn generic_audit_case(fixture: &str, suite_name: &str, case_id: &str) -> GenericAuditCase {
    let suite = serde_json::from_str::<GenericAuditSuite>(fixture)
        .unwrap_or_else(|error| panic!("`{suite_name}` audit fixture should parse: {error}"));
    suite
        .cases
        .into_iter()
        .find(|case| case.id == case_id)
        .unwrap_or_else(|| panic!("`{suite_name}` should audit text-control case `{case_id}`"))
}

fn assert_axis_count(suite: &TextControlAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
