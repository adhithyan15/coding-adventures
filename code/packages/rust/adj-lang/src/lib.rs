// The large `Err` variant is the crate's public `CompileError` enum; boxing it
// would churn the public API and all call sites for no behavior change.
#![allow(clippy::result_large_err)]
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
pub mod resolve;

mod _lexer_grammar;
mod _parser_grammar;

use lexer::grammar_lexer::GrammarLexer;
use logic_engine::Differential;
use parser::grammar_parser::{GrammarParseError, GrammarParser};

pub use adapter::{adapt_program, AdapterError};
pub use ast::{Annotation, Define, DefineKind, OptDir, Program, RelOp, Statement, Term as AstTerm};
pub use lower::{lower, ConstraintSystem, LowerError, LoweredConstraint, LoweredProgram};
pub use resolve::{resolve_imports, ImportError, ImportLimits, ImportProvider};

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

/// Result of compiling a program that may `import` other files: either an
/// import-graph failure (cycle, bound, missing/unparseable file) from the
/// [`resolve`] stage, or a lowering failure from the merged program.
#[derive(Debug)]
pub enum CompileWithImportsError {
    Import(ImportError),
    Lower(LowerError),
}

/// Import-aware compile: resolve the import graph rooted at `root_id` (driven by
/// the injected [`ImportProvider`]), then lower the merged program (MYCIN-2026
/// M3). The library performs **no** filesystem I/O — the provider is the only
/// thing that reads files, so the caller controls the sandbox. See
/// [`resolve::resolve_imports`] for the graph guarantees.
pub fn compile_with_imports(
    root_id: &str,
    provider: &dyn ImportProvider,
    limits: ImportLimits,
) -> Result<LoweredProgram, CompileWithImportsError> {
    let program =
        resolve_imports(root_id, provider, limits).map_err(CompileWithImportsError::Import)?;
    lower(&program).map_err(CompileWithImportsError::Lower)
}

/// Run a **differential** over a lowered program's `? h` query lines:
/// treat the program's queries as the competing hypotheses, rank them by
/// posterior, and return the comparative [`Differential`] decision (argmax +
/// between-hypothesis margin, with a kickback when an open uncertainty
/// could flip the ranking).
///
/// This is the natural reading of a multi-`?` adj-lang program: the queries
/// *are* the differential. A program with a single `?` yields a
/// determinate, single-hypothesis result.
pub fn decide(lowered: &LoweredProgram) -> Differential {
    logic_engine::differential(&lowered.queries, &lowered.kb)
}

/// Source text → differential decision in one step (`compile` then
/// [`decide`]).
pub fn compile_and_decide(src: &str) -> Result<Differential, CompileError> {
    let lowered = compile(src)?;
    Ok(decide(&lowered))
}
