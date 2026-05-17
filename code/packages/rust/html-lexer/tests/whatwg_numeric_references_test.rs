use coding_adventures_html_lexer::{
    create_html_lexer, create_html_lexer_with_context, HtmlLexContext, Token,
};
use serde::Deserialize;

const WHATWG_NUMERIC_REFERENCES: &str = include_str!("fixtures/whatwg-numeric-references.json");
const MISSING_SEMICOLON: &str = "missing-semicolon-after-character-reference";

#[derive(Debug, Deserialize)]
struct NumericSuite {
    format: String,
    cases: Vec<NumericCase>,
}

#[derive(Debug, Deserialize)]
struct NumericCase {
    value: u32,
    decimal: String,
    hex: String,
    decimal_missing_semicolon: String,
    hex_missing_semicolon: String,
    characters: String,
    codepoints: Vec<u32>,
    diagnostics: Vec<String>,
}

#[test]
fn whatwg_numeric_character_references_decode_in_data_state() {
    let suite = load_suite();
    assert_eq!(suite.format, "whatwg-html-numeric-character-references/v1");
    assert_eq!(suite.cases.len(), 2210);

    for case in &suite.cases {
        assert_characters_match_codepoints(case);
        assert_numeric_reference_form_decodes_in_data(case, &case.decimal, false);
        assert_numeric_reference_form_decodes_in_data(case, &case.hex, false);
        assert_numeric_reference_form_decodes_in_data(case, &case.decimal_missing_semicolon, true);
        assert_numeric_reference_form_decodes_in_data(case, &case.hex_missing_semicolon, true);
    }
}

#[test]
fn whatwg_numeric_character_references_decode_in_attributes() {
    let suite = load_suite();

    for case in &suite.cases {
        assert_numeric_reference_form_decodes_in_attribute(case, &case.decimal, false);
        assert_numeric_reference_form_decodes_in_attribute(case, &case.hex, false);
        assert_numeric_reference_form_decodes_in_attribute(
            case,
            &case.decimal_missing_semicolon,
            true,
        );
        assert_numeric_reference_form_decodes_in_attribute(case, &case.hex_missing_semicolon, true);
    }
}

#[test]
fn whatwg_numeric_character_references_decode_in_rcdata() {
    let suite = load_suite();

    for case in &suite.cases {
        assert_numeric_reference_form_decodes_in_rcdata(case, &case.decimal, false);
        assert_numeric_reference_form_decodes_in_rcdata(case, &case.hex, false);
        assert_numeric_reference_form_decodes_in_rcdata(
            case,
            &case.decimal_missing_semicolon,
            true,
        );
        assert_numeric_reference_form_decodes_in_rcdata(case, &case.hex_missing_semicolon, true);
    }
}

#[test]
fn digitless_numeric_references_stay_literal_while_reporting_absence_of_digits() {
    for reference in ["&#;", "&#x;", "&#X;"] {
        let (tokens, diagnostics) = lex_data(&format!("before {reference} after"));
        assert_eq!(
            tokens,
            vec![Token::Text(format!("before {reference} after")), Token::Eof],
            "data reference {reference}"
        );
        assert_eq!(
            diagnostics,
            vec!["absence-of-digits-in-numeric-character-reference"],
            "data reference {reference}"
        );

        let (tokens, diagnostics) = lex_data(&format!("<p title='before {reference} after'>"));
        assert_eq!(
            single_title_attribute(&tokens),
            format!("before {reference} after"),
            "attribute reference {reference}"
        );
        assert_eq!(
            diagnostics,
            vec!["absence-of-digits-in-numeric-character-reference"],
            "attribute reference {reference}"
        );

        let (tokens, diagnostics) = lex_title_rcdata(&format!("before {reference} after"));
        assert_eq!(
            tokens,
            vec![Token::Text(format!("before {reference} after")), Token::Eof],
            "RCDATA reference {reference}"
        );
        assert_eq!(
            diagnostics,
            vec!["absence-of-digits-in-numeric-character-reference"],
            "RCDATA reference {reference}"
        );
    }
}

