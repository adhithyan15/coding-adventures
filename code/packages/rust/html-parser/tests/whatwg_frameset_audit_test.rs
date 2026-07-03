mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_DOCUMENT_SHELL_AUDIT: &str = include_str!("fixtures/whatwg-document-shell-audit.json");
const WHATWG_FOREIGN_AUDIT: &str = include_str!("fixtures/whatwg-foreign-audit.json");
const WHATWG_FORM_INTERACTIVE_AUDIT: &str =
    include_str!("fixtures/whatwg-form-interactive-audit.json");
const WHATWG_FORMATTING_AUDIT: &str = include_str!("fixtures/whatwg-formatting-audit.json");
const WHATWG_FRAMESET_AUDIT: &str = include_str!("fixtures/whatwg-frameset-audit.json");
const WHATWG_HEAD_BODY_AUDIT: &str = include_str!("fixtures/whatwg-head-body-audit.json");
const WHATWG_LEGACY_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-legacy-element-audit.json");
const WHATWG_NOSCRIPT_AUDIT: &str = include_str!("fixtures/whatwg-noscript-audit.json");
const WHATWG_PARAGRAPH_AUDIT: &str = include_str!("fixtures/whatwg-paragraph-audit.json");
const WHATWG_TEMPLATE_AUDIT: &str = include_str!("fixtures/whatwg-template-audit.json");
const WHATWG_TEXT_CONTROL_AUDIT: &str = include_str!("fixtures/whatwg-text-control-audit.json");
const WHATWG_TREE_INSERTION_AUDIT: &str = include_str!("fixtures/whatwg-tree-insertion-audit.json");
const WHATWG_VOID_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-void-element-audit.json");

struct FramesetCrossAxisCase {
    id: &'static str,
    frameset_axis: &'static str,
    data_snippet: &'static str,
    suites: &'static [(&'static str, &'static str, &'static str)],
}

const FRAMESET_SHELL_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "html-element-boundary",
    ),
    (
        "head-body",
        WHATWG_HEAD_BODY_AUDIT,
        "body-frameset-transition",
    ),
];
const FRAME_ELEMENT_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "body-frameset-boundary",
    ),
    ("formatting", WHATWG_FORMATTING_AUDIT, "paragraph-boundary"),
    (
        "head-body",
        WHATWG_HEAD_BODY_AUDIT,
        "body-frameset-transition",
    ),
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-basic-boundary",
    ),
    (
        "void-element",
        WHATWG_VOID_ELEMENT_AUDIT,
        "legacy-void-elements",
    ),
];
const NOFRAMES_CONTENT_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "head-element-boundary",
    ),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "head-boundary"),
    ("noscript", WHATWG_NOSCRIPT_AUDIT, "head-noscript-disabled"),
    (
        "text-control",
        WHATWG_TEXT_CONTROL_AUDIT,
        "noscript-scripting",
    ),
];
const BODY_COMPATIBILITY_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
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
    (
        "head-body",
        WHATWG_HEAD_BODY_AUDIT,
        "body-frameset-transition",
    ),
    (
        "legacy-element",
        WHATWG_LEGACY_ELEMENT_AUDIT,
        "pending-spec-boundary",
    ),
    (
        "void-element",
        WHATWG_VOID_ELEMENT_AUDIT,
        "body-void-elements",
    ),
];
const FOREIGN_BOUNDARY_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "body-frameset-boundary",
    ),
    ("foreign", WHATWG_FOREIGN_AUDIT, "svg-boundary"),
    (
        "head-body",
        WHATWG_HEAD_BODY_AUDIT,
        "body-frameset-transition",
    ),
];
const FRAMESET_TEMPLATE_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "head-element-boundary",
    ),
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
const FRAMESET_CROSS_AXIS_CASES: &[FramesetCrossAxisCase] = &[
    FramesetCrossAxisCase {
        id: "domjs-unsafe-dat-160",
        frameset_axis: "frameset-shell",
        data_snippet: "<frameset><html></frameset>",
        suites: FRAMESET_SHELL_CROSS_AXIS_SUITES,
    },
    FramesetCrossAxisCase {
        id: "tests19-dat-1059",
        frameset_axis: "frame-element",
        data_snippet: "<!doctype html><p><frameset><frame>",
        suites: FRAME_ELEMENT_CROSS_AXIS_SUITES,
    },
    FramesetCrossAxisCase {
        id: "noscript01-dat-336",
        frameset_axis: "noframes-content",
        data_snippet: "<head><noscript><noframes>XXX</noscript></noframes></noscript>",
        suites: NOFRAMES_CONTENT_CROSS_AXIS_SUITES,
    },
    FramesetCrossAxisCase {
        id: "pending-spec-changes-dat-346",
        frameset_axis: "body-compatibility",
        data_snippet: "<input type=\"hidden\"><frameset>",
        suites: BODY_COMPATIBILITY_CROSS_AXIS_SUITES,
    },
    FramesetCrossAxisCase {
        id: "plain-text-unsafe-dat-364",
        frameset_axis: "foreign-boundary",
        data_snippet: "<svg>\0<frameset>",
        suites: FOREIGN_BOUNDARY_CROSS_AXIS_SUITES,
    },
    FramesetCrossAxisCase {
        id: "template-dat-494",
        frameset_axis: "template-boundary",
        data_snippet: "<frameset><template><frame></frame></template></frameset>",
        suites: FRAMESET_TEMPLATE_CROSS_AXIS_SUITES,
    },
];
const POST_PARSE_REPAIR_EVIDENCE: &[(&str, &str)] = &[
    ("domjs-unsafe-dat-160", "frameset-shell"),
    ("tests19-dat-1059", "frame-element"),
    ("noscript01-dat-336", "noframes-content"),
    ("pending-spec-changes-dat-346", "body-compatibility"),
    ("plain-text-unsafe-dat-364", "foreign-boundary"),
    ("template-dat-494", "template-boundary"),
];

