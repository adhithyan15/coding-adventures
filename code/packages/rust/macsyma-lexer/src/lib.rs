//! Grammar-driven MACSYMA tokenizer.
//!
//! The token grammar is compiled into this crate at build time, so runtime
//! callers do not need filesystem access to `code/grammars/macsyma`.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

pub fn create_macsyma_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    GrammarLexer::new(source, &grammar)
}

pub fn tokenize_macsyma(source: &str) -> Vec<Token> {
    let mut lexer = create_macsyma_lexer(source);
    lexer
        .tokenize()
        .unwrap_or_else(|err| panic!("MACSYMA tokenization failed: {err}"))
}