fn load_suite() -> NumericSuite {
    serde_json::from_str(WHATWG_NUMERIC_REFERENCES)
        .expect("WHATWG numeric reference fixture should parse")
}

fn assert_characters_match_codepoints(case: &NumericCase) {
    let actual = case
        .characters
        .chars()
        .map(|ch| ch as u32)
        .collect::<Vec<_>>();
    assert_eq!(actual, case.codepoints, "value U+{:X}", case.value);
}

fn assert_numeric_reference_form_decodes_in_data(
    case: &NumericCase,
    reference: &str,
    missing_semicolon: bool,
) {
    let source = format!("before {reference}/ after");
    let expected = format!("before {}/ after", case.characters);
    let (tokens, diagnostics) = lex_data(&source);

    assert_eq!(
        tokens,
        vec![Token::Text(expected), Token::Eof],
        "data value U+{:X} reference {reference}",
        case.value
    );
    assert_diagnostics(case, missing_semicolon, diagnostics, reference);
}

fn assert_numeric_reference_form_decodes_in_attribute(
    case: &NumericCase,
    reference: &str,
    missing_semicolon: bool,
) {
    let source = format!("<p title='before {reference}/ after'>");
    let expected = format!("before {}/ after", case.characters);
    let (tokens, diagnostics) = lex_data(&source);

    assert_eq!(
        single_title_attribute(&tokens),
        expected,
        "attribute value U+{:X} reference {reference}",
        case.value
    );
    assert_diagnostics(case, missing_semicolon, diagnostics, reference);
}

fn assert_numeric_reference_form_decodes_in_rcdata(
    case: &NumericCase,
    reference: &str,
    missing_semicolon: bool,
) {
    let source = format!("before {reference}/ after");
    let expected = format!("before {}/ after", case.characters);
    let (tokens, diagnostics) = lex_title_rcdata(&source);

    assert_eq!(
        tokens,
        vec![Token::Text(expected), Token::Eof],
        "RCDATA value U+{:X} reference {reference}",
        case.value
    );
    assert_diagnostics(case, missing_semicolon, diagnostics, reference);
}

fn assert_diagnostics(
    case: &NumericCase,
    missing_semicolon: bool,
    diagnostics: Vec<String>,
    reference: &str,
) {
    let mut expected = case.diagnostics.clone();
    if missing_semicolon {
        expected.push(MISSING_SEMICOLON.to_string());
    }

    assert_eq!(
        diagnostics, expected,
        "value U+{:X} reference {reference}",
        case.value
    );
}

fn lex_data(source: &str) -> (Vec<Token>, Vec<String>) {
    let mut lexer = create_html_lexer().expect("HTML lexer should build");
    lexer.push(source).expect("push should succeed");
    lexer.finish().expect("finish should succeed");
    let diagnostics = lexer
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.code.clone())
        .collect::<Vec<_>>();
    (lexer.drain_tokens(), diagnostics)
}

fn lex_title_rcdata(source: &str) -> (Vec<Token>, Vec<String>) {
    let context = HtmlLexContext::for_element_text("title").expect("title should map to RCDATA");
    let mut lexer = create_html_lexer_with_context(&context).expect("HTML lexer should build");
    lexer.push(source).expect("push should succeed");
    lexer.finish().expect("finish should succeed");
    let diagnostics = lexer
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.code.clone())
        .collect::<Vec<_>>();
    (lexer.drain_tokens(), diagnostics)
}

fn single_title_attribute(tokens: &[Token]) -> String {
    match tokens {
        [Token::StartTag {
            name,
            attributes,
            self_closing,
        }, Token::Eof] => {
            assert_eq!(name, "p");
            assert!(!self_closing);
            assert_eq!(attributes.len(), 1);
            assert_eq!(attributes[0].name, "title");
            attributes[0].value.clone()
        }
        other => panic!("expected one start tag plus EOF, got {other:?}"),
    }
}
