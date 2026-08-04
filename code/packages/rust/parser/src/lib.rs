//! # Parser — recursive descent parsing from tokens to abstract syntax trees.
//!
//! This crate provides two parser implementations:
//!
//! - **`parser`** — A hand-written recursive descent parser for a Python subset.
//!   It directly constructs a typed AST from the token stream.
//!
//! - **`grammar_parser`** — A grammar-driven universal parser that reads a
//!   `.grammar` file (via `grammar-tools`) and parses any language whose
//!   grammar is defined in the project's EBNF format.
//!
//! Both produce tree structures that downstream tools (compilers, interpreters,
//! formatters) can walk to analyze or transform source code.

// The parser's `ParseError`/grammar error types carry rich diagnostic context
// (spans, expected-token sets) by value. Boxing them to satisfy `result_large_err`
// would churn every `Result<_, _>` return across the recursive-descent API for no
// behavioral gain, so the lint is allowed crate-wide.
#![allow(clippy::result_large_err)]

pub mod ast;
pub mod parser;
pub mod grammar_parser;

pub use ast::ASTNode;
pub use parser::{Parser, ParseError};
pub use grammar_parser::{
    GrammarParser, GrammarParseError, GrammarASTNode, ASTNodeOrToken, ASTVisitor,
    walk_ast, find_nodes, collect_tokens,
};
