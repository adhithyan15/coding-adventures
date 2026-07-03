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
const WHATWG_PARAGRAPH_AUDIT: &str = include_str!("fixtures/whatwg-paragraph-audit.json");
const WHATWG_SELECT_LIST_AUDIT: &str = include_str!("fixtures/whatwg-select-list-audit.json");
const WHATWG_TABLE_AUDIT: &str = include_str!("fixtures/whatwg-table-audit.json");
const WHATWG_TEMPLATE_AUDIT: &str = include_str!("fixtures/whatwg-template-audit.json");
const WHATWG_TREE_INSERTION_AUDIT: &str = include_str!("fixtures/whatwg-tree-insertion-audit.json");
const WHATWG_VOID_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-void-element-audit.json");
struct TreeInsertionCrossAxisCase {
    id: &'static str,
    tree_axis: &'static str,
    data_snippet: &'static str,
    fragment_context: Option<&'static str>,
    suites: &'static [(&'static str, &'static str, &'static str)],
}

const TREE_ADOPTION_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
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
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-table-boundary",
    ),
    ("table", WHATWG_TABLE_AUDIT, "foster-parenting"),
];
const TREE_TABLE_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
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
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "adoption-agency-formatting",
    ),
    ("select-list", WHATWG_SELECT_LIST_AUDIT, "select-in-table"),
    ("table", WHATWG_TABLE_AUDIT, "select-in-table"),
];
const TREE_TEMPLATE_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "head-element-boundary",
    ),
    ("frameset", WHATWG_FRAMESET_AUDIT, "template-boundary"),
    (
        "head-body",
        WHATWG_HEAD_BODY_AUDIT,
        "body-frameset-transition",
    ),
    ("template", WHATWG_TEMPLATE_AUDIT, "frameset-template"),
    (
        "void-element",
        WHATWG_VOID_ELEMENT_AUDIT,
        "legacy-void-elements",
    ),
];
const TREE_FOREIGN_FRAGMENT_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    ("foreign", WHATWG_FOREIGN_AUDIT, "foreign-fragment"),
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "interactive-formatting",
    ),
    (
        "fragment-context",
        WHATWG_FRAGMENT_CONTEXT_AUDIT,
        "fragment-foreign-context",
    ),
];
const TREE_HTML_FRAGMENT_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "shell-fragment-context",
    ),
    (
        "fragment-context",
        WHATWG_FRAGMENT_CONTEXT_AUDIT,
        "fragment-shell-context",
    ),
];
const TREE_INSERTION_CROSS_AXIS_CASES: &[TreeInsertionCrossAxisCase] = &[
    TreeInsertionCrossAxisCase {
        id: "adoption01-dat-6",
        tree_axis: "adoption-agency",
        data_snippet: "<table><a>1<p>2</a>3</p>",
        fragment_context: None,
        suites: TREE_ADOPTION_CROSS_AXIS_SUITES,
    },
    TreeInsertionCrossAxisCase {
        id: "tables01-dat-453",
        tree_axis: "table-insertion",
        data_snippet: "<div><table><svg><foreignObject><select><table><s>",
        fragment_context: None,
        suites: TREE_TABLE_CROSS_AXIS_SUITES,
    },
    TreeInsertionCrossAxisCase {
        id: "template-dat-494",
        tree_axis: "template-insertion",
        data_snippet: "<frameset><template><frame></frame></template></frameset>",
        fragment_context: None,
        suites: TREE_TEMPLATE_CROSS_AXIS_SUITES,
    },
    TreeInsertionCrossAxisCase {
        id: "foreign-fragment-dat-1",
        tree_axis: "foreign-fragment",
        data_snippet: "<nobr>X",
        fragment_context: Some("svg path"),
        suites: TREE_FOREIGN_FRAGMENT_CROSS_AXIS_SUITES,
    },
    TreeInsertionCrossAxisCase {
        id: "tests-innerhtml-1-dat-1",
        tree_axis: "html-fragment",
        data_snippet: "<body><span>",
        fragment_context: Some("body"),
        suites: TREE_HTML_FRAGMENT_CROSS_AXIS_SUITES,
    },
];
const POST_PARSE_REPAIR_EVIDENCE: &[(&str, &str, &str)] = &[
    ("adoption01-dat-1", "adoption01.dat:1", "adoption-agency"),
    ("tables01-dat-436", "tables01.dat:436", "table-insertion"),
    ("template-dat-455", "template.dat:455", "template-insertion"),
    (
        "tests-innerhtml-1-dat-1",
        "tests_innerHTML_1.dat:1",
        "html-fragment",
    ),
    (
        "foreign-fragment-dat-1",
        "foreign-fragment.dat:1",
        "foreign-fragment",
    ),
];

