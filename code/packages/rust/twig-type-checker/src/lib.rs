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
