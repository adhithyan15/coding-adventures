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
const WHATWG_HEAD_BODY_AUDIT: &str = include_str!("fixtures/whatwg-head-body-audit.json");
const WHATWG_LEGACY_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-legacy-element-audit.json");
const WHATWG_PARAGRAPH_AUDIT: &str = include_str!("fixtures/whatwg-paragraph-audit.json");
const WHATWG_SELECT_LIST_AUDIT: &str = include_str!("fixtures/whatwg-select-list-audit.json");
const WHATWG_TABLE_AUDIT: &str = include_str!("fixtures/whatwg-table-audit.json");
const WHATWG_TEMPLATE_AUDIT: &str = include_str!("fixtures/whatwg-template-audit.json");
const WHATWG_TEXT_CONTROL_AUDIT: &str = include_str!("fixtures/whatwg-text-control-audit.json");
const WHATWG_TREE_INSERTION_AUDIT: &str = include_str!("fixtures/whatwg-tree-insertion-audit.json");
const WHATWG_VOID_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-void-element-audit.json");
struct DocumentShellCrossAxisCase {
    id: &'static str,
    document_shell_axis: &'static str,
    data_snippet: &'static str,
    suites: &'static [(&'static str, &'static str, &'static str)],
}

const DOCTYPE_SHELL_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "formatting-reconstruction",
    ),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "body-boundary"),
];
const HTML_SHELL_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-text-mode-boundary",
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
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "body-boundary"),
    (
        "legacy-element",
        WHATWG_LEGACY_ELEMENT_AUDIT,
        "tricky-parser-recovery",
    ),
    (
        "text-control",
        WHATWG_TEXT_CONTROL_AUDIT,
        "pre-listing-newline",
    ),
];
const HEAD_SHELL_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
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
const BODY_SHELL_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
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
const COMMENT_SHELL_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "character-reference",
        WHATWG_CHARACTER_REFERENCE_AUDIT,
        "character-reference-rcdata-boundary",
    ),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "head-text-mode"),
    ("text-control", WHATWG_TEXT_CONTROL_AUDIT, "rcdata-controls"),
];
const FRAGMENT_SHELL_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "fragment-context",
        WHATWG_FRAGMENT_CONTEXT_AUDIT,
        "fragment-shell-context",
    ),
    ("frameset", WHATWG_FRAMESET_AUDIT, "frameset-shell"),
    (
        "tree-insertion",
        WHATWG_TREE_INSERTION_AUDIT,
        "html-fragment",
    ),
];
const IMPLICIT_SHELL_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "form-control",
    ),
    ("table", WHATWG_TABLE_AUDIT, "foster-parenting"),
    ("void-element", WHATWG_VOID_ELEMENT_AUDIT, "void-in-table"),
];
const DOCUMENT_SHELL_CROSS_AXIS_CASES: &[DocumentShellCrossAxisCase] = &[
    DocumentShellCrossAxisCase {
        id: "doctype01-dat-116",
        document_shell_axis: "doctype-and-quirks",
        data_snippet: "<!DOCTYPE HTML SYSTEM \"http://www.w3.org/DTD/HTML4-strict.dtd\"><body><b>Mine!</b></body>",
        suites: DOCTYPE_SHELL_CROSS_AXIS_SUITES,
    },
    DocumentShellCrossAxisCase {
        id: "tricky01-dat-9",
        document_shell_axis: "html-element-boundary",
        data_snippet: "<b><nobr><div>This text is in a div inside a nobr</nobr>",
        suites: HTML_SHELL_CROSS_AXIS_SUITES,
    },
    DocumentShellCrossAxisCase {
        id: "template-dat-556",
        document_shell_axis: "head-element-boundary",
        data_snippet: "<body><table><tr><td><select><template>Foo</template><caption>A</table>",
        suites: HEAD_SHELL_CROSS_AXIS_SUITES,
    },
    DocumentShellCrossAxisCase {
        id: "tests10-dat-17",
        document_shell_axis: "body-frameset-boundary",
        data_snippet: "<!DOCTYPE html><body><table><tr><td><select><svg><g>foo</g><g>bar</g><p>baz</table><p>quux",
        suites: BODY_SHELL_CROSS_AXIS_SUITES,
    },
    DocumentShellCrossAxisCase {
        id: "tests6-dat-4",
        document_shell_axis: "comment-whitespace-shell",
        data_snippet: "<!doctype html><title><!--&amp;--></title>",
        suites: COMMENT_SHELL_CROSS_AXIS_SUITES,
    },
    DocumentShellCrossAxisCase {
        id: "tests-innerhtml-1-dat-5",
        document_shell_axis: "shell-fragment-context",
        data_snippet: "<frameset><span>",
        suites: FRAGMENT_SHELL_CROSS_AXIS_SUITES,
    },
    DocumentShellCrossAxisCase {
        id: "tests7-dat-19",
        document_shell_axis: "implicit-document-shell",
        data_snippet: "<!doctype html><table><input type=hidDEN></table>",
        suites: IMPLICIT_SHELL_CROSS_AXIS_SUITES,
    },
];
const POST_PARSE_REPAIR_EVIDENCE: &[(&str, &str)] = &[
    ("tricky01-dat-3", "html-element-boundary"),
    ("tests26-dat-1251", "body-frameset-boundary"),
];

