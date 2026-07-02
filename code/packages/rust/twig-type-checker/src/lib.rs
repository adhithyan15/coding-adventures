//! # twig-type-checker — TW05-B: Base Static Type Checker for Twig
//!
//! This crate is the sixth stage of the Rust Twig pipeline.  It accepts a
//! parsed [`twig_parser::Program`] (from `twig-parser`), builds a type
//! environment, infers a base [`TwigKind`] for every expression, and reports
//! violations as [`type_checker_protocol::TypeErrorDiagnostic`] spans.
//!
//! ## Where it sits in the pipeline
//!
//! ```text
//! Twig source
//!     │
//!     ▼  twig_parser::parse
//! Program (typed AST from LANG48/TW05-A)
//!     │
//!     ▼  twig_type_checker::check_program   ← THIS CRATE (TW05-B)
//! TypedProgram { program, env }
//!     │
//!     ▼  twig_ir_compiler::compile_program  (conditional on typed_mode)
//! IIRModule
//! ```
//!
//! ## What TW05-B checks
//!
//! | Check | Example error |
//! |-------|--------------|
//! | Unresolved variable | `unresolved variable \`foo\`` |
//! | Call arity | `arity error: \`f\` expects 1 argument, got 2` |
//! | Non-exhaustive match | `non-exhaustive match on union \`Expr\`: unmatched variants: \`NameRef\`` |
//!
//! Refinement predicates (`(Int 0 256)`, `(Member int …)`) are recorded in
//! the AST but not checked here — that's TW05-C's job with the constraint
//! solver.
//!
//! ## Typed modes
//!
//! The `(typed …)` clause in a `(module …)` declaration controls enforcement:
//!
//! | Clause | Mode | Behaviour |
//! |--------|------|-----------|
//! | `(typed off)` or absent | `Off` / `None` | Skip type checking entirely |
//! | `(typed lenient)` | `Lenient` | Check and collect errors; `ok: true` always |
//! | `(typed strict)` | `Strict` | Check; `ok: false` when errors exist |
//!
//! The `twig-ir-compiler` reads `TypeCheckResult::ok` and either passes
//! through (lenient/off) or returns a `TwigCompileError` (strict).
//!
//! ## Example
//!
//! ```no_run
//! use twig_type_checker::{type_check, TwigKind};
//!
//! // A well-typed program.
//! let result = type_check("(define (double x) (+ x x)) (double 7)")
//!     .expect("parse should succeed");
//! assert!(result.ok);
//! assert!(result.errors.is_empty());
//!
//! // Wrong arity (but no module → Off mode → no errors).
//! let result2 = type_check("(define (f x) x) (f 1 2)")
//!     .expect("parse should succeed");
//! // No module decl → typed_mode = None → Off → ok: true, no errors.
//! assert!(result2.ok);
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod arity;
/// LANG54: `TwigRefinementBridge` — implements `lang_refinement_protocol::RefinementBridge`
/// for Twig's `Expr` AST and `TwigKind` type system.
pub mod bridge;
pub mod check;
pub mod env;
pub mod errors;
pub mod exhaustiveness;
pub mod kinds;
pub mod narrowing;
pub mod profile;

pub use env::TypeEnv;
pub use errors::TwigTypeCheckError;
pub use kinds::TwigKind;
pub use profile::TwigLanguageProfile;

// Re-export the AnnotatedNode type so callers don't need to depend on
// type-declarations directly just to get the typed_ast out.
pub use type_checker_protocol::TypeErrorDiagnostic;
pub use type_declarations::AnnotatedNode;

use twig_parser::{emit_type_declarations, parse, parse_to_ast, Program, TypedMode};
use type_checker_protocol::{TypeCheckResult, TypeChecker};

// ---------------------------------------------------------------------------
// TypedProgram
// ---------------------------------------------------------------------------

/// The output of the TW05-B type checker.
///
/// Combines the original `Program` (unchanged) with the populated `TypeEnv`
/// so downstream stages (TW05-C, IDE tools) can query the type environment
/// without re-traversal.
///
/// The `program` field is a clone of the input: TW05-B is a *pure* pass that
/// does not modify the AST.
#[derive(Debug, Clone)]
pub struct TypedProgram {
    /// The parsed program (unchanged from input).
    pub program: Program,
    /// The type environment built during Pass 1.
    pub env: TypeEnv,
}

// ---------------------------------------------------------------------------
// Public API — LANG50: GrammarASTNode path (compilation-first)
// ---------------------------------------------------------------------------

/// Parse Twig source and type-check via the generic `grammar-type-checker`,
/// returning a fully-annotated [`AnnotatedNode`] tree ready for IIR emission.
///
/// This is the **compilation-first** entry point introduced in LANG50.
/// Every node in the returned tree carries a [`type_declarations::KindDecl`]
/// that maps directly to an IIR `type_hint` via [`AnnotatedNode::iir_hint`].
///
/// ## Pipeline
///
/// ```text
/// source → parse_to_ast → GrammarASTNode
///                             │
///        parse → Program → emit_type_declarations → TypeDeclarations
///                                                         │
///                        grammar_type_checker::check ←───┘
///                             │
///                         TypeCheckResult<AnnotatedNode>
/// ```
///
/// ## Errors
///
/// Returns `Err(TwigTypeCheckError::Parse(…))` on lex/parse failure.
/// Type errors are carried in `TypeCheckResult::errors`; `ok` reflects
/// the module's `(typed …)` mode.
///
/// # Example
///
/// ```no_run
/// use twig_type_checker::type_check_source;
///
/// let result = type_check_source("(module m (typed strict)) (define x : int 42)")
///     .expect("parse should succeed");
/// assert!(result.ok);
/// // Every node has an IIR type hint — literals get "i64", "bool", etc.
/// ```
pub fn type_check_source(
    source: &str,
) -> Result<TypeCheckResult<AnnotatedNode>, TwigTypeCheckError> {
    // 1. Parse to raw grammar AST (keeps position info for error messages).
    let raw = parse_to_ast(source)?;
    // 2. Parse to typed AST (needed to emit TypeDeclarations).
    let program = parse(source)?;
    // 3. Emit language-agnostic type declarations from the typed AST.
    let decls = emit_type_declarations(&program);
    // 4. Run the generic checker with the Twig language profile.
    Ok(grammar_type_checker::check(&raw, &decls, &TwigLanguageProfile))
}

// ---------------------------------------------------------------------------
// Public API — legacy Program path (backward compat)
// ---------------------------------------------------------------------------

/// Parse and type-check a Twig source string in one call.
///
/// Returns `Err(TwigTypeCheckError::Parse(…))` if the source fails to lex
/// or parse.  Type errors (kind mismatches, arity errors, …) are *not*
/// returned as `Err` — they live in `TypeCheckResult::errors`.
///
/// Whether `ok: bool` reflects errors depends on the `(typed …)` mode
/// declared in the module header:
/// - Off / absent → `ok: true`, `errors: []` (checker skipped).
/// - Lenient → `ok: true` regardless; errors are collected as warnings.
/// - Strict → `ok: errors.is_empty()`.
///
/// # Example
///
/// ```no_run
/// use twig_type_checker::type_check;
///
/// let res = type_check("(+ 1 2)").unwrap();
/// assert!(res.ok);
/// ```
pub fn type_check(
    source: &str,
) -> Result<TypeCheckResult<TypedProgram>, TwigTypeCheckError> {
    let program = parse(source)?;
    Ok(check_program(&program, None))
}

/// Type-check an already-parsed [`Program`].
///
/// `mode_override`, when `Some`, replaces the `(typed …)` clause in
/// `program.module_info`.  Pass `None` to use the module's own declaration.
///
/// If both `mode_override` and `module_info.typed_mode` are `None`, the
/// checker returns immediately with `ok: true` and an empty error list.
pub fn check_program(
    program: &Program,
    mode_override: Option<TypedMode>,
) -> TypeCheckResult<TypedProgram> {
    // Determine the effective mode.
    let mode: TypedMode = mode_override
        .or_else(|| {
            program
                .module_info
                .as_ref()
                .and_then(|mi| mi.typed_mode.clone())
        })
        .unwrap_or(TypedMode::Off);

    // Off mode: skip entirely.
    if mode == TypedMode::Off {
        return TypeCheckResult {
            typed_ast: TypedProgram {
                program: program.clone(),
                env: TypeEnv::new(),
            },
            errors: vec![],
            ok: true,
        };
    }

    // ── Pass 1: collect declarations ──────────────────────────────────────
    let mut env = TypeEnv::new();
    check::collect_forms(program, &mut env, &mode);

    // ── Pass 2: walk expression bodies ────────────────────────────────────
    let mut scope = env::ScopeStack::new();
    let mut errors = Vec::new();
    check::check_forms(program, &env, &mut scope, &mode, &mut errors);

    // Determine ok: strict requires zero errors; lenient is always ok.
    let ok = match mode {
        TypedMode::Strict => errors.is_empty(),
        _ => true,
    };

    TypeCheckResult {
        typed_ast: TypedProgram {
            program: program.clone(),
            env,
        },
        errors,
        ok,
    }
}

