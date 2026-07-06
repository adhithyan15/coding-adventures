mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_BLOCK_BOUNDARY_AUDIT: &str = include_str!("fixtures/whatwg-block-boundary-audit.json");
const WHATWG_DOCUMENT_SHELL_AUDIT: &str = include_str!("fixtures/whatwg-document-shell-audit.json");
const WHATWG_FORMATTING_AUDIT: &str = include_str!("fixtures/whatwg-formatting-audit.json");
const WHATWG_HEAD_BODY_AUDIT: &str = include_str!("fixtures/whatwg-head-body-audit.json");
const WHATWG_PARAGRAPH_AUDIT: &str = include_str!("fixtures/whatwg-paragraph-audit.json");
const WHATWG_RUBY_AUDIT: &str = include_str!("fixtures/whatwg-ruby-audit.json");
struct RubyCrossAxisCase {
    id: &'static str,
    ruby_axis: &'static str,
    data_snippet: &'static str,
    suites: &'static [(&'static str, &'static str, &'static str)],
}

struct RubyRepairEvidence {
    id: &'static str,
    source: &'static str,
    axis: &'static str,
    data_snippet: &'static str,
}

const RUBY_SHELL_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "html-element-boundary",
    ),
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "ruby-implied-end-tags",
    ),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "html-boundary"),
];
const RUBY_BLOCK_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-ruby-boundary",
    ),
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "ruby-implied-end-tags",
    ),
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-block-boundary",
    ),
];
const RUBY_CROSS_AXIS_CASES: &[RubyCrossAxisCase] = &[
    RubyCrossAxisCase {
        id: "ruby-dat-386",
        ruby_axis: "rb-boundary",
        data_snippet: "<html><ruby>a<rb>b<rb></ruby></html>",
        suites: RUBY_SHELL_CROSS_AXIS_SUITES,
    },
    RubyCrossAxisCase {
        id: "ruby-dat-387",
        ruby_axis: "rt-boundary",
        data_snippet: "<html><ruby>a<rb>b<rt></ruby></html>",
        suites: RUBY_SHELL_CROSS_AXIS_SUITES,
    },
    RubyCrossAxisCase {
        id: "ruby-dat-388",
        ruby_axis: "rtc-boundary",
        data_snippet: "<html><ruby>a<rb>b<rtc></ruby></html>",
        suites: RUBY_SHELL_CROSS_AXIS_SUITES,
    },
    RubyCrossAxisCase {
        id: "ruby-dat-389",
        ruby_axis: "rp-boundary",
        data_snippet: "<html><ruby>a<rb>b<rp></ruby></html>",
        suites: RUBY_SHELL_CROSS_AXIS_SUITES,
    },
    RubyCrossAxisCase {
        id: "ruby-dat-406",
        ruby_axis: "nested-ruby",
        data_snippet: "<html><ruby><rtc><ruby>a<rb>b<rt></ruby></ruby></html>",
        suites: RUBY_SHELL_CROSS_AXIS_SUITES,
    },
    RubyCrossAxisCase {
        id: "tests19-dat-1024",
        ruby_axis: "block-in-ruby",
        data_snippet: "<!doctype html><ruby><div><p><rp>",
        suites: RUBY_BLOCK_CROSS_AXIS_SUITES,
    },
];
const POST_PARSE_REPAIR_EVIDENCE: &[RubyRepairEvidence] = &[RubyRepairEvidence {
    id: "ruby-dat-387",
    source: "ruby.dat:387",
    axis: "rt-boundary",
    data_snippet: "<html><ruby>a<rb>b<rt></ruby></html>",
}];

#[derive(Debug, Deserialize)]
struct RubyAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<RubyAuditCase>,
}

#[derive(Debug, Deserialize)]
struct RubyAuditCase {
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
fn whatwg_ruby_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-ruby-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 35);
    assert_axis_count(&suite, "rb-boundary", 1);
    assert_axis_count(&suite, "rt-boundary", 4);
    assert_axis_count(&suite, "rtc-boundary", 6);
    assert_axis_count(&suite, "rp-boundary", 4);
    assert_axis_count(&suite, "block-in-ruby", 8);
    assert_axis_count(&suite, "nested-ruby", 1);
}

#[test]
fn whatwg_ruby_audit_cases_match_parser_dom_dump() {
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
fn whatwg_ruby_audit_keeps_ruby_axis_cases_cross_axis() {
    let suite = load_suite();
    let ruby_cases = suite
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<HashMap<_, _>>();
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for evidence in RUBY_CROSS_AXIS_CASES {
        let ruby_case = ruby_cases
            .get(evidence.id)
            .unwrap_or_else(|| panic!("ruby audit should include `{}`", evidence.id));
        assert_eq!(
            ruby_case.axis, evidence.ruby_axis,
            "ruby row `{}` should stay on its focused audit axis",
            evidence.id
        );
        assert!(
            !ruby_case.reason.is_empty(),
            "ruby row `{}` should keep a fixture reason",
            evidence.id
        );

        for (suite_name, fixture, expected_axis) in evidence.suites {
            let audit_case = generic_audit_case(fixture, suite_name, evidence.id);
            assert_eq!(
                audit_case.source, ruby_case.source,
                "`{suite_name}` should point ruby row `{}` at the same html5lib row as the ruby audit",
                evidence.id
            );
            assert_eq!(
                audit_case.axis, *expected_axis,
                "`{suite_name}` should keep ruby row `{}` on its focused axis",
                evidence.id
            );
            assert!(
                !audit_case.reason.is_empty(),
                "`{suite_name}` should keep a reason for ruby row `{}`",
                evidence.id
            );
        }

        let source_case = smoke_cases
            .get(&ruby_case.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", evidence.id));
        assert!(
            source_case.data.contains(evidence.data_snippet),
            "ruby row `{}` should stay tied to its html5lib input",
            evidence.id
        );

        let actual = actual_dom_dump_for_tree_case(source_case).unwrap_or_else(|error| {
            panic!(
                "case `{}` ({}) parse failed: {error}",
                ruby_case.id, ruby_case.axis
            )
        });

        assert_eq!(
            actual, source_case.document,
            "cross-axis ruby evidence case `{}` ({}) failed for input {:?}",
            ruby_case.id, ruby_case.axis, source_case.data
        );
    }
}

#[test]
fn whatwg_ruby_audit_tracks_post_parse_repair_evidence() {
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

fn load_suite() -> RubyAuditSuite {
    serde_json::from_str(WHATWG_RUBY_AUDIT).expect("WHATWG ruby audit fixture should parse")
}

fn assert_axis_count(suite: &RubyAuditSuite, axis: &str, minimum: usize) {
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
