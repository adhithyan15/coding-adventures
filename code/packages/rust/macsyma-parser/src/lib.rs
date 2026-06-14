//! Grammar-driven MACSYMA parser.
//!
//! The parser grammar is compiled into this crate at build time, so runtime
//! callers do not need filesystem access to `code/grammars/macsyma`.

use coding_adventures_macsyma_lexer::tokenize_macsyma;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

pub fn create_macsyma_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_macsyma(source);
    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar)
}

pub fn parse_macsyma(source: &str) -> GrammarASTNode {
    let mut parser = create_macsyma_parser(source);
    parser
        .parse()
        .unwrap_or_else(|err| panic!("MACSYMA parse failed: {err}"))
}
