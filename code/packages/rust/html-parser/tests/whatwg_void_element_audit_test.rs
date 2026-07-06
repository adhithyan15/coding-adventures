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
const WHATWG_VOID_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-void-element-audit.json");

struct VoidElementCrossAxisCase {
    id: &'static str,
    data_snippet: &'static str,
    suites: &'static [(&'static str, &'static str, &'static str)],
}

struct VoidElementRepairEvidence {
    id: &'static str,
    source: &'static str,
    axis: &'static str,
    data_snippet: &'static str,
    fragment_context: Option<&'static str>,
}

const POST_PARSE_REPAIR_EVIDENCE: &[VoidElementRepairEvidence] = &[
    VoidElementRepairEvidence {
        id: "adoption01-dat-13",
        source: "adoption01.dat:13",
        axis: "void-in-table",
        data_snippet: "<a><svg><tr><input></a>",
        fragment_context: None,
    },
    VoidElementRepairEvidence {
        id: "noscript01-dat-332",
        source: "noscript01.dat:332",
        axis: "metadata-void-elements",
        data_snippet: "<head><noscript><basefont><!--foo--></noscript>",
        fragment_context: None,
    },
    VoidElementRepairEvidence {
        id: "menuitem-element-dat-311",
        source: "menuitem-element.dat:311",
        axis: "body-void-elements",
        data_snippet: "<!DOCTYPE html><body><menuitem>A<hr>B",
        fragment_context: None,
    },
    VoidElementRepairEvidence {
        id: "noscript01-dat-338",
        source: "noscript01.dat:338",
        axis: "stray-void-end-tags",
        data_snippet: "<head><noscript></br><!--foo--></noscript>",
        fragment_context: None,
    },
    VoidElementRepairEvidence {
        id: "tests19-dat-1086",
        source: "tests19.dat:1086",
        axis: "void-foreign-boundary",
        data_snippet: "<!doctype html><svg></svg><frameset><frame>",
        fragment_context: None,
    },
    VoidElementRepairEvidence {
        id: "tests7-dat-17",
        source: "tests7.dat:17",
        axis: "void-in-select",
        data_snippet: "<!doctype html><select><input>X",
        fragment_context: None,
    },
    VoidElementRepairEvidence {
        id: "tests6-dat-27",
        source: "tests6.dat:27",
        axis: "void-fragment-context",
        data_snippet: "foo<col>",
        fragment_context: Some("colgroup"),
    },
    VoidElementRepairEvidence {
        id: "template-dat-494",
        source: "template.dat:494",
        axis: "legacy-void-elements",
        data_snippet: "<frameset><template><frame></frame></template></frameset>",
        fragment_context: None,
    },
];
const VOID_IN_SELECT_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "implicit-document-shell",
    ),
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "select-option",
    ),
    ("select-list", WHATWG_SELECT_LIST_AUDIT, "select-shell"),
    ("void-element", WHATWG_VOID_ELEMENT_AUDIT, "void-in-select"),
];
const VOID_IN_TABLE_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-table-boundary",
    ),
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "select-option",
    ),
    ("select-list", WHATWG_SELECT_LIST_AUDIT, "select-in-table"),
    ("table", WHATWG_TABLE_AUDIT, "select-in-table"),
    ("void-element", WHATWG_VOID_ELEMENT_AUDIT, "void-in-table"),
];
const VOID_FOREIGN_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "body-frameset-boundary",
    ),
    ("foreign", WHATWG_FOREIGN_AUDIT, "svg-boundary"),
    ("frameset", WHATWG_FRAMESET_AUDIT, "foreign-boundary"),
    (
        "head-body",
        WHATWG_HEAD_BODY_AUDIT,
        "body-frameset-transition",
    ),
    (
        "void-element",
        WHATWG_VOID_ELEMENT_AUDIT,
        "void-foreign-boundary",
    ),
];
const VOID_ELEMENT_CROSS_AXIS_CASES: &[VoidElementCrossAxisCase] = &[
    VoidElementCrossAxisCase {
        id: "tests7-dat-17",
        data_snippet: "<!doctype html><select><input>X",
        suites: VOID_IN_SELECT_CROSS_AXIS_SUITES,
    },
    VoidElementCrossAxisCase {
        id: "webkit01-dat-38",
        data_snippet: "<kbd><table></kbd><col><select><tr></table><div>",
        suites: VOID_IN_TABLE_CROSS_AXIS_SUITES,
    },
    VoidElementCrossAxisCase {
        id: "tests19-dat-1086",
        data_snippet: "<!doctype html><svg></svg><frameset><frame>",
        suites: VOID_FOREIGN_CROSS_AXIS_SUITES,
    },
];

