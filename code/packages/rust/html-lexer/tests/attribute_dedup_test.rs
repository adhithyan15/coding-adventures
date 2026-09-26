//! Duplicate attributes: the first one wins and a `duplicate-attribute` error
//! is reported, and a tag with many attributes costs time linear in them.

use coding_adventures_html_lexer::{create_html_lexer_with_context, HtmlLexContext, Token};
use std::time::{Duration, Instant};

fn lex(source: &str) -> (Vec<Token>, Vec<String>) {
    let mut lexer = create_html_lexer_with_context(&HtmlLexContext::data()).unwrap();
    lexer.push(source).unwrap();
    lexer.finish().unwrap();
    let tokens = lexer
        .drain_positioned_tokens()
        .into_iter()
        .map(|t| t.token)
        .filter(|token| !matches!(token, Token::Eof))
        .collect();
    let codes = lexer.diagnostics().iter().map(|d| d.code.clone()).collect();
    (tokens, codes)
}

#[test]
fn the_first_duplicate_wins_and_is_reported() {
    let (tokens, codes) = lex("<p a=1 b=2 a=3 b=4 c=5>");
    let Token::StartTag { attributes, .. } = &tokens[0] else {
        panic!("{tokens:?}")
    };
    let pairs: Vec<(&str, &str)> = attributes
        .iter()
        .map(|attribute| (attribute.name.as_str(), attribute.value.as_str()))
        .collect();
    assert_eq!(pairs, [("a", "1"), ("b", "2"), ("c", "5")]);
    assert_eq!(codes.iter().filter(|code| *code == "duplicate-attribute").count(), 2);
    // The next tag starts with a clean slate.
    let (tokens, codes) = lex("<p a=1><q a=2>");
    assert!(codes.is_empty(), "{codes:?}");
    assert_eq!(tokens.len(), 2);
}

#[test]
fn many_attributes_cost_linear_time() {
    let mut source = String::from("<div");
    for index in 0..30_000 {
        source.push_str(&format!(" a{index}=v"));
    }
    source.push('>');
    let started = Instant::now();
    let (tokens, _) = lex(&source);
    assert!(started.elapsed() < Duration::from_secs(30), "{:?}", started.elapsed());
    let Token::StartTag { attributes, .. } = &tokens[0] else {
        panic!("start tag")
    };
    assert_eq!(attributes.len(), 30_000);
}
