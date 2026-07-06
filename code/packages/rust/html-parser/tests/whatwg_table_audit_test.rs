mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const WHATWG_BLOCK_BOUNDARY_AUDIT: &str = include_str!("fixtures/whatwg-block-boundary-audit.json");
const WHATWG_DOCUMENT_SHELL_AUDIT: &str = include_str!("fixtures/whatwg-document-shell-audit.json");
const WHATWG_FORMATTING_AUDIT: &str = include_str!("fixtures/whatwg-formatting-audit.json");
const WHATWG_FORM_INTERACTIVE_AUDIT: &str =
    include_str!("fixtures/whatwg-form-interactive-audit.json");
const WHATWG_HEAD_BODY_AUDIT: &str = include_str!("fixtures/whatwg-head-body-audit.json");
const WHATWG_LEGACY_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-legacy-element-audit.json");
const WHATWG_PARAGRAPH_AUDIT: &str = include_str!("fixtures/whatwg-paragraph-audit.json");
const WHATWG_TABLE_AUDIT: &str = include_str!("fixtures/whatwg-table-audit.json");
const WHATWG_TREE_INSERTION_AUDIT: &str = include_str!("fixtures/whatwg-tree-insertion-audit.json");
const WHATWG_VOID_ELEMENT_AUDIT: &str = include_str!("fixtures/whatwg-void-element-audit.json");

struct CrossAxisRepairCase {
    id: &'static str,
    data_snippet: &'static str,
    suites: &'static [(&'static str, &'static str, &'static str)],
}

struct TableRepairEvidence {
    id: &'static str,
    source: &'static str,
    axis: &'static str,
    data_snippet: &'static str,
}