#[derive(Debug, Deserialize)]
struct VoidElementAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<VoidElementAuditCase>,
}

#[derive(Debug, Deserialize)]
struct VoidElementAuditCase {
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
fn whatwg_void_element_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-void-element-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 225);
    assert_axis_count(&suite, "void-in-table", 50);
    assert_axis_count(&suite, "metadata-void-elements", 50);
    assert_axis_count(&suite, "body-void-elements", 60);
    assert_axis_count(&suite, "stray-void-end-tags", 5);
    assert_axis_count(&suite, "void-foreign-boundary", 14);
    assert_axis_count(&suite, "void-in-select", 8);
    assert_axis_count(&suite, "void-fragment-context", 15);
    assert_axis_count(&suite, "legacy-void-elements", 15);
}

#[test]
fn whatwg_void_element_audit_cases_match_parser_dom_dump() {
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
fn whatwg_void_element_audit_keeps_special_context_cases_cross_axis() {
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for evidence in VOID_ELEMENT_CROSS_AXIS_CASES {
        let mut shared_source = None;

        for (suite_name, fixture, expected_axis) in evidence.suites {
            let audit_case = generic_audit_case(fixture, suite_name, evidence.id);
            assert_eq!(
                audit_case.axis, *expected_axis,
                "`{suite_name}` should keep void-element row `{}` on its focused axis",
                evidence.id
            );
            assert!(
                !audit_case.reason.is_empty(),
                "`{suite_name}` should keep a reason for void-element row `{}`",
                evidence.id
            );

            if let Some(source) = &shared_source {
                assert_eq!(
                    audit_case.source, *source,
                    "`{suite_name}` should point void-element row `{}` at the same html5lib row as the other audit axes",
                    evidence.id
                );
            } else {
                shared_source = Some(audit_case.source);
            }
        }

        let shared_source =
            shared_source.expect("void-element evidence should include at least one suite");
        let source_case = smoke_cases
            .get(&shared_source)
            .unwrap_or_else(|| panic!("case `{shared_source}` should exist in smoke fixture"));
        assert!(
            source_case.data.contains(evidence.data_snippet),
            "void-element evidence row `{}` should stay tied to its html5lib input",
            evidence.id
        );

        let actual = actual_dom_dump_for_tree_case(source_case)
            .unwrap_or_else(|error| panic!("case `{shared_source}` parse failed: {error}"));
        assert_eq!(
            actual, source_case.document,
            "cross-axis void-element evidence case `{shared_source}` failed for input {:?}",
            source_case.data
        );
    }
}

#[test]
fn whatwg_void_element_audit_tracks_post_parse_repair_evidence() {
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

    for evidence in POST_PARSE_REPAIR_EVIDENCE {
        let audit_case = audit_cases.get(evidence.id).unwrap_or_else(|| {
            panic!(
                "post-parse repair evidence case `{}` should be audited",
                evidence.id
            )
        });
        assert_eq!(
            audit_case.source, evidence.source,
            "post-parse repair evidence case `{}` should stay tied to its smoke fixture row",
            evidence.id
        );
        assert_eq!(
            audit_case.axis, evidence.axis,
            "post-parse repair evidence case `{}` should stay on its focused audit axis",
            evidence.id
        );

        let source_case = smoke_cases
            .get(evidence.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", evidence.id));
        assert!(
            source_case.data.contains(evidence.data_snippet),
            "post-parse repair evidence row `{}` should stay tied to its html5lib input",
            evidence.id
        );
        if let Some(fragment_context) = evidence.fragment_context {
            assert_eq!(
                source_case.fragment_context.as_deref(),
                Some(fragment_context),
                "post-parse repair evidence row `{}` should keep its html5lib fragment context",
                evidence.id
            );
        }
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

fn load_suite() -> VoidElementAuditSuite {
    serde_json::from_str(WHATWG_VOID_ELEMENT_AUDIT)
        .expect("WHATWG void-element audit fixture should parse")
}

fn generic_audit_case(fixture: &str, suite_name: &str, case_id: &str) -> GenericAuditCase {
    let suite = serde_json::from_str::<GenericAuditSuite>(fixture)
        .unwrap_or_else(|error| panic!("`{suite_name}` audit fixture should parse: {error}"));
    suite
        .cases
        .into_iter()
        .find(|case| case.id == case_id)
        .unwrap_or_else(|| panic!("`{suite_name}` should audit void-element case `{case_id}`"))
}

fn assert_axis_count(suite: &VoidElementAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