// ---------------------------------------------------------------------------
// Multi-module import propagation (TW05-P Part 2 / LANG71)
// ---------------------------------------------------------------------------

/// Extract the globals exported by a type-checked module.
///
/// When checking a module that imports another, the caller can:
/// 1. Type-check the dependency with `check_program`.
/// 2. Call `extract_module_exports(&dep_program, &dep_result.typed_ast.env)`
///    to get the dependency's exported names.
/// 3. Pass the returned map as `extra_globals` to
///    `check_program_with_globals` when checking the importing module.
///
/// Only names listed in `(export …)` are included; internal helpers are
/// filtered out.  Names not found in `env.globals` (e.g. re-exports of
/// imported symbols the dep itself didn't define) are silently skipped.
///
/// # Example
///
/// ```no_run
/// use twig_type_checker::{check_program, extract_module_exports};
/// use twig_parser::parse;
///
/// let span_src = "(module compiler/span (typed strict) \
///                  (export Span make-span)) \
///                 (record Span (start : int)) \
///                 (define (make-span s) (Span s))";
/// let span_prog = parse(span_src).unwrap();
/// let span_res  = check_program(&span_prog, None);
/// let exports   = extract_module_exports(&span_prog, &span_res.typed_ast.env);
/// assert!(exports.contains_key("make-span"));
/// ```
pub fn extract_module_exports(
    program: &Program,
    env: &TypeEnv,
) -> std::collections::HashMap<String, TwigKind> {
    let export_names: Vec<String> = program
        .module_info
        .as_ref()
        .map(|mi| mi.exports.clone())
        .unwrap_or_default();

    let mut out = std::collections::HashMap::new();
    for name in export_names {
        if let Some(kind) = env.globals.get(&name) {
            out.insert(name, kind.clone());
        }
    }
    out
}

/// Type-check a program with additional globals pre-seeded from imported modules.
///
/// This is identical to [`check_program`] except that `extra_globals` is
/// merged into the [`TypeEnv`] **before** Pass 1 (`collect_forms`) runs.
/// Any name in `extra_globals` is therefore visible to the module body and
/// will not produce an "unresolved variable" error.
///
/// Names defined in the module itself (via `(define …)`, `(record …)`,
/// `(union …)`) shadow any pre-seeded entry with the same name, because
/// Pass 1 overwrites entries in `env.globals`.
///
/// ## Usage pattern
///
/// ```no_run
/// use twig_type_checker::{check_program, check_program_with_globals,
///                          extract_module_exports};
/// use twig_parser::{parse, TypedMode};
/// use std::collections::HashMap;
///
/// // 1. Check the dependency.
/// let dep_src = "(module compiler/span (typed strict) (export make-span)) \
///                (define (make-span s e) (cons s e))";
/// let dep_prog = parse(dep_src).unwrap();
/// let dep_res  = check_program(&dep_prog, None);
/// let dep_exports = extract_module_exports(&dep_prog, &dep_res.typed_ast.env);
///
/// // 2. Check the importer, seeded with the dep's exports.
/// let imp_src = "(module compiler/lexer (typed strict) \
///                  (export lex) \
///                  (import compiler/span)) \
///                (define (lex src) (make-span 0 (length src)))";
/// let imp_prog = parse(imp_src).unwrap();
/// let result = check_program_with_globals(&imp_prog, None, &dep_exports);
/// assert!(result.ok);
/// ```
pub fn check_program_with_globals(
    program: &Program,
    mode_override: Option<TypedMode>,
    extra_globals: &std::collections::HashMap<String, TwigKind>,
) -> TypeCheckResult<TypedProgram> {
    // Determine the effective mode.
    let mode: TypedMode = mode_override
        .or_else(|| {
            program
                .module_info
                .as_ref()
                .and_then(|mi| mi.typed_mode.clone())
        })
        .unwrap_or(TypedMode::Off);

    // Off mode: skip entirely.
    if mode == TypedMode::Off {
        return TypeCheckResult {
            typed_ast: TypedProgram {
                program: program.clone(),
                env: TypeEnv::new(),
            },
            errors: vec![],
            ok: true,
        };
    }

    // ── Seed env with imported globals ────────────────────────────────────
    let mut env = TypeEnv::new();
    for (name, kind) in extra_globals {
        // Only pre-seed if the name is not already a builtin.  Builtins
        // registered by TypeEnv::new() are trusted; caller-supplied entries
        // for the same name would just be redundant noise.
        env.globals.entry(name.clone()).or_insert_with(|| kind.clone());
    }

    // ── Pass 1: collect declarations ──────────────────────────────────────
    check::collect_forms(program, &mut env, &mode);

    // ── Pass 2: walk expression bodies ────────────────────────────────────
    let mut scope = env::ScopeStack::new();
    let mut errors = Vec::new();
    check::check_forms(program, &env, &mut scope, &mode, &mut errors);

    // Determine ok.
    let ok = match mode {
        TypedMode::Strict => errors.is_empty(),
        _ => true,
    };

    TypeCheckResult {
        typed_ast: TypedProgram {
            program: program.clone(),
            env,
        },
        errors,
        ok,
    }
}

// ---------------------------------------------------------------------------
// TypeChecker trait implementation
// ---------------------------------------------------------------------------

/// A unit-struct checker that implements the generic `TypeChecker` trait from
/// `type-checker-protocol`.
///
/// This allows the checker to be plugged into frameworks that expect a
/// `TypeChecker<Program, TypedProgram>` without hardcoding the module path.
///
/// ```no_run
/// use twig_type_checker::TwigTypeCheckerImpl;
/// use type_checker_protocol::TypeChecker;
/// use twig_parser::parse;
///
/// let checker = TwigTypeCheckerImpl;
/// let program = parse("42").unwrap();
/// let result = checker.check(program);
/// assert!(result.ok);
/// ```
pub struct TwigTypeCheckerImpl;

