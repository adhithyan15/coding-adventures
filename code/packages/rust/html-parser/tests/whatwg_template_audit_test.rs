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
const WHATWG_FRAMESET_AUDIT: &str = include_str!("fixtures/whatwg-frameset-audit.json");
const WHATWG_HEAD_BODY_AUDIT: &str = include_str!("fixtures/whatwg-head-body-audit.json");
const WHATWG_SELECT_LIST_AUDIT: &str = include_str!("fixtures/whatwg-select-list-audit.json");
const WHATWG_TABLE_AUDIT: &str = include_str!("fixtures/whatwg-table-audit.json");
const WHATWG_TEMPLATE_AUDIT: &str = include_str!("fixtures/whatwg-template-audit.json");
const WHATWG_TREE_INSERTION_AUDIT: &str = include_str!("fixtures/whatwg-tree-insertion-audit.json");
const WHATWG_VOID_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-void-element-audit.json");

struct TemplateCrossAxisCase {
    id: &'static str,
    data_snippet: &'static str,
    suites: &'static [(&'static str, &'static str, &'static str)],
}

const POST_PARSE_REPAIR_EVIDENCE: &[(&str, &str)] = &[
    ("template-dat-461", "eof-template"),
    ("template-dat-463", "table-template"),
    ("template-dat-473", "select-template"),
    ("template-dat-488", "nested-template"),
    ("template-dat-494", "frameset-template"),
    ("template-dat-512", "head-body-template"),
];
const SELECT_TEMPLATE_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "select-option",
    ),
    ("select-list", WHATWG_SELECT_LIST_AUDIT, "select-shell"),
    ("template", WHATWG_TEMPLATE_AUDIT, "select-template"),
    (
        "tree-insertion",
        WHATWG_TREE_INSERTION_AUDIT,
        "template-insertion",
    ),
];
const TABLE_TEMPLATE_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-table-boundary",
    ),
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "head-element-boundary",
    ),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "body-boundary"),
    ("table", WHATWG_TABLE_AUDIT, "cell-boundary"),
    ("template", WHATWG_TEMPLATE_AUDIT, "table-template"),
    (
        "tree-insertion",
        WHATWG_TREE_INSERTION_AUDIT,
        "template-insertion",
    ),
];
const FRAMESET_TEMPLATE_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
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
        "tree-insertion",
        WHATWG_TREE_INSERTION_AUDIT,
        "template-insertion",
    ),
    (
        "void-element",
        WHATWG_VOID_ELEMENT_AUDIT,
        "legacy-void-elements",
    ),
];
const FOREIGN_TEMPLATE_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    ("foreign", WHATWG_FOREIGN_AUDIT, "svg-boundary"),
    (
        "template",
        WHATWG_TEMPLATE_AUDIT,
        "foreign-content-template",
    ),
    (
        "tree-insertion",
        WHATWG_TREE_INSERTION_AUDIT,
        "template-insertion",
    ),
];
const TEMPLATE_CROSS_AXIS_CASES: &[TemplateCrossAxisCase] = &[
    TemplateCrossAxisCase {
        id: "template-dat-473",
        data_snippet: "<select><template></template></select>",
        suites: SELECT_TEMPLATE_CROSS_AXIS_SUITES,
    },
    TemplateCrossAxisCase {
        id: "template-dat-483",
        data_snippet: "<body><table><template><td></tr><div></template></table>",
        suites: TABLE_TEMPLATE_CROSS_AXIS_SUITES,
    },
    TemplateCrossAxisCase {
        id: "template-dat-494",
        data_snippet: "<frameset><template><frame></frame></template></frameset>",
        suites: FRAMESET_TEMPLATE_CROSS_AXIS_SUITES,
    },
    TemplateCrossAxisCase {
        id: "template-dat-553",
        data_snippet: "<template><svg><template>",
        suites: FOREIGN_TEMPLATE_CROSS_AXIS_SUITES,
    },
];

#[derive(Debug, Deserialize)]
struct TemplateAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<TemplateAuditCase>,
}

#[derive(Debug, Deserialize)]
struct TemplateAuditCase {
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
fn whatwg_template_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-template-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 115);
    assert_axis_count(&suite, "template-shell", 4);
    assert_axis_count(&suite, "table-template", 15);
    assert_axis_count(&suite, "select-template", 5);
    assert_axis_count(&suite, "frameset-template", 3);
    assert_axis_count(&suite, "nested-template", 20);
    assert_axis_count(&suite, "eof-template", 15);
    assert_axis_count(&suite, "head-body-template", 8);
    assert_axis_count(&suite, "rawtext-template", 4);
    assert_axis_count(&suite, "foreign-content-template", 2);
    assert_axis_count(&suite, "template-fragment-context", 1);
}

#[test]
fn whatwg_template_audit_cases_match_parser_dom_dump() {
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
fn whatwg_template_audit_keeps_special_context_cases_cross_axis() {
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for evidence in TEMPLATE_CROSS_AXIS_CASES {
        let mut shared_source = None;

        for (suite_name, fixture, expected_axis) in evidence.suites {
            let audit_case = generic_audit_case(fixture, suite_name, evidence.id);
            assert_eq!(
                audit_case.axis, *expected_axis,
                "`{suite_name}` should keep template row `{}` on its focused axis",
                evidence.id
            );
            assert!(
                !audit_case.reason.is_empty(),
                "`{suite_name}` should keep a reason for template row `{}`",
                evidence.id
            );

            if let Some(source) = &shared_source {
                assert_eq!(
                    audit_case.source, *source,
                    "`{suite_name}` should point template row `{}` at the same html5lib row as the other audit axes",
                    evidence.id
                );
            } else {
                shared_source = Some(audit_case.source);
            }
        }

        let shared_source =
            shared_source.expect("template evidence should include at least one suite");
        let source_case = smoke_cases
            .get(&shared_source)
            .unwrap_or_else(|| panic!("case `{shared_source}` should exist in smoke fixture"));
        assert!(
            source_case.data.contains(evidence.data_snippet),
            "template evidence row `{}` should stay tied to its html5lib input",
            evidence.id
        );

        let actual = actual_dom_dump_for_tree_case(source_case)
            .unwrap_or_else(|error| panic!("case `{shared_source}` parse failed: {error}"));
        assert_eq!(
            actual, source_case.document,
            "cross-axis template evidence case `{shared_source}` failed for input {:?}",
            source_case.data
        );
    }
}

#[test]
fn whatwg_template_audit_tracks_post_parse_repair_evidence() {
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

fn load_suite() -> TemplateAuditSuite {
    serde_json::from_str(WHATWG_TEMPLATE_AUDIT).expect("WHATWG template audit fixture should parse")
}

fn generic_audit_case(fixture: &str, suite_name: &str, case_id: &str) -> GenericAuditCase {
    let suite = serde_json::from_str::<GenericAuditSuite>(fixture)
        .unwrap_or_else(|error| panic!("`{suite_name}` audit fixture should parse: {error}"));
    suite
        .cases
        .into_iter()
        .find(|case| case.id == case_id)
        .unwrap_or_else(|| panic!("`{suite_name}` should audit template case `{case_id}`"))
}

fn assert_axis_count(suite: &TemplateAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
