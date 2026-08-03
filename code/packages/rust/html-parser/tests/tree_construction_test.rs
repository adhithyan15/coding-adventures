mod common;

use common::{
    actual_diagnostic_codes_for_tree_case, actual_dom_dump_for_tree_case,
    parse_tree_construction_cases,
};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");

#[test]
fn html5lib_tree_construction_smoke_cases_match_dom_dump() {
    let cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE);
    assert!(!cases.is_empty(), "fixture should contain cases");

    for (index, case) in cases.iter().enumerate() {
        let actual = actual_dom_dump_for_tree_case(case)
            .expect("parser should accept any HTML or HTML fragment input");
        assert_eq!(
            actual,
            case.document,
            "tree-construction smoke case {} ({}) failed for input {:?}",
            index + 1,
            case.source,
            case.data
        );
    }
}

#[test]
fn tree_construction_diagnostic_coverage_is_ratcheted() {
    let cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE);
    let expected_error_rows = cases
        .iter()
        .map(|case| case.expected_errors.len())
        .sum::<usize>();
    let mut expected_error_cases = 0;
    let mut missing_diagnostic_cases = 0;
    let mut undeclared_diagnostic_cases = 0;

    for case in &cases {
        let actual = actual_diagnostic_codes_for_tree_case(case)
            .expect("parser should accept any HTML or HTML fragment input");
        if case.expected_errors.is_empty() {
            if !actual.is_empty() {
                undeclared_diagnostic_cases += 1;
            }
        } else {
            expected_error_cases += 1;
            if actual.is_empty() {
                missing_diagnostic_cases += 1;
            }
        }
    }

    assert_eq!(expected_error_rows, 6243);
    assert_eq!(expected_error_cases, 2183);
    assert_eq!(missing_diagnostic_cases, 397);
    assert_eq!(expected_error_cases - missing_diagnostic_cases, 1786);
    assert_eq!(undeclared_diagnostic_cases, 139);
}
