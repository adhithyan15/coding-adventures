mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_MISC_RECOVERY_AUDIT: &str =
    include_str!("fixtures/whatwg-misc-recovery-audit.json");

#[derive(Debug, Deserialize)]
struct MiscRecoveryAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<MiscRecoveryAuditCase>,
}

#[derive(Debug, Deserialize)]
struct MiscRecoveryAuditCase {
    id: String,
    source: String,
    axis: String,
    reason: String,
}

#[test]
fn whatwg_misc_recovery_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-misc-recovery-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 51);
    assert_axis_count(&suite, "xml-pi-looking-markup", 8);
    assert_axis_count(&suite, "bogus-comment-and-cdata", 14);
    assert_axis_count(&suite, "text-whitespace-shell", 8);
    assert_axis_count(&suite, "malformed-tag-open", 9);
    assert_axis_count(&suite, "legacy-compat-elements", 7);
    assert_axis_count(&suite, "custom-element-recovery", 3);
    assert_axis_count(&suite, "duplicate-doctype-recovery", 2);
}

#[test]
fn whatwg_misc_recovery_audit_cases_match_parser_dom_dump() {
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

fn load_suite() -> MiscRecoveryAuditSuite {
    serde_json::from_str(WHATWG_MISC_RECOVERY_AUDIT)
        .expect("WHATWG miscellaneous recovery audit fixture should parse")
}

fn assert_axis_count(suite: &MiscRecoveryAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