impl TypeChecker<Program, TypedProgram> for TwigTypeCheckerImpl {
    fn check(&self, ast: Program) -> TypeCheckResult<TypedProgram> {
        check_program(&ast, None)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
//
// 28 unit tests covering TW05-B (LANG50):
// - Atom kind inference (Int, Bool, Nil, Symbol)
// - Variable resolution (resolved / unresolved)
// - Define forms (value binding, function arity, annotation)
// - Arity checking (correct / too-few / too-many)
// - Type aliases (registered, opaque resolution)
// - Record and union definitions
// - Match exhaustiveness (wildcard, binding, all variants, non-exhaustive)
// - Lambda param scoping and let binding scoping
// - Begin last-kind
// - Typed modes (off / strict / lenient)
// - Direct `check_program` path
// - Parse error path
//
// Plus 12 tests covering TW05-C (LANG53):
// - refined_kind_from_range_annotation
// - refined_kind_from_membership_annotation
// - unrefined_int_annotation_stays_int
// - call_site_literal_in_range_no_error
// - call_site_literal_out_of_range_error
// - call_site_unconstrained_lenient_silent
// - call_site_unconstrained_strict_error
// - narrowing_lt_proves_call
// - narrowing_and_both_bounds
// - narrowing_not_in_else
// - refined_kinds_unify_to_int
// - no_narrowing_for_non_numeric

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: parse and type-check, panic on parse failure.
    fn tc(src: &str) -> TypeCheckResult<TypedProgram> {
        type_check(src).unwrap_or_else(|e| panic!("parse/check failed: {e}"))
    }

    // Helper: check a program with explicit Strict mode regardless of module decl.
    fn tc_strict(src: &str) -> TypeCheckResult<TypedProgram> {
        let program = parse(src).unwrap_or_else(|e| panic!("parse failed: {e}"));
        check_program(&program, Some(TypedMode::Strict))
    }

    // Helper: check a program with explicit Lenient mode.
    fn tc_lenient(src: &str) -> TypeCheckResult<TypedProgram> {
        let program = parse(src).unwrap_or_else(|e| panic!("parse failed: {e}"));
        check_program(&program, Some(TypedMode::Lenient))
    }

    // ── Atom kind inference ─────────────────────────────────────────────────

    #[test]
    fn kind_int_literal() {
        // A bare integer in strict mode should produce kind Int, no errors.
        let r = tc_strict("42");
        assert!(r.ok, "expected ok");
        assert!(r.errors.is_empty(), "unexpected errors: {:?}", r.errors);
    }

    #[test]
    fn kind_bool_literal() {
        let r = tc_strict("#t");
        assert!(r.ok);
        assert!(r.errors.is_empty());
    }

    #[test]
    fn kind_nil_literal() {
        let r = tc_strict("nil");
        assert!(r.ok);
        assert!(r.errors.is_empty());
    }

    #[test]
    fn kind_sym_literal() {
        let r = tc_strict("'foo");
        assert!(r.ok);
        assert!(r.errors.is_empty());
    }

    // ── Variable resolution ─────────────────────────────────────────────────

    #[test]
    fn kind_varref_resolved() {
        // Define x then reference it — no errors.
        let r = tc_strict("(define x 42) x");
        assert!(r.ok, "should be ok, got errors: {:?}", r.errors);
        assert!(r.errors.is_empty());
        // The env should record x as Any (value define without annotation).
        assert_eq!(
            r.typed_ast.env.globals.get("x"),
            Some(&TwigKind::Any),
            "x should be Any (no annotation)"
        );
    }

    #[test]
    fn kind_varref_unresolved() {
        // Reference an undeclared name in strict mode → error.
        let r = tc_strict("undefined-var");
        assert!(!r.ok, "should not be ok");
        assert_eq!(r.errors.len(), 1, "expected exactly 1 error");
        assert!(
            r.errors[0].message.contains("undefined-var"),
            "error message should mention the variable name"
        );
    }

    #[test]
    fn kind_varref_unresolved_lenient() {
        // In lenient mode, unresolved variable → error in errors vec, but ok: true.
        let r = tc_lenient("undefined-var");
        assert!(r.ok, "lenient mode should still be ok");
        assert_eq!(r.errors.len(), 1, "lenient should still collect errors");
    }

    // ── Define forms ────────────────────────────────────────────────────────

    #[test]
    fn define_value_no_annotation() {
        // Value define without annotation → x: Any.
        let r = tc_strict("(define x 42)");
        assert!(r.ok);
        assert_eq!(r.typed_ast.env.globals.get("x"), Some(&TwigKind::Any));
    }

    #[test]
    fn define_function_arity() {
        // Function define with 2 params → Function{arity:2}.
        let r = tc_strict("(define (f x y) x)");
        assert!(r.ok);
        assert_eq!(
            r.typed_ast.env.globals.get("f"),
            Some(&TwigKind::Function { arity: 2 })
        );
    }

    #[test]
    fn define_function_arity_zero() {
        // Zero-parameter function.
        let r = tc_strict("(define (thunk) 42)");
        assert!(r.ok);
        assert_eq!(
            r.typed_ast.env.globals.get("thunk"),
            Some(&TwigKind::Function { arity: 0 })
        );
    }

    #[test]
    fn define_annotation_int() {
        // Value define with UnrefinedInt annotation → Int.
        let r = tc_strict("(define x : int 42)");
        assert!(r.ok, "errors: {:?}", r.errors);
        assert_eq!(
            r.typed_ast.env.globals.get("x"),
            Some(&TwigKind::Int),
            "annotation should produce Int kind"
        );
    }

    // ── Arity checking ──────────────────────────────────────────────────────

    #[test]
    fn arity_correct() {
        // f/1 called with 1 argument → no error.
        let r = tc_strict("(define (f x) x) (f 1)");
        assert!(r.ok, "errors: {:?}", r.errors);
        assert!(r.errors.is_empty());
    }

    #[test]
    fn arity_too_few() {
        // f/1 called with 0 arguments → arity error.
        let r = tc_strict("(define (f x) x) (f)");
        assert!(!r.ok);
        assert_eq!(r.errors.len(), 1);
        let msg = &r.errors[0].message;
        assert!(msg.contains("expects 1 argument"), "got: {msg}");
        assert!(msg.contains("got 0"), "got: {msg}");
    }

    #[test]
    fn arity_too_many() {
        // f/1 called with 2 arguments → arity error.
        let r = tc_strict("(define (f x) x) (f 1 2)");
        assert!(!r.ok);
        assert_eq!(r.errors.len(), 1);
        let msg = &r.errors[0].message;
        assert!(msg.contains("expects 1 argument"), "got: {msg}");
        assert!(msg.contains("got 2 arguments"), "got: {msg}");
    }

    #[test]
    fn arity_zero_param_called_with_arg() {
        // Thunk called with 1 argument → arity error.
        let r = tc_strict("(define (thunk) 42) (thunk 99)");
        assert!(!r.ok);
        assert_eq!(r.errors.len(), 1);
        assert!(r.errors[0].message.contains("expects 0 arguments"), "got: {}", r.errors[0].message);
    }

    // ── Type aliases ────────────────────────────────────────────────────────

    #[test]
    fn type_alias_registered() {
        // (type Nat Int) → env.aliases contains "Nat".
        let r = tc_strict("(type Nat Int)");
        assert!(r.ok, "errors: {:?}", r.errors);
        assert!(
            r.typed_ast.env.aliases.contains_key("Nat"),
            "env.aliases should contain Nat"
        );
    }

    #[test]
    fn type_alias_opaque_resolves_to_symbol() {
        // (type Id Symbol) → globals[Id] should not be present (alias, not a binding),
        // but the alias itself should resolve to Symbol kind via type_expr_to_kind.
        let r = tc_strict("(type Id Symbol)");
        assert!(r.ok, "errors: {:?}", r.errors);
        // Verify alias is stored.
        assert!(r.typed_ast.env.aliases.contains_key("Id"));
        // Verify the kind resolves correctly.
        let kind = kinds::type_expr_to_kind(
            r.typed_ast.env.aliases.get("Id").unwrap(),
            &r.typed_ast.env,
        );
        assert_eq!(kind, TwigKind::Symbol, "Id → Symbol via alias");
    }

    // ── Record definitions ──────────────────────────────────────────────────

    #[test]
    fn record_def_registered() {
        // (record Span (start : Int) (end : Int)) → env.records["Span"] = ["start", "end"].
        let r = tc_strict("(record Span (start : int) (end : int))");
        assert!(r.ok, "errors: {:?}", r.errors);
        let fields = r.typed_ast.env.records.get("Span");
        assert!(fields.is_some(), "Span should be in env.records");
        let fields = fields.unwrap();
        assert_eq!(fields, &["start", "end"]);
        // Constructor should be exposed as Record kind.
        assert_eq!(
            r.typed_ast.env.globals.get("Span"),
            Some(&TwigKind::Record("Span".to_owned()))
        );
    }

    // ── Union definitions ────────────────────────────────────────────────────

    #[test]
    fn union_def_registered() {
        // (union Color (Red) (Green) (Blue)) → env.unions["Color"] = ["Red", "Green", "Blue"].
        let r = tc_strict("(union Color (Red) (Green) (Blue))");
        assert!(r.ok, "errors: {:?}", r.errors);
        let variants = r.typed_ast.env.unions.get("Color");
        assert!(variants.is_some(), "Color should be in env.unions");
        assert_eq!(variants.unwrap(), &["Red", "Green", "Blue"]);
        // Each variant constructor should be a zero-arity function.
        assert_eq!(
            r.typed_ast.env.globals.get("Red"),
            Some(&TwigKind::Function { arity: 0 })
        );
    }

    #[test]
    fn union_variant_constructors_arity() {
        // Variant with fields → arity equals field count.
        let r = tc_strict("(union Shape (Circle (r : int)) (Rect (w : int) (h : int)))");
        assert!(r.ok, "errors: {:?}", r.errors);
        assert_eq!(
            r.typed_ast.env.globals.get("Circle"),
            Some(&TwigKind::Function { arity: 1 })
        );
        assert_eq!(
            r.typed_ast.env.globals.get("Rect"),
            Some(&TwigKind::Function { arity: 2 })
        );
    }

    // ── Match exhaustiveness ─────────────────────────────────────────────────

    #[test]
    fn match_exhaustive_wildcard() {
        // Wildcard arm makes any match exhaustive.
        let r = tc_strict(
            "(union Color (Red) (Green) (Blue))
             (define color (Red))
             (match color (_ 0))",
        );
        assert!(r.ok, "errors: {:?}", r.errors);
        assert!(r.errors.is_empty());
    }

    #[test]
    fn match_exhaustive_binding() {
        // Bare-name binding arm also makes any match exhaustive.
        let r = tc_strict(
            "(union Color (Red) (Green) (Blue))
             (define color (Red))
             (match color (c 0))",
        );
        assert!(r.ok, "errors: {:?}", r.errors);
        assert!(r.errors.is_empty());
    }

    #[test]
    fn match_exhaustive_all_variants() {
        // All three variants covered → no exhaustiveness error.
        let r = tc_strict(
            "(union Color (Red) (Green) (Blue))
             (define color (Red))
             (match color ((Red) 1) ((Green) 2) ((Blue) 3))",
        );
        assert!(r.ok, "errors: {:?}", r.errors);
        assert!(r.errors.is_empty());
    }

    #[test]
    fn match_non_exhaustive() {
        // Missing Blue variant → error.
        // Note: the scrutinee type must resolve to Union("Color") for
        // exhaustiveness to fire.  We use a direct union constructor call.
        let r = tc_strict(
            "(union Color (Red) (Green) (Blue))
             (define color (Red))
             (match color ((Red) 1) ((Green) 2))",
        );
        // The scrutinee `color` has kind Any (define without annotation),
        // so exhaustiveness check doesn't fire.  Let's use a variant constructor
        // directly to get a Union kind…
        //
        // Actually, in TW05-B the scrutinee kind is Any for most expressions
        // unless it's annotated.  Exhaustiveness only fires for Union scrutinees.
        // To test it properly, let's test the exhaustiveness function directly.
        let _ = r; // Not the right approach.

        // Direct exhaustiveness test via check_forms.
        use twig_parser::MatchArm;
        use twig_parser::MatchPat;
        let mut env = TypeEnv::new();
        env.unions.insert(
            "Color".to_owned(),
            vec!["Red".to_owned(), "Green".to_owned(), "Blue".to_owned()],
        );
        let arms = vec![
            MatchArm {
                pat: MatchPat::Variant {
                    name: "Red".to_owned(),
                    bindings: vec![],
                },
                body: vec![twig_parser::Expr::IntLit(twig_parser::IntLit {
                    value: 1,
                    line: 1,
                    column: 1,
                })],
            },
            MatchArm {
                pat: MatchPat::Variant {
                    name: "Green".to_owned(),
                    bindings: vec![],
                },
                body: vec![twig_parser::Expr::IntLit(twig_parser::IntLit {
                    value: 2,
                    line: 1,
                    column: 1,
                })],
            },
        ];
        let mut errors = Vec::new();
        exhaustiveness::check_exhaustiveness("Color", &arms, &env, 1, 1, &mut errors);
        assert_eq!(errors.len(), 1, "expected exactly 1 exhaustiveness error");
        assert!(
            errors[0].message.contains("Blue"),
            "error should mention Blue: {}",
            errors[0].message
        );
    }

    // ── Scoping ──────────────────────────────────────────────────────────────

    #[test]
    fn lambda_params_in_scope() {
        // Lambda parameters are visible in the body.
        let r = tc_strict("(define (f x) x)");
        assert!(r.ok, "errors: {:?}", r.errors);
        assert!(r.errors.is_empty());
    }

    #[test]
    fn let_bindings_in_scope() {
        // Let-bound names are visible in the body.
        let r = tc_strict("(define x 42) (let ((y x)) y)");
        assert!(r.ok, "errors: {:?}", r.errors);
        assert!(r.errors.is_empty());
    }

    #[test]
    fn begin_returns_last_kind() {
        // (begin 1 2 3) — no errors; kinds of sub-expressions all collected.
        let r = tc_strict("(begin 1 #t nil)");
        assert!(r.ok);
        assert!(r.errors.is_empty());
    }

    // ── Typed modes ──────────────────────────────────────────────────────────

    #[test]
    fn typed_off_skips_check() {
        // (typed off) → checker skipped → ok even for bad code.
        let r = tc("(module m (typed off)) undefined-var");
        assert!(r.ok, "off mode should skip checking");
        assert!(r.errors.is_empty(), "off mode should produce no errors");
    }

    #[test]
    fn typed_strict_rejects_unresolved() {
        // (typed strict) → unresolved reference → ok: false.
        let r = tc("(module m (typed strict)) undefined-var");
        assert!(!r.ok, "strict mode should reject unresolved var");
        assert!(!r.errors.is_empty());
        assert!(r.errors[0].message.contains("undefined-var"));
    }

    #[test]
    fn typed_lenient_warns_but_ok() {
        // (typed lenient) → errors collected but ok: true.
        let r = tc("(module m (typed lenient)) undefined-var");
        assert!(r.ok, "lenient mode should be ok");
        assert!(!r.errors.is_empty(), "lenient mode should still collect errors");
    }

    #[test]
    fn no_module_info_skips_check() {
        // No module decl → Off mode → skip.
        let r = tc("undefined-var");
        assert!(r.ok, "no module info should skip checking");
        assert!(r.errors.is_empty());
    }

    // ── Direct check_program path ─────────────────────────────────────────

    #[test]
    fn check_program_direct() {
        // Using check_program directly should bypass parse and still work.
        let program = parse("(define (f x) x) (f 1)").expect("parse should succeed");
        let r = check_program(&program, Some(TypedMode::Strict));
        assert!(r.ok, "errors: {:?}", r.errors);
        assert!(r.errors.is_empty());
    }

    #[test]
    fn check_program_mode_override() {
        // mode_override should take precedence over module_info.typed_mode.
        // Source declares (typed off) but we override with Strict.
        let program = parse("(module m (typed off)) undefined-var").expect("parse should succeed");
        let r = check_program(&program, Some(TypedMode::Strict));
        assert!(!r.ok, "override should enforce Strict");
        assert!(!r.errors.is_empty());
    }

    // ── Parse error path ─────────────────────────────────────────────────────

    #[test]
    fn parse_error_path() {
        // Malformed source → Err(TwigTypeCheckError::Parse(_)).
        let result = type_check("(((unclosed");
        assert!(
            result.is_err(),
            "malformed source should return Err"
        );
        match result {
            Err(TwigTypeCheckError::Parse(_)) => {}
            other => panic!("expected TwigTypeCheckError::Parse, got {other:?}"),
        }
    }

    // ── TW05-C: RefinedInt kind from annotations (LANG53) ────────────────────

    #[test]
    fn refined_kind_from_range_annotation() {
        // A value annotated with `(Int 0 128)` should be bound to
        // `TwigKind::RefinedInt(Range { lo: Some(0), hi: Some(128) })`, not plain `Int`.
        use kinds::{type_annotation_to_kind, TwigKind};
        use lang_refined_types::Predicate;
        use twig_parser::TypeAnnotation;

        let env = TypeEnv::new();
        let ann = TypeAnnotation::RangeInt { lo: 0, hi: 128 };
        let kind = type_annotation_to_kind(&ann, &env);

        assert!(
            matches!(&kind, TwigKind::RefinedInt(Predicate::Range {
                lo: Some(0),
                hi: Some(128),
                inclusive_hi: false,
            })),
            "expected RefinedInt(Range {{0, 128}}), got {kind:?}"
        );
        // Mnemonic must be "int" regardless of refinement.
        assert_eq!(kind.mnemonic(), "int");
    }

    #[test]
    fn refined_kind_from_membership_annotation() {
        // `(Member int 1 2 5)` → `TwigKind::RefinedInt(Membership { values: [1, 2, 5] })`.
        use kinds::{type_annotation_to_kind, TwigKind};
        use lang_refined_types::Predicate;
        use twig_parser::TypeAnnotation;

        let env = TypeEnv::new();
        let ann = TypeAnnotation::MembershipInt { values: vec![1, 2, 5] };
        let kind = type_annotation_to_kind(&ann, &env);

        assert!(
            matches!(&kind, TwigKind::RefinedInt(Predicate::Membership { values }) if *values == vec![1i128, 2, 5]),
            "expected RefinedInt(Membership {{1,2,5}}), got {kind:?}"
        );
        assert_eq!(kind.mnemonic(), "int");
    }

    #[test]
    fn unrefined_int_annotation_stays_int() {
        // `int` annotation → plain `Int`, not `RefinedInt`.  Regression guard.
        use kinds::{type_annotation_to_kind, TwigKind};
        use twig_parser::TypeAnnotation;

        let env = TypeEnv::new();
        let kind = type_annotation_to_kind(&TypeAnnotation::UnrefinedInt, &env);
        assert_eq!(kind, TwigKind::Int, "UnrefinedInt should stay Int");
    }

    // ── TW05-C: call-site refinement checking ───────────────────────────────

    /// Build the "ascii-info" function with an `(Int 0 128)` param annotation,
    /// plus stubs for the arithmetic/comparison builtins used in the tests.
    ///
    /// The `twig-type-checker` does not pre-populate builtins into `TypeEnv` —
    /// in a real program those come from the standard prelude.  The stubs below
    /// let us write self-contained test programs without needing a prelude file.
    fn ascii_info_prelude() -> String {
        [
            "(define (ascii-info (x : (Int 0 128))) x)",
            "(define (< a b) 0)",     // comparison stub  — 2 args
            "(define (<= a b) 0)",
            "(define (> a b) 0)",
            "(define (>= a b) 0)",
            "(define (and a b) 0)",   // logical-and stub
        ]
        .join(" ")
    }

    #[test]
    fn call_site_literal_in_range_no_error() {
        // `(ascii-info 42)` — literal 42 ∈ [0, 128) → proven safe → no error.
        let src = format!("{} (ascii-info 42)", ascii_info_prelude());
        let r = tc_strict(&src);
        assert!(r.ok, "literal in range should produce no error; errors: {:?}", r.errors);
        assert!(r.errors.is_empty(), "unexpected errors: {:?}", r.errors);
    }

    #[test]
    fn call_site_literal_out_of_range_error() {
        // `(ascii-info 200)` — literal 200 ∉ [0, 128) → proven unsafe → refinement error.
        let src = format!("{} (ascii-info 200)", ascii_info_prelude());
        let r = tc_strict(&src);
        assert!(!r.ok, "literal out of range should produce a refinement error");
        assert!(!r.errors.is_empty(), "expected at least 1 error");
        assert!(
            r.errors.iter().any(|e| e.message.contains("refinement error")),
            "error should mention refinement; got: {:?}",
            r.errors
        );
    }

    #[test]
    fn call_site_unconstrained_lenient_silent() {
        // Calling with an unannotated variable → Unconstrained evidence → Unknown.
        // In lenient mode Unknown is silent (ok: true, no refinement error).
        //
        // `n` has no annotation so its kind is `Any` — the checker can't prove
        // safety, but lenient mode silently accepts Unknown.
        let src = format!(
            "{} (define (process n) (ascii-info n))",
            ascii_info_prelude()
        );
        let r = tc_lenient(&src);
        assert!(r.ok, "lenient mode should be ok for unconstrained call");
        let has_refinement_error = r.errors.iter().any(|e| e.message.contains("refinement error"));
        assert!(
            !has_refinement_error,
            "lenient mode should not emit refinement error for unconstrained arg; errors: {:?}",
            r.errors
        );
    }

    #[test]
    fn call_site_unconstrained_strict_error() {
        // Same call with a plain-Int variable in strict mode → Unknown → error.
        // `n : int` gives n kind `Int` (unrefined) → Unconstrained evidence → Unknown
        // → strict mode reports a refinement error.
        let src = format!(
            "{} (define (process (n : int)) (ascii-info n))",
            ascii_info_prelude()
        );
        let r = tc_strict(&src);
        assert!(!r.ok, "strict mode should fail for unconstrained Int argument");
        assert!(
            r.errors.iter().any(|e| e.message.contains("refinement error")),
            "expected refinement error; got: {:?}",
            r.errors
        );
    }

    // ── TW05-C: flow-sensitive narrowing ────────────────────────────────────

    #[test]
    fn narrowing_lt_proves_call() {
        // In the true branch of `(< x 128)`, x is narrowed to RefinedInt(x < 128).
        // The `ascii-info` annotation is `[0, 128)`.
        // Evidence: Predicated([x < 128]) — the solver checks [0,128) ∧ ¬(x < 128)
        //   which is UNSAT → ProvenSafe → no error in the then-branch.
        let src = format!(
            "{} (define (process (x : int)) (if (< x 128) (ascii-info x) 0))",
            ascii_info_prelude()
        );
        let r = tc_strict(&src);
        assert!(
            r.ok,
            "narrowing via (< x 128) should prove ascii-info safe; errors: {:?}",
            r.errors
        );
        assert!(r.errors.is_empty(), "unexpected errors: {:?}", r.errors);
    }

    #[test]
    fn narrowing_and_both_bounds() {
        // `(if (and (>= x 0) (< x 128)) (ascii-info x) 0)` —
        // Both bounds are established; x is narrowed to RefinedInt(x >= 0 AND x < 128).
        // The annotation [0, 128) is satisfied → ProvenSafe.
        let src = format!(
            "{} (define (process (x : int)) (if (and (>= x 0) (< x 128)) (ascii-info x) 0))",
            ascii_info_prelude()
        );
        let r = tc_strict(&src);
        assert!(
            r.ok,
            "and-guard [>=0 AND <128] should prove ascii-info safe; errors: {:?}",
            r.errors
        );
        assert!(r.errors.is_empty(), "unexpected errors: {:?}", r.errors);
    }

    #[test]
    fn narrowing_not_in_else() {
        // In the else branch of `(if (< x 128) ... ...)`, x is narrowed to
        // RefinedInt(NOT(x < 128)) = RefinedInt(x >= 128).
        // Calling `(ascii-info x)` in the else branch → Evidence: Predicated([¬(x<128)]).
        // The annotation [0, 128) combined with evidence x >= 128 is SAT → ProvenUnsafe → error.
        let src = format!(
            "{} (define (process (x : int)) (if (< x 128) 0 (ascii-info x)))",
            ascii_info_prelude()
        );
        let r = tc_strict(&src);
        assert!(
            !r.ok,
            "calling ascii-info in else branch (x>=128) should be unsafe; errors: {:?}",
            r.errors
        );
        assert!(
            r.errors.iter().any(|e| e.message.contains("refinement error")),
            "expected refinement error; got: {:?}",
            r.errors
        );
    }

    // ── TW05-C: TwigKind::unify with RefinedInt ─────────────────────────────

    #[test]
    fn refined_kinds_unify_to_int() {
        // Two different RefinedInt variants in then/else branches should unify to Int
        // (not Any), since both branches produce an integer.
        //
        // We test the `unify` function directly since constructing source that
        // produces RefinedInt branches requires annotated variable bindings.
        use kinds::TwigKind;
        use lang_refined_types::Predicate;

        let p1 = Predicate::Range { lo: Some(0), hi: Some(10), inclusive_hi: false };
        let p2 = Predicate::Range { lo: Some(20), hi: Some(30), inclusive_hi: false };

        // Same predicate → preserved.
        let same = TwigKind::unify(TwigKind::RefinedInt(p1.clone()), TwigKind::RefinedInt(p1.clone()));
        assert_eq!(same, TwigKind::RefinedInt(p1.clone()), "same predicate should be preserved");

        // Different predicates → widened to Int.
        let widened = TwigKind::unify(TwigKind::RefinedInt(p1.clone()), TwigKind::RefinedInt(p2));
        assert_eq!(widened, TwigKind::Int, "different RefinedInt predicates should unify to Int");

        // RefinedInt + Int → Int.
        let with_int = TwigKind::unify(TwigKind::RefinedInt(p1.clone()), TwigKind::Int);
        assert_eq!(with_int, TwigKind::Int, "RefinedInt + Int should unify to Int");

        // RefinedInt + Any → Any.
        let with_any = TwigKind::unify(TwigKind::RefinedInt(p1), TwigKind::Any);
        assert_eq!(with_any, TwigKind::Any, "RefinedInt + Any should unify to Any");
    }

    #[test]
    fn no_narrowing_for_non_numeric() {
        // Guards with non-numeric variables should not crash the narrowing code.
        // `(if (< b 1) ...)` where b has kind Bool — merge_kind_with_predicate
        // on Bool should return Bool unchanged.
        //
        // We verify this by checking a program where the guard is over a bool-typed
        // variable (defined as a bool literal) and ensuring it produces no panics.
        let src = "(define b #t) (if (< b 1) b #f)";
        // No annotation means b is Any; `< b 1` still extracts a narrowing fact
        // (VarRef + IntLit), but merge_kind_with_predicate(Any, pred) returns Any.
        let r = tc_strict(src);
        // Result might have errors (e.g. b is Any, not Int), but it should not panic.
        let _ = r; // Success: no panic.
    }
}

// ---------------------------------------------------------------------------
// TW05-O integration tests — builtin prelude registration (LANG69)
// ---------------------------------------------------------------------------
//
// TW05-O adds `TypeEnv::register_builtins()`, called from `TypeEnv::new()`,
// which pre-populates `globals` with every Twig runtime builtin as
// `TwigKind::Any`.  This eliminates "unresolved variable" warnings/errors
// for builtins in both lenient and strict mode.
//
// The tests below verify:
// 1. Arithmetic and comparison builtins resolve without errors in strict mode.
// 2. List operations (`null?`, `car`, `cdr`, `cons`, `list`) resolve.
// 3. String operations (`string-length`) resolve.
// 4. `and` and `or` (special forms, not in BUILTINS const) resolve.
// 5. Host I/O builtins (`host/read_file`) resolve.
// 6. Higher-order ops (`map`, `filter`) resolve.
// 7. Explicit `(define ...)` stubs can still shadow pre-registered builtins.
// 8. `span.tw` content (the smallest compiler module) passes in strict mode.

#[cfg(test)]
mod tw05o_tests {
    use super::*;

