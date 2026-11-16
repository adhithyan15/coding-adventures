//! # ActionScript Compiler (SWF03)
//!
//! Source-level compiler for ActionScript 1.0, 2.0, and 3.0 targeting
//! InterpreterIR (`LANG01`) and execution on `vm-core` (`LANG02`).
//!
//! Supports:
//! - AS1: dynamic prototype-based ECMAScript 3 subset with Flash timeline verbs (`stop`, `play`, `gotoAndPlay`, `gotoAndStop`, `tellTarget`, `call`, `eval`, `onClipEvent`), `_root`/`_parent`/`_this` paths.
//! - AS2: compile-time classes, interfaces, type annotations (`var x: Number`), visibility (`public`/`private`/`static`), lowered into prototype/constructor IR.
//! - AS3: packages, imports, namespaces, const, typed vectors (`Vector.<T>`), try/catch/finally, `is`/`as`, sealed class metadata.

#![deny(unsafe_code)]

pub mod ast;
pub mod capabilities;
pub mod dialect;
pub mod error;
pub mod lexer;
pub mod lowering;
pub mod parser;
pub mod runner;
pub mod token;

pub use capabilities::PureCapabilities;
pub use dialect::AsDialect;
pub use error::{AsCompilerError, MAX_AST_DEPTH, MAX_AST_NODES, MAX_INSTRUCTIONS, MAX_TOKENS};
pub use lexer::AsLexer;
pub use lowering::AsLowering;
pub use parser::AsParser;
pub use runner::AsVmRunner;
pub use token::{Span, Token, TokenKind};

/// Compile ActionScript source into an `interpreter_ir::module::IIRModule`.
pub fn compile_actionscript(
    source: &str,
    dialect: AsDialect,
    module_name: &str,
) -> Result<interpreter_ir::module::IIRModule, AsCompilerError> {
    let mut lexer = AsLexer::lexer_new(source, dialect);
    let tokens = lexer.lexer_tokenize()?;
    let mut parser = AsParser::parser_new(tokens, dialect);
    let program = parser.parser_parse_program()?;
    let mut lowering = AsLowering::lowering_new(dialect, module_name.to_string());
    lowering.lowering_compile_program(&program)
}

/// Compile and execute ActionScript source, capturing trace output.
pub fn run_actionscript(source: &str, dialect: AsDialect) -> Result<String, AsCompilerError> {
    let module = compile_actionscript(source, dialect, "as_script")?;
    let mut runner = AsVmRunner::runner_new();
    let _ = runner.runner_execute(&module)?;
    Ok(runner.runner_output())
}

/// Validates that an ActionScript dialect code is within supported bounds (1..=3).
#[inline]
pub fn validate_dialect(d: usize) -> Result<bool, AsCompilerError> {
    if (1..=3).contains(&d) {
        Ok(true)
    } else {
        Err(AsCompilerError::UnsupportedDialect)
    }
}

/// Validates that token count does not exceed security limit.
#[inline]
pub fn validate_token_count(n: usize) -> Result<bool, AsCompilerError> {
    if n <= MAX_TOKENS {
        Ok(true)
    } else {
        Err(AsCompilerError::TokenLimitExceeded)
    }
}

/// Validates that instruction count does not exceed security limit.
#[inline]
pub fn validate_instruction_count(n: usize) -> Result<bool, AsCompilerError> {
    if n <= MAX_INSTRUCTIONS {
        Ok(true)
    } else {
        Err(AsCompilerError::InstructionLimitExceeded)
    }
}
