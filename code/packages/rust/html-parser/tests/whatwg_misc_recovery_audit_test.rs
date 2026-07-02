mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_MISC_RECOVERY_AUDIT: &str =
    include_str!("fixtures/whatwg-misc-recovery-audit.json");
const POST_PARSE_REPAIR_EVIDENCE: &[(&str, &str, &str)] = &[
    (
        "comments01-dat-79",
        "comments01.dat:79",
        "xml-pi-looking-markup",
    ),
    (
        "domjs-unsafe-dat-147",
        "domjs-unsafe.dat:147",
        "duplicate-doctype-recovery",
    ),
    (
        "html5test-com-dat-283",
        "html5test-com.dat:283",
        "bogus-comment-and-cdata",
    ),
    (
        "plain-text-unsafe-dat-356",
        "plain-text-unsafe.dat:356",
        "text-whitespace-shell",
    ),
    ("tests1-dat-601", "tests1.dat:601", "malformed-tag-open"),
    (
        "tests19-dat-1021",
        "tests19.dat:1021",
        "legacy-compat-elements",
    ),
    (
        "webkit01-dat-8",
        "webkit01.dat:8",
        "custom-element-recovery",
    ),
];

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
    assert!(suite.case_count >= 74);
    assert_axis_count(&suite, "xml-pi-looking-markup", 12);
    assert_axis_count(&suite, "bogus-comment-and-cdata", 20);
    assert_axis_count(&suite, "text-whitespace-shell", 10);
    assert_axis_count(&suite, "malformed-tag-open", 14);
    assert_axis_count(&suite, "legacy-compat-elements", 13);
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

#[test]
fn whatwg_misc_recovery_audit_tracks_post_parse_repair_evidence() {
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

    for (case_id, expected_source, expected_axis) in POST_PARSE_REPAIR_EVIDENCE {
        let audit_case = audit_cases.get(case_id).unwrap_or_else(|| {
            panic!("post-parse repair evidence case `{case_id}` should be audited")
        });
        assert_eq!(
            audit_case.source, *expected_source,
            "post-parse repair evidence case `{case_id}` should stay tied to its smoke fixture row"
        );
        assert_eq!(
            audit_case.axis, *expected_axis,
            "post-parse repair evidence case `{case_id}` should stay on its focused audit axis"
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
            "post-parse repair evidence case `{}` ({}) failed for input {:?}",
            audit_case.id, audit_case.axis, source_case.data
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
