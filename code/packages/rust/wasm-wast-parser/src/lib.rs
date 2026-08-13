//! # wasm-wast-parser
//!
//! Parses the WebAssembly **text** format — both plain `.wat` modules and
//! the official spec testsuite's `.wast` script dialect (`assert_return`,
//! `assert_trap`, `invoke`, and friends) — into [`wasm_types::WasmModule`]
//! and a sequence of [`script::Directive`]s.
//!
//! ## Why this crate exists
//!
//! Every other WASM crate in this repo (`wasm-module-parser`,
//! `wasm-module-encoder`) speaks the **binary** format only. The official
//! WebAssembly spec testsuite ships almost entirely as `.wast` text files —
//! this repo cannot run it without a text-format front end, and none
//! existed. See `code/specs/W05-wasm-conformance-harness.md` for the full
//! design.
//!
//! ## Pipeline
//!
//! ```text
//! source text
//!   │  tokenizer::tokenize
//!   ▼
//! flat token stream
//!   │  module::parse_module        (a single `(module ...)` form)
//!   │  script::parse_script        (a whole .wast file: modules + directives)
//!   ▼
//! wasm_types::WasmModule  +  Vec<script::Directive>
//! ```
//!
//! `module::parse_module` does the real work: flattening folded
//! instructions, resolving symbolic identifiers (`$name`) to numeric
//! indices per their own scope, deduplicating implicit function types, and
//! **encoding** each function body straight to the same raw WASM bytecode
//! `wasm-module-parser` would have produced from a binary file — this
//! crate produces exactly the `WasmModule` shape every other WASM crate
//! already consumes, not a parallel text-specific AST.

pub mod module;
pub mod numeric;
pub mod script;
pub mod sexpr;
pub mod tokenizer;

pub use module::parse_module;
pub use script::{parse_script, Directive};

/// Every error this crate can produce, carrying a byte offset into the
/// source for error messages. Distinct from
/// [`wasm_module_parser::WasmParseError`] on purpose — a harness grading
/// `assert_malformed` needs to tell "our text parser rejected this" apart
/// from "our binary parser rejected this."
#[derive(Debug, Clone, PartialEq)]
pub enum WastParseError {
    UnterminatedBlockComment { pos: usize },
    UnterminatedString { pos: usize },
    InvalidEscape { pos: usize },
    InvalidUtf8 { pos: usize },
    UnexpectedByte { pos: usize, byte: u8 },
    UnexpectedEof,
    UnexpectedToken { pos: usize, found: String, expected: &'static str },
    UnknownInstruction { pos: usize, name: String },
    UnknownIdentifier { pos: usize, name: String, space: &'static str },
    DuplicateIdentifier { pos: usize, name: String, space: &'static str },
    InvalidNumericLiteral { pos: usize, text: String },
    InvalidNumericLiteralForType { pos: usize, text: String, ty: &'static str },
    /// Nested `(...)` forms deeper than [`sexpr::MAX_NESTING_DEPTH`] — a
    /// clean, catchable error instead of a hard stack overflow (an abort,
    /// not a `panic!`, and NOT catchable by any Rust code) on adversarially
    /// deep input like `((((((...))))))`.
    TooDeeplyNested { pos: usize },
}

impl std::fmt::Display for WastParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WastParseError::UnterminatedBlockComment { pos } => {
                write!(f, "unterminated block comment starting at byte {pos}")
            }
            WastParseError::UnterminatedString { pos } => {
                write!(f, "unterminated string literal starting at byte {pos}")
            }
            WastParseError::InvalidEscape { pos } => write!(f, "invalid string escape at byte {pos}"),
            WastParseError::InvalidUtf8 { pos } => write!(f, "invalid UTF-8 at byte {pos}"),
            WastParseError::UnexpectedByte { pos, byte } => {
                write!(f, "unexpected byte {byte:#04x} at offset {pos}")
            }
            WastParseError::UnexpectedEof => write!(f, "unexpected end of input"),
            WastParseError::UnexpectedToken { pos, found, expected } => {
                write!(f, "at byte {pos}: expected {expected}, found {found:?}")
            }
            WastParseError::UnknownInstruction { pos, name } => {
                write!(f, "at byte {pos}: unknown instruction {name:?}")
            }
            WastParseError::UnknownIdentifier { pos, name, space } => {
                write!(f, "at byte {pos}: unknown {space} identifier {name:?}")
            }
            WastParseError::DuplicateIdentifier { pos, name, space } => {
                write!(f, "at byte {pos}: duplicate {space} identifier {name:?}")
            }
            WastParseError::InvalidNumericLiteral { pos, text } => {
                write!(f, "at byte {pos}: invalid numeric literal {text:?}")
            }
            WastParseError::InvalidNumericLiteralForType { pos, text, ty } => {
                write!(f, "at byte {pos}: {text:?} is not a valid {ty} literal")
            }
            WastParseError::TooDeeplyNested { pos } => {
                write!(f, "at byte {pos}: nesting depth exceeds the parser's limit")
            }
        }
    }
}

impl std::error::Error for WastParseError {}