#[derive(Debug, Deserialize)]
struct TreeInsertionSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<TreeInsertionCase>,
}

#[derive(Debug, Deserialize)]
struct TreeInsertionCase {
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
fn whatwg_tree_insertion_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-tree-insertion-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 250);
    assert_axis_count(&suite, "adoption-agency", 19);
    assert_axis_count(&suite, "table-insertion", 15);
    assert_axis_count(&suite, "template-insertion", 100);
    assert_axis_count(&suite, "foreign-fragment", 60);
    assert_axis_count(&suite, "html-fragment", 75);
}

#[test]
fn whatwg_tree_insertion_audit_cases_match_parser_dom_dump() {
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
fn whatwg_tree_insertion_audit_keeps_tree_axis_cases_cross_axis() {
    let suite = load_suite();
    let tree_cases = suite
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<HashMap<_, _>>();
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for evidence in TREE_INSERTION_CROSS_AXIS_CASES {
        let tree_case = tree_cases
            .get(evidence.id)
            .unwrap_or_else(|| panic!("tree-insertion audit should include `{}`", evidence.id));
        assert_eq!(
            tree_case.axis, evidence.tree_axis,
            "tree-insertion row `{}` should stay on its focused audit axis",
            evidence.id
        );
        assert!(
            !tree_case.reason.is_empty(),
            "tree-insertion row `{}` should keep a fixture reason",
            evidence.id
        );

        for (suite_name, fixture, expected_axis) in evidence.suites {
            let audit_case = generic_audit_case(fixture, suite_name, evidence.id);
            assert_eq!(
                audit_case.source, tree_case.source,
                "`{suite_name}` should point tree-insertion row `{}` at the same html5lib row as the tree-insertion audit",
                evidence.id
            );
            assert_eq!(
                audit_case.axis, *expected_axis,
                "`{suite_name}` should keep tree-insertion row `{}` on its focused axis",
                evidence.id
            );
            assert!(
                !audit_case.reason.is_empty(),
                "`{suite_name}` should keep a reason for tree-insertion row `{}`",
                evidence.id
            );
        }

        let source_case = smoke_cases
            .get(&tree_case.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", evidence.id));
        assert!(
            source_case.data.contains(evidence.data_snippet),
            "tree-insertion row `{}` should stay tied to its html5lib input",
            evidence.id
        );
        if let Some(fragment_context) = evidence.fragment_context {
            assert_eq!(
                source_case.fragment_context.as_deref(),
                Some(fragment_context),
                "tree-insertion row `{}` should keep its html5lib fragment context",
                evidence.id
            );
        }

        let actual = actual_dom_dump_for_tree_case(source_case).unwrap_or_else(|error| {
            panic!(
                "case `{}` ({}) parse failed: {error}",
                tree_case.id, tree_case.axis
            )
        });

        assert_eq!(
            actual, source_case.document,
            "cross-axis tree-insertion evidence case `{}` ({}) failed for input {:?}",
            tree_case.id, tree_case.axis, source_case.data
        );
    }
}

#[test]
fn whatwg_tree_insertion_audit_tracks_post_parse_repair_evidence() {
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

    for (case_id, expected_source, expected_axis) in POST_PARSE_REPAIR_EVIDENCE {
        let audit_case = audit_cases.get(case_id).unwrap_or_else(|| {
            panic!("post-parse repair evidence case `{case_id}` should be audited")
        });
        assert_eq!(
            audit_case.source, *expected_source,
            "post-parse repair evidence case `{case_id}` should stay tied to its smoke fixture row"
        );
        assert_eq!(
            audit_case.axis, *expected_axis,
            "post-parse repair evidence case `{case_id}` should stay on its focused audit axis"
        );

        let source_case = smoke_cases
            .get(*expected_source)
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

fn load_suite() -> TreeInsertionSuite {
    serde_json::from_str(WHATWG_TREE_INSERTION_AUDIT)
        .expect("WHATWG tree insertion audit fixture should parse")
}

fn assert_axis_count(suite: &TreeInsertionSuite, axis: &str, minimum: usize) {
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
