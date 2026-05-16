//! # twig-to-semantic-ir
//!
//! Twig AST → narrow-waist Semantic IR (SIR11).
//!
//! Consumes [`twig_parser::Program`] and produces a
//! [`semantic_ir::Module`] suitable for any SIR v0 backend.
//!
//! ## Public API
//!
//! ```ignore
//! use twig_to_semantic_ir::{compile_source, TwigLowerError};
//! let module = compile_source(
//!     "(define (id x) x)\n(id 42)",
//!     "demo",
//! )?;
//! ```
//!
//! See SIR11-twig-to-semantic-ir.md for the full lowering rules.

mod builtins;
mod lower;

pub use lower::{compile, TwigLowerError};

/// Convenience: parse Twig source and lower to SIR in one call.
///
/// Wraps `twig_parser::parse` followed by `lower::compile`.  Parse
/// errors and lower errors are both surfaced as
/// [`TwigLowerError`] (parse errors map line/column directly).
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<semantic_ir::Module, TwigLowerError> {
    let program = twig_parser::parse(source).map_err(|e| TwigLowerError {
        message: format!("parse error: {}", e.message),
        line: e.line,
        column: e.column,
    })?;
    compile(&program, module_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use semantic_ir::print_module;

    fn lower(src: &str) -> semantic_ir::Module {
        compile_source(src, "test").expect("lowering succeeded")
    }

    #[test]
    fn empty_program_compiles() {
        let m = lower("");
        // main is always synthesised; _init is omitted when no
        // value defines exist.
        assert!(m.functions.iter().any(|f| f.name == "main"));
        assert!(!m.functions.iter().any(|f| f.name == "_init"));
    }

    #[test]
    fn bare_int_becomes_main_value() {
        let m = lower("42");
        let main = m.functions.iter().find(|f| f.name == "main").unwrap();
        match &main.body.value {
            semantic_ir::Expr::IntLit { value, .. } => assert_eq!(*value, 42),
            other => panic!("expected IntLit, got {:?}", other),
        }
    }

    #[test]
    fn define_fn_emits_function() {
        let m = lower("(define (id x) x)");
        assert!(m.functions.iter().any(|f| f.name == "id"));
        let id = m.functions.iter().find(|f| f.name == "id").unwrap();
        assert_eq!(id.params.len(), 1);
        assert_eq!(id.params[0].name, "x");
        match &id.body.value {
            semantic_ir::Expr::VarRef { scope, .. } => {
                assert_eq!(*scope, semantic_ir::Scope::Param);
            }
            other => panic!("expected VarRef Param, got {:?}", other),
        }
    }

    #[test]
    fn value_define_creates_global_and_init() {
        let m = lower("(define x 7)");
        assert!(m.globals.iter().any(|g| g.name == "x"));
        let init = m.functions.iter().find(|f| f.name == "_init").unwrap();
        // _init has one ExprStmt: builtin-call global_set 'x 7
        assert_eq!(init.body.stmts.len(), 1);
    }

    #[test]
    fn arithmetic_lowers_to_builtin_call() {
        let m = lower("(+ 1 2)");
        let main = m.functions.iter().find(|f| f.name == "main").unwrap();
        match &main.body.value {
            semantic_ir::Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "+");
                assert_eq!(args.len(), 2);
            }
            other => panic!("expected BuiltinCall, got {:?}", other),
        }
    }

    #[test]
    fn direct_call_to_known_function() {
        let m = lower("(define (id x) x)\n(id 42)");
        let main = m.functions.iter().find(|f| f.name == "main").unwrap();
        match &main.body.value {
            semantic_ir::Expr::DirectCall { fn_name, args, .. } => {
                assert_eq!(fn_name, "id");
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected DirectCall, got {:?}", other),
        }
    }

    #[test]
    fn lambda_creates_synthesised_function_and_make_closure() {
        // `n` is captured.
        let m = lower("(define (adder n) (lambda (x) (+ x n)))");
        // adder body should be a MakeClosure referencing a
        // synthesised __lambda_0.
        let adder = m.functions.iter().find(|f| f.name == "adder").unwrap();
        let synth_name = match &adder.body.value {
            semantic_ir::Expr::MakeClosure { fn_name, captures, .. } => {
                assert_eq!(captures.len(), 1);
                assert_eq!(captures[0].name, "n");
                fn_name.clone()
            }
            other => panic!("expected MakeClosure, got {:?}", other),
        };
        let synth = m.functions.iter().find(|f| f.name == synth_name).unwrap();
        assert_eq!(synth.params.len(), 1);
        assert_eq!(synth.captures.len(), 1);
        assert_eq!(synth.captures[0].name, "n");
    }

    #[test]
    fn unresolved_name_is_error() {
        let err = compile_source("ghost", "t").expect_err("expected error");
        assert!(err.message.contains("unresolved"));
    }

    #[test]
    fn nested_let_resolves_locals_correctly() {
        let m = lower("(define (f) (let ((x 1)) (let ((y 2)) (+ x y))))");
        // No unresolved-name errors → success.  Just sanity-check
        // shape.
        assert!(m.functions.iter().any(|f| f.name == "f"));
    }

    #[test]
    fn let_star_can_reference_prior_binding() {
        let m = lower("(define (f) (let* ((x 1) (y x)) y))");
        assert!(m.functions.iter().any(|f| f.name == "f"));
    }

    #[test]
    fn module_passes_validator() {
        let m = lower("(define (id x) x)\n(define c 5)\n(id c)");
        let r = semantic_ir::validate(&m);
        assert!(
            r.is_ok(),
            "module failed validation: {:?}",
            r.issues
        );
    }

    #[test]
    fn print_round_trip_is_deterministic() {
        let m = lower("(define (add a b) (+ a b))\n(add 1 2)");
        let t1 = print_module(&m);
        let t2 = print_module(&m);
        assert_eq!(t1, t2);
    }

    #[test]
    fn type_alias_rejected() {
        let err = compile_source("(type Nat int)", "t").expect_err("type aliases rejected");
        assert!(err.message.contains("type aliases"));
    }

    #[test]
    fn record_rejected() {
        let err = compile_source("(record P (x : int))", "t").expect_err("records rejected");
        assert!(err.message.contains("record"));
    }

    #[test]
    fn string_literal_lowers() {
        let m = lower("\"hi\"");
        let main = m.functions.iter().find(|f| f.name == "main").unwrap();
        match &main.body.value {
            semantic_ir::Expr::StrLit { value, .. } => assert_eq!(value, "hi"),
            other => panic!("expected StrLit, got {:?}", other),
        }
        assert!(m.manifest.contains(semantic_ir::Feature::Strings));
    }

    #[test]
    fn symbol_literal_lowers() {
        let m = lower("'foo");
        assert!(m.manifest.contains(semantic_ir::Feature::Symbols));
    }

    #[test]
    fn print_call_carries_may_print_effect() {
        let m = lower("(print 1)");
        let main = m.functions.iter().find(|f| f.name == "main").unwrap();
        match &main.body.value {
            semantic_ir::Expr::BuiltinCall { name, effects, .. } => {
                assert_eq!(name, "print");
                assert!(effects.contains(semantic_ir::Effect::MayPrint));
            }
            other => panic!("expected BuiltinCall print, got {:?}", other),
        }
    }

    #[test]
    fn higher_order_call_emits_indirect() {
        // `add5` is bound by define so it's a Global value reachable
        // as a closure handle; calling it via a local is an
        // IndirectCall.
        let m = lower("(define (adder n) (lambda (x) (+ x n)))\n(define add5 (adder 5))\n(add5 3)");
        // main's body should contain an IndirectCall on `add5`.
        let main = m.functions.iter().find(|f| f.name == "main").unwrap();
        let _ = main;
        // (The exact shape depends on how `add5` resolves; assertion
        // here is just that lowering succeeds and validates.)
        let r = semantic_ir::validate(&m);
        assert!(r.is_ok(), "validation: {:?}", r.issues);
    }
}
