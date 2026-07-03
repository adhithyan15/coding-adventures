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
const WHATWG_FRAGMENT_CONTEXT_AUDIT: &str =
    include_str!("fixtures/whatwg-fragment-context-audit.json");
const WHATWG_FRAMESET_AUDIT: &str = include_str!("fixtures/whatwg-frameset-audit.json");
const WHATWG_TABLE_AUDIT: &str = include_str!("fixtures/whatwg-table-audit.json");
const WHATWG_TEMPLATE_AUDIT: &str = include_str!("fixtures/whatwg-template-audit.json");
const WHATWG_TEXT_CONTROL_AUDIT: &str = include_str!("fixtures/whatwg-text-control-audit.json");
const WHATWG_TREE_INSERTION_AUDIT: &str = include_str!("fixtures/whatwg-tree-insertion-audit.json");
const WHATWG_VOID_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-void-element-audit.json");

struct FragmentCrossAxisCase {
    id: &'static str,
    context: &'static str,
    fragment_axis: &'static str,
    data_snippet: &'static str,
    suites: &'static [(&'static str, &'static str, &'static str)],
}

const POST_PARSE_REPAIR_EVIDENCE: &[(&str, &str, &str)] = &[
    ("tests4-dat-1", "div", "fragment-basic-context"),
    ("tests4-dat-2", "textarea", "fragment-text-mode-context"),
    ("tests4-dat-6", "html", "fragment-shell-context"),
    ("tests6-dat-7", "div", "fragment-block-context"),
    ("tests6-dat-18", "caption", "fragment-table-context"),
    (
        "tests-innerhtml-1-dat-75",
        "select",
        "fragment-select-context",
    ),
    (
        "foreign-fragment-dat-1",
        "svg path",
        "fragment-foreign-context",
    ),
    ("template-dat-109", "template", "fragment-template-context"),
];
const TEMPLATE_FRAGMENT_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-fragment-context",
    ),
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "form-control",
    ),
    (
        "template",
        WHATWG_TEMPLATE_AUDIT,
        "template-fragment-context",
    ),
    (
        "tree-insertion",
        WHATWG_TREE_INSERTION_AUDIT,
        "template-insertion",
    ),
    (
        "void-element",
        WHATWG_VOID_ELEMENT_AUDIT,
        "void-fragment-context",
    ),
];
const TABLE_FRAGMENT_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
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
    ("table", WHATWG_TABLE_AUDIT, "table-fragment-context"),
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
const TEXT_MODE_FRAGMENT_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "fragment-context",
    ),
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "formatting-reconstruction",
    ),
    (
        "text-control",
        WHATWG_TEXT_CONTROL_AUDIT,
        "fragment-context",
    ),
];
const FRAMESET_FRAGMENT_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "body-frameset-boundary",
    ),
    ("frameset", WHATWG_FRAMESET_AUDIT, "frame-element"),
    (
        "void-element",
        WHATWG_VOID_ELEMENT_AUDIT,
        "void-fragment-context",
    ),
];
const FRAGMENT_CROSS_AXIS_CASES: &[FragmentCrossAxisCase] = &[
    FragmentCrossAxisCase {
        id: "template-dat-109",
        context: "template",
        fragment_axis: "fragment-template-context",
        data_snippet: "<template><form><input name=\"q\"></form><div>second</div></template>",
        suites: TEMPLATE_FRAGMENT_CROSS_AXIS_SUITES,
    },
    FragmentCrossAxisCase {
        id: "tests-innerhtml-1-dat-13",
        context: "table",
        fragment_axis: "fragment-table-context",
        data_snippet: "<a><colgroup><col>",
        suites: TABLE_FRAGMENT_CROSS_AXIS_SUITES,
    },
    FragmentCrossAxisCase {
        id: "tests4-dat-3",
        context: "textarea",
        fragment_axis: "fragment-text-mode-context",
        data_snippet: "textarea content with <em>pseudo</em> <foo>markup",
        suites: TEXT_MODE_FRAGMENT_CROSS_AXIS_SUITES,
    },
    FragmentCrossAxisCase {
        id: "tests6-dat-30",
        context: "frameset",
        fragment_axis: "fragment-shell-context",
        data_snippet: "</frameset><frame>",
        suites: FRAMESET_FRAGMENT_CROSS_AXIS_SUITES,
    },
];

#[derive(Debug, Deserialize)]
struct FragmentContextAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<FragmentContextAuditCase>,
}

#[derive(Debug, Deserialize)]
struct FragmentContextAuditCase {
    id: String,
    source: String,
    context: String,
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
fn whatwg_fragment_context_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-fragment-context-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 190);
    assert_axis_count(&suite, "fragment-table-context", 90);
    assert_axis_count(&suite, "fragment-foreign-context", 60);
    assert_axis_count(&suite, "fragment-shell-context", 13);
    assert_axis_count(&suite, "fragment-text-mode-context", 6);
    assert_axis_count(&suite, "fragment-block-context", 5);
    assert_axis_count(&suite, "fragment-select-context", 5);
    assert_axis_count(&suite, "fragment-basic-context", 3);
    assert_axis_count(&suite, "fragment-template-context", 1);
}

