use std::collections::HashSet;

use grammar_tools::{
    compiler::{compile_parser_grammar, compile_token_grammar},
    cross_validator::cross_validate,
    parser_grammar::{parse_parser_grammar, validate_parser_grammar},
    token_grammar::{parse_token_grammar, token_names, validate_token_grammar},
};

const TOKENS: &str = include_str!("../../../../grammars/spice/berkeley.tokens");
const GRAMMAR: &str = include_str!("../../../../grammars/spice/berkeley.grammar");

#[test]
fn berkeley_spice_grammar_validates_and_compiles() {
    let token_grammar = parse_token_grammar(TOKENS).expect("parse berkeley.tokens");
    let parser_grammar = parse_parser_grammar(GRAMMAR).expect("parse berkeley.grammar");

    let token_issues = validate_token_grammar(&token_grammar);
    assert!(
        token_issues.is_empty(),
        "token grammar issues:\n{}",
        token_issues.join("\n")
    );

    let token_names: HashSet<String> = token_names(&token_grammar);
    let parser_issues = validate_parser_grammar(&parser_grammar, Some(&token_names));
    assert!(
        parser_issues.is_empty(),
        "parser grammar issues:\n{}",
        parser_issues.join("\n")
    );

    let cross_issues = cross_validate(&token_grammar, &parser_grammar);
    assert!(
        cross_issues.is_empty(),
        "cross-validation issues:\n{}",
        cross_issues.join("\n")
    );

    let compiled_tokens = compile_token_grammar(&token_grammar, "spice/berkeley.tokens");
    assert!(compiled_tokens.contains("pub fn token_grammar() -> TokenGrammar"));

    let compiled_parser = compile_parser_grammar(&parser_grammar, "spice/berkeley.grammar");
    assert!(compiled_parser.contains("pub fn parser_grammar() -> ParserGrammar"));
}
