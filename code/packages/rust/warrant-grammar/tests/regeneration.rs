//! Byte-level generated-data drift checks, independent of syntax verdicts.
use coding_adventures_warrant_grammar::{parser_grammar, token_grammar};
use grammar_tools::compiler::{compile_parser_grammar, compile_token_grammar};
use grammar_tools::cross_validator::cross_validate;
use grammar_tools::parser_grammar::parse_parser_grammar;
use grammar_tools::token_grammar::parse_token_grammar;

const TOKENS: &str = include_str!("../../../../grammars/warrant/warrant.tokens");
const GRAMMAR: &str = include_str!("../../../../grammars/warrant/warrant.grammar");

#[test]
fn canonical_grammar_cross_validation_is_clean() {
    let tokens = parse_token_grammar(TOKENS).unwrap();
    let grammar = parse_parser_grammar(GRAMMAR).unwrap();
    assert!(cross_validate(&tokens, &grammar).is_empty());
}

#[test]
fn generated_bytes_match_the_canonical_sources() {
    assert_eq!(compile_token_grammar(&parse_token_grammar(TOKENS).unwrap(), "warrant.tokens"), include_str!("../src/_tokens.rs"));
    assert_eq!(compile_parser_grammar(&parse_parser_grammar(GRAMMAR).unwrap(), "warrant.grammar"), include_str!("../src/_parser.rs"));
    // Also round-trip each actual returned artifact through the emitter, so a
    // public wrapper cannot accidentally return a different embedded grammar.
    assert_eq!(compile_token_grammar(&token_grammar(), "warrant.tokens"), include_str!("../src/_tokens.rs"));
    assert_eq!(compile_parser_grammar(&parser_grammar(), "warrant.grammar"), include_str!("../src/_parser.rs"));
}
