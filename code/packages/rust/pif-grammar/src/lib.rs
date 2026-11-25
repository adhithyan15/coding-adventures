//! Parse harness for the Proof Interchange Format grammar (VER32 section 4).
//!
//! The canonical grammar lives in `code/grammars/pif/`; this crate embeds both
//! files and exposes [`parse`], which runs the repository's grammar-driven
//! lexer and parser over a byte string. It exists so the grammar has tests that
//! run in CI (PIF-001); the trusted reader (PIF-002) is a separate program.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;
use parser::grammar_parser::{GrammarASTNode, GrammarParser, DEFAULT_MAX_RULE_DEPTH};

/// The canonical token grammar, `code/grammars/pif/pif.tokens`.
pub const TOKENS: &str = include_str!("../../../../grammars/pif/pif.tokens");
/// The canonical parser grammar, `code/grammars/pif/pif.grammar`.
pub const GRAMMAR: &str = include_str!("../../../../grammars/pif/pif.grammar");

/// Lex `source` with the canonical PIF tokens.  A byte no token matches is a
/// lexical error, reported as `lex: ...`.
pub fn lex(source: &str) -> Result<Vec<Token>, String> {
    let token_grammar =
        grammar_tools::token_grammar::parse_token_grammar(TOKENS).map_err(|e| e.to_string())?;
    GrammarLexer::new(source, &token_grammar)
        .tokenize()
        .map_err(|e| format!("lex: {e}"))
}

/// The grammar name of a token: its custom type name (`FNAT`, `KEYWORD`, ...)
/// or, for the built-in kinds, the enum's name (`Newline`, `Eof`).
pub fn kind(token: &Token) -> String {
    token
        .type_name
        .clone()
        .unwrap_or_else(|| format!("{:?}", token.type_))
}

/// Lex and parse `source` with the canonical PIF grammar.
///
/// Returns the parse tree, or a string naming the stage that refused
/// (`lex:` for a lexical error, `parse:` for a grammar error).
pub fn parse(source: &str) -> Result<GrammarASTNode, String> {
    let parser_grammar =
        grammar_tools::parser_grammar::parse_parser_grammar(GRAMMAR).map_err(|e| e.to_string())?;
    let tokens = lex(source)?;
    GrammarParser::new(tokens, parser_grammar)
        .with_max_depth(DEFAULT_MAX_RULE_DEPTH)
        .parse()
        .map_err(|e| format!("parse: {e}"))
}