#[derive(Debug, Deserialize)]
struct FramesetAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<FramesetAuditCase>,
}

#[derive(Debug, Deserialize)]
struct FramesetAuditCase {
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
fn whatwg_frameset_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-frameset-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 180);
    assert_axis_count(&suite, "frameset-shell", 70);
    assert_axis_count(&suite, "frame-element", 8);
    assert_axis_count(&suite, "noframes-content", 25);
    assert_axis_count(&suite, "body-compatibility", 30);
    assert_axis_count(&suite, "foreign-boundary", 20);
    assert_axis_count(&suite, "template-boundary", 5);
}

#[test]
fn whatwg_frameset_audit_cases_match_parser_dom_dump() {
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
fn whatwg_frameset_audit_keeps_shell_boundary_cases_cross_axis() {
    let suite = load_suite();
    let frameset_cases = suite
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<HashMap<_, _>>();
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for evidence in FRAMESET_CROSS_AXIS_CASES {
        let frameset_case = frameset_cases
            .get(evidence.id)
            .unwrap_or_else(|| panic!("frameset audit should include `{}`", evidence.id));
        assert_eq!(
            frameset_case.axis, evidence.frameset_axis,
            "frameset row `{}` should stay on its focused audit axis",
            evidence.id
        );
        assert!(
            !frameset_case.reason.is_empty(),
            "frameset row `{}` should keep a fixture reason",
            evidence.id
        );

        for (suite_name, fixture, expected_axis) in evidence.suites {
            let audit_case = generic_audit_case(fixture, suite_name, evidence.id);
            assert_eq!(
                audit_case.source, frameset_case.source,
                "`{suite_name}` should point frameset row `{}` at the same html5lib row as the frameset audit",
                evidence.id
            );
            assert_eq!(
                audit_case.axis, *expected_axis,
                "`{suite_name}` should keep frameset row `{}` on its focused axis",
                evidence.id
            );
            assert!(
                !audit_case.reason.is_empty(),
                "`{suite_name}` should keep a reason for frameset row `{}`",
                evidence.id
            );
        }

        let source_case = smoke_cases
            .get(&frameset_case.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", evidence.id));
        assert!(
            source_case.data.contains(evidence.data_snippet),
            "frameset row `{}` should stay tied to its html5lib input",
            evidence.id
        );
        let actual = actual_dom_dump_for_tree_case(source_case).unwrap_or_else(|error| {
            panic!(
                "case `{}` ({}) parse failed: {error}",
                frameset_case.id, frameset_case.axis
            )
        });

        assert_eq!(
            actual, source_case.document,
            "cross-axis frameset evidence case `{}` ({}) failed for input {:?}",
            frameset_case.id, frameset_case.axis, source_case.data
        );
    }
}

#[test]
fn whatwg_frameset_audit_tracks_post_parse_repair_evidence() {
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

fn load_suite() -> FramesetAuditSuite {
    serde_json::from_str(WHATWG_FRAMESET_AUDIT).expect("WHATWG frameset audit fixture should parse")
}

fn generic_audit_case(fixture: &str, suite_name: &str, case_id: &str) -> GenericAuditCase {
    let suite = serde_json::from_str::<GenericAuditSuite>(fixture)
        .unwrap_or_else(|error| panic!("`{suite_name}` audit fixture should parse: {error}"));
    suite
        .cases
        .into_iter()
        .find(|case| case.id == case_id)
        .unwrap_or_else(|| panic!("`{suite_name}` should audit frameset case `{case_id}`"))
}

fn assert_axis_count(suite: &FramesetAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
