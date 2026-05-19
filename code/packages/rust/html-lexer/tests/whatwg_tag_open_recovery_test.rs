use serde::Deserialize;

mod common;

const WHATWG_TAG_OPEN_RECOVERY: &str = include_str!("fixtures/whatwg-tag-open-recovery.json");

#[derive(Debug, Deserialize)]
struct TagOpenRecoverySuite {
    format: String,
    description: String,
    cases: Vec<TagOpenRecoveryCase>,
}

#[derive(Debug, Deserialize)]
struct TagOpenRecoveryCase {
    id: String,
    description: String,
    input: String,
    tokens: Vec<String>,
    #[serde(default)]
    diagnostics: Vec<String>,
}

#[test]
fn whatwg_tag_open_recovery_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-tokenizer-tag-open-recovery/v1");
    assert!(!suite.description.is_empty());
    assert!(suite.cases.len() >= 20);
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "less-than-null-reconsumes-as-text"));
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "invalid-end-tag-digit-bogus-comment"));
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "eof-in-end-tag-attributes"));
}

#[test]
fn whatwg_tag_open_recovery_cases_match_default_lexer() {
    let suite = load_suite();

    for case in &suite.cases {
        common::assert_case_description(&case.id, &case.description, "tag-open edge");
        common::assert_default_lexer_case(&case.id, &case.input, &case.tokens, &case.diagnostics);
    }
}

fn load_suite() -> TagOpenRecoverySuite {
    serde_json::from_str(WHATWG_TAG_OPEN_RECOVERY)
        .expect("WHATWG tag-open recovery fixture should parse")
}
