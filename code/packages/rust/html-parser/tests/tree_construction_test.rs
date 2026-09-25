mod common;

use std::collections::BTreeSet;

use coding_adventures_html_lexer::HtmlScriptingMode;
use common::{
    actual_diagnostic_codes_for_tree_case, actual_dom_dump_for_tree_case,
    actual_dom_dump_with_scripting, expected_failures, parse_tree_construction_cases,
};

const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");

/// Every case matches the expected DOM, except the declared expected
/// failures -- which must still fail. A case that starts passing has to be
/// taken off the list, so the list cannot go stale.
#[test]
fn html5lib_tree_construction_smoke_cases_match_dom_dump() {
    let cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE);
    assert!(!cases.is_empty(), "fixture should contain cases");
    let expected = expected_failures();

    let mut unexpected_failures = Vec::new();
    let mut unexpected_passes = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, case) in cases.iter().enumerate() {
        let actual = actual_dom_dump_for_tree_case(case)
            .expect("parser should accept any HTML or HTML fragment input");
        let passes = actual == case.document;
        if expected.contains_key(case.source.as_str()) {
            seen.insert(case.source.as_str());
            if passes {
                unexpected_passes.push(case.source.clone());
            }
        } else if !passes {
            unexpected_failures.push(format!(
                "case {} ({}) for input {:?}\n  expected: {:#?}\n  actual:   {:#?}",
                index + 1,
                case.source,
                case.data,
                case.document,
                actual
            ));
        }
    }

    let unknown: Vec<_> = expected.keys().filter(|source| !seen.contains(*source)).collect();
    assert!(unknown.is_empty(), "expected failures name no corpus case: {unknown:?}");
    assert!(
        unexpected_passes.is_empty(),
        "these now pass; remove them from tree-construction-expected-failures.txt: {unexpected_passes:?}"
    );
    assert!(
        unexpected_failures.is_empty(),
        "{} tree-construction case(s) failed:\n{}",
        unexpected_failures.len(),
        unexpected_failures.join("\n")
    );
}

/// BR02 P1.2: html5lib's rule is that a case without `#script-on` or
/// `#script-off` must produce the same tree in both modes. The corpus test
/// above runs them with scripting on; this runs them again with scripting off.
/// A case that differs is a parser bug, listed as `<source>#script-off` in the
/// expected-failure file until it is fixed.
#[test]
fn unflagged_cases_match_with_scripting_off() {
    let cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE);
    let expected = expected_failures();
    let mut unexpected_failures = Vec::new();
    let mut unexpected_passes = Vec::new();
    let mut unflagged = 0;
    for case in cases.iter().filter(|case| !case.scripting_flagged) {
        if expected.contains_key(case.source.as_str()) {
            continue;
        }
        unflagged += 1;
        let id = format!("{}#script-off", case.source);
        let actual = actual_dom_dump_with_scripting(case, HtmlScriptingMode::Disabled)
            .expect("parser should accept any HTML or HTML fragment input");
        let passes = actual == case.document;
        match (expected.contains_key(id.as_str()), passes) {
            (true, true) => unexpected_passes.push(id),
            (false, false) => unexpected_failures.push(format!(
                "{id} for input {:?}\n  expected: {:#?}\n  actual:   {:#?}",
                case.data, case.document, actual
            )),
            _ => {}
        }
    }
    assert!(unflagged > 2000, "the corpus should have many unflagged cases, found {unflagged}");
    assert!(
        unexpected_passes.is_empty(),
        "these now pass with scripting off; remove them from the expected failures: {unexpected_passes:?}"
    );
    assert!(
        unexpected_failures.is_empty(),
        "{} unflagged case(s) differ with scripting off:\n{}",
        unexpected_failures.len(),
        unexpected_failures.join("\n")
    );
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

    // +2 rows / +1 case: template.dat:124 (BR02 P1.3).
    assert_eq!(expected_error_rows, 6277);
    assert_eq!(expected_error_cases, 2198);
    assert_eq!(missing_diagnostic_cases, 0);
    assert_eq!(expected_error_cases - missing_diagnostic_cases, 2198);
    assert_eq!(undeclared_diagnostic_cases, 142);
}
