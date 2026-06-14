use coding_adventures_html_lexer::{
    create_html_lexer, create_html_lexer_with_context, HtmlLexContext, Token,
};
use serde::Deserialize;

const WHATWG_ENTITIES: &str = include_str!("fixtures/whatwg-entities.json");
const MISSING_SEMICOLON: &str = "missing-semicolon-after-character-reference";

#[derive(Debug, Deserialize)]
struct EntitySuite {
    format: String,
    source: String,
    entities: Vec<EntityCase>,
}

#[derive(Debug, Deserialize)]
struct EntityCase {
    name: String,
    characters: String,
    codepoints: Vec<u32>,
    semicolon: bool,
}

#[test]
fn whatwg_named_character_references_decode_in_data_state() {
    let suite = load_suite();
    assert_eq!(suite.format, "whatwg-html-entities/v1");
    assert_eq!(suite.source, "https://html.spec.whatwg.org/entities.json");
    assert_eq!(suite.entities.len(), 2231);

    for entity in &suite.entities {
        assert_characters_match_codepoints(entity);
        let source = format!("before {} after", entity.name);
        let expected = format!("before {} after", entity.characters);
        let (tokens, diagnostics) = lex_data(&source);

        assert_eq!(
            tokens,
            vec![Token::Text(expected), Token::Eof],
            "entity {}",
            entity.name
        );
        assert_missing_semicolon_shape(entity, diagnostics, &entity.name);
    }
}

#[test]
fn whatwg_named_character_references_decode_in_attributes() {
    let suite = load_suite();

    for entity in &suite.entities {
        let source = if entity.semicolon {
            format!("<p title=before{}after>", entity.name)
        } else {
            format!("<p title=before{}/after>", entity.name)
        };
        let expected = if entity.semicolon {
            format!("before{}after", entity.characters)
        } else {
            format!("before{}/after", entity.characters)
        };
        let (tokens, diagnostics) = lex_data(&source);

        assert_eq!(
            single_title_attribute(&tokens),
            expected,
            "entity {}",
            entity.name
        );
        assert_missing_semicolon_shape(entity, diagnostics, &entity.name);
    }
}

#[test]
fn semicolonless_legacy_references_stay_literal_when_ambiguous_in_attributes() {
    let suite = load_suite();

    for entity in suite.entities.iter().filter(|entity| !entity.semicolon) {
        let source = format!("<p title=before{}after>", entity.name);
        let (tokens, diagnostics) = lex_data(&source);

        assert_eq!(
            single_title_attribute(&tokens),
            format!("before{}after", entity.name),
            "entity {}",
            entity.name
        );
        assert!(
            diagnostics.iter().all(|code| code != MISSING_SEMICOLON),
            "ambiguous attribute reference {} should not report missing semicolon: {:?}",
            entity.name,
            diagnostics
        );
    }
}

#[test]
fn whatwg_named_character_references_decode_in_rcdata() {
    let suite = load_suite();

    for entity in &suite.entities {
        let source = format!("before {} after", entity.name);
        let expected = format!("before {} after", entity.characters);
        let (tokens, diagnostics) = lex_title_rcdata(&source);

        assert_eq!(
            tokens,
            vec![Token::Text(expected), Token::Eof],
            "entity {}",
            entity.name
        );
        assert_missing_semicolon_shape(entity, diagnostics, &entity.name);
    }
}

fn load_suite() -> EntitySuite {
    serde_json::from_str(WHATWG_ENTITIES).expect("WHATWG entities fixture should parse")
}

fn assert_characters_match_codepoints(entity: &EntityCase) {
    let actual = entity
        .characters
        .chars()
        .map(|ch| ch as u32)
        .collect::<Vec<_>>();
    assert_eq!(actual, entity.codepoints, "entity {}", entity.name);
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

fn assert_missing_semicolon_shape(entity: &EntityCase, diagnostics: Vec<String>, input: &str) {
    if entity.semicolon {
        assert!(
            diagnostics.iter().all(|code| code != MISSING_SEMICOLON),
            "semicolon entity {} should not report missing semicolon: {:?}",
            input,
            diagnostics
        );
    } else {
        assert!(
            diagnostics.iter().any(|code| code == MISSING_SEMICOLON),
            "semicolonless entity {} should report missing semicolon: {:?}",
            input,
            diagnostics
        );
    }
}
