mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_DOCUMENT_SHELL_AUDIT: &str = include_str!("fixtures/whatwg-document-shell-audit.json");
const WHATWG_FOREIGN_AUDIT: &str = include_str!("fixtures/whatwg-foreign-audit.json");
const WHATWG_FORM_INTERACTIVE_AUDIT: &str =
    include_str!("fixtures/whatwg-form-interactive-audit.json");
const WHATWG_FRAMESET_AUDIT: &str = include_str!("fixtures/whatwg-frameset-audit.json");
const WHATWG_HEAD_BODY_AUDIT: &str = include_str!("fixtures/whatwg-head-body-audit.json");
const WHATWG_LEGACY_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-legacy-element-audit.json");
const WHATWG_TABLE_AUDIT: &str = include_str!("fixtures/whatwg-table-audit.json");
const WHATWG_VOID_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-void-element-audit.json");
const POST_PARSE_REPAIR_EVIDENCE: &[(&str, &str)] = &[
    ("tricky01-dat-3", "tricky-parser-recovery"),
    ("tricky01-dat-8", "tricky-parser-recovery"),
    ("tricky01-dat-9", "tricky-parser-recovery"),
];
const PENDING_SPEC_BOUNDARY_EVIDENCE: &[PendingSpecBoundaryCase] = &[
    PendingSpecBoundaryCase {
        id: "pending-spec-changes-plain-text-unsafe-dat-345",
        source: "pending-spec-changes-plain-text-unsafe.dat:345",
        data_snippet: "<body><table>\0filler\0text\0",
    },
    PendingSpecBoundaryCase {
        id: "pending-spec-changes-dat-346",
        source: "pending-spec-changes.dat:346",
        data_snippet: "<input type=\"hidden\"><frameset>",
    },
    PendingSpecBoundaryCase {
        id: "pending-spec-changes-dat-347",
        source: "pending-spec-changes.dat:347",
        data_snippet: "<!DOCTYPE html><table><caption><svg>foo</table>bar",
    },
    PendingSpecBoundaryCase {
        id: "pending-spec-changes-dat-348",
        source: "pending-spec-changes.dat:348",
        data_snippet: "<table><tr><td><svg><desc><td></desc><circle>",
    },
];
const PENDING_SPEC_CROSS_AXIS_CASES: &[PendingSpecCrossAxisCase] = &[
    PendingSpecCrossAxisCase {
        id: "pending-spec-changes-plain-text-unsafe-dat-345",
        suites: &[
            (
                "document-shell",
                WHATWG_DOCUMENT_SHELL_AUDIT,
                "body-frameset-boundary",
            ),
            ("head-body", WHATWG_HEAD_BODY_AUDIT, "body-boundary"),
            ("table", WHATWG_TABLE_AUDIT, "foster-parenting"),
        ],
    },
    PendingSpecCrossAxisCase {
        id: "pending-spec-changes-dat-346",
        suites: &[
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
                "head-body",
                WHATWG_HEAD_BODY_AUDIT,
                "body-frameset-transition",
            ),
            (
                "void-element",
                WHATWG_VOID_ELEMENT_AUDIT,
                "body-void-elements",
            ),
        ],
    },
    PendingSpecCrossAxisCase {
        id: "pending-spec-changes-dat-347",
        suites: &[
            ("foreign", WHATWG_FOREIGN_AUDIT, "table-foreign-boundary"),
            ("table", WHATWG_TABLE_AUDIT, "caption-colgroup"),
        ],
    },
    PendingSpecCrossAxisCase {
        id: "pending-spec-changes-dat-348",
        suites: &[
            ("foreign", WHATWG_FOREIGN_AUDIT, "html-integration-point"),
            ("table", WHATWG_TABLE_AUDIT, "cell-boundary"),
        ],
    },
];

struct PendingSpecBoundaryCase {
    id: &'static str,
    source: &'static str,
    data_snippet: &'static str,
}

struct PendingSpecCrossAxisCase {
    id: &'static str,
    suites: &'static [(&'static str, &'static str, &'static str)],
}

#[derive(Debug, Deserialize)]
struct LegacyElementAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<LegacyElementAuditCase>,
}

