//! Compiler error types and bounds for ActionScript compilation.

#![deny(unsafe_code)]

use std::fmt;

/// Maximum allowed token count from lexer.
pub const MAX_TOKENS: usize = 100_000;

/// Maximum allowed AST recursion depth.
pub const MAX_AST_DEPTH: usize = 64;

/// Maximum allowed AST nodes.
pub const MAX_AST_NODES: usize = 50_000;

/// Maximum allowed instructions emitted in IIR module.
pub const MAX_INSTRUCTIONS: usize = 200_000;

/// Error variants encountered during ActionScript lexing, parsing, and lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsCompilerError {
    UnsupportedDialect,
    RecursionDepthExceeded,
    TokenLimitExceeded,
    InstructionLimitExceeded,
    SyntaxError(String),
    LoweringError(String),
    ExecutionError(String),
}

impl fmt::Display for AsCompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDialect => {
                write!(f, "unsupported ActionScript dialect (must be 1..=3)")
            }
            Self::RecursionDepthExceeded => {
                write!(f, "recursion depth exceeded limit of {}", MAX_AST_DEPTH)
            }
            Self::TokenLimitExceeded => {
                write!(f, "token count exceeded limit of {}", MAX_TOKENS)
            }
            Self::InstructionLimitExceeded => {
                write!(
                    f,
                    "instruction count exceeded limit of {}",
                    MAX_INSTRUCTIONS
                )
            }
            Self::SyntaxError(msg) => write!(f, "syntax error: {}", msg),
            Self::LoweringError(msg) => write!(f, "lowering error: {}", msg),
            Self::ExecutionError(msg) => write!(f, "execution error: {}", msg),
        }
    }
}

impl std::error::Error for AsCompilerError {}
