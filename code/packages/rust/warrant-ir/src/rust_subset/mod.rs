//! Confined Rust Subset grammar, AST, and parser (VER25a, VER-P022).

pub mod ast;
pub mod lexer;
pub mod parser;

pub use ast::*;
pub use lexer::{LexError, Lexer, Token, TokenKind};
pub use parser::{ParseError, Parser};

/// Parse a Rust source file in the confined subset.
pub fn parse_rust_file(source: &str) -> Result<SourceFile, ParseError> {
    let mut parser = Parser::new(source)?;
    parser.parse_file()
}
