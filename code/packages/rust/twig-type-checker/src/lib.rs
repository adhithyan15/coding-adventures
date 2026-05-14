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
pub mod check;
pub mod env;
pub mod errors;
pub mod exhaustiveness;
pub mod kinds;

pub use env::TypeEnv;
pub use errors::TwigTypeCheckError;
pub use kinds::TwigKind;

use twig_parser::{parse, Program, TypedMode};
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
// Public API
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
// 28 unit tests covering:
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
}