const POST_PARSE_REPAIR_EVIDENCE: &[TableRepairEvidence] = &[
    TableRepairEvidence {
        id: "adoption01-dat-6",
        source: "adoption01.dat:6",
        axis: "foster-parenting",
        data_snippet: "<table><a>1<p>2</a>3</p>",
    },
    TableRepairEvidence {
        id: "tests26-dat-4",
        source: "tests26.dat:4",
        axis: "cell-boundary",
        data_snippet: "<b><nobr>1<table><tr><td><nobr></b><i><nobr>2<nobr></i>3",
    },
    TableRepairEvidence {
        id: "tests26-dat-1251",
        source: "tests26.dat:1251",
        axis: "cell-boundary",
        data_snippet: "<b><nobr>1<table><tr><td><nobr></b><i><nobr>2<nobr></i>3",
    },
    TableRepairEvidence {
        id: "tricky01-dat-6",
        source: "tricky01.dat:6",
        axis: "cell-boundary",
        data_snippet: "<table><center> <font>a</center> <img> <tr><td> </td> </tr> </table>",
    },
    TableRepairEvidence {
        id: "tricky01-dat-7",
        source: "tricky01.dat:7",
        axis: "row-group-boundary",
        data_snippet: "<table><tr><p><a><p>You should see this text.",
    },
    TableRepairEvidence {
        id: "tricky01-dat-8",
        source: "tricky01.dat:8",
        axis: "cell-boundary",
        data_snippet: "This page contains an insanely badly-nested tag sequence.",
    },
];
const DUPLICATE_FOSTERED_NOBR_ROWS: &[(&str, &str)] = &[("tests26-dat-4", "tests26-dat-1251")];
const FOSTERED_NOBR_REPAIR_CASE_IDS: &[&str] = &["tests26-dat-4", "tests26-dat-1251"];
const FOSTERED_NOBR_REPAIR_SUITES: &[(&str, &str, &str)] = &[
    (
        "document-shell",
        WHATWG_DOCUMENT_SHELL_AUDIT,
        "body-frameset-boundary",
    ),
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "interactive-formatting",
    ),
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "interactive-formatting-boundary",
    ),
    ("head-body", WHATWG_HEAD_BODY_AUDIT, "body-boundary"),
    ("table", WHATWG_TABLE_AUDIT, "cell-boundary"),
];
const ADOPTION_TABLE_REPAIR_SUITES: &[(&str, &str, &str)] = &[
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "interactive-formatting",
    ),
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "interactive-formatting-boundary",
    ),
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-table-boundary",
    ),
    ("table", WHATWG_TABLE_AUDIT, "foster-parenting"),
    (
        "tree-insertion",
        WHATWG_TREE_INSERTION_AUDIT,
        "adoption-agency",
    ),
];
const TRICKY_CENTER_TABLE_REPAIR_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-table-boundary",
    ),
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "adoption-agency-formatting",
    ),
    (
        "legacy-element",
        WHATWG_LEGACY_ELEMENT_AUDIT,
        "tricky-parser-recovery",
    ),
    ("table", WHATWG_TABLE_AUDIT, "cell-boundary"),
    ("void-element", WHATWG_VOID_ELEMENT_AUDIT, "void-in-table"),
];
const TRICKY_PARAGRAPH_TABLE_REPAIR_SUITES: &[(&str, &str, &str)] = &[
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "interactive-formatting",
    ),
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "interactive-formatting-boundary",
    ),
    (
        "legacy-element",
        WHATWG_LEGACY_ELEMENT_AUDIT,
        "tricky-parser-recovery",
    ),
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-table-boundary",
    ),
    ("table", WHATWG_TABLE_AUDIT, "row-group-boundary"),
];
const REMAINING_TABLE_REPAIR_CASES: &[CrossAxisRepairCase] = &[
    CrossAxisRepairCase {
        id: "adoption01-dat-6",
        data_snippet: "<table><a>1<p>2</a>3</p>",
        suites: ADOPTION_TABLE_REPAIR_SUITES,
    },
    CrossAxisRepairCase {
        id: "tricky01-dat-6",
        data_snippet: "<table><center> <font>a</center> <img> <tr><td> </td> </tr> </table>",
        suites: TRICKY_CENTER_TABLE_REPAIR_SUITES,
    },
    CrossAxisRepairCase {
        id: "tricky01-dat-7",
        data_snippet: "<table><tr><p><a><p>You should see this text.",
        suites: TRICKY_PARAGRAPH_TABLE_REPAIR_SUITES,
    },
];
const INSANELY_BADLY_NESTED_REPAIR_CASE_ID: &str = "tricky01-dat-8";
const INSANELY_BADLY_NESTED_REPAIR_SUITES: &[(&str, &str, &str)] = &[
    (
        "block-boundary",
        WHATWG_BLOCK_BOUNDARY_AUDIT,
        "block-table-boundary",
    ),
    (
        "formatting",
        WHATWG_FORMATTING_AUDIT,
        "interactive-formatting-boundary",
    ),
    (
        "form-interactive",
        WHATWG_FORM_INTERACTIVE_AUDIT,
        "interactive-formatting",
    ),
    (
        "legacy-element",
        WHATWG_LEGACY_ELEMENT_AUDIT,
        "tricky-parser-recovery",
    ),
    (
        "paragraph",
        WHATWG_PARAGRAPH_AUDIT,
        "paragraph-table-boundary",
    ),
    ("table", WHATWG_TABLE_AUDIT, "cell-boundary"),
];

#[derive(Debug, Deserialize)]
struct TableAuditSuite {
    format: String,
    description: String,
    source_fixture: String,
    case_count: usize,
    counts_by_axis: BTreeMap<String, usize>,
    cases: Vec<TableAuditCase>,
}

#[derive(Debug, Deserialize)]
struct TableAuditCase {
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
fn whatwg_table_audit_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-table-audit/v1");
    assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat");
    assert!(!suite.description.is_empty());
    assert_eq!(suite.case_count, suite.cases.len());
    assert!(suite.case_count >= 450);
    assert_axis_count(&suite, "table-shell", 25);
    assert_axis_count(&suite, "row-group-boundary", 50);
    assert_axis_count(&suite, "cell-boundary", 100);
    assert_axis_count(&suite, "caption-colgroup", 65);
    assert_axis_count(&suite, "select-in-table", 50);
    assert_axis_count(&suite, "foster-parenting", 55);
    assert_axis_count(&suite, "table-fragment-context", 90);
}

