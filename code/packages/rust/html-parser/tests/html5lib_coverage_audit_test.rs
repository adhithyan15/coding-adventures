#[allow(dead_code)]
mod common;

use common::parse_tree_construction_cases;
use serde::Deserialize;

const HTML5LIB_COVERAGE_AUDIT: &str = include_str!("fixtures/html5lib-coverage-audit.json");
const TREE_CONSTRUCTION_SMOKE: &str = include_str!("fixtures/html5lib-tree-construction-smoke.dat");
const HTML5LIB_RAW_FIXTURES: &str =
    include_str!("../../html-lexer/tests/fixtures/upstream-html5lib-smoke.test");
const HTML5LIB_NORMALIZED_FIXTURES: &str =
    include_str!("../../html-lexer/tests/fixtures/html5lib-smoke.json");

#[derive(Debug, Deserialize)]
struct CoverageAudit {
    tree_construction: TreeConstructionAudit,
    tokenizer: TokenizerAudit,
}

#[derive(Debug, Deserialize)]
struct TreeConstructionAudit {
    upstream_source: String,
    upstream_cases: usize,
    local_cases: usize,
    missing: usize,
    missing_sources: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TokenizerAudit {
    upstream_source: String,
    upstream_cases: usize,
    local_raw_cases: usize,
    missing: usize,
    missing_sources: Vec<String>,
    normalized_cases: usize,
    normalized_skipped: usize,
}

#[derive(Debug, Deserialize)]
struct RawTokenizerFixtures {
    tests: Vec<RawTokenizerCase>,
}

#[derive(Debug, Deserialize)]
struct RawTokenizerCase {}

#[derive(Debug, Deserialize)]
struct NormalizedTokenizerFixtures {
    cases: Vec<NormalizedTokenizerCase>,
    skipped: Vec<SkippedTokenizerCase>,
}

#[derive(Debug, Deserialize)]
struct NormalizedTokenizerCase {}

#[derive(Debug, Deserialize)]
struct SkippedTokenizerCase {}

#[test]
fn html5lib_coverage_audit_fixture_matches_checked_local_corpora() {
    let audit: CoverageAudit = serde_json::from_str(HTML5LIB_COVERAGE_AUDIT)
        .expect("html5lib coverage audit report should parse");
    let tree_cases = parse_tree_construction_cases(TREE_CONSTRUCTION_SMOKE);
    let raw_tokenizer: RawTokenizerFixtures =
        serde_json::from_str(HTML5LIB_RAW_FIXTURES).expect("raw html5lib fixture should parse");
    let normalized_tokenizer: NormalizedTokenizerFixtures =
        serde_json::from_str(HTML5LIB_NORMALIZED_FIXTURES)
            .expect("normalized html5lib fixture should parse");

    assert_eq!(
        audit.tree_construction.upstream_source,
        "wpt/html/syntax/parsing/resources"
    );
    assert_eq!(audit.tree_construction.upstream_cases, 1934);
    assert_eq!(audit.tree_construction.local_cases, tree_cases.len());
    assert_eq!(audit.tree_construction.local_cases, 2493);
    assert_eq!(audit.tree_construction.missing, 148);
    assert_eq!(
        audit.tree_construction.missing_sources.len(),
        audit.tree_construction.missing
    );
    assert_eq!(
        missing_source_count(&audit.tree_construction, "processing-instructions.dat:"),
        124
    );
    assert_eq!(
        missing_source_count(&audit.tree_construction, "void-in-phrasing.dat:"),
        13
    );
    assert_eq!(
        missing_source_count(&audit.tree_construction, "plain-text-unsafe.dat:"),
        0
    );
    assert_eq!(
        missing_source_count(&audit.tree_construction, "html5test-com.dat:"),
        7
    );
    assert_eq!(
        missing_source_count(&audit.tree_construction, "tests1.dat:"),
        3
    );
    assert_eq!(
        missing_source_count(&audit.tree_construction, "adoption02.dat:"),
        1
    );

    assert_eq!(audit.tokenizer.upstream_source, "html5lib-tests/tokenizer");
    assert_eq!(audit.tokenizer.upstream_cases, 6806);
    assert_eq!(audit.tokenizer.local_raw_cases, raw_tokenizer.tests.len());
    assert_eq!(audit.tokenizer.local_raw_cases, 7015);
    assert_eq!(
        audit.tokenizer.normalized_cases,
        normalized_tokenizer.cases.len()
    );
    assert_eq!(audit.tokenizer.normalized_cases, 7242);
    assert_eq!(
        audit.tokenizer.normalized_skipped,
        normalized_tokenizer.skipped.len()
    );
    assert_eq!(audit.tokenizer.normalized_skipped, 0);
    assert_eq!(audit.tokenizer.missing, 0);
    assert!(audit.tokenizer.missing_sources.is_empty());
}

fn missing_source_count(audit: &TreeConstructionAudit, prefix: &str) -> usize {
    audit
        .missing_sources
        .iter()
        .filter(|source| source.starts_with(prefix))
        .count()
}
