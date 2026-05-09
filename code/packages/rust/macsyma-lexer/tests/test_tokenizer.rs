use coding_adventures_macsyma_lexer::tokenize_macsyma;
use lexer::token::TokenType;

fn token_types(source: &str) -> Vec<String> {
    tokenize_macsyma(source)
        .into_iter()
        .filter(|token| token.type_ != TokenType::Eof)
        .map(|token| token.effective_type_name().to_string())
        .collect()
}

fn token_values(source: &str) -> Vec<String> {
    tokenize_macsyma(source)
        .into_iter()
        .filter(|token| token.type_ != TokenType::Eof)
        .map(|token| token.value)
        .collect()
}

#[test]
fn tokenizes_numbers_names_percent_constants_and_history_references() {
    assert_eq!(
        token_types("42 3.14 1.5e10 x %pi % %i1 %o2"),
        vec!["NUMBER", "NUMBER", "NUMBER", "NAME", "NAME", "NAME", "NAME", "NAME"]
    );
    assert_eq!(token_values("%pi % %i1 %o2"), vec!["%pi", "%", "%i1", "%o2"]);
}

#[test]
fn uses_compiled_longest_match_operator_order() {
    assert_eq!(
        token_types("f(x) := x ** 2 <= y >= z -> q"),
        vec![
            "NAME", "LPAREN", "NAME", "RPAREN", "COLONEQ", "NAME", "STAREQ", "NUMBER", "LEQ",
            "NAME", "GEQ", "NAME", "ARROW", "NAME",
        ]
    );
}

#[test]
fn promotes_macsyma_keywords() {
    let pairs: Vec<(String, String)> = tokenize_macsyma("x and y or not false")
        .into_iter()
        .filter(|token| token.type_ != TokenType::Eof)
        .map(|token| (token.effective_type_name().to_string(), token.value))
        .collect();

    assert_eq!(
        pairs,
        vec![
            ("NAME".to_string(), "x".to_string()),
            ("KEYWORD".to_string(), "and".to_string()),
            ("NAME".to_string(), "y".to_string()),
            ("KEYWORD".to_string(), "or".to_string()),
            ("KEYWORD".to_string(), "not".to_string()),
            ("KEYWORD".to_string(), "false".to_string()),
        ]
    );
}

#[test]
fn skips_whitespace_and_comments() {
    assert_eq!(
        token_types("/* ignored */\n x + y"),
        vec!["NAME", "PLUS", "NAME"]
    );
}

#[test]
fn preserves_distinct_statement_terminators() {
    assert_eq!(token_types("x; y$"), vec!["NAME", "SEMI", "NAME", "DOLLAR"]);
}