    // Helper: parse and strict-check without a module declaration wrapper.
    fn strict(src: &str) -> TypeCheckResult<TypedProgram> {
        let program = parse(src).unwrap_or_else(|e| panic!("parse failed: {e}"));
        check_program(&program, Some(TypedMode::Strict))
    }

    // ── Test 1: arithmetic builtins resolve ────────────────────────────────

    #[test]
    fn builtin_arithmetic_resolves() {
        // (+ 1 2) is a call to the pre-registered `+` builtin.
        // In strict mode, an unresolved callee would produce an error.
        let r = strict("(define x (+ 1 2))");
        assert!(r.ok,
            "arithmetic builtins should resolve in strict mode; errors: {:?}", r.errors);
        assert!(r.errors.is_empty(),
            "unexpected errors: {:?}", r.errors);

        // Also verify comparison operators.
        let r2 = strict("(define y (< 1 2)) (define z (>= 3 3)) (define w (<= 0 1))");
        assert!(r2.ok,
            "comparison builtins should resolve; errors: {:?}", r2.errors);
    }

    // ── Test 2: list builtins resolve ──────────────────────────────────────

    #[test]
    fn builtin_list_ops_resolve() {
        // null?, car, cdr, cons, list, length — all list-manipulation builtins.
        let src = "(define a (null? nil)) \
                   (define b (cons 1 nil)) \
                   (define c (car (list 1 2))) \
                   (define d (length (list 1)))";
        let r = strict(src);
        assert!(r.ok,
            "list builtins should resolve in strict mode; errors: {:?}", r.errors);
    }

