mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_CHARACTER_REFERENCE_AUDIT: &str =
    include_str!("fixtures/whatwg-character-reference-audit.json");
const CHARACTER_REFERENCE_EVIDENCE: &[(&str, &str, &str)] = &[
    (
        "entities01-dat-170",
        "entities01.dat:170",
        "character-reference-named-boundary",
    ),
    (
        "entities01-dat-178",
        "entities01.dat:178",
        "character-reference-ambiguous-ampersand",
    ),
    (
        "entities01-dat-181",
        "entities01.dat:181",
        "character-reference-numeric-boundary",
    ),
    (
        "entities02-dat-245",
        "entities02.dat:245",
        "character-reference-attribute-boundary",
    ),
    (
        "tests16-dat-849",
        "tests16.dat:849",
        "character-reference-rcdata-boundary",
    ),
    (
        "tests4-dat-4",
        "tests4.dat:4",
        "character-reference-fragment-context",
    ),
];

#[derive(Debug, Deserialize)]
struct CharacterReferenceAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<CharacterReferenceAuditCase>,
}

#[derive(Debug, Deserialize)]
struct CharacterReferenceAuditCase {
    id: String,
    source: String,
    axis: String,
    reason: String,
}

#[test]
fn whatwg_character_reference_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-character-reference-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 150);
    assert_axis_count(&suite, "character-reference-numeric-boundary", 60);
    assert_axis_count(&suite, "character-reference-named-boundary", 40);
    assert_axis_count(&suite, "character-reference-attribute-boundary", 20);
    assert_axis_count(&suite, "character-reference-rcdata-boundary", 16);
    assert_axis_count(&suite, "character-reference-ambiguous-ampersand", 10);
    assert_axis_count(&suite, "character-reference-fragment-context", 1);
}

#[test]
fn whatwg_character_reference_audit_cases_match_parser_dom_dump() {
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
fn whatwg_character_reference_audit_tracks_executable_evidence() {
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

    for (case_id, expected_source, expected_axis) in CHARACTER_REFERENCE_EVIDENCE {
        let audit_case = audit_cases.get(case_id).unwrap_or_else(|| {
            panic!("character-reference evidence case `{case_id}` should be audited")
        });
        assert_eq!(
            audit_case.source, *expected_source,
            "character-reference evidence case `{case_id}` should stay tied to its smoke fixture row"
        );
        assert_eq!(
            audit_case.axis, *expected_axis,
            "character-reference evidence case `{case_id}` should stay on its focused audit axis"
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
            "character-reference evidence case `{}` ({}) failed for input {:?}",
            audit_case.id, audit_case.axis, source_case.data
        );
    }
}

fn load_suite() -> CharacterReferenceAuditSuite {
    serde_json::from_str(WHATWG_CHARACTER_REFERENCE_AUDIT)
        .expect("WHATWG character-reference audit fixture should parse")
}

fn assert_axis_count(suite: &CharacterReferenceAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
