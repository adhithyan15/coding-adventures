//! Grammar-driven APL tokenizer.
//!
//! The token grammar is compiled into this crate at build time, so runtime
//! callers do not need filesystem access to `code/grammars/apl`.
//!
//! Per MA05 §3 bullet 4, every APL primitive in this historical-core subset
//! is a single dedicated Unicode code point (see `code/grammars/apl/apl.tokens`),
//! so this crate needs no pre/post-tokenize hooks — unlike, say, `wolfram-lexer`,
//! which strips bracketed newlines. Which glyph reading applies (monadic vs.
//! dyadic) is a parser-production concern, not a lexer one (MA05 §3 bullet 3).

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

pub fn create_apl_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    GrammarLexer::new(source, &grammar)
}

pub fn tokenize_apl(source: &str) -> Vec<Token> {
    let mut lexer = create_apl_lexer(source);
    lexer
        .tokenize()
        .unwrap_or_else(|err| panic!("APL tokenization failed: {err}"))
}