    // ── Test 3: string builtins resolve ───────────────────────────────────

    #[test]
    fn builtin_string_ops_resolve() {
        // string-length, string-append, string=?
        let src = "(define a (string-length \"hello\")) \
                   (define b (string-append \"x\" \"y\")) \
                   (define c (string=? \"a\" \"a\"))";
        let r = strict(src);
        assert!(r.ok,
            "string builtins should resolve in strict mode; errors: {:?}", r.errors);
    }

    // ── Test 4: and / or resolve ───────────────────────────────────────────

    #[test]
    fn builtin_and_or_resolve() {
        // `and` and `or` are special-cased in the IR compiler but are parsed
        // as regular Apply nodes — they must be pre-registered too.
        let src = "(define a (and #t #f)) (define b (or #f #t))";
        let r = strict(src);
        assert!(r.ok,
            "`and`/`or` should resolve in strict mode; errors: {:?}", r.errors);
    }

    // ── Test 5: host I/O builtins resolve ─────────────────────────────────

    #[test]
    fn builtin_host_io_resolves() {
        // host/read_file is a host builtin dispatched by the VM.
        let src = "(define contents (host/read_file \"/dev/null\"))";
        let r = strict(src);
        assert!(r.ok,
            "host/read_file should resolve in strict mode; errors: {:?}", r.errors);
    }

