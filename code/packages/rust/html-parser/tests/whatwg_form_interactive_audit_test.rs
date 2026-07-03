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
const WHATWG_FRAMESET_AUDIT: &str = include_str!("fixtures/whatwg-frameset-audit.json");
const WHATWG_LIST_ITEM_AUDIT: &str = include_str!("fixtures/whatwg-list-item-audit.json");
const WHATWG_NOSCRIPT_AUDIT: &str = include_str!("fixtures/whatwg-noscript-audit.json");
const WHATWG_PARAGRAPH_AUDIT: &str = include_str!("fixtures/whatwg-paragraph-audit.json");
const WHATWG_SELECT_LIST_AUDIT: &str = include_str!("fixtures/whatwg-select-list-audit.json");
const WHATWG_TABLE_AUDIT: &str = include_str!("fixtures/whatwg-table-audit.json");
const WHATWG_TEXT_CONTROL_AUDIT: &str = include_str!("fixtures/whatwg-text-control-audit.json");
const WHATWG_TREE_INSERTION_AUDIT: &str = include_str!("fixtures/whatwg-tree-insertion-audit.json");
const WHATWG_VOID_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-void-element-audit.json");
struct FormInteractiveCrossAxisCase {
    id: &'static str,
    form_axis: &'static str,
    data_snippet: &'static str,
    fragment_context: Option<&'static str>,
    suites: &'static [(&'static str, &'static str, &'static str)],
}

const INTERACTIVE_FORMATTING_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "interactive-formatting-boundary",
    ),
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-table-boundary",
    ),
    ("table", WHATWG_TABLE_AUDIT, "foster-parenting"),
    (
        "tree-insertion",
        WHATWG_TREE_INSERTION_AUDIT,
        "adoption-agency",
    ),
];
const BUTTON_BOUNDARY_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-form-boundary",
    ),
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "interactive-formatting-boundary",
    ),
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-form-boundary",
    ),
];
const FORM_CONTROL_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    ("foreign", WHATWG_FOREIGN_AUDIT, "table-foreign-boundary"),
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
const SELECT_OPTION_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-table-boundary",
    ),
    ("foreign", WHATWG_FOREIGN_AUDIT, "html-integration-point"),
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
const TEXTAREA_RAWTEXT_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "character-reference",
        WHATWG_CHARACTER_REFERENCE_AUDIT,
        "character-reference-rcdata-boundary",
    ),
    ("text-control", WHATWG_TEXT_CONTROL_AUDIT, "rcdata-controls"),
];
const FRAGMENT_CONTEXT_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "fragment-context",
        WHATWG_FRAGMENT_CONTEXT_AUDIT,
        "fragment-select-context",
    ),
    (
        "select-list",
        WHATWG_SELECT_LIST_AUDIT,
        "select-fragment-context",
    ),
    (
        "tree-insertion",
        WHATWG_TREE_INSERTION_AUDIT,
        "html-fragment",
    ),
    (
        "void-element",
        WHATWG_VOID_ELEMENT_AUDIT,
        "void-fragment-context",
    ),
];
const STRAY_INTERACTIVE_END_TAG_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
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
    ("frameset", WHATWG_FRAMESET_AUDIT, "noframes-content"),
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
const FORM_INTERACTIVE_CROSS_AXIS_CASES: &[FormInteractiveCrossAxisCase] = &[
    FormInteractiveCrossAxisCase {
        id: "adoption01-dat-6",
        form_axis: "interactive-formatting",
        data_snippet: "<table><a>1<p>2</a>3</p>",
        fragment_context: None,
        suites: INTERACTIVE_FORMATTING_CROSS_AXIS_SUITES,
    },
    FormInteractiveCrossAxisCase {
        id: "tests20-dat-2",
        form_axis: "button-boundary",
        data_snippet: "<!doctype html><p><button><address>",
        fragment_context: None,
        suites: BUTTON_BOUNDARY_CROSS_AXIS_SUITES,
    },
    FormInteractiveCrossAxisCase {
        id: "adoption01-dat-13",
        form_axis: "form-control",
        data_snippet: "<a><svg><tr><input></a>",
        fragment_context: None,
        suites: FORM_CONTROL_CROSS_AXIS_SUITES,
    },
    FormInteractiveCrossAxisCase {
        id: "tables01-dat-453",
        form_axis: "select-option",
        data_snippet: "<div><table><svg><foreignObject><select><table><s>",
        fragment_context: None,
        suites: SELECT_OPTION_CROSS_AXIS_SUITES,
    },
    FormInteractiveCrossAxisCase {
        id: "tests16-dat-860",
        form_axis: "textarea-rawtext",
        data_snippet: "<!doctype html><textarea>&lt;/textarea></textarea>",
        fragment_context: None,
        suites: TEXTAREA_RAWTEXT_CROSS_AXIS_SUITES,
    },
    FormInteractiveCrossAxisCase {
        id: "tests-innerhtml-1-dat-76",
        form_axis: "fragment-context",
        data_snippet: "<input><option>",
        fragment_context: Some("select"),
        suites: FRAGMENT_CONTEXT_CROSS_AXIS_SUITES,
    },
    FormInteractiveCrossAxisCase {
        id: "tests1-dat-675",
        form_axis: "stray-interactive-end-tags",
        data_snippet: "</strong></b></em></i></u></strike></s></blink></tt></pre>",
        fragment_context: None,
        suites: STRAY_INTERACTIVE_END_TAG_CROSS_AXIS_SUITES,
    },
];
const POST_PARSE_REPAIR_EVIDENCE: &[(&str, &str)] = &[
    ("tests26-dat-4", "interactive-formatting"),
    ("tests26-dat-1251", "interactive-formatting"),
    ("tricky01-dat-5", "form-control"),
    ("tricky01-dat-7", "interactive-formatting"),
    ("tricky01-dat-8", "interactive-formatting"),
    ("tricky01-dat-9", "interactive-formatting"),
];

