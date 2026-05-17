use coding_adventures_html_lexer::{
    apply_html_lex_context, create_html_lexer, DoctypeSeed, HtmlLexContext, HtmlLexer,
    HtmlTokenizerState, SourcePosition, Token,
};
use serde::Deserialize;

const WHATWG_INPUT_STREAM: &str = include_str!("fixtures/whatwg-input-stream.json");

#[derive(Debug, Deserialize)]
struct InputStreamSuite {
    format: String,
    newline_forms: Vec<NewlineForm>,
    cases: Vec<InputStreamCase>,
    position_cases: Vec<PositionCase>,
}

#[derive(Debug, Deserialize)]
struct NewlineForm {
    id: String,
    source: String,
    normalized: String,
}

#[derive(Debug, Deserialize)]
struct InputStreamCase {
    id: String,
    input: String,
    normalized: String,
    #[serde(default)]
    initial_state: Option<String>,
    #[serde(default)]
    last_start_tag: Option<String>,
    #[serde(default)]
    current_comment: Option<String>,
    #[serde(default)]
    current_doctype: Option<FixtureDoctypeSeed>,
}

#[derive(Debug, Deserialize)]
struct PositionCase {
    id: String,
    input: String,
    diagnostic: String,
    expected_line: usize,
    expected_column: usize,
    #[serde(default)]
    initial_state: Option<String>,
    #[serde(default)]
    last_start_tag: Option<String>,
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
    diagnostics: Vec<String>,
}

#[test]
fn whatwg_input_stream_fixture_parses() {
    let suite = load_suite();

    assert_eq!(
        suite.format,
        "whatwg-html-input-stream-preprocessing/v1"
    );
    assert_eq!(suite.newline_forms.len(), 4);
    assert_eq!(suite.cases.len(), 92);
    assert_eq!(suite.position_cases.len(), 16);
    assert!(suite.newline_forms.iter().any(|form| {
        form.id == "crlf" && form.source == "\r\n" && form.normalized == "\n"
    }));
    assert!(suite.newline_forms.iter().any(|form| {
        form.id == "mixed" && form.source == "\r\n\r\n\r" && form.normalized == "\n\n\n"
    }));
}

#[test]
fn whatwg_input_stream_cr_and_crlf_match_preprocessed_lf_stream() {
    let suite = load_suite();

    for case in &suite.cases {
        let expected = lex_case(case, &case.normalized);

        for chunks in chunkings(&case.input) {
            let actual = lex_case_chunks(case, &chunks);
            assert_eq!(
                actual, expected,
                "case `{}` chunks {:?} should match preprocessed stream {:?}",
                case.id, chunks, case.normalized
            );
        }
    }
}

#[test]
fn whatwg_input_stream_newline_positions_are_stable_across_chunks() {
    let suite = load_suite();

    for case in &suite.position_cases {
        for chunks in chunkings(&case.input) {
            let position = diagnostic_position(case, &chunks);
            assert_eq!(
                (position.line, position.column),
                (case.expected_line, case.expected_column),
                "case `{}` chunks {:?} should report {:?} at normalized line/column",
                case.id, chunks, case.diagnostic
            );
        }
    }
}

fn load_suite() -> InputStreamSuite {
    serde_json::from_str(WHATWG_INPUT_STREAM).expect("WHATWG input stream fixture should parse")
}

fn lex_case(case: &InputStreamCase, source: &str) -> Lexed {
    lex_case_chunks(case, &[source])
}

fn lex_case_chunks(case: &InputStreamCase, chunks: &[&str]) -> Lexed {
    let mut lexer = configured_lexer(
        case.initial_state.as_deref(),
        case.last_start_tag.as_deref(),
        case.current_comment.as_deref(),
        case.current_doctype.as_ref(),
    );
    for chunk in chunks {
        lexer.push(chunk).expect("push should succeed");
    }
    lexer.finish().expect("finish should succeed");
    Lexed {
        tokens: lexer.drain_tokens(),
        diagnostics: lexer
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.code.clone())
            .collect(),
    }
}

fn diagnostic_position(case: &PositionCase, chunks: &[&str]) -> SourcePosition {
    let mut lexer = configured_lexer(
        case.initial_state.as_deref(),
        case.last_start_tag.as_deref(),
        None,
        None,
    );
    for chunk in chunks {
        lexer.push(chunk).expect("push should succeed");
    }
    lexer.finish().expect("finish should succeed");
    lexer
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code == case.diagnostic)
        .unwrap_or_else(|| panic!("case `{}` did not report {}", case.id, case.diagnostic))
        .position
}

fn configured_lexer(
    initial_state: Option<&str>,
    last_start_tag: Option<&str>,
    current_comment: Option<&str>,
    current_doctype: Option<&FixtureDoctypeSeed>,
) -> HtmlLexer {
    let state = initial_state
        .and_then(HtmlTokenizerState::from_html5lib_state)
        .unwrap_or(HtmlTokenizerState::Data);
    let mut context = HtmlLexContext::new(state);
    if let Some(last_start_tag) = last_start_tag {
        context = context.with_last_start_tag(last_start_tag);
    }
    if let Some(current_comment) = current_comment {
        context = context.with_current_comment(current_comment);
    }
    if let Some(current_doctype) = current_doctype {
        context = context.with_current_doctype(DoctypeSeed {
            name: current_doctype.name.clone(),
            public_identifier: current_doctype.public_identifier.clone(),
            system_identifier: current_doctype.system_identifier.clone(),
            force_quirks: current_doctype.force_quirks,
        });
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

    let mut chunkings = vec![vec![source]];
    for boundary in boundaries {
        chunkings.push(vec![&source[..boundary], &source[boundary..]]);
    }
    chunkings
}