    // ── Test 6: higher-order ops resolve ──────────────────────────────────

    #[test]
    fn builtin_hof_resolves() {
        // map and filter are higher-order builtins registered in LANG55.
        let src = "(define a (map nil nil)) (define b (filter nil nil))";
        let r = strict(src);
        assert!(r.ok,
            "map/filter should resolve in strict mode; errors: {:?}", r.errors);
    }

    // ── Test 7: stub shadows pre-registered builtin ────────────────────────

    #[test]
    fn builtin_does_not_block_stub_shadow() {
        // An explicit (define (+ a b) ...) should shadow the pre-registered `+`
        // without causing errors — Pass 1 overwrites the pre-registered Any.
        let src = "(define (+ a b) 0) (define result (+ 1 2))";
        let r = strict(src);
        assert!(r.ok,
            "stub define should shadow pre-registered builtin; errors: {:?}", r.errors);
    }

    // ── Test 8: span.tw content compiles in strict mode ──────────────────

    #[test]
    fn span_tw_strict_mode_compiles() {
        // Minimal reproduction of span.tw in strict mode.
        // Uses: record Span, and, >=, <= (builtins), Span constructor (record).
        // No imports needed — all dependencies are within this snippet.
        let src = "\
            (module compiler/span (typed strict) \
              (export Span make-span dummy-span)) \
            (record Span (source-id : int) (start : int) (end : int)) \
            (define (make-span source-id start end) \
              (if (and (>= start 0) (<= start end)) \
                (Span source-id start end) \
                nil)) \
            (define (dummy-span) (Span 0 0 0))";
        let r = strict(src);
        assert!(r.ok,
            "span.tw content should pass strict mode after builtin registration; \
             errors: {:?}", r.errors);
        assert!(r.errors.is_empty(),
            "unexpected type errors in span.tw strict check: {:?}", r.errors);
    }
}

// ---------------------------------------------------------------------------
// TW05-P Part 1 tests — generated symbol registration (LANG70)
// ---------------------------------------------------------------------------
//
// LANG70 extends `register_record` and `register_union` to also register
// the symbols the IR compiler generates for each record/union declaration:
//
//   Record `Foo` with field `x`:
//     - `foo?`   → the record predicate
//     - `foo-x`  → the field accessor
//
//   Union variant `Bar` with field `y`:
//     - `Bar?`   → the variant predicate   (original case, NOT lowercased)
//     - `bar-y`  → the field accessor      (lowercased prefix)
//
// These tests verify that strict-mode modules which call their own generated
// symbols no longer produce "unresolved variable" errors.

#[cfg(test)]
mod tw05p1_tests {
    use super::*;

    // Helper: strict-check an inline source snippet.
    fn strict(src: &str) -> TypeCheckResult<TypedProgram> {
        let program = parse(src).unwrap_or_else(|e| panic!("parse failed: {e}"));
        check_program(&program, Some(TypedMode::Strict))
    }

    // ── Test 1: record predicate resolves ─────────────────────────────────
    //
    // `(record Point (x : int) (y : int))` generates `point?`.
    // A strict-mode function calling `(point? v)` must not produce
    // "unresolved variable `point?`".

    #[test]
    fn record_predicate_resolves_in_strict() {
        let src = "(record Point (x : int) (y : int)) \
                   (define (is-point? v) (point? v))";
        let r = strict(src);
        assert!(r.ok,
            "record predicate `point?` should resolve in strict mode; \
             errors: {:?}", r.errors);
        assert!(r.errors.is_empty(),
            "unexpected errors: {:?}", r.errors);
    }

    // ── Test 2: record field accessor resolves ────────────────────────────
    //
    // `(record Point (x : int) (y : int))` generates `point-x` and `point-y`.
    // Strict-mode calls to those accessors must resolve.

    #[test]
    fn record_accessor_resolves_in_strict() {
        let src = "(record Point (x : int) (y : int)) \
                   (define (get-x p) (point-x p)) \
                   (define (get-y p) (point-y p))";
        let r = strict(src);
        assert!(r.ok,
            "record accessors `point-x`/`point-y` should resolve; \
             errors: {:?}", r.errors);
    }

