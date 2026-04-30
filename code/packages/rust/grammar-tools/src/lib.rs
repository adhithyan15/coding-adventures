//! # Grammar Tools — parsing and validating language grammar specifications.
//!
//! Every programming language has two levels of structure:
//!
//! 1. **Lexical grammar** (.tokens files) — what are the "words"?
//!    Tokens like `NUMBER`, `PLUS`, `IF` are the atoms of the language.
//!    A lexer breaks raw source code into these tokens.
//!
//! 2. **Syntactic grammar** (.grammar files) — what are the "sentences"?
//!    Rules like `expression = term { PLUS term }` describe how tokens
//!    combine into meaningful structures. A parser reads tokens and builds
//!    an abstract syntax tree (AST) according to these rules.
//!
//! This crate reads both kinds of grammar files and produces structured
//! Rust data types that downstream tools (lexer generators, parser
//! generators, syntax highlighters) can consume.
//!
//! # The chicken-and-egg problem
//!
//! To read grammar files, we need a parser. But grammar files *define*
//! parsers. How do we parse the grammar without already having a parser?
//!
//! The answer: we write a simple hand-coded parser (called a "recursive
//! descent parser") that understands the fixed EBNF notation used in
//! .grammar files. This hand-written parser is the bootstrap — it reads
//! the grammar that will eventually be used to generate more sophisticated
//! parsers.
//!
//! # Crate structure
//!
//! - **[`token_grammar`]** — parse and validate `.tokens` files.
//! - **[`parser_grammar`]** — parse and validate `.grammar` files (EBNF).
//! - **[`cross_validator`]** — check that `.tokens` and `.grammar` files
//!   are consistent with each other.

pub mod codegen;
pub mod compiler;
pub mod cross_validator;
pub mod parser_grammar;
pub mod token_grammar;
