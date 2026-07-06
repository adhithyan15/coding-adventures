mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_MISC_RECOVERY_AUDIT: &str = include_str!("fixtures/whatwg-misc-recovery-audit.json");
const POST_PARSE_REPAIR_EVIDENCE: &[MiscRecoveryEvidence] = &[
    MiscRecoveryEvidence {
        id: "comments01-dat-79",
        source: "comments01.dat:79",
        axis: "xml-pi-looking-markup",
        data_snippet: r#"<?xml version="1.0">Hi"#,
    },
    MiscRecoveryEvidence {
        id: "domjs-unsafe-dat-147",
        source: "domjs-unsafe.dat:147",
        axis: "duplicate-doctype-recovery",
        data_snippet: "<!DOCTYPE html><!DOCTYPE html>",
    },
    MiscRecoveryEvidence {
        id: "html5test-com-dat-283",
        source: "html5test-com.dat:283",
        axis: "bogus-comment-and-cdata",
        data_snippet: "<!--foo--bar-->",
    },
    MiscRecoveryEvidence {
        id: "plain-text-unsafe-dat-356",
        source: "plain-text-unsafe.dat:356",
        axis: "text-whitespace-shell",
        data_snippet: "\0",
    },
    MiscRecoveryEvidence {
        id: "tests1-dat-601",
        source: "tests1.dat:601",
        axis: "malformed-tag-open",
        data_snippet: "<",
    },
    MiscRecoveryEvidence {
        id: "tests19-dat-1021",
        source: "tests19.dat:1021",
        axis: "legacy-compat-elements",
        data_snippet: r#"<!doctype html><isindex type="hidden">"#,
    },
    MiscRecoveryEvidence {
        id: "webkit01-dat-8",
        source: "webkit01.dat:8",
        axis: "custom-element-recovery",
        data_snippet: r#"<foo bar="baz"></foo><potato quack="duck"></potato>"#,
    },
];

struct MiscRecoveryEvidence {
    id: &'static str,
    source: &'static str,
    axis: &'static str,
    data_snippet: &'static str,
}

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