#[derive(Debug, Deserialize)]
struct FormInteractiveAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<FormInteractiveAuditCase>,
}

#[derive(Debug, Deserialize)]
struct FormInteractiveAuditCase {
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
fn whatwg_form_interactive_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-form-interactive-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 400);
    assert_axis_count(&suite, "interactive-formatting", 120);
    assert_axis_count(&suite, "button-boundary", 90);
    assert_axis_count(&suite, "form-control", 30);
    assert_axis_count(&suite, "select-option", 120);
    assert_axis_count(&suite, "textarea-rawtext", 20);
    assert_axis_count(&suite, "fragment-context", 7);
    assert_axis_count(&suite, "stray-interactive-end-tags", 4);
}

#[test]
fn whatwg_form_interactive_audit_cases_match_parser_dom_dump() {
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
fn whatwg_form_interactive_audit_keeps_form_axis_cases_cross_axis() {
    let suite = load_suite();
    let form_cases = suite
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<HashMap<_, _>>();
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for evidence in FORM_INTERACTIVE_CROSS_AXIS_CASES {
        let form_case = form_cases
            .get(evidence.id)
            .unwrap_or_else(|| panic!("form/interactive audit should include `{}`", evidence.id));
        assert_eq!(
            form_case.axis, evidence.form_axis,
            "form/interactive row `{}` should stay on its focused audit axis",
            evidence.id
        );
        assert!(
            !form_case.reason.is_empty(),
            "form/interactive row `{}` should keep a fixture reason",
            evidence.id
        );

        for (suite_name, fixture, expected_axis) in evidence.suites {
            let audit_case = generic_audit_case(fixture, suite_name, evidence.id);
            assert_eq!(
                audit_case.source, form_case.source,
                "`{suite_name}` should point form/interactive row `{}` at the same html5lib row as the form audit",
                evidence.id
            );
            assert_eq!(
                audit_case.axis, *expected_axis,
                "`{suite_name}` should keep form/interactive row `{}` on its focused axis",
                evidence.id
            );
            assert!(
                !audit_case.reason.is_empty(),
                "`{suite_name}` should keep a reason for form/interactive row `{}`",
                evidence.id
            );
        }

        let source_case = smoke_cases
            .get(&form_case.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", evidence.id));
        assert!(
            source_case.data.contains(evidence.data_snippet),
            "form/interactive row `{}` should stay tied to its html5lib input",
            evidence.id
        );
        if let Some(fragment_context) = evidence.fragment_context {
            assert_eq!(
                source_case.fragment_context.as_deref(),
                Some(fragment_context),
                "form/interactive row `{}` should keep its html5lib fragment context",
                evidence.id
            );
        }

        let actual = actual_dom_dump_for_tree_case(source_case).unwrap_or_else(|error| {
            panic!(
                "case `{}` ({}) parse failed: {error}",
                form_case.id, form_case.axis
            )
        });

        assert_eq!(
            actual, source_case.document,
            "cross-axis form/interactive evidence case `{}` ({}) failed for input {:?}",
            form_case.id, form_case.axis, source_case.data
        );
    }
}

#[test]
fn whatwg_form_interactive_audit_tracks_post_parse_repair_evidence() {
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

fn load_suite() -> FormInteractiveAuditSuite {
    serde_json::from_str(WHATWG_FORM_INTERACTIVE_AUDIT)
        .expect("WHATWG form/interactive audit fixture should parse")
}

fn assert_axis_count(suite: &FormInteractiveAuditSuite, axis: &str, minimum: usize) {
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
