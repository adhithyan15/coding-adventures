//! # java-to-semantic-ir
//!
//! Java CST → narrow-waist Semantic IR, **v0.10.0**.
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
//! ## Scope (v0.10.0 — JV02 milestones M0 + M1 + M2a + M2b + M3a + M3b + M4a + M4b + M4c + M4d)
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
//! increment/decrement as bare statements (M2a); classic `for` (desugared
//! to `while`, since SIR's `Stmt::ForRange` is a canonical counting loop
//! too narrow for Java's fully general three-clause form) and enhanced
//! `for` (→ `Stmt::ForEach` directly, M2b) — every block, including a
//! classic `for`'s own init-declared variable's scope, is a real lexical
//! scope, mirroring the SIR validator's own block-scoping contract
//! exactly; every method in the class body (static or instance, typed
//! parameters, two-pass name resolution so forward references and
//! recursion work), bare unqualified calls (`Expr::DirectCall`), and
//! `return` in tail position only (M3a); lambda expressions with
//! explicitly-typed parameters (`Expr::MakeClosure`, hoisting the body to
//! a synthesized top-level function), captures discovered on-resolve
//! (effectively-final enforced — assigning to a captured local is
//! rejected), and both lambda-body shapes (M3b, though a lambda value
//! can only be created and passed around this milestone, never actually
//! *invoked* — see `lower.rs`'s own module doc comment); single-
//! dimensional array types with a bare `{ ... }` literal initializer
//! (`Expr::SeqLit`), indexing reads (`Expr::SeqIndex`), and `.length`
//! (`Expr::SeqLen`) — enough for a real `for (int i = 0; i < xs.length;
//! i++) { ...xs[i]... }` loop (M4a); and plain indexed assignment
//! (`xs[i] = v;` → `Stmt::SeqSet`, M4b — compound-assignment/increment-
//! decrement on an indexed target remain deferred, since naively
//! lowering either would double-evaluate the index expression); and
//! `new`-based array-creation expressions (M4c) — `new int[]{1, 2, 3}`
//! (delegates to the same array-literal lowering M4a already built) and
//! `new int[N]` (a compile-time-constant, non-negative, capped-size
//! sized/uninitialized array, zero-filled — a non-constant size needs a
//! repeat/fill SIR primitive that doesn't exist yet, so is deferred
//! rather than attempted); and real multi-dimensional arrays (M4d) —
//! array types and explicitly-typed literal declarations (`int[][] grid
//! = {{1,2},{3,4}}`, including genuinely ragged rows), and chained index
//! reads (`grid[i][j]`) via a generalized `lower_primary_expression`
//! suffix-chain dispatch, capped at a small dimension limit. A *mixed*
//! index-then-`.length` chain (`grid[i].length`) and a *chained*
//! indexed-assignment target (`grid[i][j] = v;`) remain deferred.
//! Everything else (`switch`, `break`/`continue` — SIR has no IR
//! primitive for either — qualified calls, method overloading, untyped/
//! `var`-inferred lambda parameters, indirect calls through a closure
//! value, `var`-inferred multi-dimensional array literals, multi-
//! dimensional `new` array-creation forms, compound-assignment/
//! increment-decrement on an indexed target, a non-constant or
//! reference-typed `new T[N]`, field/array *field* access beyond
//! `.length`, casts, additional classes, non-`main` entry shapes) is out
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
