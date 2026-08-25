//! # java-to-semantic-ir
//!
//! Java CST → narrow-waist Semantic IR, **v0.3.0**.
//!
//! This is the first frontend for [SIR29](../../../specs/SIR29-nominal-static-oop-profile.md),
//! the nominal/static-dispatch OOP profile extension of the SIR10 narrow-waist
//! IR (see [JV02](../../../specs/JV02-java-to-semantic-ir.md) for this
//! frontend's full milestone plan). It consumes the generic `GrammarASTNode`
//! CST produced by the `coding-adventures-java-parser` crate and emits a
//! [`semantic_ir::Module`].
//!
//! ## Pipeline
//!
//! ```text
//! Java source
//!    │
//!    ▼  coding_adventures_java_parser::parse_java(src, "21")
//! parser::grammar_parser::GrammarASTNode   (generic CST)
//!    │
//!    ▼  java_to_semantic_ir::compile
//! semantic_ir::Module                      (per SIR10 + SIR17 + SIR29)
//! ```
//!
//! ## Public API
//!
//! ```
//! use java_to_semantic_ir::compile_source;
//! let module = compile_source(
//!     "class Main { public static void main(String[] args) { 42; } }",
//!     "demo",
//! )
//! .unwrap();
//! assert!(module.functions.iter().any(|f| f.name == "main"));
//! ```
//!
//! ## Scope (v0.3.0 — JV02 milestones M0 + M1 + M2a)
//!
//! Java requires an explicit `class`/`main`-method wrapper at the source
//! level (unlike Ruby/Python/JS, which allow bare top-level statements) —
//! this crate recognizes exactly that minimal shape: one top-level class
//! declaring a `public static void main(String[] args)` method. Supported
//! so far: literal expressions (`42`/`3.14`/`true`/`false`/`null`/
//! `"str"`, M0); local variable declarations (explicit primitive/
//! `String` types, or `var` type inference), re-assignment, arithmetic/
//! comparison/logical operators, and `+`-based string concatenation
//! (M1); `if`/`else`, `while`, `do`/`while`, and compound-assignment/
//! increment/decrement as bare statements (M2a) — every `if`/`while`/
//! `do`-`while` body is its own real lexical scope, mirroring the SIR
//! validator's own block-scoping contract exactly. Everything else
//! (`for`-loops, `switch`, method calls, field/array access, lambdas,
//! casts, additional classes or methods, non-`main` entry shapes) is out
//! of scope so far and returns a clean [`JavaLowerError`] rather than
//! being silently mis-lowered — see `lower.rs`'s own module doc for the
//! exact boundary and JV02's own milestone table for what comes next.

mod lower;
pub use lower::{compile, JavaLowerError};

/// Parse `source` as Java (default version `"21"`, matching
/// `coding_adventures_java_lexer::DEFAULT_VERSION`) and lower it into a
/// [`semantic_ir::Module`] in one step, mirroring every other
/// `-to-semantic-ir` frontend's `compile_source` convenience wrapper.
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<semantic_ir::Module, JavaLowerError> {
    let tree = coding_adventures_java_parser::parse_java(
        source,
        coding_adventures_java_lexer::DEFAULT_VERSION,
    )
    .map_err(|msg| JavaLowerError {
        message: format!("parse error: {msg}"),
        line: 1,
        column: 1,
    })?;
    compile(&tree, module_name)
}