    // ── Test 3: union variant predicate resolves ──────────────────────────
    //
    // `(union Color (Red) (Green))` generates `Red?` and `Green?`.
    // Note: variant predicates keep the ORIGINAL case (`Red?`, not `red?`).

    #[test]
    fn union_variant_predicate_resolves_in_strict() {
        let src = "(union Color (Red) (Green) (Blue)) \
                   (define (is-red? c) (Red? c)) \
                   (define (is-green? c) (Green? c))";
        let r = strict(src);
        assert!(r.ok,
            "union variant predicates `Red?`/`Green?` should resolve; \
             errors: {:?}", r.errors);
    }

    // ── Test 4: union variant field accessor resolves ──────────────────────
    //
    // `(union Shape (Circle (radius : int)) (Rect (w : int) (h : int)))` generates:
    //   `circle-radius`, `rect-w`, `rect-h`  (lowercased variant name prefix)

    #[test]
    fn union_variant_field_accessor_resolves_in_strict() {
        let src = "(union Shape \
                     (Circle (radius : int)) \
                     (Rect (w : int) (h : int))) \
                   (define (get-radius s) (circle-radius s)) \
                   (define (get-width s) (rect-w s))";
        let r = strict(src);
        assert!(r.ok,
            "union variant field accessors `circle-radius`/`rect-w` should resolve; \
             errors: {:?}", r.errors);
    }

    // ── Test 5: diagnostic.tw content compiles in strict mode ─────────────
    //
    // diagnostic.tw has 3 define forms that call own union constructors only.
    // No accessor calls.  Should pass trivially.

    #[test]
    fn diagnostic_tw_strict_mode_compiles() {
        let src = "\
            (module compiler/diagnostic (typed strict) \
              (export Severity SevError SevWarning SevInfo \
                      SevError? SevWarning? SevInfo? \
                      Diagnostic diagnostic? diagnostic-severity \
                      diagnostic-message diagnostic-span \
                      make-error make-warning make-info)) \
            (union Severity (SevError) (SevWarning) (SevInfo)) \
            (record Diagnostic (severity : any) (message : any) (span : any)) \
            (define (make-error message span) \
              (Diagnostic (SevError) message span)) \
            (define (make-warning message span) \
              (Diagnostic (SevWarning) message span)) \
            (define (make-info message span) \
              (Diagnostic (SevInfo) message span))";
        let r = strict(src);
        assert!(r.ok,
            "diagnostic.tw content should pass strict mode; \
             errors: {:?}", r.errors);
        assert!(r.errors.is_empty(),
            "unexpected type errors: {:?}", r.errors);
    }

    // ── Test 6: iir-builder.tw content compiles in strict mode ────────────
    //
    // iir-builder.tw has 8 define forms that call own IirBuilder accessors
    // (`iirbuilder-name`, `iirbuilder-reg-count`, etc.) and builtins.
    // This confirms that accessor registration enables strict mode for
    // a module that uses its own record's generated accessors.

    #[test]
    fn iir_builder_tw_strict_mode_compiles() {
        // Minimal reproduction: IirBuilder record + 3 representative functions.
        // Uses own accessors (iirbuilder-name, iirbuilder-instrs, etc.) and
        // builtins (cons, reverse).
        let src = "\
            (module compiler/iir-builder (typed strict) \
              (export IirBuilder iirbuilder? \
                      iirbuilder-name iirbuilder-instrs \
                      iirbuilder-reg-count iirbuilder-label-count \
                      new-builder append-instr finalise-builder)) \
            (record IirBuilder \
              (name        : any) \
              (instrs      : any) \
              (reg-count   : int) \
              (label-count : int)) \
            (define (iirbuilder-with-instrs b new-instrs) \
              (IirBuilder (iirbuilder-name b) new-instrs \
                          (iirbuilder-reg-count b) (iirbuilder-label-count b))) \
            (define (new-builder fn-name) \
              (IirBuilder fn-name nil 0 0)) \
            (define (append-instr b instr) \
              (iirbuilder-with-instrs b (cons instr (iirbuilder-instrs b)))) \
            (define (finalise-builder b) \
              (reverse (iirbuilder-instrs b)))";
        let r = strict(src);
        assert!(r.ok,
            "iir-builder.tw content should pass strict mode after accessor \
             registration; errors: {:?}", r.errors);
        assert!(r.errors.is_empty(),
            "unexpected type errors in iir-builder strict check: {:?}", r.errors);
    }
}

// ---------------------------------------------------------------------------
// TW05-P Part 2 / LANG71 — multi-module import propagation
// ---------------------------------------------------------------------------
//
// Each test:
//   1. Type-checks all dependency modules (which are themselves in strict mode).
//   2. Calls `extract_module_exports` on each to get the exported globals.
//   3. Merges the exports into a single `extra_globals` map.
//   4. Calls `check_program_with_globals` on the module under test (strict mode).
//   5. Asserts `ok: true` and `errors.is_empty()`.
//
// The `.tw` source files are embedded at compile time via `include_str!`.
// Paths are relative to this source file (`src/lib.rs`):
//   ../../../twig/compiler/  →  code/packages/twig/compiler/

#[cfg(test)]
mod tw05p2_tests {
    use super::*;
    use std::collections::HashMap;

    // ── helper ──────────────────────────────────────────────────────────────

    /// Type-check a source string, returning (Program, TypeEnv).
    fn checked(src: &str) -> (Program, TypeEnv) {
        let prog = parse(src).unwrap_or_else(|e| panic!("parse failed: {e}"));
        let res = check_program(&prog, None);
        (prog, res.typed_ast.env)
    }

    /// Merge exported globals from a dep program+env into an accumulator map.
    fn add_exports(
        acc: &mut HashMap<String, TwigKind>,
        prog: &Program,
        env: &TypeEnv,
    ) {
        let exports = extract_module_exports(prog, env);
        for (k, v) in exports {
            acc.entry(k).or_insert(v);
        }
    }

    // ── Source constants ─────────────────────────────────────────────────────

    const SPAN_SRC: &str =
        include_str!("../../../twig/compiler/span.tw");
    const TOKEN_SRC: &str =
        include_str!("../../../twig/compiler/token.tw");
    const DIAGNOSTIC_SRC: &str =
        include_str!("../../../twig/compiler/diagnostic.tw");
    const AST_SRC: &str =
        include_str!("../../../twig/compiler/ast.tw");
    const IIR_TYPES_SRC: &str =
        include_str!("../../../twig/compiler/iir-types.tw");
    const IIR_BUILDER_SRC: &str =
        include_str!("../../../twig/compiler/iir-builder.tw");
    const LEXER_SRC: &str =
        include_str!("../../../twig/compiler/lexer.tw");
    const CST_PARSER_SRC: &str =
        include_str!("../../../twig/compiler/cst-parser.tw");
    const PARSER_SRC: &str =
        include_str!("../../../twig/compiler/parser.tw");
    const EMIT_SRC: &str =
        include_str!("../../../twig/compiler/emit.tw");
    const MAIN_SRC: &str =
        include_str!("../../../twig/compiler/main.tw");

    // ── Tests ────────────────────────────────────────────────────────────────

    #[test]
    fn lexer_tw_strict_with_imported_globals() {
        // lexer.tw imports: compiler/span, compiler/token
        let (span_prog, span_env) = checked(SPAN_SRC);
        let (tok_prog, tok_env) = checked(TOKEN_SRC);

        let mut globals: HashMap<String, TwigKind> = HashMap::new();
        add_exports(&mut globals, &span_prog, &span_env);
        add_exports(&mut globals, &tok_prog, &tok_env);

        let lexer_prog = parse(LEXER_SRC)
            .unwrap_or_else(|e| panic!("lexer.tw parse failed: {e}"));
        let r = check_program_with_globals(&lexer_prog, None, &globals);
        assert!(
            r.ok,
            "lexer.tw should pass strict mode with imported globals; \
             errors: {:?}",
            r.errors
        );
        assert!(r.errors.is_empty(),
            "unexpected type errors in lexer.tw strict check: {:?}", r.errors);
    }

