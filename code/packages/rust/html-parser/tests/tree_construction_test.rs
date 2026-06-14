mod common;

use common::{actual_dom_dump_for_tree_case, parse_tree_construction_cases};

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
