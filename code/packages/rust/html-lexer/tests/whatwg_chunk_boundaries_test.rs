use coding_adventures_html_lexer::{
    apply_html_lex_context, create_html_lexer, Diagnostic, DoctypeSeed, HtmlLexContext, HtmlLexer,
    HtmlTokenizerState, Token,
};
use serde::Deserialize;

const WHATWG_CHUNK_BOUNDARIES: &str = include_str!("fixtures/whatwg-chunk-boundaries.json");

#[derive(Debug, Deserialize)]
struct ChunkBoundarySuite {
    format: String,
    cases: Vec<ChunkBoundaryCase>,
}

#[derive(Debug, Deserialize)]
struct ChunkBoundaryCase {
    id: String,
    input: String,
    split_points: Vec<usize>,
    #[serde(default)]
    initial_state: Option<String>,
    #[serde(default)]
    last_start_tag: Option<String>,
    #[serde(default)]
    current_end_tag: Option<String>,
    #[serde(default)]
    current_comment: Option<String>,
    #[serde(default)]
    current_doctype: Option<FixtureDoctypeSeed>,
    #[serde(default)]
    temporary_buffer: Option<String>,
    #[serde(default)]
    return_state: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct FixtureDoctypeSeed {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    public_identifier: Option<String>,
    #[serde(default)]
    system_identifier: Option<String>,
    #[serde(default)]
    force_quirks: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct Lexed {
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
}

#[test]
fn whatwg_chunk_boundary_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-tokenizer-chunk-boundaries/v1");
    assert_eq!(suite.cases.len(), 32);
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "seeded-character-reference-rcdata"));
    assert!(suite
        .cases
        .iter()
        .all(|case| case.split_points.len() == case.input.chars().count() + 1));
}

#[test]
fn whatwg_tokenizer_output_is_invariant_across_chunk_boundaries() {
    let suite = load_suite();

    for case in &suite.cases {
        let expected = lex_case(case, &[case.input.as_str()]);

        for chunks in chunkings(&case.input) {
            let actual = lex_case(case, &chunks);
            assert_eq!(
                actual, expected,
                "case `{}` chunks {:?} should match single-chunk lexing",
                case.id, chunks
            );
        }
    }
}

fn load_suite() -> ChunkBoundarySuite {
    serde_json::from_str(WHATWG_CHUNK_BOUNDARIES)
        .expect("WHATWG chunk-boundary fixture should parse")
}

fn lex_case(case: &ChunkBoundaryCase, chunks: &[&str]) -> Lexed {
    let mut lexer = configured_lexer(case);
    for chunk in chunks {
        lexer.push(chunk).expect("push should succeed");
    }
    lexer.finish().expect("finish should succeed");
    Lexed {
        tokens: lexer.drain_tokens(),
        diagnostics: lexer.diagnostics().to_vec(),
    }
}

fn configured_lexer(case: &ChunkBoundaryCase) -> HtmlLexer {
    let state = case
        .initial_state
        .as_deref()
        .and_then(HtmlTokenizerState::from_html5lib_state)
        .unwrap_or(HtmlTokenizerState::Data);
    let mut context = HtmlLexContext::new(state);
    if let Some(last_start_tag) = case.last_start_tag.as_deref() {
        context = context.with_last_start_tag(last_start_tag);
    }
    if let Some(current_end_tag) = case.current_end_tag.as_deref() {
        context = context.with_current_end_tag(current_end_tag);
    }
    if let Some(current_comment) = case.current_comment.as_deref() {
        context = context.with_current_comment(current_comment);
    }
    if let Some(current_doctype) = case.current_doctype.as_ref() {
        context = context.with_current_doctype(DoctypeSeed {
            name: current_doctype.name.clone(),
            public_identifier: current_doctype.public_identifier.clone(),
            system_identifier: current_doctype.system_identifier.clone(),
            force_quirks: current_doctype.force_quirks,
        });
    }
    if let Some(temporary_buffer) = case.temporary_buffer.as_deref() {
        context = context.with_temporary_buffer(temporary_buffer);
    }
    if let Some(return_state) = case
        .return_state
        .as_deref()
        .and_then(HtmlTokenizerState::from_html5lib_state)
    {
        context = context.with_return_state(return_state);
    }

    let mut lexer = create_html_lexer().expect("HTML lexer should build");
    apply_html_lex_context(&mut lexer, &context).expect("HTML lexer context should apply");
    lexer
}

fn chunkings(source: &str) -> Vec<Vec<&str>> {
    let mut boundaries = source
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(source.len()))
        .collect::<Vec<_>>();
    boundaries.sort_unstable();
    boundaries.dedup();

    boundaries
        .into_iter()
        .map(|boundary| vec![&source[..boundary], &source[boundary..]])
        .collect()
}