#[test]
fn whatwg_table_audit_cases_match_parser_dom_dump() {
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
fn whatwg_table_audit_tracks_post_parse_repair_evidence() {
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

#[test]
fn whatwg_table_audit_keeps_duplicate_fostered_nobr_rows_in_lockstep() {
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

    for (left_id, right_id) in DUPLICATE_FOSTERED_NOBR_ROWS {
        let left = audit_cases
            .get(left_id)
            .unwrap_or_else(|| panic!("duplicate evidence case `{left_id}` should be audited"));
        let right = audit_cases
            .get(right_id)
            .unwrap_or_else(|| panic!("duplicate evidence case `{right_id}` should be audited"));

        assert_eq!(
            left.axis, right.axis,
            "duplicate fostered `nobr` evidence rows should stay on the same audit axis"
        );
        assert_eq!(
            left.reason, right.reason,
            "duplicate fostered `nobr` evidence rows should keep the same audit reason"
        );

        let left_source = smoke_cases
            .get(&left.source)
            .unwrap_or_else(|| panic!("case `{left_id}` should exist in smoke fixture"));
        let right_source = smoke_cases
            .get(&right.source)
            .unwrap_or_else(|| panic!("case `{right_id}` should exist in smoke fixture"));

        assert_eq!(
            left_source.data, right_source.data,
            "duplicate fostered `nobr` evidence rows should keep matching input"
        );
        assert_eq!(
            left_source.document, right_source.document,
            "duplicate fostered `nobr` evidence rows should keep matching expected DOM"
        );
    }
}

#[test]
fn whatwg_table_audit_keeps_fostered_nobr_repair_cases_cross_axis() {
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();
    let mut duplicate_source_data = None;
    let mut duplicate_source_document = None;

    for case_id in FOSTERED_NOBR_REPAIR_CASE_IDS {
        let mut shared_source = None;

        for (suite_name, fixture, expected_axis) in FOSTERED_NOBR_REPAIR_SUITES {
            let audit_case = generic_audit_case(fixture, suite_name, case_id);
            assert_eq!(
                audit_case.axis, *expected_axis,
                "`{suite_name}` should keep fostered `nobr` repair row `{case_id}` on its focused axis"
            );
            assert!(
                !audit_case.reason.is_empty(),
                "`{suite_name}` should keep a reason for fostered `nobr` repair row `{case_id}`"
            );

            if let Some(source) = &shared_source {
                assert_eq!(
                    audit_case.source, *source,
                    "`{suite_name}` should point fostered `nobr` repair row `{case_id}` at the same html5lib source as the other audit axes"
                );
            } else {
                shared_source = Some(audit_case.source);
            }
        }

        let shared_source =
            shared_source.expect("fostered `nobr` repair evidence should include a source row");
        let source_case = smoke_cases
            .get(&shared_source)
            .unwrap_or_else(|| panic!("case `{shared_source}` should exist in smoke fixture"));
        assert!(
            source_case
                .data
                .contains("<b><nobr>1<table><tr><td><nobr></b><i><nobr>2<nobr></i>3"),
            "repair evidence should stay tied to the duplicated fostered `nobr` html5lib input"
        );

        if let Some(previous_data) = &duplicate_source_data {
            assert_eq!(
                source_case.data, *previous_data,
                "duplicated fostered `nobr` repair rows should keep matching input"
            );
        } else {
            duplicate_source_data = Some(source_case.data.clone());
        }
        if let Some(previous_document) = &duplicate_source_document {
            assert_eq!(
                source_case.document, *previous_document,
                "duplicated fostered `nobr` repair rows should keep matching expected DOM"
            );
        } else {
            duplicate_source_document = Some(source_case.document.clone());
        }

        let actual = actual_dom_dump_for_tree_case(source_case).unwrap_or_else(|error| {
            panic!("case `{shared_source}` (`{case_id}`) parse failed: {error}")
        });
        assert_eq!(
            actual, source_case.document,
            "cross-axis fostered `nobr` repair evidence case `{shared_source}` failed for input {:?}",
            source_case.data
        );
    }
}

#[test]
fn whatwg_table_audit_keeps_remaining_repair_cases_cross_axis() {
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();

    for repair_case in REMAINING_TABLE_REPAIR_CASES {
        let mut shared_source = None;

        for (suite_name, fixture, expected_axis) in repair_case.suites {
            let audit_case = generic_audit_case(fixture, suite_name, repair_case.id);
            assert_eq!(
                audit_case.axis, *expected_axis,
                "`{suite_name}` should keep repair row `{}` on its focused axis",
                repair_case.id
            );
            assert!(
                !audit_case.reason.is_empty(),
                "`{suite_name}` should keep a reason for repair row `{}`",
                repair_case.id
            );

            if let Some(source) = &shared_source {
                assert_eq!(
                    audit_case.source, *source,
                    "`{suite_name}` should point repair row `{}` at the same html5lib row as the other audit axes",
                    repair_case.id
                );
            } else {
                shared_source = Some(audit_case.source);
            }
        }

        let shared_source =
            shared_source.expect("repair evidence should include at least one suite");
        let source_case = smoke_cases
            .get(&shared_source)
            .unwrap_or_else(|| panic!("case `{shared_source}` should exist in smoke fixture"));
        assert!(
            source_case.data.contains(repair_case.data_snippet),
            "repair evidence row `{}` should stay tied to its html5lib input",
            repair_case.id
        );

        let actual = actual_dom_dump_for_tree_case(source_case)
            .unwrap_or_else(|error| panic!("case `{shared_source}` parse failed: {error}"));
        assert_eq!(
            actual, source_case.document,
            "cross-axis repair evidence case `{shared_source}` failed for input {:?}",
            source_case.data
        );
    }
}

#[test]
fn whatwg_table_audit_keeps_insanely_badly_nested_repair_case_cross_axis() {
    let smoke_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE)
        .into_iter()
        .map(|case| (case.source.clone(), case))
        .collect::<HashMap<_, _>>();
    let mut shared_source = None;

    for (suite_name, fixture, expected_axis) in INSANELY_BADLY_NESTED_REPAIR_SUITES {
        let audit_case =
            generic_audit_case(fixture, suite_name, INSANELY_BADLY_NESTED_REPAIR_CASE_ID);
        assert_eq!(
            audit_case.axis, *expected_axis,
            "`{suite_name}` should keep the insanely badly nested repair row on its focused axis"
        );
        assert!(
            !audit_case.reason.is_empty(),
            "`{suite_name}` should keep a reason for the repair-bearing row"
        );

        if let Some(source) = &shared_source {
            assert_eq!(
                audit_case.source, *source,
                "`{suite_name}` should point at the same html5lib row as the other repair axes"
            );
        } else {
            shared_source = Some(audit_case.source);
        }
    }

    let shared_source = shared_source.expect("repair evidence should include at least one suite");
    let source_case = smoke_cases
        .get(&shared_source)
        .unwrap_or_else(|| panic!("case `{shared_source}` should exist in smoke fixture"));
    assert!(
        source_case
            .data
            .contains("This page contains an insanely badly-nested tag sequence."),
        "repair evidence should stay tied to the html5lib badly nested table case"
    );

    let actual = actual_dom_dump_for_tree_case(source_case)
        .unwrap_or_else(|error| panic!("case `{shared_source}` parse failed: {error}"));
    assert_eq!(
        actual, source_case.document,
        "cross-axis repair evidence case `{shared_source}` failed for input {:?}",
        source_case.data
    );
}

fn load_suite() -> TableAuditSuite {
    serde_json::from_str(WHATWG_TABLE_AUDIT).expect("WHATWG table audit fixture should parse")
}

fn generic_audit_case(fixture: &str, suite_name: &str, case_id: &str) -> GenericAuditCase {
    let suite = serde_json::from_str::<GenericAuditSuite>(fixture)
        .unwrap_or_else(|error| panic!("`{suite_name}` audit fixture should parse: {error}"));
    suite
        .cases
        .into_iter()
        .find(|case| case.id == case_id)
        .unwrap_or_else(|| panic!("`{suite_name}` should audit repair case `{case_id}`"))
}

fn assert_axis_count(suite: &TableAuditSuite, axis: &str, minimum: usize) {
    let count = suite.counts_by_axis.get(axis).copied().unwrap_or_default();
    assert!(
        count >= minimum,
        "expected at least {minimum} `{axis}` cases, found {count}"
    );
}
