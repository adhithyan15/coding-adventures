mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_PROCESSING_INSTRUCTION_AUDIT: &str =
    include_str!("fixtures/whatwg-processing-instruction-audit.json");
const EXECUTABLE_EVIDENCE: &[ProcessingInstructionEvidence] = &[
    ProcessingInstructionEvidence {
        source: "processing-instructions.dat:1",
        axis: "valid-target-and-data",
        data_snippet: "<?something>",
    },
    ProcessingInstructionEvidence {
        source: "processing-instructions.dat:66",
        axis: "invalid-target-recovery",
        data_snippet: "<?xml>",
    },
    ProcessingInstructionEvidence {
        source: "processing-instructions.dat:101",
        axis: "eof-recovery",
        data_snippet: "<?start",
    },
    ProcessingInstructionEvidence {
        source: "processing-instructions.dat:110",
        axis: "insertion-context",
        data_snippet: "<table><?something>",
    },
];

struct ProcessingInstructionEvidence {
    source: &'static str,
    axis: &'static str,
    data_snippet: &'static str,
}

#[derive(Debug, Deserialize)]
struct ProcessingInstructionAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<ProcessingInstructionAuditCase>,
}

#[derive(Debug, Deserialize)]
struct ProcessingInstructionAuditCase {
    id: String,
    source: String,
    axis: String,
    reason: String,
}

#[test]
fn whatwg_processing_instruction_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(
        suite.format,
        "whatwg-html-processing-instruction-audit/v1"
    );
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert_eq!(suite.case_count, 124);
    assert_axis_count(&suite, "valid-target-and-data", 65);
    assert_axis_count(&suite, "invalid-target-recovery", 35);
    assert_axis_count(&suite, "eof-recovery", 7);
    assert_axis_count(&suite, "insertion-context", 17);
}

#[test]
fn whatwg_processing_instruction_audit_cases_match_parser_dom_dump() {
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
fn whatwg_processing_instruction_audit_tracks_executable_evidence() {
    let suite = load_suite();
    let audit_cases = suite
        .cases
        .iter()
        .map(|case| (case.source.as_str(), case))
        .collect::<HashMap<_, _>>();
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for evidence in EXECUTABLE_EVIDENCE {
        let audit_case = audit_cases
            .get(evidence.source)
            .unwrap_or_else(|| panic!("evidence case `{}` should be audited", evidence.source));
        assert_eq!(audit_case.axis, evidence.axis);
        let source_case = smoke_cases
            .get(evidence.source)
            .unwrap_or_else(|| panic!("case `{}` should exist in smoke fixture", evidence.source));
        assert!(
            source_case.data.contains(evidence.data_snippet),
            "evidence case `{}` should keep its characteristic input",
            evidence.source
        );
        let actual = actual_dom_dump_for_tree_case(source_case)
            .unwrap_or_else(|error| panic!("case `{}` parse failed: {error}", evidence.source));
        assert_eq!(actual, source_case.document);
    }
}

fn load_suite() -> ProcessingInstructionAuditSuite {
    serde_json::from_str(WHATWG_PROCESSING_INSTRUCTION_AUDIT)
        .expect("WHATWG processing-instruction audit fixture should parse")
}

fn assert_axis_count(suite: &ProcessingInstructionAuditSuite, axis: &str, expected: usize) {
    assert_eq!(
        suite.counts_by_axis.get(axis).copied().unwrap_or_default(),
        expected,
        "unexpected `{axis}` case count"
    );
}
