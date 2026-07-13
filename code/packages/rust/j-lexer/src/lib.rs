//! Grammar-driven J tokenizer.
//!
//! The token grammar is compiled into this crate at build time, so runtime
//! callers do not need filesystem access to `code/grammars/j`.
//!
//! Unlike `apl-lexer` (MA05 §3 bullet 4: every APL primitive is a single
//! dedicated Unicode code point), J is ASCII-only, so overloaded base
//! characters need explicit `.`/`:`-suffixed digraphs to spell related but
//! distinct primitives (MA06 §1 bullet 1, §3 last bullet) — see
//! `code/grammars/j/j.tokens` for the full longest-match-first digraph
//! ordering this crate's compiled grammar embeds. As with `apl-lexer`, which
//! of a glyph's two readings (monadic vs. dyadic) applies is a
//! parser-production concern, not a lexer one (MA06 §3 last bullet).

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

pub fn create_j_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    GrammarLexer::new(source, &grammar)
}

pub fn tokenize_j(source: &str) -> Vec<Token> {
    let mut lexer = create_j_lexer(source);
    lexer
        .tokenize()
        .unwrap_or_else(|err| panic!("J tokenization failed: {err}"))
}
