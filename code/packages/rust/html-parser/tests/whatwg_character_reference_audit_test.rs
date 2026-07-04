mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_BLOCK_BOUNDARY_AUDIT: &str = include_str!("fixtures/whatwg-block-boundary-audit.json");
const WHATWG_CHARACTER_REFERENCE_AUDIT: &str =
    include_str!("fixtures/whatwg-character-reference-audit.json");
const WHATWG_DOCUMENT_SHELL_AUDIT: &str = include_str!("fixtures/whatwg-document-shell-audit.json");
const WHATWG_FORM_INTERACTIVE_AUDIT: &str =
    include_str!("fixtures/whatwg-form-interactive-audit.json");
const WHATWG_FORMATTING_AUDIT: &str = include_str!("fixtures/whatwg-formatting-audit.json");
const WHATWG_FRAGMENT_CONTEXT_AUDIT: &str =
    include_str!("fixtures/whatwg-fragment-context-audit.json");
const WHATWG_HEAD_BODY_AUDIT: &str = include_str!("fixtures/whatwg-head-body-audit.json");
const WHATWG_TEXT_CONTROL_AUDIT: &str = include_str!("fixtures/whatwg-text-control-audit.json");
struct CharacterReferenceCrossAxisCase {
    id: &'static str,
    character_reference_axis: &'static str,
    data_snippet: &'static str,
    suites: &'static [(&'static str, &'static str, &'static str)],
}

const ATTRIBUTE_BLOCK_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[(
    "block-boundary",
    WHATWG_BLOCK_BOUNDARY_AUDIT,
    "block-grouping-boundary",
)];
const ATTRIBUTE_FORMATTING_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[(
    "formatting",
    WHATWG_FORMATTING_AUDIT,
    "formatting-reconstruction",
)];
const RCDATA_TITLE_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "implicit-document-shell",
    ),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "head-text-mode"),
    ("text-control", WHATWG_TEXT_CONTROL_AUDIT, "rcdata-controls"),
];
const RCDATA_TEXTAREA_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "textarea-rawtext",
    ),
    ("text-control", WHATWG_TEXT_CONTROL_AUDIT, "rcdata-controls"),
];
const FRAGMENT_CONTEXT_CROSS_AXIS_SUITES: &[(&str, &str, &str)] = &[
    (
        "fragment-context",
        WHATWG_FRAGMENT_CONTEXT_AUDIT,
        "fragment-text-mode-context",
    ),
    (
        "text-control",
        WHATWG_TEXT_CONTROL_AUDIT,
        "fragment-context",
    ),
];
const CHARACTER_REFERENCE_CROSS_AXIS_CASES: &[CharacterReferenceCrossAxisCase] = &[
    CharacterReferenceCrossAxisCase {
        id: "entities02-dat-245",
        character_reference_axis: "character-reference-attribute-boundary",
        data_snippet: r#"<div bar="ZZ&gt;YY"></div>"#,
        suites: ATTRIBUTE_BLOCK_CROSS_AXIS_SUITES,
    },
    CharacterReferenceCrossAxisCase {
        id: "tests2-dat-18",
        character_reference_axis: "character-reference-attribute-boundary",
        data_snippet: "<!DOCTYPE html></b test<b &=&amp>X",
        suites: ATTRIBUTE_FORMATTING_CROSS_AXIS_SUITES,
    },
    CharacterReferenceCrossAxisCase {
        id: "tests6-dat-3",
        character_reference_axis: "character-reference-rcdata-boundary",
        data_snippet: "<!doctype html><title>&amp;</title>",
        suites: RCDATA_TITLE_CROSS_AXIS_SUITES,
    },
    CharacterReferenceCrossAxisCase {
        id: "tests16-dat-860",
        character_reference_axis: "character-reference-rcdata-boundary",
        data_snippet: "<!doctype html><textarea>&lt;/textarea></textarea>",
        suites: RCDATA_TEXTAREA_CROSS_AXIS_SUITES,
    },
    CharacterReferenceCrossAxisCase {
        id: "tests4-dat-4",
        character_reference_axis: "character-reference-fragment-context",
        data_snippet: "this is &#x0043;DATA inside a <style> element",
        suites: FRAGMENT_CONTEXT_CROSS_AXIS_SUITES,
    },
];
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
fn whatwg_character_reference_audit_keeps_character_reference_axis_cases_cross_axis() {
    let suite = load_suite();
    let character_reference_cases = suite
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<HashMap<_, _>>();
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for evidence in CHARACTER_REFERENCE_CROSS_AXIS_CASES {
        let character_reference_case =
            character_reference_cases
                .get(evidence.id)
                .unwrap_or_else(|| {
                    panic!("character-reference audit should include `{}`", evidence.id)
                });
        assert_eq!(
            character_reference_case.axis, evidence.character_reference_axis,
            "character-reference row `{}` should stay on its focused audit axis",
            evidence.id
        );
        assert!(
            !character_reference_case.reason.is_empty(),
            "character-reference row `{}` should keep a fixture reason",
            evidence.id
        );

        for (suite_name, fixture, expected_axis) in evidence.suites {
            let audit_case = generic_audit_case(fixture, suite_name, evidence.id);
            assert_eq!(
                audit_case.source, character_reference_case.source,
                "`{suite_name}` should point character-reference row `{}` at the same html5lib row as the character-reference audit",
                evidence.id
            );
            assert_eq!(
                audit_case.axis, *expected_axis,
                "`{suite_name}` should keep character-reference row `{}` on its focused axis",
                evidence.id
            );
            assert!(
                !audit_case.reason.is_empty(),
                "`{suite_name}` should keep a reason for character-reference row `{}`",
                evidence.id
            );
        }

        let source_case = smoke_cases
            .get(&character_reference_case.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", evidence.id));
        assert!(
            source_case.data.contains(evidence.data_snippet),
            "character-reference row `{}` should stay tied to its html5lib input",
            evidence.id
        );
        let actual = actual_dom_dump_for_tree_case(source_case).unwrap_or_else(|error| {
            panic!(
                "case `{}` ({}) parse failed: {error}",
                character_reference_case.id, character_reference_case.axis
            )
        });

        assert_eq!(
            actual, source_case.document,
            "cross-axis character-reference evidence case `{}` ({}) failed for input {:?}",
            character_reference_case.id, character_reference_case.axis, source_case.data
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

fn generic_audit_case(raw_fixture: &str, suite_name: &str, case_id: &str) -> GenericAuditCase {
    let suite = serde_json::from_str::<GenericAuditSuite>(raw_fixture)
        .unwrap_or_else(|error| panic!("{suite_name} audit fixture should parse: {error}"));
    suite
        .cases
        .into_iter()
        .find(|case| case.id == case_id)
        .unwrap_or_else(|| panic!("{suite_name} audit should include `{case_id}`"))
}
