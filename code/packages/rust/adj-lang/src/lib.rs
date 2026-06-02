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
//! ## Surface syntax (v0.1)
//!
//! ```adj
//! % comments start with %
//!
//! prior 0.10 for acs
//!   source "Pope JH et al., NEJM 1995;342(16):1163-70"
//!
//! contributes 1.5 from pmh(hypertension) to acs
//!   source "HEART Score; Six AJ et al., 2008"
//!   trust empirical
//!
//! interacts 1.3 when symptom_quality(pressure_like)
//!                and associated_symptom(diaphoresis)
//!                for acs
//!   source "[empirical] synergy"
//!
//! observe pmh(hypertension)
//! observe symptom_quality(pressure_like)
//!
//! ? acs   % query
//! ```
//!
//! ## Pipeline
//!
//! `source text → [lexer] → tokens → [parser] → AST → [lower] →
//! (KnowledgeBase, queries)`
//!
//! The KB is then handed to [`logic_engine::search`] in
//! `SearchMode::LRAggregate` (or any other mode the caller wants).
//!
//! ## What v0.1 does NOT cover
//!
//! - **Uncertainty markers** (ADJ46 A5) — `uncertain X over {a,b,c}`
//!   is reserved syntax but not yet lowered. ADJ47-D.
//! - **Counterfactual queries** (A8) — `? acs given pmh(htn)=true`.
//!   ADJ47-D.
//! - **Source disagreement aggregation** (A9) — `contributes 1.5
//!   ... via "AHA 2021"` with multiple `via` clauses per
//!   (conclusion, evidence). ADJ47-D.
//! - **Kickback** (A7) — surfaces as an engine-layer search-result
//!   variant, not a surface keyword. Cross-cutting work.
//!
//! Each of these is a small additive change to the grammar; the
//! current parser is structured so adding a statement kind is a
//! single new arm of [`parser::parse_statement`].

pub mod ast;
pub mod lexer;
pub mod lower;
pub mod parser;

pub use ast::{Annotation, Program, Statement, Term as AstTerm};
pub use lexer::{lex, LexError, Token, TokenKind};
pub use lower::{lower, LowerError, LoweredProgram};
pub use parser::{parse, ParseError};

/// Top-level convenience: source text → lowered program (KB + queries).
///
/// Returns the first error encountered in either the lexer or the
/// parser if anything fails. Mainly for tests and the
/// `examples/`-style invocation; production callers will typically
/// split into [`lex`], [`parse`], and [`lower`] so they can route
/// errors to their own diagnostic surface.
pub fn compile(src: &str) -> Result<LoweredProgram, CompileError> {
    let tokens = lex(src).map_err(CompileError::Lex)?;
    let program = parse(&tokens).map_err(CompileError::Parse)?;
    lower(&program).map_err(CompileError::Lower)
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompileError {
    Lex(LexError),
    Parse(ParseError),
    Lower(LowerError),
}
