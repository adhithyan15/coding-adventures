use coding_adventures_html_lexer::create_html_lexer;
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
        assert!(
            !case.description.is_empty(),
            "case `{}` should describe its tag-open edge",
            case.id
        );
        let mut lexer = create_html_lexer().expect("HTML lexer should build");
        lexer
            .push(&case.input)
            .unwrap_or_else(|error| panic!("case `{}` push failed: {error:?}", case.id));
        lexer
            .finish()
            .unwrap_or_else(|error| panic!("case `{}` finish failed: {error:?}", case.id));

        common::assert_token_summaries(&case.id, lexer.drain_tokens(), &case.tokens);
        common::assert_diagnostic_codes(&case.id, lexer.diagnostics(), &case.diagnostics);
    }
}

fn load_suite() -> TagOpenRecoverySuite {
    serde_json::from_str(WHATWG_TAG_OPEN_RECOVERY)
        .expect("WHATWG tag-open recovery fixture should parse")
}
