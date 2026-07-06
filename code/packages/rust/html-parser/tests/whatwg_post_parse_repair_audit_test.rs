mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_POST_PARSE_REPAIR_AUDIT: &str =
    include_str!("fixtures/whatwg-post-parse-repair-audit.json");

struct PostParseRepairEvidence {
    id: &'static str,
    source: &'static str,
    axis: &'static str,
    data_snippet: &'static str,
}

const POST_PARSE_REPAIR_EVIDENCE: &[PostParseRepairEvidence] = &[
    PostParseRepairEvidence {
        id: "adoption01-dat-6",
        source: "adoption01.dat:6",
        axis: "adoption-table-foster-parenting",
        data_snippet: "<table><a>1<p>2</a>3</p>",
    },
    PostParseRepairEvidence {
        id: "tests26-dat-4",
        source: "tests26.dat:4",
        axis: "fostered-nobr-cell-continuation",
        data_snippet: "<b><nobr>1<table><tr><td><nobr></b><i><nobr>2<nobr></i>3",
    },
    PostParseRepairEvidence {
        id: "tests26-dat-1251",
        source: "tests26.dat:1251",
        axis: "fostered-nobr-cell-continuation",
        data_snippet: "<b><nobr>1<table><tr><td><nobr></b><i><nobr>2<nobr></i>3",
    },
    PostParseRepairEvidence {
        id: "tricky01-dat-8",
        source: "tricky01.dat:8",
        axis: "insanely-badly-nested-table-sequence",
        data_snippet: "This page contains an insanely badly-nested tag sequence.",
    },
    PostParseRepairEvidence {
        id: "tricky01-dat-6",
        source: "tricky01.dat:6",
        axis: "tricky-center-table-void-recovery",
        data_snippet: "<table><center> <font>a</center> <img> <tr><td> </td> </tr> </table>",
    },
    PostParseRepairEvidence {
        id: "tricky01-dat-7",
        source: "tricky01.dat:7",
        axis: "tricky-paragraph-rowgroup-recovery",
        data_snippet: "<table><tr><p><a><p>You should see this text.",
    },
];

#[derive(Debug, Deserialize)]
struct PostParseRepairAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<PostParseRepairAuditCase>,
}

#[derive(Debug, Deserialize)]
struct PostParseRepairAuditCase {
    id: String,
    source: String,
    axis: String,
    reason: String,
}

#[test]
fn whatwg_post_parse_repair_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-post-parse-repair-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert_eq!(suite.case_count, 6);
    assert_axis_count(&suite, "adoption-table-foster-parenting", 1);
    assert_axis_count(&suite, "fostered-nobr-cell-continuation", 2);
    assert_axis_count(&suite, "insanely-badly-nested-table-sequence", 1);
    assert_axis_count(&suite, "tricky-center-table-void-recovery", 1);
    assert_axis_count(&suite, "tricky-paragraph-rowgroup-recovery", 1);
}

#[test]
fn whatwg_post_parse_repair_audit_cases_match_parser_dom_dump() {
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
fn whatwg_post_parse_repair_audit_tracks_post_parse_repair_evidence() {
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

fn load_suite() -> PostParseRepairAuditSuite {
    serde_json::from_str(WHATWG_POST_PARSE_REPAIR_AUDIT)
        .expect("WHATWG post-parse repair audit fixture should parse")
}

fn assert_axis_count(suite: &PostParseRepairAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