#[test]
fn whatwg_fragment_context_audit_cases_match_parser_dom_dump() {
    let suite = load_suite();
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for case in &suite.cases {
        assert!(
            !case.id.is_empty()
                && !case.context.is_empty()
                && !case.axis.is_empty()
                && !case.reason.is_empty(),
            "case `{}` should carry audit metadata",
            case.source
        );
        let source_case = smoke_cases
            .get(&case.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", case.source));
        assert_eq!(
            source_case.fragment_context.as_deref(),
            Some(case.context.as_str()),
            "case `{}` should keep its fragment context",
            case.source
        );
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
fn whatwg_fragment_context_audit_keeps_special_context_cases_cross_axis() {
    let suite = load_suite();
    let fragment_cases = suite
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<HashMap<_, _>>();
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for evidence in FRAGMENT_CROSS_AXIS_CASES {
        let fragment_case = fragment_cases
            .get(evidence.id)
            .unwrap_or_else(|| panic!("fragment-context audit should include `{}`", evidence.id));
        assert_eq!(
            fragment_case.context, evidence.context,
            "fragment-context row `{}` should keep its focused fragment context",
            evidence.id
        );
        assert_eq!(
            fragment_case.axis, evidence.fragment_axis,
            "fragment-context row `{}` should stay on its focused audit axis",
            evidence.id
        );
        assert!(
            !fragment_case.reason.is_empty(),
            "fragment-context row `{}` should keep a fixture reason",
            evidence.id
        );

        for (suite_name, fixture, expected_axis) in evidence.suites {
            let audit_case = generic_audit_case(fixture, suite_name, evidence.id);
            assert_eq!(
                audit_case.source, fragment_case.source,
                "`{suite_name}` should point fragment row `{}` at the same html5lib row as the fragment audit",
                evidence.id
            );
            assert_eq!(
                audit_case.axis, *expected_axis,
                "`{suite_name}` should keep fragment row `{}` on its focused axis",
                evidence.id
            );
            assert!(
                !audit_case.reason.is_empty(),
                "`{suite_name}` should keep a reason for fragment row `{}`",
                evidence.id
            );
        }

        let source_case = smoke_cases
            .get(&fragment_case.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", evidence.id));
        assert_eq!(
            source_case.fragment_context.as_deref(),
            Some(evidence.context),
            "fragment row `{}` should keep its smoke fragment context",
            evidence.id
        );
        assert!(
            source_case.data.contains(evidence.data_snippet),
            "fragment row `{}` should stay tied to its html5lib input",
            evidence.id
        );
        let actual = actual_dom_dump_for_tree_case(source_case).unwrap_or_else(|error| {
            panic!(
                "case `{}` ({}) parse failed: {error}",
                fragment_case.id, fragment_case.axis
            )
        });

        assert_eq!(
            actual, source_case.document,
            "cross-axis fragment evidence case `{}` ({}) failed for input {:?}",
            fragment_case.id, fragment_case.axis, source_case.data
        );
    }
}

#[test]
fn whatwg_fragment_context_audit_tracks_post_parse_repair_evidence() {
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

    for (case_id, expected_context, expected_axis) in POST_PARSE_REPAIR_EVIDENCE {
        let audit_case = audit_cases.get(case_id).unwrap_or_else(|| {
            panic!("post-parse repair evidence case `{case_id}` should be audited")
        });
        assert_eq!(
            audit_case.context, *expected_context,
            "post-parse repair evidence case `{case_id}` should keep its focused fragment context"
        );
        assert_eq!(
            audit_case.axis, *expected_axis,
            "post-parse repair evidence case `{case_id}` should stay on its focused audit axis"
        );

        let source_case = smoke_cases
            .get(&audit_case.source)
            .unwrap_or_else(|| panic!("case `{case_id}` should exist in smoke fixture"));
        assert_eq!(
            source_case.fragment_context.as_deref(),
            Some(*expected_context),
            "post-parse repair evidence case `{case_id}` should keep its smoke fragment context"
        );
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

fn load_suite() -> FragmentContextAuditSuite {
    serde_json::from_str(WHATWG_FRAGMENT_CONTEXT_AUDIT)
        .expect("WHATWG fragment-context audit fixture should parse")
}

fn generic_audit_case(fixture: &str, suite_name: &str, case_id: &str) -> GenericAuditCase {
    let suite = serde_json::from_str::<GenericAuditSuite>(fixture)
        .unwrap_or_else(|error| panic!("`{suite_name}` audit fixture should parse: {error}"));
    suite
        .cases
        .into_iter()
        .find(|case| case.id == case_id)
        .unwrap_or_else(|| panic!("`{suite_name}` should audit fragment case `{case_id}`"))
}

fn assert_axis_count(suite: &FragmentContextAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