    #[test]
    fn cst_parser_tw_strict_with_imported_globals() {
        // cst-parser.tw imports: compiler/token
        let (tok_prog, tok_env) = checked(TOKEN_SRC);

        let mut globals: HashMap<String, TwigKind> = HashMap::new();
        add_exports(&mut globals, &tok_prog, &tok_env);

        let prog = parse(CST_PARSER_SRC)
            .unwrap_or_else(|e| panic!("cst-parser.tw parse failed: {e}"));
        let r = check_program_with_globals(&prog, None, &globals);
        assert!(
            r.ok,
            "cst-parser.tw should pass strict mode with imported globals; \
             errors: {:?}",
            r.errors
        );
        assert!(r.errors.is_empty(),
            "unexpected type errors in cst-parser.tw strict check: {:?}", r.errors);
    }

    #[test]
    fn parser_tw_strict_with_imported_globals() {
        // parser.tw imports: compiler/cst-parser, compiler/token,
        //                    compiler/ast, compiler/span
        let (span_prog, span_env) = checked(SPAN_SRC);
        let (tok_prog, tok_env) = checked(TOKEN_SRC);
        let (ast_prog, ast_env) = checked(AST_SRC);

        // cst-parser itself needs token exports pre-seeded
        let mut cst_globals: HashMap<String, TwigKind> = HashMap::new();
        add_exports(&mut cst_globals, &tok_prog, &tok_env);
        let cst_prog = parse(CST_PARSER_SRC)
            .unwrap_or_else(|e| panic!("cst-parser.tw parse failed: {e}"));
        let cst_res = check_program_with_globals(&cst_prog, None, &cst_globals);

        let mut globals: HashMap<String, TwigKind> = HashMap::new();
        add_exports(&mut globals, &span_prog, &span_env);
        add_exports(&mut globals, &tok_prog, &tok_env);
        add_exports(&mut globals, &ast_prog, &ast_env);
        add_exports(&mut globals, &cst_prog, &cst_res.typed_ast.env);

        let prog = parse(PARSER_SRC)
            .unwrap_or_else(|e| panic!("parser.tw parse failed: {e}"));
        let r = check_program_with_globals(&prog, None, &globals);
        assert!(
            r.ok,
            "parser.tw should pass strict mode with imported globals; \
             errors: {:?}",
            r.errors
        );
        assert!(r.errors.is_empty(),
            "unexpected type errors in parser.tw strict check: {:?}", r.errors);
    }

    #[test]
    fn emit_tw_strict_with_imported_globals() {
        // emit.tw imports: compiler/span, compiler/ast,
        //                  compiler/iir-types, compiler/iir-builder
        let (span_prog, span_env) = checked(SPAN_SRC);
        let (ast_prog, ast_env) = checked(AST_SRC);
        let (iir_t_prog, iir_t_env) = checked(IIR_TYPES_SRC);

        // iir-builder needs iir-types exports
        let mut ib_globals: HashMap<String, TwigKind> = HashMap::new();
        add_exports(&mut ib_globals, &iir_t_prog, &iir_t_env);
        let ib_prog = parse(IIR_BUILDER_SRC)
            .unwrap_or_else(|e| panic!("iir-builder.tw parse failed: {e}"));
        let ib_res = check_program_with_globals(&ib_prog, None, &ib_globals);

        let mut globals: HashMap<String, TwigKind> = HashMap::new();
        add_exports(&mut globals, &span_prog, &span_env);
        add_exports(&mut globals, &ast_prog, &ast_env);
        add_exports(&mut globals, &iir_t_prog, &iir_t_env);
        add_exports(&mut globals, &ib_prog, &ib_res.typed_ast.env);

        let prog = parse(EMIT_SRC)
            .unwrap_or_else(|e| panic!("emit.tw parse failed: {e}"));
        let r = check_program_with_globals(&prog, None, &globals);
        assert!(
            r.ok,
            "emit.tw should pass strict mode with imported globals; \
             errors: {:?}",
            r.errors
        );
        assert!(r.errors.is_empty(),
            "unexpected type errors in emit.tw strict check: {:?}", r.errors);
    }

    #[test]
    fn main_tw_strict_with_imported_globals() {
        // main.tw imports: all 9 other compiler modules.
        // Build exports bottom-up following the dependency graph.

        // Leaf modules (no imports from compiler/)
        let (span_prog, span_env) = checked(SPAN_SRC);
        let (tok_prog, tok_env) = checked(TOKEN_SRC);
        let (diag_prog, diag_env) = checked(DIAGNOSTIC_SRC);
        let (ast_prog, ast_env) = checked(AST_SRC);
        let (iir_t_prog, iir_t_env) = checked(IIR_TYPES_SRC);

        // iir-builder ← iir-types
        let mut ib_g: HashMap<String, TwigKind> = HashMap::new();
        add_exports(&mut ib_g, &iir_t_prog, &iir_t_env);
        let ib_prog = parse(IIR_BUILDER_SRC).unwrap();
        let ib_res = check_program_with_globals(&ib_prog, None, &ib_g);

        // diagnostic ← span (already checked above, re-check with span seed)
        let mut diag_g: HashMap<String, TwigKind> = HashMap::new();
        add_exports(&mut diag_g, &span_prog, &span_env);
        let diag_prog2 = parse(DIAGNOSTIC_SRC).unwrap();
        let diag_res2 = check_program_with_globals(&diag_prog2, None, &diag_g);

        // lexer ← span, token
        let mut lex_g: HashMap<String, TwigKind> = HashMap::new();
        add_exports(&mut lex_g, &span_prog, &span_env);
        add_exports(&mut lex_g, &tok_prog, &tok_env);
        let lex_prog = parse(LEXER_SRC).unwrap();
        let lex_res = check_program_with_globals(&lex_prog, None, &lex_g);

        // cst-parser ← token
        let mut cst_g: HashMap<String, TwigKind> = HashMap::new();
        add_exports(&mut cst_g, &tok_prog, &tok_env);
        let cst_prog = parse(CST_PARSER_SRC).unwrap();
        let cst_res = check_program_with_globals(&cst_prog, None, &cst_g);

        // parser ← cst-parser, token, ast, span
        let mut par_g: HashMap<String, TwigKind> = HashMap::new();
        add_exports(&mut par_g, &span_prog, &span_env);
        add_exports(&mut par_g, &tok_prog, &tok_env);
        add_exports(&mut par_g, &ast_prog, &ast_env);
        add_exports(&mut par_g, &cst_prog, &cst_res.typed_ast.env);
        let par_prog = parse(PARSER_SRC).unwrap();
        let par_res = check_program_with_globals(&par_prog, None, &par_g);

        // emit ← span, ast, iir-types, iir-builder
        let mut emit_g: HashMap<String, TwigKind> = HashMap::new();
        add_exports(&mut emit_g, &span_prog, &span_env);
        add_exports(&mut emit_g, &ast_prog, &ast_env);
        add_exports(&mut emit_g, &iir_t_prog, &iir_t_env);
        add_exports(&mut emit_g, &ib_prog, &ib_res.typed_ast.env);
        let emit_prog = parse(EMIT_SRC).unwrap();
        let emit_res = check_program_with_globals(&emit_prog, None, &emit_g);

        // main ← all 9
        let mut main_g: HashMap<String, TwigKind> = HashMap::new();
        add_exports(&mut main_g, &span_prog, &span_env);
        add_exports(&mut main_g, &tok_prog, &tok_env);
        add_exports(&mut main_g, &diag_prog, &diag_env);
        // use re-checked diag with span seed for completeness
        add_exports(&mut main_g, &diag_prog2, &diag_res2.typed_ast.env);
        add_exports(&mut main_g, &ast_prog, &ast_env);
        add_exports(&mut main_g, &iir_t_prog, &iir_t_env);
        add_exports(&mut main_g, &ib_prog, &ib_res.typed_ast.env);
        add_exports(&mut main_g, &lex_prog, &lex_res.typed_ast.env);
        add_exports(&mut main_g, &cst_prog, &cst_res.typed_ast.env);
        add_exports(&mut main_g, &par_prog, &par_res.typed_ast.env);
        add_exports(&mut main_g, &emit_prog, &emit_res.typed_ast.env);

        let prog = parse(MAIN_SRC)
            .unwrap_or_else(|e| panic!("main.tw parse failed: {e}"));
        let r = check_program_with_globals(&prog, None, &main_g);
        assert!(
            r.ok,
            "main.tw should pass strict mode with all imported globals; \
             errors: {:?}",
            r.errors
        );
        assert!(r.errors.is_empty(),
            "unexpected type errors in main.tw strict check: {:?}", r.errors);
    }
}