#[derive(Debug, Deserialize)]
struct DocumentShellAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<DocumentShellAuditCase>,
}

#[derive(Debug, Deserialize)]
struct DocumentShellAuditCase {
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
fn whatwg_document_shell_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-document-shell-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 700);
    assert_axis_count(&suite, "doctype-and-quirks", 35);
    assert_axis_count(&suite, "html-element-boundary", 200);
    assert_axis_count(&suite, "head-element-boundary", 125);
    assert_axis_count(&suite, "body-frameset-boundary", 300);
    assert_axis_count(&suite, "comment-whitespace-shell", 15);
    assert_axis_count(&suite, "shell-fragment-context", 10);
    assert_axis_count(&suite, "implicit-document-shell", 15);
}

#[test]
fn whatwg_document_shell_audit_cases_match_parser_dom_dump() {
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
fn whatwg_document_shell_audit_keeps_shell_axis_cases_cross_axis() {
    let suite = load_suite();
    let document_shell_cases = suite
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<HashMap<_, _>>();
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for evidence in DOCUMENT_SHELL_CROSS_AXIS_CASES {
        let document_shell_case = document_shell_cases
            .get(evidence.id)
            .unwrap_or_else(|| panic!("document-shell audit should include `{}`", evidence.id));
        assert_eq!(
            document_shell_case.axis, evidence.document_shell_axis,
            "document-shell row `{}` should stay on its focused audit axis",
            evidence.id
        );
        assert!(
            !document_shell_case.reason.is_empty(),
            "document-shell row `{}` should keep a fixture reason",
            evidence.id
        );

        for (suite_name, fixture, expected_axis) in evidence.suites {
            let audit_case = generic_audit_case(fixture, suite_name, evidence.id);
            assert_eq!(
                audit_case.source, document_shell_case.source,
                "`{suite_name}` should point document-shell row `{}` at the same html5lib row as the document-shell audit",
                evidence.id
            );
            assert_eq!(
                audit_case.axis, *expected_axis,
                "`{suite_name}` should keep document-shell row `{}` on its focused axis",
                evidence.id
            );
            assert!(
                !audit_case.reason.is_empty(),
                "`{suite_name}` should keep a reason for document-shell row `{}`",
                evidence.id
            );
        }

        let source_case = smoke_cases
            .get(&document_shell_case.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", evidence.id));
        assert!(
            source_case.data.contains(evidence.data_snippet),
            "document-shell row `{}` should stay tied to its html5lib input",
            evidence.id
        );
        let actual = actual_dom_dump_for_tree_case(source_case).unwrap_or_else(|error| {
            panic!(
                "case `{}` ({}) parse failed: {error}",
                document_shell_case.id, document_shell_case.axis
            )
        });

        assert_eq!(
            actual, source_case.document,
            "cross-axis document-shell evidence case `{}` ({}) failed for input {:?}",
            document_shell_case.id, document_shell_case.axis, source_case.data
        );
    }
}

#[test]
fn whatwg_document_shell_audit_tracks_post_parse_repair_evidence() {
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

fn load_suite() -> DocumentShellAuditSuite {
    serde_json::from_str(WHATWG_DOCUMENT_SHELL_AUDIT)
        .expect("WHATWG document-shell audit fixture should parse")
}

fn generic_audit_case(fixture: &str, suite_name: &str, case_id: &str) -> GenericAuditCase {
    let suite = serde_json::from_str::<GenericAuditSuite>(fixture)
        .unwrap_or_else(|error| panic!("`{suite_name}` audit fixture should parse: {error}"));
    suite
        .cases
        .into_iter()
        .find(|case| case.id == case_id)
        .unwrap_or_else(|| panic!("`{suite_name}` should audit document-shell case `{case_id}`"))
}

fn assert_axis_count(suite: &DocumentShellAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
