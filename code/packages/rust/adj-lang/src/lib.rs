//! # adj-lang — surface syntax for the adjudication framework
//!
//! A small probabilistic-logic DSL whose programs lower directly to
//! a `logic-engine` [`KnowledgeBase`]. Designed to be readable by a
//! domain expert (ED physician, M&A lawyer, security researcher,
//! investigative journalist) without a Rust compiler in scope.
//!
//! Dissolves ADJ46 awkwardness items
//! [A10](../../../specs/data/adj46/AWKWARDNESS.md) (rulebook surface
//! is hand-written Rust) and A4 (joint contributions look like
//! ordinary multi-body rules) by making each clause kind a distinct
//! keyword.
//!
//! ## Pipeline
//!
//! ```text
//!  .adj source
//!     │ [GrammarLexer with adj_lang.tokens]
//!     ▼
//!  Vec<Token>
//!     │ [GrammarParser with adj_lang.grammar]
//!     ▼
//!  GrammarASTNode (generic parse tree)
//!     │ [adapter::adapt_program]
//!     ▼
//!  ast::Program (typed)
//!     │ [lower::lower]
//!     ▼
//!  LoweredProgram { kb, queries }
//! ```
//!
//! The lexer and parser are not hand-written: they're driven by
//! `code/grammars/adj_lang.tokens` and `code/grammars/adj_lang.grammar`,
//! compiled into `_lexer_grammar.rs` / `_parser_grammar.rs` by the
//! `grammar-tools` CLI. This crate is therefore conformant with the
//! rest of the repo's grammar-driven language frontends — the same
//! grammars can be reused by any other language port of the
//! adj-lang frontend.
//!
//! See [`code/grammars/adj_lang.tokens`](../../../grammars/adj_lang.tokens)
//! and [`code/grammars/adj_lang.grammar`](../../../grammars/adj_lang.grammar)
//! for the canonical source of truth.

pub mod adapter;
pub mod ast;
pub mod lower;

mod _lexer_grammar;
mod _parser_grammar;

use lexer::grammar_lexer::GrammarLexer;
use parser::grammar_parser::{GrammarParseError, GrammarParser};

pub use adapter::{adapt_program, AdapterError};
pub use ast::{Annotation, Program, Statement, Term as AstTerm};
pub use lower::{lower, LowerError, LoweredProgram};

/// Result of compilation. Either the typed program produced by the
/// adapter, or an error from the lexer, parser, adapter, or
/// lowering stage.
#[derive(Debug)]
pub enum CompileError {
    Lex(String),
    Parse(GrammarParseError),
    Adapt(AdapterError),
    Lower(LowerError),
}

/// Tokenize + parse + adapt: produce a typed [`Program`] from
/// source text.
pub fn parse(src: &str) -> Result<Program, CompileError> {
    let token_grammar = _lexer_grammar::token_grammar();
    let mut grammar_lexer = GrammarLexer::new(src, &token_grammar);
    let tokens = grammar_lexer
        .tokenize()
        .map_err(|e| CompileError::Lex(format!("{e}")))?;
    let parser_grammar = _parser_grammar::parser_grammar();
    let mut grammar_parser = GrammarParser::new(tokens, parser_grammar);
    let ast = grammar_parser.parse().map_err(CompileError::Parse)?;
    adapt_program(&ast).map_err(CompileError::Adapt)
}

/// Top-level convenience: source text → lowered program (KB +
/// queries).
pub fn compile(src: &str) -> Result<LoweredProgram, CompileError> {
    let program = parse(src)?;
    lower(&program).map_err(CompileError::Lower)
}