#[derive(Debug, Deserialize)]
struct LegacyElementAuditCase {
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
fn whatwg_legacy_element_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-legacy-element-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 44);
    assert_axis_count(&suite, "legacy-isindex", 4);
    assert_axis_count(&suite, "obsolete-menuitem", 20);
    assert_axis_count(&suite, "main-element-boundary", 3);
    assert_axis_count(&suite, "search-element-boundary", 3);
    assert_axis_count(&suite, "pending-spec-boundary", 4);
    assert_axis_count(&suite, "tricky-parser-recovery", 9);
    assert_axis_count(&suite, "namespace-sensitivity", 1);
}

#[test]
fn whatwg_legacy_element_audit_cases_match_parser_dom_dump() {
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
fn whatwg_legacy_element_audit_tracks_post_parse_repair_evidence() {
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

#[test]
fn whatwg_legacy_element_audit_tracks_pending_spec_boundary_evidence() {
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

    for evidence in PENDING_SPEC_BOUNDARY_EVIDENCE {
        let audit_case = audit_cases.get(evidence.id).unwrap_or_else(|| {
            panic!(
                "pending spec boundary evidence case `{}` should be audited",
                evidence.id
            )
        });
        assert_eq!(
            audit_case.source, evidence.source,
            "pending spec boundary case `{}` should stay tied to its WHATWG source row",
            evidence.id
        );
        assert_eq!(
            audit_case.axis, "pending-spec-boundary",
            "pending spec boundary case `{}` should stay on its focused audit axis",
            evidence.id
        );
        assert!(
            !audit_case.reason.is_empty(),
            "pending spec boundary case `{}` should keep a fixture reason",
            evidence.id
        );

        let source_case = smoke_cases
            .get(evidence.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", evidence.source));
        assert!(
            source_case.data.contains(evidence.data_snippet),
            "pending spec boundary case `{}` should stay tied to its html5lib input",
            evidence.id
        );
        let actual = actual_dom_dump_for_tree_case(source_case).unwrap_or_else(|error| {
            panic!(
                "case `{}` ({}) parse failed: {error}",
                audit_case.id, audit_case.axis
            )
        });

        assert_eq!(
            actual, source_case.document,
            "pending spec boundary evidence case `{}` ({}) failed for input {:?}",
            audit_case.id, audit_case.axis, source_case.data
        );
    }
}

#[test]
fn whatwg_legacy_element_audit_keeps_pending_spec_cases_cross_axis() {
    let legacy_suite = load_suite();

    for cross_axis_case in PENDING_SPEC_CROSS_AXIS_CASES {
        let legacy_case = legacy_suite
            .cases
            .iter()
            .find(|case| case.id == cross_axis_case.id)
            .unwrap_or_else(|| panic!("legacy audit should include `{}`", cross_axis_case.id));

        assert_eq!(
            legacy_case.axis, "pending-spec-boundary",
            "legacy audit should keep `{}` on its pending spec axis",
            cross_axis_case.id
        );

        for (suite_name, fixture, expected_axis) in cross_axis_case.suites {
            let audit_case = generic_audit_case(fixture, suite_name, cross_axis_case.id);
            assert_eq!(
                audit_case.axis, *expected_axis,
                "`{suite_name}` should keep pending spec row `{}` on its focused axis",
                cross_axis_case.id
            );
            assert_eq!(
                legacy_case.source, audit_case.source,
                "`{suite_name}` should point pending spec row `{}` at the same WHATWG source row",
                cross_axis_case.id
            );
            assert!(
                !legacy_case.reason.is_empty() && !audit_case.reason.is_empty(),
                "cross-axis pending spec case `{}` should keep fixture reasons",
                cross_axis_case.id
            );
        }
    }
}

fn load_suite() -> LegacyElementAuditSuite {
    serde_json::from_str(WHATWG_LEGACY_ELEMENT_AUDIT)
        .expect("WHATWG legacy/edge element audit fixture should parse")
}

fn generic_audit_case(fixture: &str, suite_name: &str, case_id: &str) -> GenericAuditCase {
    let suite = serde_json::from_str::<GenericAuditSuite>(fixture)
        .unwrap_or_else(|error| panic!("`{suite_name}` audit fixture should parse: {error}"));
    suite
        .cases
        .into_iter()
        .find(|case| case.id == case_id)
        .unwrap_or_else(|| panic!("`{suite_name}` should audit pending spec case `{case_id}`"))
}

fn assert_axis_count(suite: &LegacyElementAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
