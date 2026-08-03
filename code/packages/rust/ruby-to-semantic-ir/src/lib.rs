//! # ruby-to-semantic-ir
//!
//! Ruby AST → narrow-waist [Semantic IR](semantic_ir).
//!
//! Phase 5 of the [Ruby parser project](../../../specs/ruby-parser.md):
//! the first frontend that consumes [`ruby_parser`]'s
//! [`GrammarASTNode`](parser::grammar_parser::GrammarASTNode) and
//! emits a [`semantic_ir::Module`] suitable for any SIR backend.
//!
//! ## Pipeline
//!
//! ```text
//! Ruby source
//!    │
//!    ▼  ruby-lexer  →  ruby-parser
//! GrammarASTNode (rule = "program")
//!    │
//!    ▼  ruby_to_semantic_ir::compile_source     ← THIS CRATE
//! semantic_ir::Module
//!    │
//!    ▼  semantic-ir-to-{rust, typescript, go, python}
//! target source
//! ```
//!
//! ## Public API
//!
//! ```ignore
//! use ruby_to_semantic_ir::{compile_source, RubyLowerError};
//!
//! let module = compile_source(
//!     "x = 1\ny = 2\nputs(x + y)\n",
//!     "demo",
//! )?;
//! // `module` is a `semantic_ir::Module`.
//! ```
//!
//! See [the crate README](../README.md) for the v0 lowering rules
//! and the explicit list of deferred Ruby features.

use coding_adventures_ruby_parser::parse_ruby;

mod lower;

pub use lower::{compile, RubyLowerError};

/// Parse Ruby source and lower it to SIR in a single call.
///
/// Wraps [`parse_ruby`] followed by [`compile`].  The current
/// `parse_ruby` panics on parse errors (legacy behaviour shared with
/// the rest of the grammar-driven parsers); a future refactor will
/// surface those as [`RubyLowerError`]s with proper line/column.
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<semantic_ir::Module, RubyLowerError> {
    let ast = parse_ruby(source);
    compile(&ast, module_name)
}


#[cfg(test)]
mod tests {
    use super::*;
    use semantic_ir::{Effect, Expr, ParamKind, Scope, Stmt};

    fn lower(src: &str) -> semantic_ir::Module {
        compile_source(src, "test").expect("lowering succeeded")
    }

    fn main_body(m: &semantic_ir::Module) -> &semantic_ir::Block {
        let f = m
            .functions
            .iter()
            .find(|f| f.name == "main")
            .expect("main function present");
        &f.body
    }

    /// Name bound by a first-sighting assignment — a `LetBinding` (parallel
    /// `let`) OR a `LetStarBinding` (sequential `let*`).  The frontend emits
    /// `let*` when the RHS reads an earlier local in the same block (Ruby
    /// assignments are sequential); both are "a binding" for shape assertions.
    fn binding_name(s: &Stmt) -> Option<&str> {
        match s {
            Stmt::LetBinding { name, .. } | Stmt::LetStarBinding { name, .. } => {
                Some(name.as_str())
            }
            _ => None,
        }
    }

    /// The RHS value of a `LetBinding` / `LetStarBinding` (see [`binding_name`]).
    fn binding_value(s: &Stmt) -> Option<&Expr> {
        match s {
            Stmt::LetBinding { value, .. } | Stmt::LetStarBinding { value, .. } => Some(value),
            _ => None,
        }
    }

    // -----------------------------------------------------------------
    // Phase P7 (Ruby 1.0) — default / optional parameters.
    //
    // Before P7 the grammar `param` rule was `[ "*" | "**" ] NAME`, with
    // no default-value branch — so `def f(a = 1)` did not even PARSE.  P7
    // extends the grammar to `param = [ "*" | "**" ] NAME [ EQUALS
    // expression ]` and the lowerer (`extract_params`) carries the
    // default subtree into `Param.default`.
    // -----------------------------------------------------------------

    #[test]
    fn def_default_param_lowers_to_param_default_intlit() {
        // `def f(a = 1)` → the single Param carries `default: Some(IntLit 1)`.
        let m = lower("def f(a = 1)\n  a + 0\nend\n");
        let f = m.functions.iter().find(|f| f.name == "f").expect("fn f");
        assert_eq!(f.params.len(), 1);
        assert_eq!(f.params[0].name, "a");
        assert_eq!(f.params[0].kind, ParamKind::Required);
        match &f.params[0].default {
            Some(boxed) => assert!(
                matches!(**boxed, Expr::IntLit { value: 1, .. }),
                "expected default IntLit(1), got {:?}",
                boxed
            ),
            None => panic!("expected a default value, got None"),
        }
    }

    #[test]
    fn def_default_param_observes_default_params_feature() {
        // A defaulted param must make the manifest declare `DefaultParams`
        // (otherwise the validator rejects the used-but-undeclared feature).
        let m = lower("def f(a = 1)\n  a + 0\nend\n");
        assert!(
            m.manifest.contains(semantic_ir::Feature::DefaultParams),
            "manifest should declare DefaultParams; declared = {:?}",
            m.manifest
        );
    }

    #[test]
    fn bare_identifier_method_body_lowers_to_param_varref() {
        // Regression for the `factor` KEYWORD-swallow bug: `def f(a)\n a\nend`
        // used to mis-parse (the lone `a` ate the closing `end`, so no
        // `def_statement` survived and lowering produced no function `f`).
        // With the grammar guard in place the method lowers normally and its
        // tail expression is a reference to the parameter `a`.
        let m = lower("def f(a)\n  a\nend\n");
        let f = m.functions.iter().find(|f| f.name == "f").expect("fn f present");
        assert_eq!(f.params.len(), 1);
        assert_eq!(f.params[0].name, "a");
        assert!(
            matches!(
                &f.body.value,
                Expr::VarRef { name, scope, .. }
                    if name == "a" && *scope == Scope::Param
            ),
            "method body tail should be VarRef(\"a\", Param), got {:?}",
            f.body.value
        );
    }

    #[test]
    fn def_default_param_can_reference_earlier_param() {
        // `def f(a, b = a + 1)` — Ruby defaults are call-time and may
        // reference EARLIER params.  `a` inside the default for `b` must
        // resolve to `Scope::Param`, not an unbound local.
        let m = lower("def f(a, b = a + 1)\n  b + 0\nend\n");
        let f = m.functions.iter().find(|f| f.name == "f").expect("fn f");
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.params[0].name, "a");
        assert!(f.params[0].default.is_none(), "first param has no default");
        assert_eq!(f.params[1].name, "b");
        let default = f.params[1]
            .default
            .as_ref()
            .expect("second param has a default");
        // The default is `a + 1` → BuiltinCall("+", [VarRef(a, Param), IntLit 1]).
        match &**default {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "+");
                match &args[0] {
                    Expr::VarRef { name, scope, .. } => {
                        assert_eq!(name, "a");
                        assert_eq!(*scope, Scope::Param);
                    }
                    other => panic!("expected VarRef(a, Param), got {:?}", other),
                }
                assert!(matches!(args[1], Expr::IntLit { value: 1, .. }));
            }
            other => panic!("expected BuiltinCall(+), got {:?}", other),
        }
        // And the module validates (the default checks in param scope).
        assert!(
            semantic_ir::validate(&m).is_ok(),
            "module with param-scoped default should validate: {:?}",
            semantic_ir::validate(&m).issues
        );
    }

    #[test]
    fn def_required_param_keeps_no_default() {
        // Regression: an ordinary positional param keeps `default: None`
        // and does NOT trip the DefaultParams feature.
        let m = lower("def k(a)\nend\n");
        let k = m.functions.iter().find(|f| f.name == "k").expect("fn k");
        assert!(k.params[0].default.is_none());
        assert!(!m.manifest.contains(semantic_ir::Feature::DefaultParams));
    }

    #[test]
    fn def_rest_param_never_carries_default() {
        // Splat params keep `default: None` even though the grammar is
        // permissive — `extract_params` attaches defaults only to ordinary
        // params.
        let m = lower("def f(a = 1, *r)\nend\n");
        let f = m.functions.iter().find(|f| f.name == "f").expect("fn f");
        assert_eq!(f.params.len(), 2);
        assert!(f.params[0].default.is_some());
        assert_eq!(f.params[1].kind, ParamKind::Rest);
        assert!(f.params[1].default.is_none());
    }

    #[test]
    fn call_omitting_default_lowers_to_partial_arg_list() {
        // A call that omits the defaulted arg must lower with FEWER args —
        // the frontend lowers the args present and does NOT pad.  The SIR
        // validator now permits a DirectCall/BuiltinCall that omits trailing
        // defaulted params.
        let m = lower("def f(a, b = 1)\n  a + b\nend\nf(5)\n");
        // The whole module (def with default + partial call) validates.
        assert!(
            semantic_ir::validate(&m).is_ok(),
            "module with partial call should validate: {:?}",
            semantic_ir::validate(&m).issues
        );
        // Locate the call in main's body and confirm it carries a single arg.
        let body = main_body(&m);
        // The defaulted-arg value `1` must NOT appear in the call's lowered
        // args — i.e. the call carries exactly one arg (the literal `5`),
        // proving the frontend does not pad omitted defaults.
        let mut found_int5 = false;
        let mut found_int1_as_arg = false;
        for s in &body.stmts {
            if let Stmt::ExprStmt { expr, .. } = s {
                scan_call_args(expr, &mut found_int5, &mut found_int1_as_arg);
            }
        }
        scan_call_args(&body.value, &mut found_int5, &mut found_int1_as_arg);
        assert!(found_int5, "expected the literal arg 5 in the call to f");
        assert!(
            !found_int1_as_arg,
            "frontend must NOT pad the omitted default (no synthesised arg 1)"
        );
    }

    /// Walk an expression; for every call node record whether its arg list
    /// contains an `IntLit 5` and whether it contains an `IntLit 1`.  Used
    /// to prove `f(5)` lowers to a single-arg call (the `b = 1` default is
    /// NOT padded in by the frontend).
    fn scan_call_args(e: &Expr, found5: &mut bool, found1: &mut bool) {
        if let Expr::DirectCall { args, .. } | Expr::BuiltinCall { args, .. } = e {
            for a in args {
                if matches!(a, Expr::IntLit { value: 5, .. }) {
                    *found5 = true;
                }
                if matches!(a, Expr::IntLit { value: 1, .. }) {
                    *found1 = true;
                }
                scan_call_args(a, found5, found1);
            }
        }
    }

    // -----------------------------------------------------------------
    // Phase KW7 (Ruby 1.0 unblock) — keyword parameters & arguments.
    //
    // The frontend now PRODUCES `Param { kind: Keyword }` for `def f(a:)` /
    // `def f(a: 1)` and `Expr::KeywordArg` for `f(a: 1)`.  Required vs
    // optional rides on the existing `default` field (`None` ⇒ required,
    // `Some` ⇒ optional) exactly as positional optionals do — the
    // distinguishing axis is `ParamKind::Keyword`.
    // -----------------------------------------------------------------

    #[test]
    fn def_required_keyword_param_lowers_to_keyword_no_default() {
        // `def f(a:)` → the single Param is `Keyword` with `default: None`.
        let m = lower("def f(a:)\n  a + 0\nend\n");
        let f = m.functions.iter().find(|f| f.name == "f").expect("fn f");
        assert_eq!(f.params.len(), 1);
        assert_eq!(f.params[0].kind, ParamKind::Keyword);
        assert!(
            f.params[0].default.is_none(),
            "required keyword param must have NO default"
        );
    }

    #[test]
    fn def_optional_keyword_param_lowers_to_keyword_with_default_intlit() {
        // `def f(a: 1)` → `Keyword` with `default: Some(IntLit 1)`.
        let m = lower("def f(a: 1)\n  a + 0\nend\n");
        let f = m.functions.iter().find(|f| f.name == "f").expect("fn f");
        assert_eq!(f.params[0].kind, ParamKind::Keyword);
        match &f.params[0].default {
            Some(boxed) => assert!(
                matches!(&**boxed, Expr::IntLit { value: 1, .. }),
                "expected default IntLit(1), got {:?}",
                boxed
            ),
            None => panic!("expected a default value, got None"),
        }
    }

    #[test]
    fn def_mixed_positional_and_keyword_params_lower_in_order() {
        // `def f(a, b:, c: 2)` → positional-required `a`, required keyword
        // `b`, optional keyword `c`.  The declared order is preserved.
        let m = lower("def f(a, b:, c: 2)\n  a + b + c\nend\n");
        let f = m.functions.iter().find(|f| f.name == "f").expect("fn f");
        assert_eq!(f.params.len(), 3);
        // a — positional required.
        assert_eq!(f.params[0].kind, ParamKind::Required);
        assert!(f.params[0].default.is_none());
        // b — required keyword.
        assert_eq!(f.params[1].kind, ParamKind::Keyword);
        assert!(f.params[1].default.is_none());
        // c — optional keyword with default 2.
        assert_eq!(f.params[2].kind, ParamKind::Keyword);
        assert!(
            matches!(f.params[2].default.as_deref(), Some(Expr::IntLit { value: 2, .. })),
            "c must default to IntLit(2)"
        );
    }

    #[test]
    fn keyword_param_observes_keyword_params_feature() {
        // Any keyword param must make the manifest declare `KeywordParams`
        // (the SIR validator gates the feature otherwise).
        let m = lower("def f(a:)\n  a + 0\nend\n");
        assert!(
            m.manifest.contains(semantic_ir::Feature::KeywordParams),
            "manifest should declare KeywordParams; declared = {:?}",
            m.manifest
        );
    }

    #[test]
    fn positional_param_never_trips_keyword_params_feature() {
        // Regression: an ordinary positional param must NOT observe
        // KeywordParams (only a `:`-suffixed param or a keyword arg does).
        let m = lower("def f(a)\n  a + 0\nend\n");
        assert_eq!(
            m.functions.iter().find(|f| f.name == "f").unwrap().params[0].kind,
            ParamKind::Required
        );
        assert!(!m.manifest.contains(semantic_ir::Feature::KeywordParams));
    }

    #[test]
    fn keyword_call_arg_lowers_to_keyword_arg_node() {
        // `f(x: 2)` → the call's arg list holds an `Expr::KeywordArg`
        // wrapper `{ name: "x", value: IntLit 2 }` (NOT a trailing hash).
        // Use a BARE call to `f` (not wrapped in `puts`) so the arg is the
        // direct child of the DirectCall.
        let m = lower("def f(x:)\n  x + 0\nend\nf(x: 2)\n");
        let body = main_body(&m);
        match &body.value {
            Expr::DirectCall { fn_name, args, .. } if fn_name == "f" => {
                assert_eq!(args.len(), 1, "expected exactly one (keyword) arg");
                match &args[0] {
                    Expr::KeywordArg { name, value, .. } => {
                        assert_eq!(name, "x");
                        assert!(
                            matches!(&**value, Expr::IntLit { value: 2, .. }),
                            "keyword value must be IntLit(2), got {:?}",
                            value
                        );
                    }
                    other => panic!("expected KeywordArg, got {:?}", other),
                }
            }
            other => panic!("expected DirectCall to f, got {:?}", other),
        }
        // The whole module round-trips through the validator.
        assert!(
            semantic_ir::validate(&m).is_ok(),
            "module with a keyword call arg should validate: {:?}",
            semantic_ir::validate(&m).issues
        );
    }

    #[test]
    fn keyword_arg_follows_positional_arg() {
        // `g(1, y: 2)` → `args: [IntLit(1), KeywordArg{ name:"y", .. }]`.
        // The positional stays bare and precedes the keyword.
        let m = lower("def g(a, y:)\n  a + y\nend\ng(1, y: 2)\n");
        let body = main_body(&m);
        match &body.value {
            Expr::DirectCall { fn_name, args, .. } if fn_name == "g" => {
                assert_eq!(args.len(), 2);
                assert!(
                    matches!(&args[0], Expr::IntLit { value: 1, .. }),
                    "first arg must be the bare positional IntLit(1), got {:?}",
                    args[0]
                );
                assert!(
                    matches!(&args[1], Expr::KeywordArg { name, .. } if name == "y"),
                    "second arg must be the keyword `y`, got {:?}",
                    args[1]
                );
            }
            other => panic!("expected DirectCall to g, got {:?}", other),
        }
        assert!(semantic_ir::validate(&m).is_ok());
    }

    #[test]
    fn keyword_arg_observes_keyword_params_feature() {
        // A keyword ARG (call side) also trips the feature, even when the
        // callee's params happen to be declared elsewhere.
        let m = lower("def f(x:)\n  x + 0\nend\nf(x: 3)\n");
        assert!(
            m.manifest.contains(semantic_ir::Feature::KeywordParams),
            "a keyword call arg must observe KeywordParams; manifest = {:?}",
            m.manifest
        );
    }

    #[test]
    fn omitting_required_keyword_is_rejected_by_validator() {
        // `def h(x:)` declares a REQUIRED keyword; a call `h()` that omits it
        // must be rejected by the SIR validator (round-trip proof that the
        // required-ness produced by the frontend is enforced downstream).
        let m = lower("def h(x:)\n  x + 0\nend\nh()\n");
        let result = semantic_ir::validate(&m);
        assert!(
            !result.is_ok(),
            "omitting a required keyword must fail validation, but it passed"
        );
    }

    #[test]
    fn supplying_required_keyword_validates() {
        // The mirror of the above: supplying the required keyword validates.
        let m = lower("def h(x:)\n  x + 0\nend\nh(x: 9)\n");
        assert!(
            semantic_ir::validate(&m).is_ok(),
            "supplying a required keyword should validate: {:?}",
            semantic_ir::validate(&m).issues
        );
    }

    #[test]
    fn optional_keyword_may_be_omitted_at_call() {
        // `def f(a: 1)` — an OPTIONAL keyword may be omitted; `f()` validates
        // (the backend fills the default).
        let m = lower("def f(a: 1)\n  a + 0\nend\nf()\n");
        assert!(
            semantic_ir::validate(&m).is_ok(),
            "omitting an optional keyword should validate: {:?}",
            semantic_ir::validate(&m).issues
        );
    }

    #[test]
    fn empty_program_compiles_to_nil_main() {
        let m = lower("");
        assert_eq!(m.name, "test");
        let b = main_body(&m);
        assert!(b.stmts.is_empty());
        assert!(matches!(b.value, Expr::NilLit { .. }));
    }

    #[test]
    fn assignment_becomes_let_binding() {
        let m = lower("x = 1");
        let b = main_body(&m);
        assert_eq!(b.stmts.len(), 1);
        match &b.stmts[0] {
            Stmt::LetBinding { name, value, .. } => {
                assert_eq!(name, "x");
                assert!(matches!(value, Expr::IntLit { value: 1, .. }));
            }
            other => panic!("expected LetBinding, got {:?}", other),
        }
        // Block value is NilLit because the assignment is not a
        // tail expression.
        assert!(matches!(b.value, Expr::NilLit { .. }));
    }

    #[test]
    fn tail_expression_becomes_block_value() {
        let m = lower("1 + 2");
        let b = main_body(&m);
        assert!(b.stmts.is_empty());
        match &b.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "+");
                assert_eq!(args.len(), 2);
                assert!(matches!(args[0], Expr::IntLit { value: 1, .. }));
                assert!(matches!(args[1], Expr::IntLit { value: 2, .. }));
            }
            other => panic!("expected BuiltinCall(+, …), got {:?}", other),
        }
    }

    #[test]
    fn three_assignments_become_three_let_bindings() {
        let m = lower("x = 1\ny = 2\nz = x + y\n");
        let b = main_body(&m);
        assert_eq!(b.stmts.len(), 3);
        let names: Vec<&str> = b.stmts.iter().filter_map(binding_name).collect();
        assert_eq!(names, vec!["x", "y", "z"]);
        // The third RHS references x and y as locals, so it is a sequential
        // `LetStarBinding` (`let*`), not a parallel `LetBinding`.
        if let Some(value) = binding_value(&b.stmts[2]) {
            match value {
                Expr::BuiltinCall { name, args, .. } => {
                    assert_eq!(name, "+");
                    assert!(
                        matches!(&args[0], Expr::VarRef { name, scope, .. } if name == "x" && *scope == Scope::Local)
                    );
                    assert!(
                        matches!(&args[1], Expr::VarRef { name, scope, .. } if name == "y" && *scope == Scope::Local)
                    );
                }
                other => panic!("expected BuiltinCall(+, …), got {:?}", other),
            }
        }
    }

    #[test]
    fn reassignment_routes_through_assign() {
        // First `x = 1` binds; second `x = 2` re-binds and so must
        // emit `Stmt::Assign`, not another `LetBinding`.  SIR's
        // semantics require this distinction so backends (Rust,
        // TypeScript, Go) can emit `let`/`const`/`var` correctly.
        let m = lower("x = 1\nx = 2\n");
        let b = main_body(&m);
        assert_eq!(b.stmts.len(), 2);
        assert!(matches!(&b.stmts[0], Stmt::LetBinding { name, .. } if name == "x"));
        match &b.stmts[1] {
            Stmt::Assign { name, scope, value, .. } => {
                assert_eq!(name, "x");
                assert_eq!(*scope, Scope::Local);
                assert!(matches!(value, Expr::IntLit { value: 2, .. }));
            }
            other => panic!("expected Assign, got {:?}", other),
        }
    }

    #[test]
    fn puts_call_becomes_builtin_with_may_print() {
        let m = lower("puts(42)");
        let b = main_body(&m);
        // The call is the tail expression — it becomes the value.
        match &b.value {
            Expr::BuiltinCall { name, args, effects, .. } => {
                assert_eq!(name, "puts");
                assert_eq!(args.len(), 1);
                assert!(matches!(args[0], Expr::IntLit { value: 42, .. }));
                assert!(effects.contains(Effect::MayPrint));
            }
            other => panic!("expected BuiltinCall(puts, …), got {:?}", other),
        }
    }

    #[test]
    fn string_literal_is_preserved() {
        let m = lower(r#"x = "hello""#);
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::LetBinding { value, .. } => match value {
                Expr::StrLit { value, .. } => assert_eq!(value, "hello"),
                other => panic!("expected StrLit, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    }

    #[test]
    fn mixed_assignment_and_tail_call() {
        let m = lower("x = 1\nputs(x)");
        let b = main_body(&m);
        assert_eq!(b.stmts.len(), 1);
        assert!(matches!(&b.stmts[0], Stmt::LetBinding { name, .. } if name == "x"));
        // The trailing `puts(x)` is a method call without an
        // assignment around it, so it becomes the block's value.
        match &b.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "puts");
                assert!(
                    matches!(&args[0], Expr::VarRef { name, scope, .. } if name == "x" && *scope == Scope::Local)
                );
            }
            other => panic!("expected BuiltinCall(puts, …), got {:?}", other),
        }
    }

    #[test]
    fn lowered_module_passes_sir_validator() {
        // The validator is the canonical "is this module well-formed"
        // check — every lowering pass must produce a module that
        // passes it.  This is the load-bearing acceptance criterion
        // for the SIR pipeline.
        let m = lower("x = 1\ny = 2\nputs(x + y)\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected our output: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------
    // Phase 6a — `def name(params) … end` method definitions.
    // -----------------------------------------------------------------

    #[test]
    fn def_lowers_to_top_level_function() {
        let m = lower("def add(x, y)\n  x + y\nend\n");
        // The user-defined function comes first; `main` is last.
        let add = m
            .functions
            .iter()
            .find(|f| f.name == "add")
            .expect("expected `add` function");
        assert_eq!(add.params.len(), 2);
        assert_eq!(add.params[0].name, "x");
        assert_eq!(add.params[1].name, "y");
        match &add.body.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "+");
                assert_eq!(args.len(), 2);
            }
            other => panic!("expected BuiltinCall(+, …), got {:?}", other),
        }
        // `main` is still present and exported.
        assert!(m.functions.iter().any(|f| f.name == "main"));
        assert!(m.exports.iter().any(|e| e.name == "main"));
    }

    // ─────────────────────────────────────────────────────────────────
    // Phase FC — implicit return of a trailing `if`/`unless` from a
    // method body.  Ruby has no explicit `return`: a method's value is
    // its last evaluated expression, and `if`/`unless` ARE expressions.
    // Before this fix a body ending in a conditional promoted nothing to
    // the block `value`, so the method returned `nil` on every backend.
    // ─────────────────────────────────────────────────────────────────

    /// Fetch the body `Block` of a lowered top-level `def` by name.
    fn fn_body<'a>(m: &'a semantic_ir::Module, name: &str) -> &'a semantic_ir::Block {
        &m.functions
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("expected `{}` function", name))
            .body
    }

    #[test]
    fn def_body_ending_in_if_returns_the_if_expr() {
        // `bigger` ends in an `if/else` — the whole conditional is the
        // method's return value, so it must land in `body.value` (NOT be
        // dropped as a bare tail statement leaving `value = nil`).
        let m = lower("def bigger(a, b)\n  if a > b\n    a\n  else\n    b\n  end\nend\n");
        let body = fn_body(&m, "bigger");
        match &body.value {
            Expr::If { then_branch, else_branch, .. } => {
                // Each branch's own tail promotes to a VarRef value.
                assert!(
                    matches!(&then_branch.value, Expr::VarRef { name, .. } if name == "a"),
                    "then-branch value should be `a`, got {:?}",
                    then_branch.value
                );
                assert!(
                    matches!(&else_branch.value, Expr::VarRef { name, .. } if name == "b"),
                    "else-branch value should be `b`, got {:?}",
                    else_branch.value
                );
            }
            other => panic!("expected body.value = Expr::If, got {:?}", other),
        }
        // The conditional was promoted, so it is NOT also left dangling as
        // a discarded statement in the body.
        assert!(
            !body
                .stmts
                .iter()
                .any(|s| matches!(s, Stmt::ExprStmt { expr: Expr::If { .. }, .. })),
            "the tail `if` must be promoted to value, not duplicated as a stmt"
        );
    }

    #[test]
    fn def_body_ending_in_unless_returns_the_if_expr() {
        // `unless c` lowers to `if !c` — still promoted to the body value.
        let m = lower("def pick(a, b)\n  unless a\n    b\n  else\n    a\n  end\nend\n");
        let body = fn_body(&m, "pick");
        match &body.value {
            Expr::If { cond, .. } => assert!(
                matches!(&**cond, Expr::BuiltinCall { name, .. } if name == "not"),
                "`unless` cond should be wrapped in `not`, got {:?}",
                cond
            ),
            other => panic!("expected body.value = Expr::If, got {:?}", other),
        }
    }

    #[test]
    fn def_body_leading_stmts_then_tail_if_are_both_kept() {
        // Statements BEFORE the tail conditional stay in `stmts`; only the
        // final `if` is promoted to `value`.
        let m = lower(
            "def f(a)\n  x = a + 1\n  if x > 0\n    x\n  else\n    0\n  end\nend\n",
        );
        let body = fn_body(&m, "f");
        assert!(
            !body.stmts.is_empty(),
            "the `x = a + 1` binding must remain a statement"
        );
        assert!(
            matches!(&body.value, Expr::If { .. }),
            "the tail `if` must be the body value, got {:?}",
            body.value
        );
    }

    #[test]
    fn nested_tail_if_promotes_recursively() {
        // The else-branch itself ends in an `if`; that inner conditional
        // must promote to the else-branch's value (recursion through
        // `lower_clause_statements` → `lower_tail_value`).
        let m = lower(
            "def grade(n)\n  if n > 90\n    1\n  else\n    if n > 80\n      2\n    else\n      3\n    end\n  end\nend\n",
        );
        let body = fn_body(&m, "grade");
        let Expr::If { else_branch, .. } = &body.value else {
            panic!("expected body.value = Expr::If, got {:?}", body.value);
        };
        assert!(
            matches!(&else_branch.value, Expr::If { .. }),
            "the nested tail `if` must promote to the else-branch value, got {:?}",
            else_branch.value
        );
    }

    #[test]
    fn def_body_tail_if_module_passes_sir_validator() {
        let m = lower("def bigger(a, b)\n  if a > b\n    a\n  else\n    b\n  end\nend\n");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected our output: {:?}", result);
    }

    // ─────────────────────────────────────────────────────────────────
    // Phase FC (cont.) — implicit return of a trailing `case` from a
    // method body.  `case` (both `case/when` value-matching and `case/in`
    // pattern-matching) already lowers to a chained `Expr::If`; promoting
    // it in tail position makes the method return the matched arm's value
    // instead of `nil`.
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn def_body_ending_in_case_when_returns_the_chained_if() {
        // `grade` ends in a `case/when` — the whole chain is the method's
        // return value, landing in `body.value` (a chained `Expr::If`), and
        // each arm's own tail (`"A"`, `"B"`, `"C"`) promotes to that arm's
        // block value.
        let m = lower(
            "def grade(n)\n  case n\n  when 90\n    \"A\"\n  when 80\n    \"B\"\n  else\n    \"C\"\n  end\nend\n",
        );
        let body = fn_body(&m, "grade");
        let Expr::If { then_branch, else_branch, .. } = &body.value else {
            panic!("expected body.value = chained Expr::If, got {:?}", body.value);
        };
        // First arm value is the string "A".
        assert!(
            matches!(&then_branch.value, Expr::StrLit { value, .. } if value == "A"),
            "first when-arm value should be \"A\", got {:?}",
            then_branch.value
        );
        // The else_branch is itself another If (the `when 80` step), proving
        // the chain unwound rather than collapsing to nil.
        assert!(
            matches!(&else_branch.value, Expr::If { .. }),
            "case chain should nest in the else branch, got {:?}",
            else_branch.value
        );
        // The tail `case` was promoted, not left dangling as a statement.
        assert!(
            !body
                .stmts
                .iter()
                .any(|s| matches!(s, Stmt::ExprStmt { expr: Expr::If { .. }, .. })),
            "the tail `case` must be promoted to value, not duplicated as a stmt"
        );
    }

    #[test]
    fn def_body_ending_in_case_in_pattern_returns_value() {
        // `case/in` pattern matching shares the same `case_statement` rule and
        // lowers to a chained `Expr::If`; a method ending in one returns the
        // matched arm's value.
        let m = lower(
            "def classify(x)\n  case x\n  in 0\n    \"zero\"\n  else\n    \"other\"\n  end\nend\n",
        );
        let body = fn_body(&m, "classify");
        assert!(
            matches!(&body.value, Expr::If { .. }),
            "the tail `case/in` must be the body value, got {:?}",
            body.value
        );
    }

    #[test]
    fn def_body_leading_stmts_then_tail_case_are_both_kept() {
        // A statement before the tail `case` stays in `stmts`; only the final
        // `case` is promoted to `value`.
        let m = lower(
            "def label(n)\n  m = n + 1\n  case m\n  when 1\n    \"one\"\n  else\n    \"many\"\n  end\nend\n",
        );
        let body = fn_body(&m, "label");
        assert!(
            !body.stmts.is_empty(),
            "the `m = n + 1` binding must remain a statement"
        );
        assert!(
            matches!(&body.value, Expr::If { .. }),
            "the tail `case` must be the body value, got {:?}",
            body.value
        );
    }

    #[test]
    fn def_body_tail_case_module_passes_sir_validator() {
        let m = lower(
            "def grade(n)\n  case n\n  when 90\n    \"A\"\n  else\n    \"C\"\n  end\nend\n",
        );
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected our output: {:?}", result);
    }

    #[test]
    fn def_rest_param_lowers_to_kind_rest() {
        // M3: `def f(*r); end` → one Param whose kind is Rest (previously the
        // splat prefix was dropped and the param lowered as Required).
        let m = lower("def f(*r)\nend\n");
        let f = m.functions.iter().find(|f| f.name == "f").expect("fn f");
        assert_eq!(f.params.len(), 1);
        assert_eq!(f.params[0].name, "r");
        assert_eq!(f.params[0].kind, ParamKind::Rest);
    }

    #[test]
    fn def_kwrest_param_lowers_to_kind_kwrest() {
        // M3: `def g(**o); end` → one Param whose kind is KwRest.
        let m = lower("def g(**o)\nend\n");
        let g = m.functions.iter().find(|f| f.name == "g").expect("fn g");
        assert_eq!(g.params.len(), 1);
        assert_eq!(g.params[0].name, "o");
        assert_eq!(g.params[0].kind, ParamKind::KwRest);
    }

    #[test]
    fn def_required_then_rest_preserves_order_and_kinds() {
        // M3: `def h(a, *r); end` → [Required(a), Rest(r)].
        let m = lower("def h(a, *r)\nend\n");
        let h = m.functions.iter().find(|f| f.name == "h").expect("fn h");
        assert_eq!(h.params.len(), 2);
        assert_eq!(h.params[0].kind, ParamKind::Required);
        assert_eq!(h.params[1].name, "r");
        assert_eq!(h.params[1].kind, ParamKind::Rest);
    }

    #[test]
    fn def_plain_param_stays_required() {
        // Regression: an ordinary positional param keeps kind Required.
        let m = lower("def k(a)\nend\n");
        let k = m.functions.iter().find(|f| f.name == "k").expect("fn k");
        assert_eq!(k.params[0].kind, ParamKind::Required);
    }

    #[test]
    fn block_captures_outer_local() {
        // M4: a block referencing an enclosing local captures it. The hoisted
        // `__block_0` gains a capture named `x`, and inside its body the read
        // of `x` is rewritten from Local to Capture so the SIR validates.
        let m = lower("def run\n  x = 10\n  [1, 2, 3].each { |n| n + x }\nend\n");
        let blk = m
            .functions
            .iter()
            .find(|f| f.name == "__block_0")
            .expect("hoisted block function");
        assert!(
            blk.captures.iter().any(|c| c.name == "x"),
            "block should capture outer local `x`; captures = {:?}",
            blk.captures.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
        // The block param `n` stays a param; `x` is NOT a param.
        assert!(blk.params.iter().any(|p| p.name == "n"));
        assert!(!blk.params.iter().any(|p| p.name == "x"));
        // The whole module validates (the capture rewrite keeps the SIR sound).
        assert!(
            semantic_ir::validate(&m).is_ok(),
            "validation failed: {:?}",
            semantic_ir::validate(&m).issues
        );
    }

    #[test]
    fn block_does_not_capture_block_param_or_fresh_local() {
        // Regression: a block whose body only uses its own param (and a fresh
        // block-internal local) captures nothing.
        let m = lower("def run\n  [1, 2, 3].each do |n|\n    y = n + 1\n    y\n  end\nend\n");
        let blk = m
            .functions
            .iter()
            .find(|f| f.name == "__block_0")
            .expect("hoisted block function");
        assert!(blk.captures.is_empty(), "captures = {:?}", blk.captures);
    }

    #[test]
    fn block_reassigning_outer_name_is_not_captured() {
        // A name assigned inside the block is block-local (v0: capture-then-
        // reassign would need by-reference capture), so it is NOT captured.
        let m = lower("def run\n  x = 1\n  [1, 2, 3].each do |n|\n    x = n\n    x\n  end\nend\n");
        let blk = m
            .functions
            .iter()
            .find(|f| f.name == "__block_0")
            .expect("hoisted block function");
        assert!(
            !blk.captures.iter().any(|c| c.name == "x"),
            "an in-block-assigned name must not be captured; captures = {:?}",
            blk.captures.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn def_with_no_params_lowers_cleanly() {
        let m = lower("def hello\nend\n");
        let hello = m
            .functions
            .iter()
            .find(|f| f.name == "hello")
            .expect("expected `hello` function");
        assert!(hello.params.is_empty());
        // Empty body → block.value is NilLit.
        assert!(matches!(hello.body.value, Expr::NilLit { .. }));
    }

    #[test]
    fn def_does_not_leak_locals_to_outer_scope() {
        // The method body declares `x` as a local (via `x = 1`); the
        // outer program also has `x = 2`.  Both should be first
        // occurrences (LetBinding), because the method's locals are
        // confined to its body.
        let m = lower("def inner\n  x = 1\nend\nx = 2\n");
        let main = m.functions.iter().find(|f| f.name == "main").unwrap();
        // First main statement is the no-op for the hoisted def.
        // Second is the outer `x = 2` LetBinding.
        let outer_x = main.body.stmts.iter().find_map(|s| match s {
            Stmt::LetBinding { name, .. } if name == "x" => Some(name.as_str()),
            _ => None,
        });
        assert_eq!(outer_x, Some("x"));
        let inner = m.functions.iter().find(|f| f.name == "inner").unwrap();
        let inner_x_kind = match &inner.body.stmts[0] {
            Stmt::LetBinding { name, .. } => format!("LetBinding({})", name),
            Stmt::Assign { name, .. } => format!("Assign({})", name),
            other => format!("{:?}", other),
        };
        // The method's `x = 1` is a LetBinding (first occurrence in
        // the *method scope*), not an Assign.
        assert!(
            inner_x_kind.starts_with("LetBinding"),
            "expected LetBinding in method body, got {inner_x_kind}"
        );
    }

    #[test]
    fn def_with_param_reassignment_routes_through_assign() {
        // Parameters are pre-declared as locals inside the method,
        // so `x = 2` re-binds and emits `Stmt::Assign`.
        let m = lower("def f(x)\n  x = 2\nend\n");
        let f = m.functions.iter().find(|f| f.name == "f").unwrap();
        match &f.body.stmts[0] {
            Stmt::Assign { name, scope, .. } => {
                assert_eq!(name, "x");
                assert_eq!(*scope, Scope::Local);
            }
            other => panic!("expected Assign(x), got {:?}", other),
        }
    }

    // -----------------------------------------------------------------
    // Phase 6b — `if … else … end` / `unless … else … end`
    // -----------------------------------------------------------------

    #[test]
    fn if_lowers_to_expr_if() {
        let m = lower("x = 1\nif x\n  y = 2\nend\n");
        let main = main_body(&m);
        // main.stmts: LetBinding(x=1), ExprStmt(If { ... })
        let if_stmt = main.stmts.iter().find_map(|s| match s {
            Stmt::ExprStmt { expr: Expr::If { cond, .. }, .. } => Some(cond),
            _ => None,
        });
        assert!(if_stmt.is_some(), "expected If expr-stmt in main body");
    }

    #[test]
    fn if_else_lowers_with_else_branch() {
        let m = lower("if x\n  y = 1\nelse\n  y = 2\nend\n");
        let main = main_body(&m);
        let if_expr = main
            .stmts
            .iter()
            .find_map(|s| match s {
                Stmt::ExprStmt { expr: e @ Expr::If { .. }, .. } => Some(e),
                _ => None,
            })
            .expect("expected If expr");
        // The else_branch has one statement.
        if let Expr::If { else_branch, .. } = if_expr {
            assert!(
                !else_branch.stmts.is_empty()
                    || !matches!(else_branch.value, Expr::NilLit { .. }),
                "expected non-empty else branch"
            );
        }
    }

    #[test]
    fn unless_negates_condition() {
        let m = lower("unless x\n  y = 1\nend\n");
        let main = main_body(&m);
        let if_expr = main
            .stmts
            .iter()
            .find_map(|s| match s {
                Stmt::ExprStmt { expr: e @ Expr::If { .. }, .. } => Some(e),
                _ => None,
            })
            .expect("expected If expr");
        // `unless cond` lowers to `if !cond` — the cond should be a
        // `BuiltinCall("not", ...)`.
        if let Expr::If { cond, .. } = if_expr {
            assert!(
                matches!(&**cond, Expr::BuiltinCall { name, .. } if name == "not"),
                "expected `unless` to wrap cond in `not` builtin, got {:?}",
                cond
            );
        }
    }

    #[test]
    fn if_elsif_else_chain_nests_right() {
        let m = lower("if x\n  a = 1\nelsif y\n  a = 2\nelse\n  a = 3\nend\n");
        let main = main_body(&m);
        let if_expr = main
            .stmts
            .iter()
            .find_map(|s| match s {
                Stmt::ExprStmt { expr: e @ Expr::If { .. }, .. } => Some(e),
                _ => None,
            })
            .expect("expected If expr");
        // The outer if's else_branch.value should itself be another If
        // (the elsif).  The inner If's else_branch contains the final
        // else clause.
        if let Expr::If { else_branch, .. } = if_expr {
            match &else_branch.value {
                Expr::If { .. } => {} // ✓
                other => panic!("expected nested If in else_branch.value, got {:?}", other),
            }
        }
    }

    #[test]
    fn if_module_passes_sir_validator() {
        let m = lower(concat!(
            "x = 1\n",
            "if x\n",
            "  y = 2\n",
            "else\n",
            "  y = 3\n",
            "end\n",
        ));
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected our output: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------
    // Phase 6c — `while … end` / `until … end`
    // -----------------------------------------------------------------

    #[test]
    fn program_with_end_marker_lowers_only_the_code() {
        // Phase FC — a trailing `__END__` data section is stripped by the
        // lexer, so the program lowers cleanly from just the code above it
        // (the data is neither parsed nor lowered).
        let m = lower("x = 1\nputs(x)\n__END__\nthis is data, not ruby code!\n");
        let b = main_body(&m);
        // The `x = 1` binding is present and the data produced no stmts
        // beyond the code (puts is the trailing value).
        assert!(
            b.stmts.iter().any(|s| matches!(s, Stmt::LetBinding { name, .. } if name == "x")),
            "expected `x = 1` from the code section"
        );
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected a program with a trailing __END__: {:?}",
            result
        );
    }

    #[test]
    fn while_lowers_to_stmt_while() {
        let m = lower("x = 0\nwhile x\n  y = 1\nend\n");
        let main = main_body(&m);
        let while_stmt = main.stmts.iter().find_map(|s| match s {
            Stmt::While { cond, .. } => Some(cond),
            _ => None,
        });
        assert!(while_stmt.is_some(), "expected While stmt");
    }

    #[test]
    fn until_negates_condition() {
        let m = lower("x = 0\nuntil x\n  y = 1\nend\n");
        let main = main_body(&m);
        let while_stmt = main.stmts.iter().find_map(|s| match s {
            Stmt::While { cond, .. } => Some(cond),
            _ => None,
        }).expect("expected While stmt");
        assert!(
            matches!(while_stmt, Expr::BuiltinCall { name, .. } if name == "not"),
            "expected `until` to wrap cond in `not`, got {:?}",
            while_stmt
        );
    }

    #[test]
    fn while_module_passes_sir_validator() {
        let m = lower("x = 0\nwhile x\n  y = 1\nend\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected our output: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------
    // Phase 6d — array `[a, b, c]` and hash `{a: 1, b => 2}` literals.
    // -----------------------------------------------------------------

    #[test]
    fn array_literal_lowers_to_seq_lit() {
        let m = lower("x = [1, 2, 3]\n");
        let main = main_body(&m);
        let arr = main.stmts.iter().find_map(|s| match s {
            Stmt::LetBinding { value: Expr::SeqLit { items, .. }, .. } => Some(items),
            _ => None,
        }).expect("expected SeqLit");
        assert_eq!(arr.len(), 3);
        assert!(matches!(arr[0], Expr::IntLit { value: 1, .. }));
    }

    #[test]
    fn empty_array_literal_lowers_to_empty_seq_lit() {
        let m = lower("x = []\n");
        let main = main_body(&m);
        let arr = main.stmts.iter().find_map(|s| match s {
            Stmt::LetBinding { value: Expr::SeqLit { items, .. }, .. } => Some(items),
            _ => None,
        }).expect("expected SeqLit");
        assert!(arr.is_empty());
    }

    #[test]
    fn hash_shorthand_lowers_to_map_lit_with_sym_keys() {
        let m = lower("x = {a: 1, b: 2}\n");
        let main = main_body(&m);
        let entries = main
            .stmts
            .iter()
            .find_map(|s| match s {
                Stmt::LetBinding { value: Expr::MapLit { entries, .. }, .. } => Some(entries),
                _ => None,
            })
            .expect("expected MapLit");
        assert_eq!(entries.len(), 2);
        // Keys should be SymLit("a") and SymLit("b").
        assert!(
            matches!(&entries[0].key, Expr::SymLit { name, .. } if name == "a")
        );
        assert!(
            matches!(&entries[1].key, Expr::SymLit { name, .. } if name == "b")
        );
    }

    #[test]
    fn array_and_hash_modules_pass_sir_validator() {
        let m = lower("x = [1, 2]\ny = {a: 1}\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected our output: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------
    // Phase 6e — symbol literals `:foo` / `:"bar"`.
    // -----------------------------------------------------------------

    #[test]
    fn symbol_lowers_to_sym_lit() {
        let m = lower("x = :foo\n");
        let main = main_body(&m);
        let sym = main.stmts.iter().find_map(|s| match s {
            Stmt::LetBinding { value: Expr::SymLit { name, .. }, .. } => Some(name.as_str()),
            _ => None,
        });
        assert_eq!(sym, Some("foo"));
    }

    #[test]
    fn quoted_symbol_lowers_to_sym_lit_with_spaces() {
        let m = lower(r#"x = :"hello world""#);
        let main = main_body(&m);
        let sym = main.stmts.iter().find_map(|s| match s {
            Stmt::LetBinding { value: Expr::SymLit { name, .. }, .. } => Some(name.as_str()),
            _ => None,
        });
        assert_eq!(sym, Some("hello world"));
    }

    #[test]
    fn keyword_symbol_lowers_cleanly() {
        // `:def` — the symbol name happens to be a Ruby keyword.
        let m = lower("x = :def\n");
        let main = main_body(&m);
        let sym = main.stmts.iter().find_map(|s| match s {
            Stmt::LetBinding { value: Expr::SymLit { name, .. }, .. } => Some(name.as_str()),
            _ => None,
        });
        assert_eq!(sym, Some("def"));
    }

    #[test]
    fn symbol_module_passes_sir_validator() {
        let m = lower("x = :foo\ny = :bar\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected our output: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------
    // Phase 6f — `class Foo … end` / `module Foo … end`.
    //
    // v0 lowering collapses the namespace: nested `def`s are hoisted
    // to top-level Functions (same machinery as program-level `def`
    // hoisting), and the class/module declaration itself emits a
    // no-op `Stmt::ExprStmt(NilLit)`.  This is documented in the
    // CHANGELOG as a known v0 limitation that lands properly when
    // SIR grows a `class` / `namespace` node.
    // -----------------------------------------------------------------

    #[test]
    fn class_with_method_hoists_def_to_top_level() {
        // The nested `def greet … end` should appear as a top-level
        // `greet` function on the resulting module, exactly as
        // `def greet … end` at the program top level would.
        let m = lower("class Foo\n  def greet\n  end\nend\n");
        // O2: a class method hoists under a class-qualified name (`Foo__greet`).
        let greet = m.functions.iter().find(|f| f.name == "Foo__greet");
        assert!(
            greet.is_some(),
            "expected `Foo__greet` to be hoisted to top-level functions, got {:?}",
            m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        // `main` is still present (no top-level statements were lost).
        assert!(m.functions.iter().any(|f| f.name == "main"));
    }

    #[test]
    fn empty_class_lowers_to_class_def_stmt() {
        // Phase 14a (FC): `class Foo; end` lowers to a real
        // `Stmt::ClassDef { name: "Foo", body: vec![], .. }`,
        // replacing the pre-14a NilLit no-op contract.
        let m = lower("class Foo\nend\n");
        // Only `main` exists — empty class has no methods to hoist.
        assert_eq!(m.functions.len(), 1);
        assert_eq!(m.functions[0].name, "main");
        let main = main_body(&m);
        assert_eq!(main.stmts.len(), 1);
        match &main.stmts[0] {
            Stmt::ClassDef { name, body, .. } => {
                assert_eq!(name, "Foo");
                assert!(body.is_empty(), "Phase 14a body is always empty");
            }
            other => panic!("expected Stmt::ClassDef, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------
    // Phase 14a (FC) — empty `class Foo; end` first-class lowering.
    // -----------------------------------------------------------------

    #[test]
    fn empty_class_requests_classes_feature() {
        // Phase 14a (FC): emitting a `Stmt::ClassDef` triggers the
        // `Feature::Classes` manifest entry.  Without the manifest
        // entry, the validator would reject the module under its
        // strict declared-vs-used check.
        let m = lower("class Foo\nend\n");
        assert!(
            m.manifest.contains(semantic_ir::Feature::Classes),
            "manifest should contain Feature::Classes; got {:?}",
            m.manifest
        );
    }

    #[test]
    fn empty_class_module_passes_sir_validator() {
        // E2E: lower → validate.  Catches any drift between the
        // lowerer's manifest request and the validator's feature
        // accounting for the new ClassDef stmt.
        let m = lower("class Foo\nend\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected our class-only module: {:?}",
            result
        );
    }

    #[test]
    fn empty_class_preserves_class_name_verbatim() {
        // The class name must be the verbatim Ruby identifier — not
        // case-folded, not prefixed.  Smoke-test a non-"Foo" name to
        // make sure the helper doesn't pick up the `class` keyword
        // token instead of the Name token.
        let m = lower("class WidgetFactory\nend\n");
        let main = main_body(&m);
        match &main.stmts[0] {
            Stmt::ClassDef { name, .. } => assert_eq!(name, "WidgetFactory"),
            other => panic!("expected Stmt::ClassDef, got {:?}", other),
        }
    }

    #[test]
    fn class_with_method_body_still_emits_class_def_and_hoists_method() {
        // A method-*only* class keeps an empty `ClassDef.body` under
        // both Phase 14a and 14b: method defs are hoisted to top-level
        // `Function`s (SIR v0 has no method-as-statement node), so they
        // are not represented inside the body `Vec<Stmt>`.  Phase 14b
        // changes the *non-def* statements (see the tests below); the
        // method-hoist contract is unchanged.
        let m = lower("class Foo\n  def bar\n  end\nend\n");
        // O2: `bar` was hoisted to top-level under the class-qualified name.
        assert!(
            m.functions.iter().any(|f| f.name == "Foo__bar"),
            "expected hoisted `Foo__bar` function; got functions {:?}",
            m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        // The class statement itself produced a ClassDef in main (main[0]);
        // O2 appends a `__def_method__` registration after it.
        let main = main_body(&m);
        match &main.stmts[0] {
            Stmt::ClassDef { name, body, .. } => {
                assert_eq!(name, "Foo");
                assert!(
                    body.is_empty(),
                    "method-only class body stays empty (defs hoist); got {:?}",
                    body
                );
            }
            other => panic!("expected Stmt::ClassDef, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------
    // Phase 14b (FC) — class body with method defs + executable
    // statements.  Method defs continue to hoist to top-level
    // Functions; every *non-def* statement is now preserved in
    // `Stmt::ClassDef.body` (Phase 14a discarded them).
    // -----------------------------------------------------------------

    #[test]
    fn class_body_preserves_executable_statement_and_hoists_method() {
        // `MAX = 10` (an executable class-body statement) must survive
        // into `ClassDef.body`.  Phase 15c: a constant assignment lowers
        // to `Stmt::Assign { scope: Const }` (was `LetBinding`), while
        // `def bar` is still hoisted to a top-level Function.
        let m = lower("class Foo\n  MAX = 10\n  def bar\n  end\nend\n");
        assert!(
            m.functions.iter().any(|f| f.name == "Foo__bar"),
            "expected hoisted `Foo__bar`; got {:?}",
            m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        let main = main_body(&m);
        match &main.stmts[0] {
            Stmt::ClassDef { name, body, .. } => {
                assert_eq!(name, "Foo");
                assert_eq!(
                    body.len(),
                    1,
                    "the def is hoisted (0 body stmts) and only `MAX = 10` stays; got {:?}",
                    body
                );
                match &body[0] {
                    Stmt::Assign { name, scope, value, .. } => {
                        assert_eq!(name, "MAX");
                        assert_eq!(*scope, Scope::Const);
                        assert!(matches!(value, Expr::IntLit { value: 10, .. }));
                    }
                    other => panic!("expected Assign(MAX, Const), got {:?}", other),
                }
            }
            other => panic!("expected Stmt::ClassDef, got {:?}", other),
        }
    }

    #[test]
    fn class_body_preserves_multiple_statements_in_source_order() {
        // Two constant assignments must appear in `body` in the same
        // order they were written.  Phase 15c: constant assignments
        // lower to `Stmt::Assign { scope: Const }`.
        let m = lower("class Cfg\n  A = 1\n  B = 2\nend\n");
        let main = main_body(&m);
        match &main.stmts[0] {
            Stmt::ClassDef { body, .. } => {
                assert_eq!(body.len(), 2, "both assignments preserved; got {:?}", body);
                let names: Vec<&str> = body
                    .iter()
                    .map(|s| match s {
                        Stmt::Assign { name, scope, .. } => {
                            assert_eq!(*scope, Scope::Const, "constant → Const scope");
                            name.as_str()
                        }
                        other => panic!("expected Assign(Const), got {:?}", other),
                    })
                    .collect();
                assert_eq!(names, vec!["A", "B"]);
            }
            other => panic!("expected Stmt::ClassDef, got {:?}", other),
        }
    }

    #[test]
    fn class_with_body_statements_passes_sir_validator() {
        // E2E: a class mixing an executable statement and a method def
        // must lower to a module the validator accepts.  Guards against
        // drift between the lowerer's populated body and the
        // validator's new `check_stmt_seq`-based ClassDef walk.
        let m = lower("class Foo\n  MAX = 10\n  def bar\n  end\nend\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected class with populated body: {:?}",
            result
        );
    }

    #[test]
    fn nested_class_methods_hoisted_exactly_once() {
        // A class nested inside another class lowers to a nested
        // `ClassDef` in the outer class's body.  Both methods must be
        // hoisted to top-level Functions *exactly once* — a regression
        // here (double-hoist) would create duplicate function names and
        // trip the validator's name-uniqueness check.
        let m = lower(
            "class Outer\n  def o\n  end\n  class Inner\n    def i\n    end\n  end\nend\n",
        );
        // O2: each method hoists under its own class-qualified name.
        let count = |needle: &str| m.functions.iter().filter(|f| f.name == needle).count();
        assert_eq!(count("Outer__o"), 1, "`Outer__o` hoisted exactly once");
        assert_eq!(count("Inner__i"), 1, "`Inner__i` hoisted exactly once");
        // Outer's body carries the nested Inner ClassDef, followed (O2) by
        // Inner's method registration — nested-class registrations live in the
        // enclosing body right after the nested ClassDef, mirroring how a
        // top-level class's registrations follow it in `main`.
        let main = main_body(&m);
        match &main.stmts[0] {
            Stmt::ClassDef { name, body, .. } => {
                assert_eq!(name, "Outer");
                assert_eq!(
                    body.len(),
                    2,
                    "Inner ClassDef + its `__def_method__` registration; got {:?}",
                    body
                );
                match &body[0] {
                    Stmt::ClassDef { name, body, .. } => {
                        assert_eq!(name, "Inner");
                        assert!(body.is_empty(), "Inner is method-only → empty body");
                    }
                    other => panic!("expected nested ClassDef Inner, got {:?}", other),
                }
                assert!(
                    matches!(&body[1], Stmt::ExprStmt {
                        expr: Expr::BuiltinCall { name, .. }, .. } if name == "__def_method__"),
                    "Inner's `i` registration must follow the nested ClassDef; got {:?}",
                    body[1]
                );
            }
            other => panic!("expected Stmt::ClassDef Outer, got {:?}", other),
        }
        // Whole module still validates (no duplicate names).
        assert!(semantic_ir::validate(&m).is_ok());
    }

    // -----------------------------------------------------------------
    // Phase 14c (FC) — inheritance `class Foo < Bar`.  The superclass
    // name lands in `Stmt::ClassDef.superclass` (semantic-ir 0.3.0);
    // a base class keeps `superclass: None`.
    // -----------------------------------------------------------------

    #[test]
    fn class_with_superclass_records_parent_name() {
        // `class Dog < Animal` → ClassDef.superclass == Some("Animal").
        let m = lower("class Dog < Animal\nend\n");
        let main = main_body(&m);
        match &main.stmts[0] {
            Stmt::ClassDef { name, superclass, body, .. } => {
                assert_eq!(name, "Dog");
                assert_eq!(superclass.as_deref(), Some("Animal"));
                assert!(body.is_empty());
            }
            other => panic!("expected Stmt::ClassDef, got {:?}", other),
        }
    }

    #[test]
    fn base_class_has_no_superclass() {
        // A class without `< Parent` keeps `superclass: None` — the
        // 14c grammar's superclass clause is optional.
        let m = lower("class Widget\nend\n");
        let main = main_body(&m);
        match &main.stmts[0] {
            Stmt::ClassDef { name, superclass, .. } => {
                assert_eq!(name, "Widget");
                assert_eq!(*superclass, None, "base class must have no superclass");
            }
            other => panic!("expected Stmt::ClassDef, got {:?}", other),
        }
    }

    #[test]
    fn subclass_with_body_records_superclass_and_hoists_methods() {
        // Inheritance composes with Phase 14b: the superclass is
        // captured AND the body's `def`s hoist while non-def statements
        // stay in the body.
        let m = lower("class Cat < Animal\n  LEGS = 4\n  def meow\n  end\nend\n");
        // O2: `meow` hoists under the class-qualified name `Cat__meow`.
        assert!(
            m.functions.iter().any(|f| f.name == "Cat__meow"),
            "expected hoisted `Cat__meow`; got {:?}",
            m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        let main = main_body(&m);
        match &main.stmts[0] {
            Stmt::ClassDef { name, superclass, body, .. } => {
                assert_eq!(name, "Cat");
                assert_eq!(superclass.as_deref(), Some("Animal"));
                assert_eq!(body.len(), 1, "only `LEGS = 4` stays in body; got {:?}", body);
                // Phase 15c: `LEGS = 4` is a constant assignment → Assign(Const).
                assert!(matches!(&body[0], Stmt::Assign { name, scope, .. } if name == "LEGS" && *scope == Scope::Const));
            }
            other => panic!("expected Stmt::ClassDef, got {:?}", other),
        }
    }

    #[test]
    fn subclass_passes_sir_validator() {
        // E2E: a subclass with a populated body lowers to a module the
        // validator accepts (the superclass name is advisory; it is
        // not resolved against a symbol table).
        let m = lower("class Cat < Animal\n  LEGS = 4\nend\n");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected subclass module: {:?}", result);
    }

    #[test]
    fn comparison_in_class_body_is_not_mistaken_for_superclass() {
        // A `<` *inside* a body statement (a comparison expression)
        // must not be lifted as the superclass: only the direct
        // `class NAME < NAME` separator counts.  `class Plain` here has
        // no superclass even though its body contains `a < b`.
        let m = lower("class Plain\n  a = 1\n  a < 2\nend\n");
        let main = main_body(&m);
        match &main.stmts[0] {
            Stmt::ClassDef { name, superclass, .. } => {
                assert_eq!(name, "Plain");
                assert_eq!(
                    *superclass, None,
                    "a comparison in the body must not become the superclass"
                );
            }
            other => panic!("expected Stmt::ClassDef, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------
    // Phase 14e (FC) — singleton class `class << receiver … end` →
    // `Stmt::SingletonClassDef`.  Body handling mirrors class/module:
    // method defs hoist to top-level Functions; non-def statements stay
    // in the body.  Triggers `Feature::Classes`.
    // -----------------------------------------------------------------

    #[test]
    fn singleton_class_of_self_lowers_to_singleton_class_def() {
        // `class << self; end` → SingletonClassDef { target: "self" }.
        let m = lower("class << self\nend\n");
        let main = main_body(&m);
        match &main.stmts[0] {
            Stmt::SingletonClassDef { target, body, .. } => {
                assert_eq!(target, "self");
                assert!(body.is_empty(), "empty singleton body");
            }
            other => panic!("expected Stmt::SingletonClassDef, got {:?}", other),
        }
    }

    #[test]
    fn singleton_class_requests_classes_feature() {
        // A singleton class is a class-opening construct → it requests
        // `Feature::Classes` (not a separate feature).
        let m = lower("class << self\nend\n");
        assert!(
            m.manifest.contains(semantic_ir::Feature::Classes),
            "manifest should contain Feature::Classes; got {:?}",
            m.manifest
        );
    }

    #[test]
    fn singleton_class_hoists_methods_and_keeps_statements() {
        // `class << self; X = 1; def foo; end; end` — `foo` hoists to a
        // top-level Function; `X = 1` stays in the singleton body.
        let m = lower("class << self\n  X = 1\n  def foo\n  end\nend\n");
        assert!(
            m.functions.iter().any(|f| f.name == "foo"),
            "expected hoisted `foo`; got {:?}",
            m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        let main = main_body(&m);
        match &main.stmts[0] {
            Stmt::SingletonClassDef { target, body, .. } => {
                assert_eq!(target, "self");
                assert_eq!(body.len(), 1, "only `X = 1` stays in body; got {:?}", body);
                // Phase 15c: `X = 1` is a constant assignment → Assign(Const).
                assert!(matches!(&body[0], Stmt::Assign { name, scope, .. } if name == "X" && *scope == Scope::Const));
            }
            other => panic!("expected Stmt::SingletonClassDef, got {:?}", other),
        }
    }

    #[test]
    fn singleton_class_passes_sir_validator() {
        // E2E: lower → validate for a singleton class with a body.
        let m = lower("class << self\n  X = 1\n  def foo\n  end\nend\n");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected singleton-class module: {:?}", result);
    }

    #[test]
    fn ordinary_class_still_lowers_to_class_def_not_singleton() {
        // Regression guard: the singleton dispatch must not hijack the
        // ordinary `class Foo` form (no `singleton_receiver` child).
        let m = lower("class Foo\nend\n");
        let main = main_body(&m);
        assert!(
            matches!(&main.stmts[0], Stmt::ClassDef { .. }),
            "ordinary class must lower to ClassDef, got {:?}",
            main.stmts[0]
        );
    }

    // -----------------------------------------------------------------
    // Phase 15a (FC) — instance variables `@x`.  Reads lower to
    // `VarRef { scope: Instance }`, assignments to
    // `Assign { scope: Instance }` (no `let` declaration needed).
    // Triggers `Feature::InstanceVars`.
    // -----------------------------------------------------------------

    #[test]
    fn instance_var_read_lowers_to_instance_scope() {
        // A bare `@x` read with no prior assignment lowers to
        // `VarRef { scope: Instance }` — and crucially does NOT error
        // as an undefined local (reading an unset ivar is nil in Ruby).
        let m = lower("@x\n");
        match &main_body(&m).value {
            Expr::VarRef { name, scope, .. } => {
                assert_eq!(name, "@x");
                assert_eq!(*scope, Scope::Instance);
            }
            other => panic!("expected VarRef instance, got {:?}", other),
        }
        assert!(
            m.manifest.contains(semantic_ir::Feature::InstanceVars),
            "manifest should declare InstanceVars; got {:?}",
            m.manifest
        );
    }

    #[test]
    fn instance_var_assignment_lowers_to_instance_assign() {
        // `@count = 0` → `Assign { scope: Instance }` (not a
        // `LetBinding`), since an instance var is never a local.
        let m = lower("@count = 0\n");
        match &main_body(&m).stmts[0] {
            Stmt::Assign { name, scope, .. } => {
                assert_eq!(name, "@count");
                assert_eq!(*scope, Scope::Instance);
            }
            other => panic!("expected Assign instance, got {:?}", other),
        }
    }

    #[test]
    fn instance_var_read_without_assignment_passes_validator() {
        // E2E: reading an unset `@x` (here as an assignment RHS, with no
        // prior `@x = …`) must lower to a module the validator accepts —
        // the key win over the pre-15a local-modelling, which rejected
        // `@x` as an unknown local.
        let m = lower("y = @x\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected unset-ivar read: {:?}",
            result
        );
    }

    #[test]
    fn instance_var_in_method_roundtrips_through_validator() {
        // `def inc; @count = @count + 1; end` — assignment + read of the
        // same ivar inside a method body; validates end-to-end.
        let m = lower("def inc\n  @count = @count + 1\nend\n");
        assert!(
            m.functions.iter().any(|f| f.name == "inc"),
            "expected hoisted `inc`; got {:?}",
            m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        assert!(semantic_ir::validate(&m).is_ok(), "validator rejected ivar method: {:?}", semantic_ir::validate(&m));
    }

    #[test]
    fn class_var_double_at_is_not_instance_scope() {
        // `@@x` (double `@`) is a *class* variable — Phase 15b lowers it
        // to `Assign { scope: ClassVar }` (not Instance, not a local),
        // requesting `ClassVars` (and never `InstanceVars`).
        let m = lower("@@total = 0\n");
        match &main_body(&m).stmts[0] {
            Stmt::Assign { name, scope, .. } => {
                assert_eq!(name, "@@total");
                assert_eq!(*scope, Scope::ClassVar);
            }
            other => panic!("expected `@@total` Assign(ClassVar), got {:?}", other),
        }
        assert!(
            m.manifest.contains(semantic_ir::Feature::ClassVars),
            "`@@x` should request ClassVars; got {:?}",
            m.manifest
        );
        assert!(
            !m.manifest.contains(semantic_ir::Feature::InstanceVars),
            "`@@x` must not request InstanceVars (it is a class var)"
        );
    }

    #[test]
    fn class_var_read_lowers_to_classvar_scope() {
        // A bare `@@x` read lowers to `VarRef { scope: ClassVar }` and
        // requests `Feature::ClassVars` (no declaration needed).
        let m = lower("@@x\n");
        match &main_body(&m).value {
            Expr::VarRef { name, scope, .. } => {
                assert_eq!(name, "@@x");
                assert_eq!(*scope, Scope::ClassVar);
            }
            other => panic!("expected VarRef classvar, got {:?}", other),
        }
        assert!(
            m.manifest.contains(semantic_ir::Feature::ClassVars),
            "manifest should declare ClassVars; got {:?}",
            m.manifest
        );
    }

    #[test]
    fn class_var_read_without_assignment_passes_validator() {
        // E2E: reading an unset `@@x` (as an assignment RHS) validates —
        // class vars need no prior declaration.
        let m = lower("y = @@x\n");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected unset-cvar read: {:?}", result);
    }

    #[test]
    fn class_var_in_method_roundtrips_through_validator() {
        // `def bump; @@count = @@count + 1; end` — class-var assign +
        // read inside a method; validates end-to-end.
        let m = lower("def bump\n  @@count = @@count + 1\nend\n");
        assert!(
            m.functions.iter().any(|f| f.name == "bump"),
            "expected hoisted `bump`; got {:?}",
            m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        assert!(semantic_ir::validate(&m).is_ok(), "validator rejected cvar method: {:?}", semantic_ir::validate(&m));
    }

    #[test]
    fn instance_and_class_vars_are_distinct_scopes() {
        // `@x` → Instance, `@@x` → ClassVar — the single/double `@`
        // distinction routes to different scopes (and both features).
        let m = lower("@x = 1\n@@x = 2\n");
        let stmts = &main_body(&m).stmts;
        match (&stmts[0], &stmts[1]) {
            (Stmt::Assign { scope: s0, .. }, Stmt::Assign { scope: s1, .. }) => {
                assert_eq!(*s0, Scope::Instance, "`@x` → Instance");
                assert_eq!(*s1, Scope::ClassVar, "`@@x` → ClassVar");
            }
            other => panic!("expected two Assigns, got {:?}", other),
        }
        assert!(m.manifest.contains(semantic_ir::Feature::InstanceVars));
        assert!(m.manifest.contains(semantic_ir::Feature::ClassVars));
    }

    // -----------------------------------------------------------------
    // Phase 15c (FC) — constants `FOO` / `MyClass` → `Scope::Const`.
    // -----------------------------------------------------------------

    #[test]
    fn const_read_lowers_to_const_scope() {
        // A bare uppercase-initial name read lowers to
        // `VarRef { scope: Const }` and requests `Feature::Constants`
        // (no declaration needed).
        let m = lower("MAX\n");
        match &main_body(&m).value {
            Expr::VarRef { name, scope, .. } => {
                assert_eq!(name, "MAX");
                assert_eq!(*scope, Scope::Const);
            }
            other => panic!("expected VarRef const, got {:?}", other),
        }
        assert!(
            m.manifest.contains(semantic_ir::Feature::Constants),
            "manifest should declare Constants; got {:?}",
            m.manifest
        );
    }

    #[test]
    fn const_assignment_lowers_to_const_assign_not_letbinding() {
        // `MAX = 10` lowers to `Stmt::Assign { scope: Const }` (never a
        // `LetBinding`); the constant is not registered as a local.
        let m = lower("MAX = 10\n");
        match &main_body(&m).stmts[0] {
            Stmt::Assign { name, scope, .. } => {
                assert_eq!(name, "MAX");
                assert_eq!(*scope, Scope::Const);
            }
            other => panic!("expected Assign(MAX, Const), got {:?}", other),
        }
        assert!(m.manifest.contains(semantic_ir::Feature::Constants));
    }

    #[test]
    fn const_read_without_assignment_passes_validator() {
        // E2E: reading a constant with no prior assignment (as an
        // assignment RHS) validates — a constant needs no `let`.
        let m = lower("y = MAX\n");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected unset-const read: {:?}", result);
    }

    #[test]
    fn lowercase_name_stays_local_not_const() {
        // Regression: a lowercase-initial name is an ordinary local,
        // NOT a constant — it must keep `Scope::Local` and must not
        // request `Constants`.
        let m = lower("foo = 1\nfoo\n");
        match &main_body(&m).value {
            Expr::VarRef { name, scope, .. } => {
                assert_eq!(name, "foo");
                assert_eq!(*scope, Scope::Local, "lowercase name stays Local");
            }
            other => panic!("expected VarRef(foo, Local), got {:?}", other),
        }
        assert!(
            !m.manifest.contains(semantic_ir::Feature::Constants),
            "a lowercase local must not request Constants"
        );
    }

    // -----------------------------------------------------------------
    // Phase 15d (FC) — scoped lookup `Foo::Bar` → qualified `Scope::Const`.
    // -----------------------------------------------------------------

    #[test]
    fn scope_resolution_lowers_to_qualified_const() {
        // `Foo::Bar` folds into a single `VarRef { scope: Const }` whose
        // name is the qualified path `"Foo::Bar"`, and requests
        // `Feature::Constants`.
        let m = lower("Foo::Bar\n");
        match &main_body(&m).value {
            Expr::VarRef { name, scope, .. } => {
                assert_eq!(name, "Foo::Bar", "qualified constant path preserved");
                assert_eq!(*scope, Scope::Const);
            }
            other => panic!("expected VarRef(Foo::Bar, Const), got {:?}", other),
        }
        assert!(
            m.manifest.contains(semantic_ir::Feature::Constants),
            "manifest should declare Constants; got {:?}",
            m.manifest
        );
    }

    #[test]
    fn scope_resolution_chain_lowers_to_full_path() {
        // `A::B::C` collapses to `VarRef { Const, "A::B::C" }`.
        let m = lower("A::B::C\n");
        match &main_body(&m).value {
            Expr::VarRef { name, scope, .. } => {
                assert_eq!(name, "A::B::C");
                assert_eq!(*scope, Scope::Const);
            }
            other => panic!("expected VarRef(A::B::C, Const), got {:?}", other),
        }
    }

    #[test]
    fn scope_resolution_passes_sir_validator() {
        // E2E: a scoped constant lookup (as an assignment RHS) validates —
        // a constant needs no prior `let`.
        let m = lower("y = Foo::Bar\n");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected scoped const: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 14d (FC) — `module M … end` → `Stmt::ModuleDef`.  Mirrors
    // ClassDef (minus inheritance): method defs hoist to top-level
    // Functions; non-def statements stay in the module body.
    // -----------------------------------------------------------------

    #[test]
    fn empty_module_lowers_to_module_def_stmt() {
        // Phase 14d: `module M; end` lowers to a first-class
        // `Stmt::ModuleDef { name: "M", body: vec![], .. }`, replacing
        // the pre-14d NilLit no-op contract.
        let m = lower("module M\nend\n");
        let main = main_body(&m);
        assert_eq!(main.stmts.len(), 1);
        match &main.stmts[0] {
            Stmt::ModuleDef { name, body, .. } => {
                assert_eq!(name, "M");
                assert!(body.is_empty(), "empty module → empty body");
            }
            other => panic!("expected Stmt::ModuleDef, got {:?}", other),
        }
    }

    #[test]
    fn empty_module_requests_modules_feature() {
        // Emitting a `Stmt::ModuleDef` triggers the `Feature::Modules`
        // manifest entry (distinct from `Classes`).
        let m = lower("module M\nend\n");
        assert!(
            m.manifest.contains(semantic_ir::Feature::Modules),
            "manifest should contain Feature::Modules; got {:?}",
            m.manifest
        );
        assert!(
            !m.manifest.contains(semantic_ir::Feature::Classes),
            "a module-only program must not request Feature::Classes"
        );
    }

    #[test]
    fn empty_module_passes_sir_validator() {
        // E2E: lower → validate for a module-only program.
        let m = lower("module M\nend\n");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected module-only program: {:?}", result);
    }

    #[test]
    fn module_with_def_hoists_def_to_top_level() {
        // MX1 (mixins) — method defs inside a module still hoist to top-level
        // Functions, but now under a MODULE-QUALIFIED name (`M__helper`, not the
        // bare `helper`), exactly like class methods.  This is what lets a later
        // `include M` reference `M`'s methods without top-level name collisions.
        // The ModuleDef body itself stays empty (defs hoist out); the
        // registration follows the ModuleDef (asserted separately below).
        let m = lower("module M\n  def helper\n  end\nend\n");
        assert!(
            m.functions.iter().any(|f| f.name == "M__helper"),
            "expected module-qualified `M__helper`; got {:?}",
            m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        let main = main_body(&m);
        match &main.stmts[0] {
            Stmt::ModuleDef { name, body, .. } => {
                assert_eq!(name, "M");
                assert!(body.is_empty(), "method-only module body stays empty (defs hoist)");
            }
            other => panic!("expected Stmt::ModuleDef, got {:?}", other),
        }
    }

    #[test]
    fn module_body_preserves_executable_statement() {
        // A non-def statement in a module body is preserved in
        // ModuleDef.body (Phase 14d), mirroring ClassDef Phase 14b.
        let m = lower("module Config\n  VERSION = 3\nend\n");
        let main = main_body(&m);
        match &main.stmts[0] {
            Stmt::ModuleDef { name, body, .. } => {
                assert_eq!(name, "Config");
                assert_eq!(body.len(), 1, "VERSION = 3 preserved; got {:?}", body);
                // Phase 15c: `VERSION = 3` is a constant assignment → Assign(Const).
                assert!(matches!(&body[0], Stmt::Assign { name, scope, .. } if name == "VERSION" && *scope == Scope::Const));
            }
            other => panic!("expected Stmt::ModuleDef, got {:?}", other),
        }
    }

    #[test]
    fn class_module_lowering_passes_sir_validator() {
        let m = lower(concat!(
            "class Foo\n",
            "  def bar\n",
            "  end\n",
            "end\n",
            "module M\n",
            "  def helper\n",
            "  end\n",
            "end\n",
        ));
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected our output: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------
    // MX1 (mixins) — module methods register keyed by the module name
    // (`__def_method__("M", …)`), and `include`/`extend` in a class or
    // module body lower to `__include__`/`__extend__("Owner", "Module")`.
    // NO core-IR change: everything rides the existing `BuiltinCall` node,
    // the same slots classes use.  Execution is proven later (MX2–MX6);
    // these tests assert the lowered IR SHAPE only.
    // -----------------------------------------------------------------

    #[test]
    fn module_method_registers_def_method_keyed_by_module() {
        // `module M; def greet; "hi"; end; end` must emit
        // `__def_method__("M", "greet", MakeClosure(M__greet))` right after the
        // `ModuleDef` — the SAME builtin classes use, keyed by the module name.
        let m = lower("module M\n  def greet\n    \"hi\"\n  end\nend\n");
        let args = find_builtin_stmt(&m, "__def_method__");
        assert_eq!(args.len(), 3, "__def_method__ takes (class, method, closure)");
        assert!(
            matches!(&args[0], Expr::StrLit { value, .. } if value == "M"),
            "first arg is the module name \"M\"; got {:?}",
            args[0]
        );
        assert!(
            matches!(&args[1], Expr::StrLit { value, .. } if value == "greet"),
            "second arg is the bare method name \"greet\"; got {:?}",
            args[1]
        );
        assert!(
            matches!(&args[2], Expr::MakeClosure { fn_name, .. } if fn_name == "M__greet"),
            "third arg closes over the module-qualified hoisted fn `M__greet`; got {:?}",
            args[2]
        );
        // The registration is keyed on the bare name but the hoisted fn is
        // module-qualified, so it never collides with a same-named top-level def.
        assert!(m.functions.iter().any(|f| f.name == "M__greet"));
    }

    #[test]
    fn class_include_lowers_to_include_builtin() {
        // `class C; include M; end` → `__include__("C", "M")` — keyed by the
        // enclosing class (the include target) and the included module name.
        let m = lower("class C\n  include M\nend\n");
        let args = find_builtin_stmt(&m, "__include__");
        assert_eq!(args.len(), 2, "__include__ takes (owner, module)");
        assert!(
            matches!(&args[0], Expr::StrLit { value, .. } if value == "C"),
            "owner is the enclosing class \"C\"; got {:?}",
            args[0]
        );
        assert!(
            matches!(&args[1], Expr::StrLit { value, .. } if value == "M"),
            "module is the included constant \"M\"; got {:?}",
            args[1]
        );
    }

    #[test]
    fn class_extend_lowers_to_extend_builtin() {
        // `class C; extend M; end` → `__extend__("C", "M")`.
        let m = lower("class C\n  extend M\nend\n");
        let args = find_builtin_stmt(&m, "__extend__");
        assert_eq!(args.len(), 2, "__extend__ takes (owner, module)");
        assert!(matches!(&args[0], Expr::StrLit { value, .. } if value == "C"));
        assert!(matches!(&args[1], Expr::StrLit { value, .. } if value == "M"));
    }

    #[test]
    fn module_include_lowers_to_include_builtin_keyed_by_module() {
        // `include`/`extend` also work inside a MODULE body (module composition):
        // `module Outer; include Inner; end` → `__include__("Outer", "Inner")`.
        let m = lower("module Outer\n  include Inner\nend\n");
        let args = find_builtin_stmt(&m, "__include__");
        assert!(matches!(&args[0], Expr::StrLit { value, .. } if value == "Outer"));
        assert!(matches!(&args[1], Expr::StrLit { value, .. } if value == "Inner"));
    }

    #[test]
    fn module_and_include_program_passes_sir_validator() {
        // E2E lower → validate: a module defining a method plus a class that
        // includes it is a well-formed SIR module (all builtins ride the
        // existing `BuiltinCall`, so the validator accepts them as it does the
        // OOP `__def_method__`/`__new__` family).
        let m = lower(concat!(
            "module Greet\n",
            "  def greet\n",
            "    \"hi\"\n",
            "  end\n",
            "end\n",
            "class C\n",
            "  include Greet\n",
            "end\n",
        ));
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected module+include program: {:?}",
            result
        );
        // Sanity: both directives are present in the lowered output.
        let _ = find_builtin_stmt(&m, "__def_method__");
        let _ = find_builtin_stmt(&m, "__include__");
    }

    #[test]
    fn non_mixin_call_in_class_body_stays_ordinary() {
        // A call that is NOT `include`/`extend`/`attr_*` in a class body must be
        // left untouched — no `__include__`/`__extend__` emitted for it.
        let m = lower("class C\n  puts \"hi\"\nend\n");
        let has_mixin = main_body(&m).stmts.iter().any(|s| matches!(
            s,
            Stmt::ExprStmt { expr: Expr::BuiltinCall { name, .. }, .. }
                if name == "__include__" || name == "__extend__"
        ));
        assert!(!has_mixin, "ordinary class-body call must not become a mixin directive");
    }

    // -----------------------------------------------------------------
    // Phase 6g — method-with-block lowering: `do … end` / `{ … }`.
    // -----------------------------------------------------------------

    #[test]
    fn brace_block_hoists_to_synthetic_function_and_make_closure() {
        let m = lower("each { puts(1) }");
        let block_fn = m
            .functions
            .iter()
            .find(|f| f.name == "__block_0")
            .expect("expected `__block_0` synthetic function");
        assert!(block_fn.params.is_empty());
        match &block_fn.body.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "puts");
                assert!(matches!(args[0], Expr::IntLit { value: 1, .. }));
            }
            other => panic!("expected BuiltinCall(puts, …) tail, got {:?}", other),
        }
        let main = main_body(&m);
        let call_args = main.stmts.iter().find_map(|s| match s {
            Stmt::ExprStmt { expr: Expr::BuiltinCall { args, .. }, .. }
            | Stmt::ExprStmt { expr: Expr::DirectCall { args, .. }, .. } => Some(args),
            _ => None,
        }).expect("expected ExprStmt(call)");
        let last_arg = call_args.last().expect("call must have ≥1 arg (the closure)");
        assert!(matches!(last_arg, Expr::MakeClosure { fn_name, .. } if fn_name == "__block_0"));
    }

    #[test]
    fn do_block_with_pipe_params_lowers_to_function_with_params() {
        let m = lower("each do |x|\n  puts(x)\nend\n");
        let block_fn = m
            .functions
            .iter()
            .find(|f| f.name == "__block_0")
            .expect("expected __block_0");
        assert_eq!(block_fn.params.len(), 1);
        assert_eq!(block_fn.params[0].name, "x");
        if let Expr::BuiltinCall { args, .. } = &block_fn.body.value {
            assert!(matches!(&args[0], Expr::VarRef { name, scope, .. } if name == "x" && *scope == Scope::Param));
        } else {
            panic!("expected BuiltinCall body");
        }
    }

    #[test]
    fn multiple_blocks_get_distinct_synthetic_names() {
        let m = lower("each { puts(1) }\nmap { puts(2) }\n");
        assert!(m.functions.iter().any(|f| f.name == "__block_0"));
        assert!(m.functions.iter().any(|f| f.name == "__block_1"));
    }

    #[test]
    fn block_module_declares_closures_feature() {
        let m = lower("each { puts(1) }\n");
        let has_closures = format!("{:?}", m.manifest).contains("Closures");
        assert!(has_closures);
    }

    #[test]
    fn block_lowering_passes_sir_validator() {
        let m = lower("each do |x|\n  puts(x)\nend\n");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected our output: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 21a (FC) — block-local variables `{ |x; y| … }`.
    //
    // Names after the `;` in the pipe header are *block-local*: declared
    // in the block body's local scope (so VarRefs resolve as
    // `Scope::Local`) but NOT added to the synthetic function's
    // parameter list.  Params before the `;` stay `Scope::Param`.
    // -----------------------------------------------------------------

    #[test]
    fn block_local_is_not_a_param() {
        let m = lower("each do |x; y|\n  puts(x)\nend\n");
        let block_fn = m
            .functions
            .iter()
            .find(|f| f.name == "__block_0")
            .expect("expected __block_0");
        // Only `x` is a parameter; `y` (block-local) is excluded.
        assert_eq!(block_fn.params.len(), 1);
        assert_eq!(block_fn.params[0].name, "x");
        assert!(block_fn.params.iter().all(|p| p.name != "y"));
    }

    #[test]
    fn block_local_varref_resolves_as_local_not_param() {
        // Assign the block-local inside the body, then reference it: the
        // VarRef must resolve to Scope::Local (it is declared, not a param).
        let m = lower("each do |x; y|\n  y = x\n  puts(y)\nend\n");
        let block_fn = m
            .functions
            .iter()
            .find(|f| f.name == "__block_0")
            .expect("expected __block_0");
        // `y` is declared as a local (via the `;` clause) and is NOT a param.
        assert!(block_fn.params.iter().all(|p| p.name != "y"));
        // The `puts(y)` tail call references `y` as Scope::Local.
        if let Expr::BuiltinCall { args, .. } = &block_fn.body.value {
            assert!(matches!(
                &args[0],
                Expr::VarRef { name, scope, .. } if name == "y" && *scope == Scope::Local
            ), "expected y as Scope::Local, got {:?}", args.first());
        } else {
            panic!("expected BuiltinCall body, got {:?}", block_fn.body.value);
        }
    }

    #[test]
    fn block_with_block_local_passes_sir_validator() {
        let m = lower("each do |x; y|\n  y = x\n  puts(y)\nend\n");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected our output: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 21b (FC) — implicit numbered block parameters `_1`..`_9`.
    //
    // A block with NO explicit `|...|` header may reference `_1`..`_9`
    // as positional parameters; arity = the highest index used.  The
    // lowerer scans the body and synthesizes params `_1`..`_<max>`
    // (Scope::Param), without descending into nested blocks.
    // -----------------------------------------------------------------

    #[test]
    fn numbered_block_param_synthesizes_single_param() {
        let m = lower("each { puts(_1) }");
        let block_fn = m
            .functions
            .iter()
            .find(|f| f.name == "__block_0")
            .expect("expected __block_0");
        assert_eq!(block_fn.params.len(), 1);
        assert_eq!(block_fn.params[0].name, "_1");
    }

    #[test]
    fn numbered_block_param_arity_is_highest_index() {
        // Using `_2` implies arity 2 → params `_1, _2`, even though `_1`
        // is not referenced.
        let m = lower("each { puts(_2) }");
        let block_fn = m
            .functions
            .iter()
            .find(|f| f.name == "__block_0")
            .expect("expected __block_0");
        let names: Vec<&str> = block_fn.params.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["_1", "_2"]);
    }

    #[test]
    fn numbered_block_param_passes_sir_validator() {
        let m = lower("each { puts(_1) }");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected our output: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 21c (FC) — implicit `it` block parameter (Ruby 3.4).
    //
    // A header-less block referencing a bare `it` gets a single
    // synthesized parameter named `it`.  Guard: `it` adjacent to a
    // preceding `.` (method name) or a following `(` (call) does NOT
    // trigger the implicit param.  Numbered params take precedence.
    // -----------------------------------------------------------------

    #[test]
    fn implicit_it_synthesizes_single_param() {
        let m = lower("each { puts(it) }");
        let block_fn = m
            .functions
            .iter()
            .find(|f| f.name == "__block_0")
            .expect("expected __block_0");
        assert_eq!(block_fn.params.len(), 1);
        assert_eq!(block_fn.params[0].name, "it");
    }

    #[test]
    fn implicit_it_method_call_does_not_synthesize_param() {
        // `it(1)` is a real method call, NOT the implicit param → the
        // block should have zero synthesized params.
        let m = lower("each { it(1) }");
        let block_fn = m
            .functions
            .iter()
            .find(|f| f.name == "__block_0")
            .expect("expected __block_0");
        assert!(block_fn.params.is_empty(), "got {:?}", block_fn.params);
    }

    #[test]
    fn implicit_it_passes_sir_validator() {
        let m = lower("each { puts(it) }");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected our output: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 6h — paren-less method calls (`puts 1`, `puts 1, 2`).
    // -----------------------------------------------------------------

    #[test]
    fn no_paren_call_with_single_arg_lowers_to_builtin_call() {
        let m = lower("puts 1");
        let b = main_body(&m);
        match &b.value {
            Expr::BuiltinCall { name, args, effects, .. } => {
                assert_eq!(name, "puts");
                assert!(matches!(args[0], Expr::IntLit { value: 1, .. }));
                assert!(effects.contains(Effect::MayPrint));
            }
            other => panic!("expected BuiltinCall(puts, …), got {:?}", other),
        }
    }

    #[test]
    fn no_paren_call_with_multiple_args() {
        let m = lower("puts 1, 2, 3");
        let b = main_body(&m);
        match &b.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "puts");
                assert_eq!(args.len(), 3);
            }
            other => panic!("expected BuiltinCall(puts, …), got {:?}", other),
        }
    }

    #[test]
    fn no_paren_call_with_binary_expr_arg_groups_correctly() {
        let m = lower("puts 1 + 2");
        let b = main_body(&m);
        match &b.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "puts");
                assert_eq!(args.len(), 1);
                match &args[0] {
                    Expr::BuiltinCall { name, args: inner, .. } => {
                        assert_eq!(name, "+");
                        assert_eq!(inner.len(), 2);
                    }
                    other => panic!("expected nested BuiltinCall(+, …), got {:?}", other),
                }
            }
            other => panic!("expected BuiltinCall(puts, …), got {:?}", other),
        }
    }

    #[test]
    fn no_paren_call_module_passes_sir_validator() {
        let m = lower("x = 1\nputs x, x + 1\n");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected our output: {:?}", result);
    }

    #[test]
    fn paren_form_still_lowers_unchanged() {
        let m = lower("puts(42)");
        let b = main_body(&m);
        assert!(matches!(
            &b.value,
            Expr::BuiltinCall { name, args, .. } if name == "puts" && args.len() == 1
        ));
    }

    // -----------------------------------------------------------------
    // Phase 6i — comparison operators (`==`, `!=`, `<`, `>`, `<=`, `>=`).
    // -----------------------------------------------------------------

    #[test]
    fn equality_op_lowers_to_builtin_call() {
        let m = lower("x = 1 == 2");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::LetBinding { value, .. } => match value {
                Expr::BuiltinCall { name, args, .. } => {
                    assert_eq!(name, "==");
                    assert!(matches!(args[0], Expr::IntLit { value: 1, .. }));
                    assert!(matches!(args[1], Expr::IntLit { value: 2, .. }));
                }
                other => panic!("expected BuiltinCall(==), got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    }

    #[test]
    fn less_than_op_lowers_to_builtin_call() {
        let m = lower("5 < 10");
        let b = main_body(&m);
        match &b.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "<");
                assert!(matches!(args[0], Expr::IntLit { value: 5, .. }));
                assert!(matches!(args[1], Expr::IntLit { value: 10, .. }));
            }
            other => panic!("expected BuiltinCall(<), got {:?}", other),
        }
    }

    #[test]
    fn all_six_comparison_operators_lower_with_correct_names() {
        for op in &["==", "!=", "<", ">", "<=", ">="] {
            let src = format!("x = 1 {op} 2");
            let m = lower(&src);
            let b = main_body(&m);
            if let Stmt::LetBinding { value: Expr::BuiltinCall { name, .. }, .. } =
                &b.stmts[0]
            {
                assert_eq!(name, op);
            } else {
                panic!("expected LetBinding(BuiltinCall(`{op}`))");
            }
        }
    }

    #[test]
    fn comparison_has_lower_precedence_than_arithmetic() {
        let m = lower("1 + 2 < 5");
        let b = main_body(&m);
        match &b.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "<");
                match &args[0] {
                    Expr::BuiltinCall { name: inner, .. } => assert_eq!(inner, "+"),
                    other => panic!("expected `+` LHS, got {:?}", other),
                }
                assert!(matches!(args[1], Expr::IntLit { value: 5, .. }));
            }
            other => panic!("expected top-level `<`, got {:?}", other),
        }
    }

    #[test]
    fn comparison_used_in_if_condition_passes_validator() {
        let m = lower("x = 5\nif x < 10\n  y = 1\nend\n");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected our output: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 6j — control-flow keywords: `return`, `break`, `next`.
    // -----------------------------------------------------------------

    #[test]
    fn return_with_value_lowers_to_divergent_builtin_call() {
        let m = lower("return 42");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::ExprStmt { expr: Expr::BuiltinCall { name, args, effects, .. }, .. } => {
                assert_eq!(name, "return");
                assert!(matches!(args[0], Expr::IntLit { value: 42, .. }));
                assert!(effects.contains(Effect::Divergent));
            }
            other => panic!("expected ExprStmt(BuiltinCall(return)), got {:?}", other),
        }
    }

    #[test]
    fn bare_return_lowers_with_nil_arg() {
        let m = lower("return");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::ExprStmt { expr: Expr::BuiltinCall { name, args, .. }, .. } => {
                assert_eq!(name, "return");
                assert!(matches!(args[0], Expr::NilLit { .. }));
            }
            other => panic!("expected ExprStmt(BuiltinCall(return, [nil])), got {:?}", other),
        }
    }

    #[test]
    fn break_and_next_lower_to_their_respective_builtins() {
        for kw in &["break", "next"] {
            let src = format!("{kw} 1");
            let m = lower(&src);
            let b = main_body(&m);
            match &b.stmts[0] {
                Stmt::ExprStmt { expr: Expr::BuiltinCall { name, args, effects, .. }, .. } => {
                    assert_eq!(name, kw);
                    assert!(matches!(args[0], Expr::IntLit { value: 1, .. }));
                    assert!(effects.contains(Effect::Divergent));
                }
                other => panic!("expected BuiltinCall({kw}), got {:?}", other),
            }
        }
    }

    #[test]
    fn return_inside_def_body() {
        let m = lower("def f(x)\n  return x + 1\nend\n");
        let f = m.functions.iter().find(|f| f.name == "f").expect("expected fn f");
        let has_return = f.body.stmts.iter().any(|s| matches!(
            s,
            Stmt::ExprStmt { expr: Expr::BuiltinCall { name, .. }, .. } if name == "return"
        ));
        assert!(has_return);
    }

    #[test]
    fn return_module_passes_sir_validator() {
        let m = lower("def f(x)\n  return x\nend\n");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected our output: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 11a — `break`/`next` WITH VALUES (coverage-confirmation)
    //
    // The Phase 6j lowering arm already maps an optional trailing
    // expression after `break`/`next` into the single `BuiltinCall`
    // argument (bare → NilLit).  These pins lock that contract from
    // additional angles: a value-carrying `break`, a value-carrying
    // `next`, a bare `break` (NilLit arg), and a validator end-to-end
    // run where the payload is a resolved local variable.
    // -----------------------------------------------------------------

    #[test]
    fn break_with_value_lowers_to_int_arg() {
        let m = lower("break 5");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::ExprStmt { expr: Expr::BuiltinCall { name, args, effects, .. }, .. } => {
                assert_eq!(name, "break");
                assert!(matches!(args[0], Expr::IntLit { value: 5, .. }));
                assert!(effects.contains(Effect::Divergent));
            }
            other => panic!("expected ExprStmt(BuiltinCall(break, [5])), got {:?}", other),
        }
    }

    #[test]
    fn next_with_value_lowers_to_int_arg() {
        let m = lower("next 7");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::ExprStmt { expr: Expr::BuiltinCall { name, args, effects, .. }, .. } => {
                assert_eq!(name, "next");
                assert!(matches!(args[0], Expr::IntLit { value: 7, .. }));
                assert!(effects.contains(Effect::Divergent));
            }
            other => panic!("expected ExprStmt(BuiltinCall(next, [7])), got {:?}", other),
        }
    }

    #[test]
    fn bare_break_lowers_with_nil_arg() {
        let m = lower("break");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::ExprStmt { expr: Expr::BuiltinCall { name, args, .. }, .. } => {
                assert_eq!(name, "break");
                assert!(matches!(args[0], Expr::NilLit { .. }));
            }
            other => panic!("expected ExprStmt(BuiltinCall(break, [nil])), got {:?}", other),
        }
    }

    #[test]
    fn break_with_local_var_value_passes_sir_validator() {
        // Assign `x` first so the VarRef payload resolves, then break
        // with it; the lowered module must satisfy the SIR validator.
        let m = lower("x = 1\nbreak x\n");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected our output: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 11b — `redo` keyword (restart current loop iteration)
    //
    // `redo` lowers to a ZERO-argument Divergent BuiltinCall, distinct
    // from `break`/`next` (which always carry an operand, NilLit when
    // bare).  It restarts the current loop iteration without re-checking
    // the condition, so it diverges from straight-line control flow.
    // -----------------------------------------------------------------

    #[test]
    fn redo_lowers_to_zero_arg_divergent_builtin() {
        let m = lower("redo");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::ExprStmt { expr: Expr::BuiltinCall { name, args, effects, .. }, .. } => {
                assert_eq!(name, "redo");
                assert!(args.is_empty(), "redo carries no operand, got {:?}", args);
                assert!(effects.contains(Effect::Divergent));
            }
            other => panic!("expected ExprStmt(BuiltinCall(redo, [])), got {:?}", other),
        }
    }

    #[test]
    fn redo_inside_while_body_lowers() {
        // `redo` nested in a loop body still lowers to its builtin; find
        // the `Stmt::While` and confirm its body holds BuiltinCall(redo).
        let m = lower("x = 0\nwhile x\n  redo\nend\n");
        let b = main_body(&m);
        let while_body = b.stmts.iter().find_map(|s| match s {
            Stmt::While { body, .. } => Some(body),
            _ => None,
        }).expect("expected a Stmt::While in the module body");
        let has_redo = while_body.stmts.iter().any(|s| matches!(
            s,
            Stmt::ExprStmt { expr: Expr::BuiltinCall { name, .. }, .. } if name == "redo"
        ));
        assert!(has_redo, "expected BuiltinCall(redo) in the while body");
    }

    #[test]
    fn redo_module_passes_sir_validator() {
        let m = lower("x = 0\nwhile x\n  redo\nend\n");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected our output: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 11c — `retry` keyword (re-execute enclosing begin block)
    //
    // `retry` mirrors `redo`: a bare keyword lowering to a ZERO-argument
    // Divergent BuiltinCall.  It re-runs the enclosing `begin` block from
    // the top inside a `rescue` clause, so it diverges from straight-line
    // control flow.
    // -----------------------------------------------------------------

    #[test]
    fn retry_lowers_to_zero_arg_divergent_builtin() {
        let m = lower("retry");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::ExprStmt { expr: Expr::BuiltinCall { name, args, effects, .. }, .. } => {
                assert_eq!(name, "retry");
                assert!(args.is_empty(), "retry carries no operand, got {:?}", args);
                assert!(effects.contains(Effect::Divergent));
            }
            other => panic!("expected ExprStmt(BuiltinCall(retry, [])), got {:?}", other),
        }
    }

    #[test]
    fn retry_inside_begin_rescue_lowers() {
        // `retry` nested in a rescue clause lowers to its builtin; find
        // the `Stmt::TryCatch` and confirm its catch block holds
        // BuiltinCall(retry).
        let m = lower("begin\n  x = 1\nrescue\n  retry\nend\n");
        let b = main_body(&m);
        let rescues = b.stmts.iter().find_map(|s| match s {
            Stmt::TryCatch { rescues, .. } => Some(rescues),
            _ => None,
        }).expect("expected a Stmt::TryCatch in the module body");
        let has_retry = rescues.iter().any(|rc| rc.body.iter().any(|s| matches!(
            s,
            Stmt::ExprStmt { expr: Expr::BuiltinCall { name, .. }, .. } if name == "retry"
        )));
        assert!(has_retry, "expected BuiltinCall(retry) in a rescue clause body");
    }

    #[test]
    fn retry_module_passes_sir_validator() {
        let m = lower("begin\n  x = 1\nrescue\n  retry\nend\n");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected our output: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 11d — `return` WITH VALUE (coverage-confirmation)
    //
    // The Phase 6j arm already folds an optional trailing expression
    // after `return` into the single BuiltinCall argument (bare ->
    // NilLit).  Existing pins cover `return 42`, bare `return`, and
    // `return x + 1` inside a def, plus a def-body validator run.  These
    // pins lock the contract from new payload angles: an array literal,
    // a string literal, and a validator end-to-end run where the payload
    // is a top-level local (distinct from the existing def-body pin).
    // -----------------------------------------------------------------

    #[test]
    fn return_with_array_value_lowers_to_seqlit_arg() {
        let m = lower("return [1, 2]");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::ExprStmt { expr: Expr::BuiltinCall { name, args, effects, .. }, .. } => {
                assert_eq!(name, "return");
                assert!(matches!(args[0], Expr::SeqLit { .. }), "got {:?}", args[0]);
                assert!(effects.contains(Effect::Divergent));
            }
            other => panic!("expected ExprStmt(BuiltinCall(return, [array])), got {:?}", other),
        }
    }

    #[test]
    fn return_with_string_value_lowers_to_strlit_arg() {
        let m = lower("return \"ok\"");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::ExprStmt { expr: Expr::BuiltinCall { name, args, effects, .. }, .. } => {
                assert_eq!(name, "return");
                assert!(matches!(args[0], Expr::StrLit { .. }), "got {:?}", args[0]);
                assert!(effects.contains(Effect::Divergent));
            }
            other => panic!("expected ExprStmt(BuiltinCall(return, [string])), got {:?}", other),
        }
    }

    #[test]
    fn return_with_top_level_local_value_passes_sir_validator() {
        // Assign `x` first so the VarRef payload resolves, then return it
        // at top level (distinct from the existing def-body validator pin).
        let m = lower("x = 5\nreturn x\n");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected our output: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 6k — unary minus → BuiltinCall("neg", [x]).
    // -----------------------------------------------------------------

    #[test]
    fn unary_minus_on_number_lowers_to_neg_builtin() {
        let m = lower("x = -5");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::LetBinding { value, .. } => match value {
                Expr::BuiltinCall { name, args, .. } => {
                    assert_eq!(name, "neg");
                    assert!(matches!(args[0], Expr::IntLit { value: 5, .. }));
                }
                other => panic!("expected BuiltinCall(neg, [5]), got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    }

    #[test]
    fn unary_minus_on_name_carries_scope() {
        let m = lower("x = 1\ny = -x\n");
        let b = main_body(&m);
        match &b.stmts[1] {
            Stmt::LetBinding { value: Expr::BuiltinCall { name, args, .. }, .. }
            | Stmt::LetStarBinding { value: Expr::BuiltinCall { name, args, .. }, .. } => {
                assert_eq!(name, "neg");
                assert!(matches!(
                    &args[0],
                    Expr::VarRef { name, scope, .. } if name == "x" && *scope == Scope::Local
                ));
            }
            other => panic!("expected LetBinding(BuiltinCall(neg, [VarRef(x)])), got {:?}", other),
        }
    }

    #[test]
    fn double_unary_minus_nests_correctly() {
        let m = lower("x = --5");
        let b = main_body(&m);
        if let Stmt::LetBinding {
            value: Expr::BuiltinCall { name: outer, args: outer_args, .. },
            ..
        } = &b.stmts[0]
        {
            assert_eq!(outer, "neg");
            match &outer_args[0] {
                Expr::BuiltinCall { name: inner, args: inner_args, .. } => {
                    assert_eq!(inner, "neg");
                    assert!(matches!(inner_args[0], Expr::IntLit { value: 5, .. }));
                }
                other => panic!("expected inner neg, got {:?}", other),
            }
        } else {
            panic!("expected outer LetBinding(BuiltinCall(neg, …))");
        }
    }

    #[test]
    fn unary_minus_with_binary_plus_resolves_precedence_correctly() {
        let m = lower("x = -5 + 3");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::LetBinding { value: Expr::BuiltinCall { name: outer, args, .. }, .. } => {
                assert_eq!(outer, "+");
                match &args[0] {
                    Expr::BuiltinCall { name: inner, args: inner_args, .. } => {
                        assert_eq!(inner, "neg");
                        assert!(matches!(inner_args[0], Expr::IntLit { value: 5, .. }));
                    }
                    other => panic!("expected LHS = neg(5), got {:?}", other),
                }
                assert!(matches!(args[1], Expr::IntLit { value: 3, .. }));
            }
            other => panic!("expected LetBinding(BuiltinCall(+, …)), got {:?}", other),
        }
    }

    #[test]
    fn unary_minus_module_passes_sir_validator() {
        let m = lower("x = -5\ny = -(1 + 2)\n");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected our output: {:?}", result);
    }

    #[test]
    fn module_with_def_passes_sir_validator() {
        // v0 grammar can't yet parse nested method calls in args
        // (`puts(add(1, 2))` — `add(1, 2)` isn't a `factor`).  The
        // grammar extension lands in Phase 6+.  Until then, exercise
        // the `def` lowering with sibling statements that the
        // grammar already accepts.
        let m = lower(concat!(
            "def add(x, y)\n",
            "  x + y\n",
            "end\n",
            "z = 3 + 4\n",
            "puts(z)\n",
        ));
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected our output: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------
    // Phase 6l — method receiver chains lowering
    // -----------------------------------------------------------------
    //
    // SIR encoding: `foo.bar(args)` becomes
    //   BuiltinCall("__method__", [receiver, StrLit("bar"), ...args])
    // Receiver-first arg layout is the contract — backends can rely on it.
    // BuiltinCall (not DirectCall) is used because validator only
    // checks DirectCall.fn_name against the module's function table.

    #[test]
    fn dot_chain_lowers_to_method_builtincall() {
        let m = lower("foo = 1\nx = foo.bar\n");
        let b = main_body(&m);
        // Second stmt = `x = foo.bar`
        match &b.stmts[1] {
            Stmt::LetBinding { name, value, .. } | Stmt::LetStarBinding { name, value, .. } => {
                assert_eq!(name, "x");
                match value {
                    Expr::BuiltinCall { name, args, .. } => {
                        assert_eq!(name, "__method__");
                        assert_eq!(args.len(), 2, "receiver + method-name");
                        assert!(matches!(
                            &args[0],
                            Expr::VarRef { name, .. } if name == "foo"
                        ));
                        assert!(matches!(
                            &args[1],
                            Expr::StrLit { value, .. } if value == "bar"
                        ));
                    }
                    other => panic!("expected BuiltinCall(__method__, …), got {:?}", other),
                }
            }
            other => panic!("expected LetBinding, got {:?}", other),
        }
    }

    #[test]
    fn dot_chain_two_steps_nests_outer_recv() {
        // `foo.bar.baz` → outer is `.baz` with receiver = inner `.bar`.
        let m = lower("foo = 1\ny = foo.bar.baz\n");
        let b = main_body(&m);
        let value = match &b.stmts[1] {
            Stmt::LetBinding { value, .. } | Stmt::LetStarBinding { value, .. } => value,
            other => panic!("expected LetBinding, got {:?}", other),
        };
        match value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "__method__");
                // Outer = `.baz`; args[0] is the inner `foo.bar` BuiltinCall.
                assert!(matches!(
                    &args[1],
                    Expr::StrLit { value, .. } if value == "baz"
                ));
                match &args[0] {
                    Expr::BuiltinCall { name: inner_name, args: inner_args, .. } => {
                        assert_eq!(inner_name, "__method__");
                        assert!(matches!(
                            &inner_args[0],
                            Expr::VarRef { name, .. } if name == "foo"
                        ));
                        assert!(matches!(
                            &inner_args[1],
                            Expr::StrLit { value, .. } if value == "bar"
                        ));
                    }
                    other => panic!("expected inner BuiltinCall, got {:?}", other),
                }
            }
            other => panic!("expected BuiltinCall(__method__, …), got {:?}", other),
        }
    }

    #[test]
    fn dot_call_with_args_includes_them_after_method_name() {
        // `obj.add(1, 2)` → BuiltinCall("__method__", [obj, "add", 1, 2])
        let m = lower("obj = 1\nr = obj.add(1, 2)\n");
        let b = main_body(&m);
        let value = match &b.stmts[1] {
            Stmt::LetBinding { value, .. } | Stmt::LetStarBinding { value, .. } => value,
            other => panic!("expected LetBinding, got {:?}", other),
        };
        match value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "__method__");
                assert_eq!(args.len(), 4, "receiver + method + 2 args");
                assert!(matches!(
                    &args[0],
                    Expr::VarRef { name, .. } if name == "obj"
                ));
                assert!(matches!(
                    &args[1],
                    Expr::StrLit { value, .. } if value == "add"
                ));
                assert!(matches!(&args[2], Expr::IntLit { value: 1, .. }));
                assert!(matches!(&args[3], Expr::IntLit { value: 2, .. }));
            }
            other => panic!("expected BuiltinCall(__method__, …), got {:?}", other),
        }
    }

    #[test]
    fn dot_chain_on_method_call_head() {
        // `puts(1).then_something` — head is a method_call (puts is a
        // known builtin → BuiltinCall("puts")), tail is one dot_call.
        // We use puts so the head call lowers as a recognised builtin
        // and the validator doesn't trip on an undeclared function.
        let m = lower("z = puts(1).then_something\n");
        let b = main_body(&m);
        let value = match &b.stmts[0] {
            Stmt::LetBinding { value, .. } => value,
            other => panic!("expected LetBinding, got {:?}", other),
        };
        match value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "__method__");
                // Receiver is the inner `puts(1)` BuiltinCall.
                match &args[0] {
                    Expr::BuiltinCall { name: inner_name, args: inner_args, .. } => {
                        assert_eq!(inner_name, "puts");
                        assert_eq!(inner_args.len(), 1);
                        assert!(matches!(&inner_args[0], Expr::IntLit { value: 1, .. }));
                    }
                    other => panic!("expected inner BuiltinCall(puts, …), got {:?}", other),
                }
                assert!(matches!(
                    &args[1],
                    Expr::StrLit { value, .. } if value == "then_something"
                ));
            }
            other => panic!("expected outer BuiltinCall(__method__, …), got {:?}", other),
        }
    }

    #[test]
    fn dot_chain_module_passes_sir_validator() {
        // Receiver `a` lives as a function parameter so the validator's
        // var-ref resolution succeeds; the chain itself uses BuiltinCall
        // so the unknown-fn check doesn't apply.  Strings feature is
        // auto-added by the lowerer when any dot_call fires.
        //
        // Why a function param (not `a = 1\nx = a.b...`)?  The validator
        // groups consecutive LetBindings into one parallel-let block —
        // RHS expressions can't see names bound by sibling LetBindings.
        // Function params don't have that constraint.
        let m = lower("def chain(a)\n  a.b.c(42)\nend\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected dot-chain output: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------
    // Phase 6m — logical operators lowering
    // -----------------------------------------------------------------
    //
    // SIR encoding:
    //   `a || b`  → BuiltinCall("or",  [a, b])
    //   `a && b`  → BuiltinCall("and", [a, b])
    //   `!x` / `not x` → BuiltinCall("not", [x])
    // Both symbol (`||`/`&&`/`!`) and keyword (`or`/`and`/`not`)
    // forms collapse to the same builtin in v0.

    // NOTE on def-body logical operators: the v0 parser framework
    // mis-handles bare `a || b` (or `&&`, etc.) as the tail
    // expression of a def body — `method_call_no_paren("def", ...)`
    // shadows `def_statement` because the alternation doesn't
    // back-track cleanly when the first operand is a NAME.  Wrapping
    // the expression in parens forces the LPAREN-expression-RPAREN
    // path through `factor`, which the framework parses cleanly.
    // We use the parens-wrapped form throughout these tests.  See
    // lessons.md (Ruby parser alternation ambiguity).

    #[test]
    fn logical_or_symbol_lowers_to_or_builtin() {
        let m = lower("def myor(a, b)\n  (a || b)\nend\n");
        let names: Vec<&str> = m.functions.iter().map(|f| f.name.as_str()).collect();
        let f = m
            .functions
            .iter()
            .find(|f| f.name == "myor")
            .unwrap_or_else(|| panic!("expected myor function; got functions {:?}", names));
        match &f.body.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "or");
                assert_eq!(args.len(), 2);
                assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "a"));
                assert!(matches!(&args[1], Expr::VarRef { name, .. } if name == "b"));
            }
            other => panic!("expected BuiltinCall(or, …), got {:?}", other),
        }
    }

    #[test]
    fn logical_and_symbol_lowers_to_and_builtin() {
        let m = lower("def myand(a, b)\n  (a && b)\nend\n");
        let f = m.functions.iter().find(|f| f.name == "myand").unwrap();
        match &f.body.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "and");
                assert_eq!(args.len(), 2);
            }
            other => panic!("expected BuiltinCall(and, …), got {:?}", other),
        }
    }

    #[test]
    fn logical_keyword_form_lowers_same_as_symbol() {
        // `a or b` (keyword) should produce the same builtin as `a || b`.
        let m = lower("def myor2(a, b)\n  (a or b)\nend\n");
        let f = m.functions.iter().find(|f| f.name == "myor2").unwrap();
        assert!(
            matches!(&f.body.value, Expr::BuiltinCall { name, .. } if name == "or")
        );
    }

    #[test]
    fn logical_not_symbol_lowers_to_not_builtin() {
        let m = lower("def mynot(x)\n  (!x)\nend\n");
        let f = m.functions.iter().find(|f| f.name == "mynot").unwrap();
        match &f.body.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "not");
                assert_eq!(args.len(), 1);
                assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "x"));
            }
            other => panic!("expected BuiltinCall(not, …), got {:?}", other),
        }
    }

    #[test]
    fn logical_chain_and_then_or_nests_correctly() {
        // `a && b || c` parses as `(a && b) || c` — precedence test.
        // SIR: BuiltinCall("or", [BuiltinCall("and", [a, b]), c])
        let m = lower("def chain(a, b, c)\n  (a && b || c)\nend\n");
        let f = m.functions.iter().find(|f| f.name == "chain").unwrap();
        match &f.body.value {
            Expr::BuiltinCall { name, args, .. } if name == "or" => {
                assert_eq!(args.len(), 2);
                assert!(matches!(&args[1], Expr::VarRef { name, .. } if name == "c"));
                match &args[0] {
                    Expr::BuiltinCall { name: inner_name, args: inner_args, .. }
                        if inner_name == "and" =>
                    {
                        assert_eq!(inner_args.len(), 2);
                        assert!(matches!(&inner_args[0], Expr::VarRef { name, .. } if name == "a"));
                        assert!(matches!(&inner_args[1], Expr::VarRef { name, .. } if name == "b"));
                    }
                    other => panic!("expected inner BuiltinCall(and, …), got {:?}", other),
                }
            }
            other => panic!("expected outer BuiltinCall(or, …), got {:?}", other),
        }
    }

    #[test]
    fn logical_module_passes_sir_validator() {
        let m = lower(concat!(
            "def chain(a, b, c)\n",
            "  (a && b || !c)\n",
            "end\n",
        ));
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected logical-op output: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------
    // Phase 6n — range expressions lowering
    // -----------------------------------------------------------------
    //
    // SIR encoding:
    //   `a..b`  → BuiltinCall("range", [a, b, BoolLit(false)])  ; inclusive
    //   `a...b` → BuiltinCall("range", [a, b, BoolLit(true)])   ; exclusive
    //
    // The third arg is the exclusive-end flag — keeping the builtin's
    // name uniform across both forms means downstream emitters can
    // pattern-match on `range` once and read the flag.

    #[test]
    fn inclusive_range_lowers_to_range_builtin_with_false_flag() {
        // `def r() 1..5 end` — RHS of the def body is a range literal.
        let m = lower("def r\n  1..5\nend\n");
        let f = m.functions.iter().find(|f| f.name == "r").unwrap();
        match &f.body.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "range");
                assert_eq!(args.len(), 3, "expected [start, end, exclusive_flag]");
                assert!(matches!(&args[0], Expr::IntLit { value: 1, .. }));
                assert!(matches!(&args[1], Expr::IntLit { value: 5, .. }));
                assert!(
                    matches!(&args[2], Expr::BoolLit { value: false, .. }),
                    "inclusive `..` should set the exclusive flag to false; got {:?}",
                    args[2]
                );
            }
            other => panic!("expected BuiltinCall(range, …), got {:?}", other),
        }
    }

    #[test]
    fn exclusive_range_lowers_to_range_builtin_with_true_flag() {
        // `def r() 1...5 end` — exclusive form sets flag to true.
        let m = lower("def r\n  1...5\nend\n");
        let f = m.functions.iter().find(|f| f.name == "r").unwrap();
        match &f.body.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "range");
                assert_eq!(args.len(), 3);
                assert!(
                    matches!(&args[2], Expr::BoolLit { value: true, .. }),
                    "exclusive `...` should set the flag to true; got {:?}",
                    args[2]
                );
            }
            other => panic!("expected BuiltinCall(range, …), got {:?}", other),
        }
    }

    #[test]
    fn range_with_variable_operands_uses_var_refs() {
        // `def r(a, b) (a..b) end` — range over function params.  Params
        // are in scope so the VarRef survives validation.  Parens wrap
        // the range so the bare-NAME-led body doesn't trigger the
        // `method_call_no_paren` framework ambiguity (lessons.md).
        let m = lower("def r(a, b)\n  (a..b)\nend\n");
        let f = m.functions.iter().find(|f| f.name == "r").unwrap();
        match &f.body.value {
            Expr::BuiltinCall { name, args, .. } if name == "range" => {
                assert_eq!(args.len(), 3);
                assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "a"));
                assert!(matches!(&args[1], Expr::VarRef { name, .. } if name == "b"));
            }
            other => panic!("expected BuiltinCall(range, …), got {:?}", other),
        }
    }

    #[test]
    fn range_module_passes_sir_validator() {
        // End-to-end smoke check: a range expression survives validation.
        // Parens for the same reason as above.
        let m = lower(concat!(
            "def r(a, b)\n",
            "  (a..b)\n",
            "end\n",
        ));
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected range output: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------
    // Phase 10a (FC) — inclusive range `1..5` coverage confirmation.
    //
    // Inclusive ranges lower to `BuiltinCall("range", [a, b, false])`
    // since Phase 6n.  Phase 10a (a coverage-confirmation phase, cf.
    // 16b/16c) adds explicit lowering pins for inclusive ranges in
    // positions the 6n tests skipped: assignment-RHS at statement level,
    // string endpoints, and as an array-literal element.  Each asserts
    // the inclusive flag is `false` so an accidental flag flip is caught.
    // -----------------------------------------------------------------

    #[test]
    fn inclusive_range_in_assignment_rhs_lowers_with_false_flag() {
        // `x = 1..5` at statement level — the assigned value is the range.
        // A first-occurrence binding lowers to `LetBinding` (the frontend
        // reserves `Assign` for re-binding an already-declared name), so
        // accept either binding form and read its `value`.
        let m = lower("x = 1..5\n");
        let found = main_body(&m).stmts.iter().find_map(|s| match s {
            Stmt::LetBinding { value, .. } | Stmt::Assign { value, .. } => Some(value.clone()),
            _ => None,
        });
        let value = found.expect("expected a binding statement for `x`");
        match &value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "range");
                assert_eq!(args.len(), 3, "expected [start, end, exclusive_flag]");
                assert!(matches!(&args[0], Expr::IntLit { value: 1, .. }));
                assert!(matches!(&args[1], Expr::IntLit { value: 5, .. }));
                assert!(
                    matches!(&args[2], Expr::BoolLit { value: false, .. }),
                    "inclusive `..` should set the exclusive flag false; got {:?}",
                    args[2]
                );
            }
            other => panic!("expected BuiltinCall(range, …), got {:?}", other),
        }
    }

    #[test]
    fn inclusive_range_string_endpoints_lower_with_false_flag() {
        // `def r() ("a".."z") end` — string endpoints, inclusive.  Parens
        // avoid the bare-NAME method-call ambiguity (lessons.md).
        let m = lower("def r\n  (\"a\"..\"z\")\nend\n");
        let f = m.functions.iter().find(|f| f.name == "r").unwrap();
        match &f.body.value {
            Expr::BuiltinCall { name, args, .. } if name == "range" => {
                assert_eq!(args.len(), 3);
                assert!(
                    matches!(&args[0], Expr::StrLit { value, .. } if value == "a"),
                    "expected start StrLit \"a\"; got {:?}",
                    args[0]
                );
                assert!(
                    matches!(&args[1], Expr::StrLit { value, .. } if value == "z"),
                    "expected end StrLit \"z\"; got {:?}",
                    args[1]
                );
                assert!(matches!(&args[2], Expr::BoolLit { value: false, .. }));
            }
            other => panic!("expected BuiltinCall(range, …), got {:?}", other),
        }
    }

    #[test]
    fn inclusive_range_as_array_element_lowers_and_validates() {
        // `def r() [1..5] end` — a range as the sole element of an array
        // literal.  The array's element is itself the range builtin, and
        // the whole module survives validation (end-to-end pin).
        let m = lower("def r\n  [1..5]\nend\n");
        let f = m.functions.iter().find(|f| f.name == "r").unwrap();
        match &f.body.value {
            Expr::SeqLit { items, .. } => {
                assert_eq!(items.len(), 1, "expected a one-element array");
                match &items[0] {
                    Expr::BuiltinCall { name, args, .. } if name == "range" => {
                        assert_eq!(args.len(), 3);
                        assert!(matches!(&args[2], Expr::BoolLit { value: false, .. }));
                    }
                    other => panic!("expected range element, got {:?}", other),
                }
            }
            other => panic!("expected SeqLit, got {:?}", other),
        }
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected array-of-range: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 10c (FC) — endless range `1..` / `1...` lowering.
    //
    // An endless range carries a start but no end.  It lowers to the
    // same `range` builtin with the open upper bound encoded as
    // `NilLit`: `BuiltinCall("range", [start, NilLit, BoolLit(excl)])`.
    // The exclusive flag still distinguishes `..` (false) from `...`
    // (true), so downstream emitters need no new dispatch — a nil end
    // simply means "unbounded above".
    // -----------------------------------------------------------------

    #[test]
    fn endless_range_inclusive_lowers_with_nil_end() {
        // `def r() (1..) end` — endless inclusive.  Parens keep the
        // body off the bare-NAME method-call path (lessons.md).
        let m = lower("def r\n  (1..)\nend\n");
        let f = m.functions.iter().find(|f| f.name == "r").unwrap();
        match &f.body.value {
            Expr::BuiltinCall { name, args, .. } if name == "range" => {
                assert_eq!(args.len(), 3, "expected [start, nil_end, exclusive_flag]");
                assert!(matches!(&args[0], Expr::IntLit { value: 1, .. }));
                assert!(
                    matches!(&args[1], Expr::NilLit { .. }),
                    "endless range end should be NilLit; got {:?}",
                    args[1]
                );
                assert!(
                    matches!(&args[2], Expr::BoolLit { value: false, .. }),
                    "inclusive `..` should set the exclusive flag false; got {:?}",
                    args[2]
                );
            }
            other => panic!("expected BuiltinCall(range, …), got {:?}", other),
        }
    }

    #[test]
    fn endless_range_exclusive_lowers_with_nil_end() {
        // `def r() (1...) end` — endless exclusive sets the flag true.
        let m = lower("def r\n  (1...)\nend\n");
        let f = m.functions.iter().find(|f| f.name == "r").unwrap();
        match &f.body.value {
            Expr::BuiltinCall { name, args, .. } if name == "range" => {
                assert_eq!(args.len(), 3);
                assert!(matches!(&args[1], Expr::NilLit { .. }), "end should be nil");
                assert!(
                    matches!(&args[2], Expr::BoolLit { value: true, .. }),
                    "exclusive `...` should set the flag true; got {:?}",
                    args[2]
                );
            }
            other => panic!("expected BuiltinCall(range, …), got {:?}", other),
        }
    }

    #[test]
    fn endless_range_over_param_validates_e2e() {
        // `def r(a) (a..) end` — endless range whose start is a param
        // (in scope, so the VarRef survives validation).  End-to-end pin:
        // lower + validate succeeds, and the end is nil.
        let m = lower("def r(a)\n  (a..)\nend\n");
        let f = m.functions.iter().find(|f| f.name == "r").unwrap();
        match &f.body.value {
            Expr::BuiltinCall { name, args, .. } if name == "range" => {
                assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "a"));
                assert!(matches!(&args[1], Expr::NilLit { .. }));
            }
            other => panic!("expected BuiltinCall(range, …), got {:?}", other),
        }
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected endless range: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 10d (FC) — beginless range `..5` / `...5` lowering.
    //
    // A beginless range carries an end but no start.  It lowers to the
    // same `range` builtin with the open LOWER bound encoded as
    // `NilLit`: `BuiltinCall("range", [NilLit, end, BoolLit(excl)])`.
    // Endless (`1..`) and beginless (`..5`) have identical arity (one
    // operand + op); the lowerer disambiguates by the op token's
    // position relative to the operand.  Here we pin that a leading op
    // produces a NIL START (args[0]) with the operand as the end.
    // -----------------------------------------------------------------

    #[test]
    fn beginless_range_inclusive_lowers_with_nil_start() {
        // `def r() (..5) end` — beginless inclusive.  Parens keep the
        // body off the bare-NAME method-call path (lessons.md).
        let m = lower("def r\n  (..5)\nend\n");
        let f = m.functions.iter().find(|f| f.name == "r").unwrap();
        match &f.body.value {
            Expr::BuiltinCall { name, args, .. } if name == "range" => {
                assert_eq!(args.len(), 3, "expected [nil_start, end, exclusive_flag]");
                assert!(
                    matches!(&args[0], Expr::NilLit { .. }),
                    "beginless range start should be NilLit; got {:?}",
                    args[0]
                );
                assert!(matches!(&args[1], Expr::IntLit { value: 5, .. }));
                assert!(
                    matches!(&args[2], Expr::BoolLit { value: false, .. }),
                    "inclusive `..` should set the exclusive flag false; got {:?}",
                    args[2]
                );
            }
            other => panic!("expected BuiltinCall(range, …), got {:?}", other),
        }
    }

    #[test]
    fn beginless_range_exclusive_lowers_with_nil_start() {
        // `def r() (...5) end` — beginless exclusive sets the flag true.
        let m = lower("def r\n  (...5)\nend\n");
        let f = m.functions.iter().find(|f| f.name == "r").unwrap();
        match &f.body.value {
            Expr::BuiltinCall { name, args, .. } if name == "range" => {
                assert_eq!(args.len(), 3);
                assert!(matches!(&args[0], Expr::NilLit { .. }), "start should be nil");
                assert!(matches!(&args[1], Expr::IntLit { value: 5, .. }));
                assert!(
                    matches!(&args[2], Expr::BoolLit { value: true, .. }),
                    "exclusive `...` should set the flag true; got {:?}",
                    args[2]
                );
            }
            other => panic!("expected BuiltinCall(range, …), got {:?}", other),
        }
    }

    #[test]
    fn beginless_range_over_param_validates_e2e() {
        // `def r(b) (..b) end` — beginless range whose end is a param
        // (in scope, so the VarRef survives validation).  End-to-end pin:
        // nil start, VarRef end, and the module passes validation.
        let m = lower("def r(b)\n  (..b)\nend\n");
        let f = m.functions.iter().find(|f| f.name == "r").unwrap();
        match &f.body.value {
            Expr::BuiltinCall { name, args, .. } if name == "range" => {
                assert!(matches!(&args[0], Expr::NilLit { .. }));
                assert!(matches!(&args[1], Expr::VarRef { name, .. } if name == "b"));
            }
            other => panic!("expected BuiltinCall(range, …), got {:?}", other),
        }
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected beginless range: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 19a (FC) — regex literal `/pattern/flags` lowering.
    //
    // The lexer resolves the `/`-is-regex-vs-division ambiguity and emits
    // the literal as a verbatim `/p/flags` String token; the lowerer
    // splits it and emits `BuiltinCall("regex", [StrLit(pattern),
    // StrLit(flags)])` (flags = "" when none).  Pure; requests
    // `Feature::Strings`.  A path-shaped string like `"/usr/bin"` is NOT a
    // regex (the flag-letter check rejects it).
    // -----------------------------------------------------------------

    #[test]
    fn regex_literal_lowers_to_regex_builtin() {
        // `x = /foo/` — bare regex, no flags.
        let m = lower("x = /foo/\n");
        let value = main_body(&m).stmts.iter().find_map(|s| match s {
            Stmt::LetBinding { value, .. } | Stmt::Assign { value, .. } => Some(value.clone()),
            _ => None,
        }).expect("expected a binding for `x`");
        match &value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "regex");
                assert_eq!(args.len(), 2, "expected [pattern, flags]");
                assert!(
                    matches!(&args[0], Expr::StrLit { value, .. } if value == "foo"),
                    "expected pattern StrLit \"foo\"; got {:?}", args[0]
                );
                assert!(
                    matches!(&args[1], Expr::StrLit { value, .. } if value.is_empty()),
                    "expected empty flags StrLit; got {:?}", args[1]
                );
            }
            other => panic!("expected BuiltinCall(regex, …), got {:?}", other),
        }
    }

    #[test]
    fn regex_literal_with_flags_carries_flags() {
        // `x = /foo/i` — case-insensitive flag preserved in args[1].
        let m = lower("x = /foo/i\n");
        let value = main_body(&m).stmts.iter().find_map(|s| match s {
            Stmt::LetBinding { value, .. } | Stmt::Assign { value, .. } => Some(value.clone()),
            _ => None,
        }).expect("expected a binding for `x`");
        match &value {
            Expr::BuiltinCall { name, args, .. } if name == "regex" => {
                assert!(matches!(&args[0], Expr::StrLit { value, .. } if value == "foo"));
                assert!(
                    matches!(&args[1], Expr::StrLit { value, .. } if value == "i"),
                    "expected flags StrLit \"i\"; got {:?}", args[1]
                );
            }
            other => panic!("expected BuiltinCall(regex, …), got {:?}", other),
        }
    }

    #[test]
    fn regex_literal_validates_e2e() {
        // `def r() (/foo/) end` — regex as a method body survives
        // validation end-to-end.  Parens keep the body off the bare-NAME
        // method-call path (lessons.md).
        let m = lower("def r\n  (/foo/)\nend\n");
        let f = m.functions.iter().find(|f| f.name == "r").unwrap();
        match &f.body.value {
            Expr::BuiltinCall { name, .. } if name == "regex" => {}
            other => panic!("expected BuiltinCall(regex, …), got {:?}", other),
        }
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected regex literal: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 19b (FC) — regex flags `/r/i` coverage confirmation.
    //
    // Phase 19a already carries the flag letters in `args[1]` of the
    // `regex` builtin, so flags are a coverage-confirmation phase (cf.
    // 16b/16c): these pins exercise MULTI-flag combinations (the 19a
    // tests only covered a single `i`) and confirm the flag string is
    // preserved verbatim and in order.
    // -----------------------------------------------------------------

    #[test]
    fn regex_literal_multi_flag_preserves_all_flags() {
        // `x = /foo/im` — two flags; both letters preserved in order.
        let m = lower("x = /foo/im\n");
        let value = main_body(&m).stmts.iter().find_map(|s| match s {
            Stmt::LetBinding { value, .. } | Stmt::Assign { value, .. } => Some(value.clone()),
            _ => None,
        }).expect("expected a binding for `x`");
        match &value {
            Expr::BuiltinCall { name, args, .. } if name == "regex" => {
                assert!(matches!(&args[0], Expr::StrLit { value, .. } if value == "foo"));
                assert!(
                    matches!(&args[1], Expr::StrLit { value, .. } if value == "im"),
                    "expected flags StrLit \"im\" (order preserved); got {:?}", args[1]
                );
            }
            other => panic!("expected BuiltinCall(regex, …), got {:?}", other),
        }
    }

    #[test]
    fn regex_literal_all_common_flags_lower() {
        // `x = /a/mix` — the three common Ruby regex flags together.
        // Order is preserved exactly as written.
        let m = lower("x = /a/mix\n");
        let value = main_body(&m).stmts.iter().find_map(|s| match s {
            Stmt::LetBinding { value, .. } | Stmt::Assign { value, .. } => Some(value.clone()),
            _ => None,
        }).expect("expected a binding for `x`");
        match &value {
            Expr::BuiltinCall { name, args, .. } if name == "regex" => {
                assert!(matches!(&args[1], Expr::StrLit { value, .. } if value == "mix"));
            }
            other => panic!("expected BuiltinCall(regex, …), got {:?}", other),
        }
    }

    #[test]
    fn regex_literal_multi_flag_validates_e2e() {
        // `def r() (/x/im) end` — multi-flag regex survives validation.
        let m = lower("def r\n  (/x/im)\nend\n");
        let f = m.functions.iter().find(|f| f.name == "r").unwrap();
        match &f.body.value {
            Expr::BuiltinCall { name, args, .. } if name == "regex" => {
                assert!(matches!(&args[1], Expr::StrLit { value, .. } if value == "im"));
            }
            other => panic!("expected BuiltinCall(regex, …), got {:?}", other),
        }
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected multi-flag regex: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 19c (FC) — regex interpolation `/a#{b}c/` lowering.
    //
    // The `regex_body` lexer state captures `#{...}` markers verbatim
    // into the pattern, so the lowerer runs the pattern through the SAME
    // interpolation splitter strings use.  `args[0]` of the `regex`
    // builtin therefore becomes a `StrConcat` node (or a bare `VarRef`
    // for a single `#{x}` segment) when the pattern interpolates, and a
    // plain `StrLit` when it doesn't (the 19a shape, unchanged).
    // Phase 20b replaced the v0 `string_concat` builtin with the
    // first-class `Expr::StrConcat` node — regex patterns share the
    // same interpolation splitter, so they pick up the new shape too.
    // -----------------------------------------------------------------

    #[test]
    fn regex_interpolation_lowers_pattern_to_concat() {
        // `def r(b) (/a#{b}c/) end` — pattern splits into
        // [StrLit("a"), VarRef(b), StrLit("c")] under StrConcat.
        let m = lower("def r(b)\n  (/a#{b}c/)\nend\n");
        let f = m.functions.iter().find(|f| f.name == "r").unwrap();
        match &f.body.value {
            Expr::BuiltinCall { name, args, .. } if name == "regex" => {
                match &args[0] {
                    Expr::StrConcat { parts, .. } => {
                        assert_eq!(parts.len(), 3, "expected 3 pattern segments");
                        assert!(matches!(&parts[0], Expr::StrLit { value, .. } if value == "a"));
                        assert!(matches!(&parts[1], Expr::VarRef { name, .. } if name == "b"));
                        assert!(matches!(&parts[2], Expr::StrLit { value, .. } if value == "c"));
                    }
                    other => panic!("expected StrConcat pattern, got {:?}", other),
                }
                // Flags arg still present (empty here).
                assert!(matches!(&args[1], Expr::StrLit { value, .. } if value.is_empty()));
            }
            other => panic!("expected BuiltinCall(regex, …), got {:?}", other),
        }
    }

    #[test]
    fn regex_interpolation_single_marker_is_bare_varref() {
        // `def r(b) (/#{b}/) end` — a lone `#{b}` pattern lowers to a
        // single `VarRef` (no StrConcat wrapper), mirroring `"#{b}"`.
        let m = lower("def r(b)\n  (/#{b}/)\nend\n");
        let f = m.functions.iter().find(|f| f.name == "r").unwrap();
        match &f.body.value {
            Expr::BuiltinCall { name, args, .. } if name == "regex" => {
                assert!(
                    matches!(&args[0], Expr::VarRef { name, .. } if name == "b"),
                    "expected bare VarRef pattern; got {:?}", args[0]
                );
            }
            other => panic!("expected BuiltinCall(regex, …), got {:?}", other),
        }
    }

    #[test]
    fn regex_interpolation_validates_e2e() {
        // `def r(b) (/x#{b}/i) end` — interpolation + a flag, end-to-end:
        // pattern is a concat, flags = "i", and the module validates.
        let m = lower("def r(b)\n  (/x#{b}/i)\nend\n");
        let f = m.functions.iter().find(|f| f.name == "r").unwrap();
        match &f.body.value {
            Expr::BuiltinCall { name, args, .. } if name == "regex" => {
                assert!(
                    matches!(&args[0], Expr::StrConcat { .. }),
                    "expected StrConcat pattern; got {:?}", args[0]
                );
                assert!(matches!(&args[1], Expr::StrLit { value, .. } if value == "i"));
            }
            other => panic!("expected BuiltinCall(regex, …), got {:?}", other),
        }
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected interpolated regex: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 19d (FC) — `%r{...}` regex literal lowering.
    //
    // `%r{pat}flags` lexes to a verbatim `String` token; the lowerer
    // strips the `%r`, the open/close delimiters, and trailing flags,
    // then reuses `lower_regex_literal` — so `%r{...}` produces the SAME
    // `BuiltinCall("regex", [pattern, StrLit(flags)])` shape as `/.../`,
    // and interpolation inside `%r{...}` is handled for free.
    // -----------------------------------------------------------------

    #[test]
    fn percent_r_regex_lowers_to_regex_builtin() {
        // `x = %r{hello}` — brace-delimited, no flags.
        let m = lower("x = %r{hello}\n");
        let value = main_body(&m).stmts.iter().find_map(|s| match s {
            Stmt::LetBinding { value, .. } | Stmt::Assign { value, .. } => Some(value.clone()),
            _ => None,
        }).expect("expected a binding for `x`");
        match &value {
            Expr::BuiltinCall { name, args, .. } if name == "regex" => {
                assert_eq!(args.len(), 2);
                assert!(
                    matches!(&args[0], Expr::StrLit { value, .. } if value == "hello"),
                    "expected pattern StrLit \"hello\"; got {:?}", args[0]
                );
                assert!(matches!(&args[1], Expr::StrLit { value, .. } if value.is_empty()));
            }
            other => panic!("expected BuiltinCall(regex, …), got {:?}", other),
        }
    }

    #[test]
    fn percent_r_regex_empty_pattern_lowers() {
        // `x = %r{}` — empty brace-delimited regex.  Pattern is the empty
        // string; flags empty.  (v0 `%r` uses `{}` as the canonical
        // delimiter and does not slurp trailing flags — so the pattern is
        // exactly the body between the braces.)
        let m = lower("x = %r{}\n");
        let value = main_body(&m).stmts.iter().find_map(|s| match s {
            Stmt::LetBinding { value, .. } | Stmt::Assign { value, .. } => Some(value.clone()),
            _ => None,
        }).expect("expected a binding for `x`");
        match &value {
            Expr::BuiltinCall { name, args, .. } if name == "regex" => {
                assert!(
                    matches!(&args[0], Expr::StrLit { value, .. } if value.is_empty()),
                    "expected empty pattern StrLit; got {:?}", args[0]
                );
                assert!(matches!(&args[1], Expr::StrLit { value, .. } if value.is_empty()));
            }
            other => panic!("expected BuiltinCall(regex, …), got {:?}", other),
        }
    }

    #[test]
    fn percent_r_regex_validates_e2e() {
        // `def r() (%r{x}) end` — `%r{}` regex survives validation.
        let m = lower("def r\n  (%r{x})\nend\n");
        let f = m.functions.iter().find(|f| f.name == "r").unwrap();
        match &f.body.value {
            Expr::BuiltinCall { name, args, .. } if name == "regex" => {
                assert!(matches!(&args[0], Expr::StrLit { value, .. } if value == "x"));
            }
            other => panic!("expected BuiltinCall(regex, …), got {:?}", other),
        }
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected %r regex: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 6o — ternary `cond ? a : b` lowering
    // -----------------------------------------------------------------
    //
    // SIR encoding:
    //   `cond ? a : b` → Expr::If { cond, then_branch: Block{value:a},
    //                               else_branch: Block{value:b} }
    //
    // Lowering identically to `if cond then a else b end` means
    // downstream emitters need no new dispatch — the existing
    // if-lowering paths handle both syntactic forms transparently.

    #[test]
    fn ternary_lowers_to_if_expr_with_branch_blocks() {
        // `x = 1 ? 2 : 3` — the RHS of the assignment is the ternary,
        // which lowers to Expr::If { cond=1, then=2, else=3 }.
        let m = lower("x = 1 ? 2 : 3\n");
        let f = m
            .functions
            .iter()
            .find(|f| f.name == "main")
            .unwrap_or_else(|| panic!("expected main function, got {:?}",
                m.functions.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()));
        // The main body's first statement is a LetBinding whose value is the ternary.
        let stmt = f.body.stmts.first().expect("expected at least one stmt");
        let value = match stmt {
            Stmt::LetBinding { value, .. } => value,
            other => panic!("expected LetBinding, got {:?}", other),
        };
        match value {
            Expr::If { cond, then_branch, else_branch, .. } => {
                assert!(matches!(cond.as_ref(), Expr::IntLit { value: 1, .. }),
                    "expected cond = IntLit(1), got {:?}", cond);
                assert!(matches!(&then_branch.value, Expr::IntLit { value: 2, .. }),
                    "expected then = IntLit(2), got {:?}", then_branch.value);
                assert!(matches!(&else_branch.value, Expr::IntLit { value: 3, .. }),
                    "expected else = IntLit(3), got {:?}", else_branch.value);
            }
            other => panic!("expected Expr::If, got {:?}", other),
        }
    }

    #[test]
    fn ternary_right_associative_nests_in_else_branch() {
        // `x = a ? b : c ? d : e` parses as `a ? b : (c ? d : e)`.
        // SIR: outer If's else-branch contains an inner If.
        let m = lower("x = 1 ? 2 : 3 ? 4 : 5\n");
        let f = m.functions.iter().find(|f| f.name == "main").unwrap();
        let stmt = f.body.stmts.first().expect("expected at least one stmt");
        let value = match stmt {
            Stmt::LetBinding { value, .. } => value,
            other => panic!("expected LetBinding, got {:?}", other),
        };
        match value {
            Expr::If { else_branch, .. } => {
                match &else_branch.value {
                    Expr::If { cond: inner_cond, then_branch: inner_then, else_branch: inner_else, .. } => {
                        assert!(matches!(inner_cond.as_ref(), Expr::IntLit { value: 3, .. }));
                        assert!(matches!(&inner_then.value, Expr::IntLit { value: 4, .. }));
                        assert!(matches!(&inner_else.value, Expr::IntLit { value: 5, .. }));
                    }
                    other => panic!("expected nested If in else branch, got {:?}", other),
                }
            }
            other => panic!("expected outer Expr::If, got {:?}", other),
        }
    }

    #[test]
    fn ternary_module_passes_sir_validator() {
        // End-to-end smoke test: validator accepts the ternary lowering.
        let m = lower("x = 1 ? 2 : 3\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected ternary output: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------
    // Phase 6p — compound assignment lowering
    // -----------------------------------------------------------------
    //
    // SIR encoding (for each `x op= rhs`):
    //   Stmt::Assign {
    //     name: "x",
    //     scope: Local,
    //     value: Expr::BuiltinCall {
    //       name: "<op>",   // "+", "-", "*", "/", "or", "and"
    //       args: [VarRef("x"), <rhs>],
    //     },
    //   }
    //
    // Compound forms ALWAYS produce Assign (never LetBinding) even on
    // first sighting — the read of `x` before the write means the
    // binding semantically pre-exists.

    #[test]
    fn plus_equals_lowers_to_assign_with_plus_builtin() {
        // `x = 1\nx += 2` — first `x = 1` is LetBinding, second `x += 2`
        // is Assign with value = `+(x, 2)`.
        let m = lower("x = 1\nx += 2\n");
        let b = main_body(&m);
        assert_eq!(b.stmts.len(), 2);
        assert!(matches!(&b.stmts[0], Stmt::LetBinding { name, .. } if name == "x"));
        match &b.stmts[1] {
            Stmt::Assign { name, value, .. } => {
                assert_eq!(name, "x");
                match value {
                    Expr::BuiltinCall { name: op, args, .. } => {
                        assert_eq!(op, "+");
                        assert_eq!(args.len(), 2);
                        assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "x"));
                        assert!(matches!(&args[1], Expr::IntLit { value: 2, .. }));
                    }
                    other => panic!("expected BuiltinCall(+, …), got {:?}", other),
                }
            }
            other => panic!("expected Assign, got {:?}", other),
        }
    }

    #[test]
    fn all_arithmetic_compound_assigns_lower_correctly() {
        // Each of +=, -=, *=, /= maps to the corresponding binary
        // op builtin.
        for (op_src, op_builtin) in [("+=", "+"), ("-=", "-"), ("*=", "*"), ("/=", "/")] {
            let src = format!("x = 1\nx {op_src} 2\n");
            let m = lower(&src);
            let b = main_body(&m);
            match &b.stmts[1] {
                Stmt::Assign { value, .. } => match value {
                    Expr::BuiltinCall { name, .. } => {
                        assert_eq!(name, op_builtin, "wrong builtin for {op_src}");
                    }
                    other => panic!("expected BuiltinCall for {op_src}, got {:?}", other),
                },
                other => panic!("expected Assign for {op_src}, got {:?}", other),
            }
        }
    }

    // -----------------------------------------------------------------
    // Phase 8b — short-circuit op-assign (`||=`, `&&=`)
    // -----------------------------------------------------------------
    //
    // Ruby semantics:
    //   `x ||= y`  ≡  `x || (x = y)`
    //   `x &&= y`  ≡  `x && (x = y)`
    //
    // RHS must NOT be evaluated when the short-circuit branch fires,
    // so the lowering uses a gated `Expr::If`:
    //
    //   `x ||= y` → ExprStmt(If(
    //                  cond:        VarRef(x),
    //                  then_branch: Block { [],            VarRef(x) },
    //                  else_branch: Block { [Assign(x,y)], VarRef(x) },
    //                ))
    //
    // For `&&=` the two branches swap.  Phase 6p used to lower these to
    // an eager `Assign(x, BuiltinCall("or"/"and", [VarRef(x), y]))`,
    // which silently broke side-effect ordering when `y` had any.

    #[test]
    fn or_assign_lowers_to_short_circuit_if_with_assign_in_else_branch() {
        // `x ||= 2` after `x = 1`:
        //   - cond branch: VarRef(x)
        //   - then_branch: empty stmts, value VarRef(x)  (truthy → keep)
        //   - else_branch: [Assign(x, 2)], value VarRef(x)  (falsy → assign)
        let m = lower("x = 1\nx ||= 2\n");
        let b = main_body(&m);
        match &b.stmts[1] {
            Stmt::ExprStmt {
                expr: Expr::If { cond, then_branch, else_branch, .. },
                ..
            } => {
                assert!(
                    matches!(cond.as_ref(), Expr::VarRef { name, .. } if name == "x"),
                    "||= cond should be VarRef(x), got {:?}",
                    cond
                );
                assert!(
                    then_branch.stmts.is_empty(),
                    "||= then-branch should be empty (no-op when x is truthy), got {:?}",
                    then_branch.stmts
                );
                assert_eq!(
                    else_branch.stmts.len(),
                    1,
                    "||= else-branch should contain exactly one Assign"
                );
                match &else_branch.stmts[0] {
                    Stmt::Assign { name, value, .. } => {
                        assert_eq!(name, "x");
                        assert!(
                            matches!(value, Expr::IntLit { value: 2, .. }),
                            "||= else-branch should assign x = 2, got {:?}",
                            value
                        );
                    }
                    other => panic!("expected Assign in ||= else-branch, got {:?}", other),
                }
            }
            other => panic!("expected ExprStmt(If(...)) for ||=, got {:?}", other),
        }
    }

    #[test]
    fn and_assign_lowers_to_short_circuit_if_with_assign_in_then_branch() {
        // `x &&= 2` after `x = 1`:
        //   - cond branch: VarRef(x)
        //   - then_branch: [Assign(x, 2)], value VarRef(x)  (truthy → assign)
        //   - else_branch: empty stmts, value VarRef(x)     (falsy → keep)
        let m = lower("x = 1\nx &&= 2\n");
        let b = main_body(&m);
        match &b.stmts[1] {
            Stmt::ExprStmt {
                expr: Expr::If { cond, then_branch, else_branch, .. },
                ..
            } => {
                assert!(
                    matches!(cond.as_ref(), Expr::VarRef { name, .. } if name == "x"),
                    "&&= cond should be VarRef(x), got {:?}",
                    cond
                );
                assert_eq!(
                    then_branch.stmts.len(),
                    1,
                    "&&= then-branch should contain exactly one Assign"
                );
                match &then_branch.stmts[0] {
                    Stmt::Assign { name, value, .. } => {
                        assert_eq!(name, "x");
                        assert!(
                            matches!(value, Expr::IntLit { value: 2, .. }),
                            "&&= then-branch should assign x = 2, got {:?}",
                            value
                        );
                    }
                    other => panic!("expected Assign in &&= then-branch, got {:?}", other),
                }
                assert!(
                    else_branch.stmts.is_empty(),
                    "&&= else-branch should be empty (no-op when x is falsy), got {:?}",
                    else_branch.stmts
                );
            }
            other => panic!("expected ExprStmt(If(...)) for &&=, got {:?}", other),
        }
    }

    #[test]
    fn short_circuit_op_assign_marks_mutable_bindings_feature() {
        // Both ||= and &&= must declare `MutableBindings` in the
        // module's feature manifest, exactly like the eager compound
        // forms — the assignment in the gated branch is still a
        // re-binding from the validator's perspective.
        for op_src in ["||=", "&&="] {
            let src = format!("x = 1\nx {op_src} 2\n");
            let m = lower(&src);
            assert!(
                m.manifest.contains(semantic_ir::Feature::MutableBindings),
                "{op_src} module should require MutableBindings, got: {:?}",
                m.manifest
            );
        }
    }

    #[test]
    fn short_circuit_op_assign_module_passes_sir_validator() {
        // End-to-end smoke check for both forms.  The validator must
        // accept the `If` shape with an `Assign` nested in one branch
        // (which is a less-common construction than top-level Assign,
        // but well within SIR's grammar).
        for op_src in ["||=", "&&="] {
            let src = format!("x = 1\nx {op_src} 2\n");
            let m = lower(&src);
            let result = semantic_ir::validate(&m);
            assert!(
                result.is_ok(),
                "validator rejected {op_src} short-circuit lowering: {:?}",
                result
            );
        }
    }

    #[test]
    fn compound_assign_module_passes_sir_validator() {
        // End-to-end smoke check: validator accepts the compound-assign
        // lowering (requires `mutable-bindings` feature, which the
        // lowerer marks automatically).
        let m = lower("x = 1\nx += 2\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected compound-assign output: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------
    // Phase 6q — modifier conditionals/loops
    // -----------------------------------------------------------------
    //
    // `lhs if cond`    → ExprStmt(If(cond, [lhs], Nil))
    // `lhs unless cond`→ ExprStmt(If(not(cond), [lhs], Nil))
    // `lhs while cond` → While(cond, [lhs])
    // `lhs until cond` → While(not(cond), [lhs])
    //
    // Lowering identity: the modifier forms emit the same canonical
    // `Expr::If` / `Stmt::While` shapes as the leading-keyword forms,
    // so every downstream emitter sees them transparently.

    #[test]
    fn if_modifier_lowers_to_expr_if_statement() {
        // `puts "hi" if cond` → ExprStmt(If(VarRef(cond),
        //    Block{stmts:[ExprStmt(BuiltinCall(puts, ...))], value:Nil},
        //    Block{stmts:[], value:Nil}))
        let m = lower("cond = true\nputs \"hi\" if cond\n");
        let b = main_body(&m);
        // The second statement is the modifier's lowered form.
        match &b.stmts[1] {
            Stmt::ExprStmt {
                expr: Expr::If { cond, then_branch, else_branch, .. },
                ..
            } => {
                // Cond is a bare VarRef("cond") — no `not` wrapper.
                assert!(
                    matches!(cond.as_ref(), Expr::VarRef { name, .. } if name == "cond"),
                    "expected VarRef(cond), got {:?}",
                    cond
                );
                // then_branch carries the LHS statement.
                assert_eq!(then_branch.stmts.len(), 1, "expected one stmt in then-branch");
                // else_branch is empty.
                assert!(else_branch.stmts.is_empty(), "expected empty else-branch");
            }
            other => panic!("expected ExprStmt(If), got {:?}", other),
        }
    }

    #[test]
    fn unless_modifier_wraps_condition_in_not() {
        // `x = 1 unless cond` — cond becomes `not(cond)`.
        let m = lower("cond = true\nx = 1 unless cond\n");
        let b = main_body(&m);
        match &b.stmts[1] {
            Stmt::ExprStmt {
                expr: Expr::If { cond, .. },
                ..
            } => {
                assert!(
                    matches!(cond.as_ref(), Expr::BuiltinCall { name, .. } if name == "not"),
                    "expected BuiltinCall(not, ...), got {:?}",
                    cond
                );
            }
            other => panic!("expected ExprStmt(If) for unless modifier, got {:?}", other),
        }
    }

    #[test]
    fn while_modifier_lowers_to_stmt_while() {
        // `puts "tick" while cond` → Stmt::While(cond, [puts]).
        let m = lower("cond = true\nputs \"tick\" while cond\n");
        let b = main_body(&m);
        let while_cond = b.stmts.iter().find_map(|s| match s {
            Stmt::While { cond, body, .. } => Some((cond, body)),
            _ => None,
        }).expect("expected While stmt from `while` modifier");
        // Cond is bare VarRef (no `not`).
        assert!(
            matches!(while_cond.0, Expr::VarRef { name, .. } if name == "cond"),
            "expected VarRef(cond) for while modifier, got {:?}",
            while_cond.0
        );
        // Body carries the LHS as its one statement.
        assert_eq!(while_cond.1.stmts.len(), 1, "expected one stmt in body");
    }

    #[test]
    fn until_modifier_negates_condition_in_while() {
        // `x = 1 until cond` → Stmt::While(not(cond), [x = 1]).
        let m = lower("cond = false\nx = 1 until cond\n");
        let b = main_body(&m);
        let while_cond = b.stmts.iter().find_map(|s| match s {
            Stmt::While { cond, .. } => Some(cond),
            _ => None,
        }).expect("expected While stmt from `until` modifier");
        assert!(
            matches!(while_cond, Expr::BuiltinCall { name, .. } if name == "not"),
            "expected `until` to negate cond, got {:?}",
            while_cond
        );
    }

    #[test]
    fn modifier_module_passes_sir_validator() {
        // End-to-end smoke check: every modifier form goes through
        // the validator cleanly.  The `while` modifier sets the
        // `loops` feature automatically (just like the leading
        // `while_statement` form).
        let m = lower(concat!(
            "cond = true\n",
            "y = 0\n",
            "y = 1 if cond\n",
            "y = 2 unless cond\n",
            "y = 3 while cond\n",
            "y = 4 until cond\n",
        ));
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected modifier-statement output: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------
    // Phase 6r — multiple assignment
    // -----------------------------------------------------------------
    //
    // `a, b = 1, 2` fans out into two independent SIR statements:
    //   LetBinding(a, IntLit(1))
    //   LetBinding(b, IntLit(2))
    //
    // Each pair follows the same LetBinding-vs-Assign decision as the
    // single-LHS `assignment` lowerer.

    #[test]
    fn multi_assignment_lowers_to_independent_let_bindings() {
        let m = lower("a, b = 1, 2\n");
        let b = main_body(&m);
        // The single `multi_assignment` source statement produced two
        // independent SIR statements.
        assert_eq!(b.stmts.len(), 2, "expected 2 SIR stmts, got {:?}", b.stmts);
        match &b.stmts[0] {
            Stmt::LetBinding { name, value, .. } => {
                assert_eq!(name, "a");
                assert!(matches!(value, Expr::IntLit { value: 1, .. }));
            }
            other => panic!("expected LetBinding(a, 1), got {:?}", other),
        }
        match &b.stmts[1] {
            Stmt::LetBinding { name, value, .. } => {
                assert_eq!(name, "b");
                assert!(matches!(value, Expr::IntLit { value: 2, .. }));
            }
            other => panic!("expected LetBinding(b, 2), got {:?}", other),
        }
    }

    #[test]
    fn multi_assignment_redeclaration_uses_assign() {
        // After `a = 1; b = 2`, subsequent `a, b = 3, 4` re-binds — so
        // both targets must lower to `Stmt::Assign`, not `LetBinding`.
        let m = lower("a = 1\nb = 2\na, b = 3, 4\n");
        let b = main_body(&m);
        // Stmts: LetBinding(a,1), LetBinding(b,2), Assign(a,3), Assign(b,4).
        assert_eq!(b.stmts.len(), 4);
        assert!(matches!(&b.stmts[2], Stmt::Assign { name, .. } if name == "a"));
        assert!(matches!(&b.stmts[3], Stmt::Assign { name, .. } if name == "b"));
    }

    #[test]
    fn multi_assignment_three_names_emits_three_stmts() {
        // Three LHS / three RHS → three SIR stmts.
        let m = lower("a, b, c = 1, 2, 3\n");
        let b = main_body(&m);
        assert_eq!(b.stmts.len(), 3);
        for (i, (expect_name, expect_val)) in
            [("a", 1i64), ("b", 2), ("c", 3)].iter().enumerate()
        {
            match &b.stmts[i] {
                Stmt::LetBinding { name, value, .. } => {
                    assert_eq!(name, expect_name);
                    assert!(matches!(value, Expr::IntLit { value, .. } if value == expect_val));
                }
                other => panic!("expected LetBinding({}, {}), got {:?}",
                    expect_name, expect_val, other),
            }
        }
    }

    #[test]
    fn multi_assignment_arity_mismatch_errors() {
        // v0 rejects mismatched LHS/RHS counts.  `a, b = 1, 2, 3`
        // has 2 LHS and 3 RHS — the lowerer returns a RubyLowerError
        // (surfaced through the `lower` test helper's `expect`).
        let result = compile_source("a, b = 1, 2, 3\n", "test");
        assert!(
            result.is_err(),
            "expected RubyLowerError for arity mismatch, got Ok"
        );
        let msg = result.unwrap_err().message;
        assert!(
            msg.contains("LHS count == RHS count") || msg.contains("multi_assignment"),
            "expected arity-mismatch error, got: {msg}"
        );
    }

    #[test]
    fn multi_assignment_module_passes_sir_validator() {
        // End-to-end smoke check.  The `let`-based output requires no
        // new feature beyond the existing `mutable-bindings` (which the
        // first-sight LetBinding case doesn't need).
        let m = lower("a, b = 1, 2\nputs(a + b)\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected multi_assignment output: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------
    // Phase 9a (FC) — swap-safe parallel binding
    // -----------------------------------------------------------------
    //
    // Phase 6r's lowering was sequential: `a, b = b, a` became
    // `Assign(a, VarRef(b)); Assign(b, VarRef(a))` — the second assign
    // reads the NEW `a`, silently producing `a = old_b; b = old_b`
    // instead of a true swap.
    //
    // Phase 9a fixes this by detecting whether any LHS name appears in
    // any RHS expression.  If so, the lowerer introduces fresh
    // `LetStarBinding` temps (`__multi_assign_t<N>_<i>`) capturing the
    // ORIGINAL RHS values first; the LHS-binding pass then reads each
    // temp.  If no LHS appears in any RHS, the existing sequential
    // shape is preserved (no temps).
    //
    // The "needs-temps" detector is a structural walk over Expr; any
    // VarRef to an LHS name (no matter how deeply nested) triggers
    // the temp-pass.

    #[test]
    fn multi_assignment_swap_introduces_temps_to_preserve_parallel_semantics() {
        // `a, b = b, a` must NOT lower to the buggy sequential form.
        // Expected SIR:
        //   LetBinding(a, 1)
        //   LetBinding(b, 2)
        //   LetStarBinding(__multi_assign_t0_0, VarRef(b))   <- captures original b
        //   LetStarBinding(__multi_assign_t0_1, VarRef(a))   <- captures original a
        //   Assign(a, VarRef(__multi_assign_t0_0))
        //   Assign(b, VarRef(__multi_assign_t0_1))
        let m = lower("a = 1\nb = 2\na, b = b, a\n");
        let body = main_body(&m);
        assert_eq!(
            body.stmts.len(),
            6,
            "swap should emit 6 stmts (2 priors + 2 temps + 2 assigns), got {:?}",
            body.stmts
        );
        // Temp 0 captures original b.
        match &body.stmts[2] {
            Stmt::LetStarBinding { name, value, .. } => {
                assert!(name.starts_with("__multi_assign_t"));
                assert!(
                    matches!(value, Expr::VarRef { name: n, .. } if n == "b"),
                    "temp 0 should capture VarRef(b), got {:?}",
                    value
                );
            }
            other => panic!("expected LetStarBinding for temp 0, got {:?}", other),
        }
        // Temp 1 captures original a.
        match &body.stmts[3] {
            Stmt::LetStarBinding { name, value, .. } => {
                assert!(name.starts_with("__multi_assign_t"));
                assert!(
                    matches!(value, Expr::VarRef { name: n, .. } if n == "a"),
                    "temp 1 should capture VarRef(a), got {:?}",
                    value
                );
            }
            other => panic!("expected LetStarBinding for temp 1, got {:?}", other),
        }
        // a := temp 0
        match &body.stmts[4] {
            Stmt::Assign { name, value, .. } => {
                assert_eq!(name, "a");
                assert!(
                    matches!(value, Expr::VarRef { name: n, .. } if n.starts_with("__multi_assign_t")),
                    "a should be assigned from a temp, got {:?}",
                    value
                );
            }
            other => panic!("expected Assign(a, temp), got {:?}", other),
        }
        // b := temp 1
        match &body.stmts[5] {
            Stmt::Assign { name, value, .. } => {
                assert_eq!(name, "b");
                assert!(
                    matches!(value, Expr::VarRef { name: n, .. } if n.starts_with("__multi_assign_t")),
                    "b should be assigned from a temp, got {:?}",
                    value
                );
            }
            other => panic!("expected Assign(b, temp), got {:?}", other),
        }
    }

    #[test]
    fn multi_assignment_simple_case_keeps_fast_path_with_no_temps() {
        // Sanity: the new heuristic must NOT introduce temps when no
        // LHS appears in any RHS.  This preserves the Phase 6r shape
        // assumption that `a, b = 1, 2` is 2 stmts (not 4).
        let m = lower("a, b = 1, 2\n");
        let body = main_body(&m);
        assert_eq!(
            body.stmts.len(),
            2,
            "simple-case multi-assign should stay 2 stmts, got {:?}",
            body.stmts
        );
        // Nothing should be a `LetStarBinding` (those would be temps).
        for s in &body.stmts {
            assert!(
                !matches!(s, Stmt::LetStarBinding { .. }),
                "simple-case should not introduce temps, got {:?}",
                s
            );
        }
    }

    #[test]
    fn multi_assignment_partial_dependency_still_uses_temps_for_all_positions() {
        // `a, b = 1, a` — only the SECOND RHS references an LHS, but
        // the temp-pass kicks in for the whole multi-assignment so the
        // capture is uniform.  Without this, position 1 would read the
        // post-assignment `a` instead of the original.
        let m = lower("a = 7\nb = 8\na, b = 1, a\n");
        let body = main_body(&m);
        // Expect: 2 prior LetBindings + 2 temps + 2 assigns = 6 stmts.
        assert_eq!(body.stmts.len(), 6,
            "partial-dep multi-assign should emit 6 stmts, got {:?}", body.stmts);
        // Both temps must be LetStarBindings.
        assert!(matches!(&body.stmts[2], Stmt::LetStarBinding { .. }));
        assert!(matches!(&body.stmts[3], Stmt::LetStarBinding { .. }));
        // Temp 0 captures literal 1 (no LHS reference in this RHS).
        match &body.stmts[2] {
            Stmt::LetStarBinding { value, .. } => {
                assert!(
                    matches!(value, Expr::IntLit { value: 1, .. }),
                    "temp 0 should capture IntLit(1), got {:?}",
                    value
                );
            }
            _ => unreachable!(),
        }
        // Temp 1 captures the original VarRef(a).
        match &body.stmts[3] {
            Stmt::LetStarBinding { value, .. } => {
                assert!(
                    matches!(value, Expr::VarRef { name: n, .. } if n == "a"),
                    "temp 1 should capture VarRef(a), got {:?}",
                    value
                );
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn multi_assignment_swap_module_passes_sir_validator() {
        // Validator must accept the temp-pass lowering — LetStarBinding
        // adds the temp name to the env immediately, so the subsequent
        // Assign's VarRef sees it; `Feature::MutableBindings` is
        // already requested for the re-binding LHS.
        let m = lower("a = 1\nb = 2\na, b = b, a\nputs(a)\nputs(b)\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected swap-case multi-assign: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------
    // Phase 9b (FC) — splat LHS in multi-assignment
    // -----------------------------------------------------------------
    //
    // Grammar now allows `[ "*" ] NAME` per LHS target.  At most one
    // splat per LHS — the splat absorbs zero or more "extra" RHS
    // values into an `Expr::SeqLit` while non-splat targets bind to
    // fixed-position RHS values (counted from the start, or from the
    // end if a splat sits to the left).
    //
    // The splat path always routes through the swap-safe temp pass
    // (Phase 9a pattern): every RHS value is bound to a fresh
    // `LetStarBinding(__multi_assign_t<N>_<i>, rhs[i])`, then each LHS
    // is assigned from the corresponding temp(s).

    #[test]
    fn splat_lhs_at_end_absorbs_trailing_rhs_into_seqlit() {
        // `a, *b = 1, 2, 3` → a=1; b=[2, 3]
        let m = lower("a, *b = 1, 2, 3\n");
        let body = main_body(&m);
        // 3 temps + 2 LHS bindings = 5 stmts.
        assert_eq!(
            body.stmts.len(),
            5,
            "expected 5 stmts (3 temps + 2 LHS), got {:?}",
            body.stmts
        );
        // Stmt[3] binds `a` to temp 0 (= IntLit 1).
        match &body.stmts[3] {
            Stmt::LetBinding { name, value, .. } | Stmt::LetStarBinding { name, value, .. } => {
                assert_eq!(name, "a");
                assert!(
                    matches!(value, Expr::VarRef { name: n, .. } if n.starts_with("__multi_assign_t")),
                    "a should bind to a temp VarRef, got {:?}",
                    value
                );
            }
            other => panic!("expected LetBinding(a, temp), got {:?}", other),
        }
        // Stmt[4] binds `b` to SeqLit of temps 1..3.
        match &body.stmts[4] {
            Stmt::LetBinding { name, value, .. } | Stmt::LetStarBinding { name, value, .. } => {
                assert_eq!(name, "b");
                match value {
                    Expr::SeqLit { items, .. } => {
                        assert_eq!(items.len(), 2, "splat should absorb 2 values");
                        for item in items {
                            assert!(
                                matches!(item, Expr::VarRef { name: n, .. } if n.starts_with("__multi_assign_t")),
                                "splat items should be VarRef to temps, got {:?}",
                                item
                            );
                        }
                    }
                    other => panic!("expected SeqLit for splat, got {:?}", other),
                }
            }
            other => panic!("expected LetBinding(b, SeqLit), got {:?}", other),
        }
    }

    #[test]
    fn splat_lhs_at_start_absorbs_leading_rhs_into_seqlit() {
        // `*a, b = 1, 2, 3` → a=[1, 2]; b=3
        let m = lower("*a, b = 1, 2, 3\n");
        let body = main_body(&m);
        assert_eq!(body.stmts.len(), 5);
        // Stmt[3] binds `a` to SeqLit of temps 0..2.
        match &body.stmts[3] {
            Stmt::LetBinding { name, value, .. } | Stmt::LetStarBinding { name, value, .. } => {
                assert_eq!(name, "a");
                match value {
                    Expr::SeqLit { items, .. } => {
                        assert_eq!(items.len(), 2, "splat at start should absorb 2 values");
                    }
                    other => panic!("expected SeqLit for splat, got {:?}", other),
                }
            }
            other => panic!("expected LetBinding(a, SeqLit), got {:?}", other),
        }
        // Stmt[4] binds `b` to temp 2 (the last temp).
        match &body.stmts[4] {
            Stmt::LetBinding { name, value, .. } | Stmt::LetStarBinding { name, value, .. } => {
                assert_eq!(name, "b");
                assert!(
                    matches!(value, Expr::VarRef { name: n, .. } if n.starts_with("__multi_assign_t")),
                );
            }
            other => panic!("expected LetBinding(b, temp), got {:?}", other),
        }
    }

    #[test]
    fn splat_lhs_in_middle_absorbs_middle_rhs_into_seqlit() {
        // `a, *b, c = 1, 2, 3, 4` → a=1; b=[2, 3]; c=4
        let m = lower("a, *b, c = 1, 2, 3, 4\n");
        let body = main_body(&m);
        // 4 temps + 3 LHS bindings = 7 stmts.
        assert_eq!(body.stmts.len(), 7);
        // Stmt[4] binds `a` to temp 0.
        assert!(matches!(&body.stmts[4], Stmt::LetBinding { name, .. } | Stmt::LetStarBinding { name, .. } if name == "a"));
        // Stmt[5] binds `b` to SeqLit of 2 items (the middle 2 temps).
        match &body.stmts[5] {
            Stmt::LetBinding { name, value, .. } | Stmt::LetStarBinding { name, value, .. } => {
                assert_eq!(name, "b");
                match value {
                    Expr::SeqLit { items, .. } => assert_eq!(items.len(), 2),
                    other => panic!("expected SeqLit, got {:?}", other),
                }
            }
            other => panic!("expected LetBinding(b, SeqLit), got {:?}", other),
        }
        // Stmt[6] binds `c` to the last temp.
        assert!(matches!(&body.stmts[6], Stmt::LetBinding { name, .. } | Stmt::LetStarBinding { name, .. } if name == "c"));
    }

    #[test]
    fn splat_lhs_with_minimum_rhs_count_gives_empty_seqlit() {
        // `a, *b = 1` → a=1; b=[]
        let m = lower("a, *b = 1\n");
        let body = main_body(&m);
        // 1 temp + 2 LHS bindings = 3 stmts.
        assert_eq!(body.stmts.len(), 3);
        match &body.stmts[2] {
            Stmt::LetBinding { name, value, .. } => {
                assert_eq!(name, "b");
                match value {
                    Expr::SeqLit { items, .. } => assert!(items.is_empty()),
                    other => panic!("expected empty SeqLit, got {:?}", other),
                }
            }
            other => panic!("expected LetBinding(b, SeqLit), got {:?}", other),
        }
    }

    #[test]
    fn splat_lhs_requests_sequences_feature() {
        // The splat target binds a SeqLit which requires the
        // `Sequences` feature in the module manifest.
        let m = lower("a, *b = 1, 2, 3\n");
        assert!(
            m.manifest.contains(semantic_ir::Feature::Sequences),
            "splat-target multi-assignment should require Sequences"
        );
    }

    #[test]
    fn splat_lhs_module_passes_sir_validator() {
        // End-to-end smoke test for all three splat positions.
        for src in [
            "a, *b = 1, 2, 3\n",
            "*a, b = 1, 2, 3\n",
            "a, *b, c = 1, 2, 3, 4\n",
        ] {
            let m = lower(src);
            let result = semantic_ir::validate(&m);
            assert!(
                result.is_ok(),
                "validator rejected splat multi-assign `{}`: {:?}",
                src.trim(),
                result
            );
        }
    }

    #[test]
    fn splat_lhs_too_few_rhs_is_a_lower_error() {
        // `a, *b, c = 1` has 2 non-splat LHS but only 1 RHS — the
        // arity check should reject this.
        let result = compile_source("a, *b, c = 1\n", "test");
        assert!(result.is_err(), "expected RubyLowerError for splat with too-few RHS");
        let msg = result.unwrap_err().message;
        assert!(
            msg.contains("non-splat LHS") || msg.contains("RHS count"),
            "expected splat-arity error, got: {msg}"
        );
    }

    // ───────────────────────────────────────────────────────────────
    // Phase 9c (FC) — single-RHS tuple destructure (`a, b = arr`)
    // ───────────────────────────────────────────────────────────────

    #[test]
    fn single_rhs_destructure_two_lhs_reads_seq_index_zero_and_one() {
        // `a, b = arr` after `arr = [10, 20]`.  The lowering should
        // bind `arr` to a temp once via LetStarBinding, then read
        // `temp[0]` into `a` and `temp[1]` into `b`.
        let m = lower("arr = [10, 20]\na, b = arr\n");
        let body = main_body(&m);
        // arr LetBinding + temp LetStarBinding + 2 LHS LetBindings = 4.
        assert_eq!(
            body.stmts.len(),
            4,
            "expected 4 stmts (arr + temp + 2 LHS), got {:?}",
            body.stmts
        );
        // Stmt[1] is the temp LetStarBinding.
        match &body.stmts[1] {
            Stmt::LetStarBinding { name, .. } => {
                assert!(
                    name.starts_with("__multi_assign_t") && name.ends_with("_seq"),
                    "expected single-RHS temp `__multi_assign_t<N>_seq`, got `{}`",
                    name
                );
            }
            other => panic!("expected LetStarBinding(temp, arr), got {:?}", other),
        }
        // Stmt[2] binds `a` to `temp[0]`.
        match &body.stmts[2] {
            Stmt::LetBinding { name, value, .. } | Stmt::LetStarBinding { name, value, .. } => {
                assert_eq!(name, "a");
                match value {
                    Expr::SeqIndex { seq, index, .. } => {
                        assert!(
                            matches!(seq.as_ref(),
                                Expr::VarRef { name: n, .. }
                                    if n.starts_with("__multi_assign_t")),
                            "expected SeqIndex.seq to be VarRef(temp), got {:?}",
                            seq
                        );
                        assert!(
                            matches!(index.as_ref(), Expr::IntLit { value: 0, .. }),
                            "expected SeqIndex.index = IntLit(0), got {:?}",
                            index
                        );
                    }
                    other => panic!("expected SeqIndex(temp, 0), got {:?}", other),
                }
            }
            other => panic!("expected LetBinding(a, SeqIndex), got {:?}", other),
        }
        // Stmt[3] binds `b` to `temp[1]`.
        match &body.stmts[3] {
            Stmt::LetBinding { name, value, .. } | Stmt::LetStarBinding { name, value, .. } => {
                assert_eq!(name, "b");
                match value {
                    Expr::SeqIndex { index, .. } => assert!(
                        matches!(index.as_ref(), Expr::IntLit { value: 1, .. }),
                        "expected SeqIndex.index = IntLit(1), got {:?}",
                        index
                    ),
                    other => panic!("expected SeqIndex(temp, 1), got {:?}", other),
                }
            }
            other => panic!("expected LetBinding(b, SeqIndex), got {:?}", other),
        }
    }

    #[test]
    fn single_rhs_destructure_three_lhs_emits_three_seq_indexes() {
        // `a, b, c = arr` — three SeqIndex reads, indices 0/1/2.
        let m = lower("arr = [10, 20, 30]\na, b, c = arr\n");
        let body = main_body(&m);
        // arr + temp + 3 LHS = 5.
        assert_eq!(body.stmts.len(), 5);
        let expected_indices = [0i64, 1, 2];
        for (i, expected) in expected_indices.iter().enumerate() {
            // body.stmts[i + 2] is the i-th LHS binding.
            match &body.stmts[i + 2] {
                Stmt::LetBinding { value: Expr::SeqIndex { index, .. }, .. }
                | Stmt::LetStarBinding { value: Expr::SeqIndex { index, .. }, .. } => {
                    match index.as_ref() {
                        Expr::IntLit { value: v, .. } => assert_eq!(
                            v, expected,
                            "stmt[{}] expected index {} got {}",
                            i + 2,
                            expected,
                            v
                        ),
                        other => panic!("expected IntLit, got {:?}", other),
                    }
                }
                other => panic!("stmt[{}] expected LetBinding(SeqIndex), got {:?}", i + 2, other),
            }
        }
    }

    #[test]
    fn single_rhs_destructure_evaluates_rhs_exactly_once() {
        // The whole point of the temp is to evaluate the RHS once.
        // Verify the lowered SIR has exactly ONE statement holding
        // the lowered RHS (the LetStarBinding for the temp) — the
        // LHS bindings should reference the temp by name, not re-
        // evaluate the original RHS expression.
        //
        // We use a builtin-call shape (`String.new`) as a stand-in for
        // "something with potential side effects".
        let m = lower("a, b = make_pair\n");
        let body = main_body(&m);
        // temp + 2 LHS = 3 stmts.
        assert_eq!(body.stmts.len(), 3);
        // The first statement should be the LetStarBinding wrapping
        // the entire RHS expression.  Either side of the SeqIndex
        // reads is a *VarRef*, not a re-lowering of `make_pair`.
        match &body.stmts[0] {
            Stmt::LetStarBinding { name, .. } => assert!(
                name.starts_with("__multi_assign_t") && name.ends_with("_seq")
            ),
            other => panic!("expected LetStarBinding(temp, rhs), got {:?}", other),
        }
        for i in 1..=2 {
            if let Stmt::LetBinding { value: Expr::SeqIndex { seq, .. }, .. }
            | Stmt::LetStarBinding { value: Expr::SeqIndex { seq, .. }, .. } =
                &body.stmts[i]
            {
                assert!(
                    matches!(seq.as_ref(), Expr::VarRef { .. }),
                    "stmt[{}] SeqIndex.seq should be VarRef(temp), got {:?}",
                    i,
                    seq
                );
            } else {
                panic!("stmt[{}] expected LetBinding(SeqIndex), got {:?}", i, body.stmts[i]);
            }
        }
    }

    #[test]
    fn single_rhs_destructure_requests_sequences_feature() {
        // SeqIndex requires `Feature::Sequences` in the manifest.
        let m = lower("arr = [1, 2]\na, b = arr\n");
        assert!(
            m.manifest.contains(semantic_ir::Feature::Sequences),
            "single-RHS multi-assignment should require Sequences"
        );
    }

    #[test]
    fn single_rhs_destructure_module_passes_sir_validator() {
        // End-to-end: the SIR module produced by the lowerer should
        // pass the validator without errors.
        for src in [
            "arr = [1, 2]\na, b = arr\n",
            "arr = [10, 20, 30]\na, b, c = arr\n",
            // Re-bind path: pre-existing locals should re-assign cleanly.
            "a = 0\nb = 0\narr = [1, 2]\na, b = arr\n",
        ] {
            let m = lower(src);
            let result = semantic_ir::validate(&m);
            assert!(
                result.is_ok(),
                "validator rejected single-RHS destructure `{}`: {:?}",
                src.trim().replace('\n', " | "),
                result
            );
        }
    }

    #[test]
    fn single_rhs_destructure_rebind_uses_assign_not_letbinding() {
        // `a` was already declared as a local — the LHS binding for
        // `a` after `a, b = arr` should be an Assign, not a new
        // LetBinding (and `MutableBindings` should be requested).
        let m = lower("a = 0\narr = [1, 2]\na, b = arr\n");
        let body = main_body(&m);
        // a-decl, arr-decl, temp, a-assign, b-let = 5 stmts.
        assert_eq!(body.stmts.len(), 5);
        match &body.stmts[3] {
            Stmt::Assign { name, .. } => assert_eq!(name, "a"),
            other => panic!("expected Assign(a, ...) for re-bind, got {:?}", other),
        }
        match &body.stmts[4] {
            Stmt::LetBinding { name, .. } | Stmt::LetStarBinding { name, .. } => {
                assert_eq!(name, "b")
            }
            other => panic!("expected LetBinding(b, ...) for first-sighting, got {:?}", other),
        }
        assert!(
            m.manifest.contains(semantic_ir::Feature::MutableBindings),
            "re-bind path should request MutableBindings"
        );
    }

    #[test]
    fn single_rhs_destructure_lhs_4_rhs_2_no_splat_still_errors() {
        // The arity check should ONLY relax for the 1-RHS case.  A
        // 2-RHS, 4-LHS form is still a hard error.
        let result = compile_source("a, b, c, d = 1, 2\n", "test");
        assert!(result.is_err(), "expected RubyLowerError for 4 LHS / 2 RHS");
        let msg = result.unwrap_err().message;
        assert!(
            msg.contains("LHS count == RHS count") || msg.contains("tuple destructure"),
            "expected arity error mentioning tuple destructure, got: {msg}"
        );
    }

    #[test]
    fn multi_assignment_two_swaps_use_distinct_temp_counters() {
        // Two consecutive swaps in the same scope must use DIFFERENT
        // temp counters so the second swap's temps don't shadow or
        // collide with the first.  This guards against accidental
        // re-use of the counter inside a single scope.
        let m = lower(
            "a = 1\nb = 2\nc = 3\nd = 4\na, b = b, a\nc, d = d, c\n",
        );
        let body = main_body(&m);
        // Collect temp names that appear as LetStarBinding declarations.
        let mut temp_names: Vec<String> = body
            .stmts
            .iter()
            .filter_map(|s| match s {
                Stmt::LetStarBinding { name, .. } => Some(name.clone()),
                _ => None,
            })
            .collect();
        temp_names.sort();
        temp_names.dedup();
        assert_eq!(
            temp_names.len(),
            4,
            "expected 4 distinct temp names across two swaps, got {:?}",
            temp_names
        );
    }

    // -----------------------------------------------------------------
    // Phase 6s — splat / double-splat
    // -----------------------------------------------------------------
    //
    // Splat in CALL ARGS:
    //   `f(*arr)`  → DirectCall(f, [BuiltinCall("splat",        [VarRef(arr)])])
    //   `f(**hsh)` → DirectCall(f, [BuiltinCall("double_splat", [VarRef(hsh)])])
    //
    // Splat in DEF PARAMS:
    //   v0 limitation — Param has no variadic flag.  Splat-ness is
    //   LOST at the SIR level; `*args` lowers to `Param { name: "args" }`,
    //   indistinguishable from a positional parameter.  Documented for
    //   downstream emitter follow-up.

    #[test]
    fn splat_call_arg_lowers_to_splat_builtin() {
        // `arr` must be a local first so the validator accepts the VarRef.
        let m = lower("arr = [1, 2, 3]\nf(*arr)\n");
        let b = main_body(&m);
        // Find the DirectCall(f, …) — it's either the tail value
        // (promoted) or the second body stmt.
        let call_expr: &Expr = match &b.value {
            Expr::DirectCall { fn_name, .. } if fn_name == "f" => &b.value,
            _ => panic!("expected tail DirectCall(f, ...), got value={:?}", b.value),
        };
        match call_expr {
            Expr::DirectCall { args, .. } => {
                assert_eq!(args.len(), 1);
                match &args[0] {
                    Expr::BuiltinCall { name, args, .. } => {
                        assert_eq!(name, "splat");
                        assert_eq!(args.len(), 1);
                        assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "arr"));
                    }
                    other => panic!("expected BuiltinCall(splat, ...), got {:?}", other),
                }
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn double_splat_call_arg_lowers_to_double_splat_builtin() {
        let m = lower("hsh = {a: 1}\nf(**hsh)\n");
        let b = main_body(&m);
        let args = match &b.value {
            Expr::DirectCall { fn_name, args, .. } if fn_name == "f" => args,
            other => panic!("expected DirectCall(f, ...), got {:?}", other),
        };
        assert_eq!(args.len(), 1);
        match &args[0] {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "double_splat");
                assert_eq!(args.len(), 1);
                assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "hsh"));
            }
            other => panic!("expected BuiltinCall(double_splat, ...), got {:?}", other),
        }
    }

    #[test]
    fn mixed_call_args_with_splats_lower_in_order() {
        // `f(1, *arr, **hsh)` → three args:
        //   IntLit(1), BuiltinCall(splat, [VarRef(arr)]),
        //   BuiltinCall(double_splat, [VarRef(hsh)]).
        let m = lower("arr = [1]\nhsh = {a: 1}\nf(1, *arr, **hsh)\n");
        let b = main_body(&m);
        let args = match &b.value {
            Expr::DirectCall { fn_name, args, .. } if fn_name == "f" => args,
            other => panic!("expected tail DirectCall(f, ...), got {:?}", other),
        };
        assert_eq!(args.len(), 3, "expected 3 args, got {}", args.len());
        assert!(matches!(&args[0], Expr::IntLit { value: 1, .. }));
        assert!(matches!(
            &args[1],
            Expr::BuiltinCall { name, .. } if name == "splat"
        ));
        assert!(matches!(
            &args[2],
            Expr::BuiltinCall { name, .. } if name == "double_splat"
        ));
    }

    #[test]
    fn splat_param_lowers_to_bare_name_param() {
        // v0 lossy lowering: `*args` and `**kwargs` become regular
        // Params with the bare name.  Confirms the parse round-trip
        // doesn't crash and that the parameter name comes through
        // (without `*` / `**` prefix in `Param.name`).
        // Body uses a parens-wrapped expression to dodge the
        // bare-NAME-led def body ambiguity (see lessons.md).
        let m = lower("def f(a, *rest, **opts)\n  (a)\nend\n");
        let f = m.functions.iter().find(|f| f.name == "f")
            .expect("expected user-defined function `f`");
        let names: Vec<&str> = f.params.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["a", "rest", "opts"],
            "expected bare param names (v0 lossy splat lowering)"
        );
    }

    #[test]
    fn double_splat_only_arg_passes_sir_validator() {
        // Phase 22a (coverage) — end-to-end: a lone `**opts` call arg
        // lowers to a single `double_splat` BuiltinCall AND the resulting
        // module validates cleanly.  Earlier pins asserted the node shape
        // but never ran the validator on a double-splat-only call.
        // Use `puts` (a known builtin) so the validator's unknown-callee
        // check passes and we exercise only the double-splat arg path.
        let m = lower("opts = {a: 1}\nputs(**opts)\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected double-splat-only call: {:?}",
            result
        );
        let b = main_body(&m);
        // `puts` is an intrinsic — it lowers to BuiltinCall("puts", …),
        // not a DirectCall.  Its sole argument is the double-splat.
        let args = match &b.value {
            Expr::BuiltinCall { name, args, .. } if name == "puts" => args,
            other => panic!("expected BuiltinCall(puts, ...), got {:?}", other),
        };
        assert_eq!(args.len(), 1, "expected exactly 1 arg");
        assert!(matches!(
            &args[0],
            Expr::BuiltinCall { name, .. } if name == "double_splat"
        ));
    }

    #[test]
    fn double_splat_hash_literal_inner_lowers_and_validates() {
        // Phase 22a (coverage) — `f(**{a: 1})`: the double-splat operand is
        // an inline hash literal.  The inner expression must lower to a
        // `MapLit` wrapped by `double_splat`, and the module must validate.
        let m = lower("puts(**{a: 1})\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected double-splat of hash literal: {:?}",
            result
        );
        let b = main_body(&m);
        let args = match &b.value {
            Expr::BuiltinCall { name, args, .. } if name == "puts" => args,
            other => panic!("expected BuiltinCall(puts, ...), got {:?}", other),
        };
        assert_eq!(args.len(), 1);
        match &args[0] {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "double_splat");
                assert_eq!(args.len(), 1);
                assert!(
                    matches!(&args[0], Expr::MapLit { .. }),
                    "expected MapLit operand, got {:?}",
                    args[0]
                );
            }
            other => panic!("expected BuiltinCall(double_splat, ...), got {:?}", other),
        }
    }

    #[test]
    fn double_splat_after_leading_positional_lowers_in_order() {
        // Phase 22a (coverage) — `f(7, **opts)`: a positional arg followed
        // by a double-splat.  Distinct from the existing mixed pin (which
        // also threads a single `*arr` splat between them); here the splat
        // is absent so we lock the two-arg positional-then-double-splat
        // ordering specifically.
        let m = lower("opts = {a: 1}\nf(7, **opts)\n");
        let b = main_body(&m);
        let args = match &b.value {
            Expr::DirectCall { fn_name, args, .. } if fn_name == "f" => args,
            other => panic!("expected DirectCall(f, ...), got {:?}", other),
        };
        assert_eq!(args.len(), 2, "expected 2 args, got {}", args.len());
        assert!(matches!(&args[0], Expr::IntLit { value: 7, .. }));
        assert!(matches!(
            &args[1],
            Expr::BuiltinCall { name, .. } if name == "double_splat"
        ));
    }

    #[test]
    fn block_pass_call_arg_lowers_to_block_pass_builtin() {
        // Phase 22b — `f(&blk)` → DirectCall(f, [BuiltinCall("block_pass",
        // [VarRef(blk)])]).  Mirrors the splat / double_splat marker
        // pattern; the operand keeps its identity inside the envelope.
        let m = lower("blk = 1\nf(&blk)\n");
        let b = main_body(&m);
        let args = match &b.value {
            Expr::DirectCall { fn_name, args, .. } if fn_name == "f" => args,
            other => panic!("expected DirectCall(f, ...), got {:?}", other),
        };
        assert_eq!(args.len(), 1);
        match &args[0] {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "block_pass");
                assert_eq!(args.len(), 1);
                assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "blk"));
            }
            other => panic!("expected BuiltinCall(block_pass, ...), got {:?}", other),
        }
    }

    #[test]
    fn block_pass_call_arg_passes_sir_validator() {
        // Phase 22b — end-to-end: a block-pass call arg lowers cleanly
        // AND the module validates.  `puts` is a known intrinsic, so the
        // validator's unknown-callee check passes; an unknown `f` would
        // trip it.  `puts` lowers to BuiltinCall("puts", …).
        let m = lower("blk = 1\nputs(&blk)\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected block-pass call arg: {:?}",
            result
        );
        let b = main_body(&m);
        let args = match &b.value {
            Expr::BuiltinCall { name, args, .. } if name == "puts" => args,
            other => panic!("expected BuiltinCall(puts, ...), got {:?}", other),
        };
        assert_eq!(args.len(), 1);
        assert!(matches!(
            &args[0],
            Expr::BuiltinCall { name, .. } if name == "block_pass"
        ));
    }

    #[test]
    fn block_pass_after_positional_lowers_in_order() {
        // Phase 22b — `f(7, &blk)`: a positional arg followed by a
        // block-pass.  Locks the two-arg ordering (IntLit, then the
        // block_pass envelope) — block-pass is always trailing in Ruby.
        let m = lower("blk = 1\nf(7, &blk)\n");
        let b = main_body(&m);
        let args = match &b.value {
            Expr::DirectCall { fn_name, args, .. } if fn_name == "f" => args,
            other => panic!("expected DirectCall(f, ...), got {:?}", other),
        };
        assert_eq!(args.len(), 2, "expected 2 args, got {}", args.len());
        assert!(matches!(&args[0], Expr::IntLit { value: 7, .. }));
        assert!(matches!(
            &args[1],
            Expr::BuiltinCall { name, .. } if name == "block_pass"
        ));
    }

    #[test]
    fn forward_args_call_arg_lowers_to_forward_args_builtin() {
        // Phase 22c — `f(...)` → DirectCall(f, [BuiltinCall("forward_args",
        // [])]).  A nullary marker (no operand): the forwarding `...`
        // has no expression child, unlike splat/double_splat/block_pass.
        let m = lower("f(...)\n");
        let b = main_body(&m);
        let args = match &b.value {
            Expr::DirectCall { fn_name, args, .. } if fn_name == "f" => args,
            other => panic!("expected DirectCall(f, ...), got {:?}", other),
        };
        assert_eq!(args.len(), 1);
        match &args[0] {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "forward_args");
                assert!(args.is_empty(), "forward_args takes no operand");
            }
            other => panic!("expected BuiltinCall(forward_args, []), got {:?}", other),
        }
    }

    #[test]
    fn forward_all_def_and_call_passes_sir_validator() {
        // Phase 22c — the canonical forwarding round-trip:
        //   def m(...)
        //     puts(...)
        //   end
        // `def m(...)` lowers to a function with ZERO params (v0 lossy:
        // the bare `...` produces no Param node).  The inner `puts(...)`
        // forwards via the `forward_args` marker.  `puts` is a known
        // intrinsic, so the whole module validates.
        let m = lower("def m(...)\n  puts(...)\nend\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected forward-all def/call round-trip: {:?}",
            result
        );
        let f = m
            .functions
            .iter()
            .find(|f| f.name == "m")
            .expect("expected user-defined function `m`");
        assert_eq!(
            f.params.len(),
            0,
            "bare `...` forward param lowers to zero Params (v0 lossy)"
        );
    }

    #[test]
    fn beginless_range_arg_does_not_lower_to_forward_args() {
        // Phase 22c regression — `m(...5)` is a beginless exclusive-range
        // argument, NOT forward-all.  It must lower to the range builtin
        // (BuiltinCall("range", …)), confirming the direct-child-only
        // `...` detection doesn't swallow nested range operators.
        let m = lower("m(...5)\n");
        let b = main_body(&m);
        let args = match &b.value {
            Expr::DirectCall { fn_name, args, .. } if fn_name == "m" => args,
            other => panic!("expected DirectCall(m, ...), got {:?}", other),
        };
        assert_eq!(args.len(), 1);
        match &args[0] {
            Expr::BuiltinCall { name, .. } => {
                assert_eq!(
                    name, "range",
                    "beginless-range arg must be a `range` builtin, not `forward_args`"
                );
            }
            other => panic!("expected BuiltinCall(range, ...), got {:?}", other),
        }
    }

    #[test]
    fn splat_call_arg_module_passes_sir_validator() {
        // End-to-end smoke check: a module that uses splat call args
        // validates cleanly.
        let m = lower("arr = [1, 2, 3]\nputs(*arr)\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected splat call-arg output: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------
    // Phase 6x — instance / class / global variable refs
    // -----------------------------------------------------------------
    //
    // The lexer emits `@x`, `@@x`, `$x` as single Name-typed tokens
    // (sigil preserved in value).  The lowerer:
    //   - `$x` → VarRef { name: "$x", scope: Global }
    //   - `@x`, `@@x` → VarRef { name: "@x"/"@@x", scope: Local }
    //     (no dedicated SIR ivar/cvar scope in v0).
    //
    // LHS in assignments routes through the same LetBinding path —
    // first sighting → LetBinding, subsequent → Assign.

    #[test]
    fn global_var_ref_preserves_sigil_in_name() {
        // `$config = 1\nputs($config)` — the RHS read should yield a
        // VarRef whose name retains the `$` sigil.  v0 keeps `$x` on
        // Scope::Local pending a follow-up phase that auto-declares
        // Globals (the validator enforces declared globals).
        let m = lower("$config = 1\nputs($config)\n");
        let b = main_body(&m);
        let ref_expr: &Expr = match &b.value {
            Expr::BuiltinCall { name, args, .. } if name == "puts" => &args[0],
            _ => {
                b.stmts
                    .iter()
                    .find_map(|s| match s {
                        Stmt::ExprStmt {
                            expr: Expr::BuiltinCall { name, args, .. },
                            ..
                        } if name == "puts" => Some(&args[0]),
                        _ => None,
                    })
                    .expect("expected puts(...) call")
            }
        };
        match ref_expr {
            Expr::VarRef { name, scope, .. } => {
                assert_eq!(name, "$config", "gvar value should retain `$`");
                assert_eq!(*scope, Scope::Local, "v0 puts gvars on Scope::Local");
            }
            other => panic!("expected VarRef($config), got {:?}", other),
        }
    }

    #[test]
    fn instance_var_ref_lowers_with_local_scope_and_sigil_preserved() {
        // `@a = 1\n(@a)` — assignment then a tail expression-stmt
        // reading the ivar.
        let m = lower("@a = 1\n(@a)\n");
        let b = main_body(&m);
        // Either tail value or last stmt should reference @a.
        let ref_expr: &Expr = match &b.value {
            Expr::VarRef { name, .. } if name == "@a" => &b.value,
            _ => {
                // Maybe wrapped — check last stmt's expr.
                b.stmts
                    .iter()
                    .rev()
                    .find_map(|s| match s {
                        Stmt::ExprStmt { expr, .. } => Some(expr),
                        _ => None,
                    })
                    .expect("expected ExprStmt with @a ref")
            }
        };
        match ref_expr {
            Expr::VarRef { name, scope, .. } => {
                assert_eq!(name, "@a", "ivar value should retain leading `@`");
                // Phase 15a (FC): instance vars now lower to
                // `Scope::Instance` (was `Scope::Local` in the Phase 6x
                // v0 placeholder).
                assert_eq!(*scope, Scope::Instance, "Phase 15a: ivars use Scope::Instance");
            }
            other => panic!("expected VarRef(@a), got {:?}", other),
        }
    }

    #[test]
    fn class_var_ref_lowers_with_local_scope_and_double_at_preserved() {
        let m = lower("@@count = 0\n(@@count)\n");
        let b = main_body(&m);
        let ref_expr: &Expr = match &b.value {
            Expr::VarRef { name, .. } if name == "@@count" => &b.value,
            _ => b
                .stmts
                .iter()
                .rev()
                .find_map(|s| match s {
                    Stmt::ExprStmt { expr, .. } => Some(expr),
                    _ => None,
                })
                .expect("expected ExprStmt with @@count ref"),
        };
        match ref_expr {
            Expr::VarRef { name, scope, .. } => {
                assert_eq!(name, "@@count", "cvar value should retain `@@`");
                // Phase 15b (FC): class vars now lower to
                // `Scope::ClassVar` (was `Scope::Local` in the Phase 6x
                // v0 placeholder).
                assert_eq!(*scope, Scope::ClassVar, "Phase 15b: cvars use Scope::ClassVar");
            }
            other => panic!("expected VarRef(@@count), got {:?}", other),
        }
    }

    #[test]
    fn sigil_vars_module_passes_sir_validator() {
        // End-to-end smoke check across all three sigil types.
        let m = lower(concat!(
            "@a = 1\n",
            "@@b = 2\n",
            "$c = 3\n",
            "puts(@a)\n",
            "puts(@@b)\n",
            "puts($c)\n",
        ));
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected sigil-var output: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------
    // Phase 6w — arrow-lambda literal `->(params){body}`
    // -----------------------------------------------------------------
    //
    // Lowers to BuiltinCall("lambda", [MakeClosure { fn_name: "__block_<n>", captures: [] }]).
    // Body is hoisted to a top-level Function with the parens-params.

    #[test]
    fn arrow_lambda_no_params_lowers_to_lambda_builtin() {
        let m = lower("f = -> { 1 }\n");
        // Outer body should have LetBinding(f, BuiltinCall("lambda", [MakeClosure])).
        let b = main_body(&m);
        let value = match &b.stmts[0] {
            Stmt::LetBinding { name, value, .. } if name == "f" => value,
            other => panic!("expected LetBinding(f, ...), got {:?}", other),
        };
        match value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "lambda");
                assert_eq!(args.len(), 1);
                assert!(matches!(&args[0], Expr::MakeClosure { .. }));
            }
            other => panic!("expected BuiltinCall(lambda, [MakeClosure]), got {:?}", other),
        }
        // The hoisted Function should have zero params.
        let hoisted = m.functions.iter().find(|f| f.name.starts_with("__block_"));
        assert!(hoisted.is_some(), "expected a __block_<n> hoisted Function");
        assert_eq!(
            hoisted.unwrap().params.len(),
            0,
            "expected zero params on bare arrow"
        );
    }

    #[test]
    fn arrow_lambda_with_params_hoists_body_with_params() {
        let m = lower("f = ->(x, y) { x + y }\n");
        // Locate the hoisted block function.
        let hoisted = m
            .functions
            .iter()
            .find(|f| f.name.starts_with("__block_"))
            .expect("expected hoisted block function");
        let names: Vec<&str> = hoisted.params.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["x", "y"]);
    }

    #[test]
    fn lambda_keyword_form_lowers_via_method_with_block() {
        // `lambda { |x| x + 1 }` at top level uses method_with_block +
        // the `lambda` builtin (Phase 6w added "lambda"/"proc" to the
        // builtin map).  v0 limitation: `lambda { ... }` doesn't work
        // as an expression RHS (e.g. `f = lambda { ... }`) because
        // method_with_block isn't in `factor` — only at statement level.
        let m = lower("lambda { |x| x + 1 }\n");
        let b = main_body(&m);
        // method_with_block lowers to an ExprStmt(BuiltinCall(lambda, [MakeClosure]))
        // (not tail-promoted by `lower_program`).
        assert_eq!(b.stmts.len(), 1);
        let expr = match &b.stmts[0] {
            Stmt::ExprStmt { expr, .. } => expr,
            other => panic!("expected ExprStmt, got {:?}", other),
        };
        match expr {
            Expr::BuiltinCall { name, .. } => {
                assert_eq!(name, "lambda", "expected `lambda` builtin");
            }
            other => panic!("expected BuiltinCall(lambda, ...), got {:?}", other),
        }
    }

    #[test]
    fn arrow_lambda_module_passes_sir_validator() {
        let m = lower("f = ->(x) { x + 1 }\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected arrow-lambda output: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------
    // Phase 6v / 16a — `begin … rescue … ensure … end`
    // -----------------------------------------------------------------
    //
    // Phase 16a (FC): `begin/rescue/ensure/end` lowers to a first-class
    // `Stmt::TryCatch` (replacing the Phase 6v inline
    // `__rescue_marker__`/`__ensure_marker__` placeholder builtins).

    #[test]
    fn begin_without_rescue_lowers_body_inline() {
        // `begin x = 1 end` → TryCatch with just the body, no rescues/ensure.
        let m = lower("begin\n  x = 1\nend\n");
        let b = main_body(&m);
        assert_eq!(b.stmts.len(), 1);
        match &b.stmts[0] {
            Stmt::TryCatch { body, rescues, ensure_body, .. } => {
                assert_eq!(body.len(), 1);
                assert!(matches!(&body[0], Stmt::LetBinding { name, .. } if name == "x"));
                assert!(rescues.is_empty(), "no rescue clauses");
                assert!(ensure_body.is_none(), "no ensure clause");
            }
            other => panic!("expected Stmt::TryCatch, got {:?}", other),
        }
        assert!(m.manifest.contains(semantic_ir::Feature::Exceptions));
    }

    #[test]
    fn begin_with_rescue_lowers_to_rescue_clause() {
        let m = lower(
            "begin\n  x = 1\nrescue StandardError => e\n  y = 2\nend\n",
        );
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::TryCatch { body, rescues, ensure_body, .. } => {
                assert!(matches!(&body[0], Stmt::LetBinding { name, .. } if name == "x"));
                assert_eq!(rescues.len(), 1);
                let r = &rescues[0];
                assert_eq!(r.exception_types, vec!["StandardError".to_string()]);
                assert_eq!(r.binding.as_deref(), Some("e"));
                assert!(matches!(&r.body[0], Stmt::LetBinding { name, .. } if name == "y"));
                assert!(ensure_body.is_none());
            }
            other => panic!("expected Stmt::TryCatch, got {:?}", other),
        }
        assert!(m.manifest.contains(semantic_ir::Feature::Exceptions));
    }

    #[test]
    fn begin_with_ensure_lowers_to_ensure_body() {
        let m = lower("begin\n  x = 1\nensure\n  y = 2\nend\n");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::TryCatch { body, rescues, ensure_body, .. } => {
                assert!(matches!(&body[0], Stmt::LetBinding { name, .. } if name == "x"));
                assert!(rescues.is_empty());
                let ens = ensure_body.as_ref().expect("ensure body present");
                assert!(matches!(&ens[0], Stmt::LetBinding { name, .. } if name == "y"));
            }
            other => panic!("expected Stmt::TryCatch, got {:?}", other),
        }
    }

    #[test]
    fn begin_with_rescue_and_ensure_lowers_to_full_trycatch() {
        let m = lower(concat!(
            "begin\n",
            "  x = 1\n",
            "rescue StandardError => e\n",
            "  y = 2\n",
            "ensure\n",
            "  z = 3\n",
            "end\n",
        ));
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::TryCatch { body, rescues, ensure_body, .. } => {
                assert!(matches!(&body[0], Stmt::LetBinding { name, .. } if name == "x"));
                assert_eq!(rescues.len(), 1);
                assert!(matches!(&rescues[0].body[0], Stmt::LetBinding { name, .. } if name == "y"));
                let ens = ensure_body.as_ref().expect("ensure present");
                assert!(matches!(&ens[0], Stmt::LetBinding { name, .. } if name == "z"));
            }
            other => panic!("expected Stmt::TryCatch, got {:?}", other),
        }
    }

    #[test]
    fn begin_rescue_passes_sir_validator() {
        // E2E: a begin/rescue/ensure program validates end-to-end, and
        // the exception binding `e` is usable inside the rescue body.
        let m = lower(concat!(
            "begin\n",
            "  x = 1\n",
            "rescue StandardError => e\n",
            "  y = e\n",
            "ensure\n",
            "  z = 3\n",
            "end\n",
        ));
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected begin/rescue: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 16b (FC) — typed / multi-type / multi-clause rescue.
    // -----------------------------------------------------------------

    #[test]
    fn rescue_multi_type_lowers_all_exception_types() {
        // `rescue Foo, Bar => e` → one RescueClause whose exception_types
        // lists BOTH classes, in source order.
        let m = lower("begin\n  x = 1\nrescue Foo, Bar => e\n  y = 2\nend\n");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::TryCatch { rescues, .. } => {
                assert_eq!(rescues.len(), 1);
                assert_eq!(
                    rescues[0].exception_types,
                    vec!["Foo".to_string(), "Bar".to_string()],
                    "both exception classes preserved in order"
                );
                assert_eq!(rescues[0].binding.as_deref(), Some("e"));
            }
            other => panic!("expected Stmt::TryCatch, got {:?}", other),
        }
    }

    #[test]
    fn multiple_rescue_clauses_lower_to_separate_clauses() {
        // Two `rescue` clauses → two RescueClauses, each with its own
        // exception type and binding, in source order.
        let m = lower(concat!(
            "begin\n",
            "  x = 1\n",
            "rescue TypeError => e\n",
            "  y = 2\n",
            "rescue NameError => f\n",
            "  z = 3\n",
            "end\n",
        ));
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::TryCatch { rescues, .. } => {
                assert_eq!(rescues.len(), 2, "two rescue clauses");
                assert_eq!(rescues[0].exception_types, vec!["TypeError".to_string()]);
                assert_eq!(rescues[0].binding.as_deref(), Some("e"));
                assert_eq!(rescues[1].exception_types, vec!["NameError".to_string()]);
                assert_eq!(rescues[1].binding.as_deref(), Some("f"));
            }
            other => panic!("expected Stmt::TryCatch, got {:?}", other),
        }
    }

    #[test]
    fn multi_clause_rescue_passes_sir_validator() {
        // E2E: a begin with two typed rescue clauses (each binding used in
        // its own body) validates, confirming per-clause binding scope.
        let m = lower(concat!(
            "begin\n",
            "  x = 1\n",
            "rescue TypeError => e\n",
            "  y = e\n",
            "rescue NameError => f\n",
            "  z = f\n",
            "end\n",
        ));
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected multi-clause rescue: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 16c (FC) — ensure clause coverage / hardening.
    // -----------------------------------------------------------------

    #[test]
    fn ensure_only_lowers_with_no_rescues() {
        // `begin … ensure … end` (no rescue) → TryCatch with an empty
        // `rescues` and a populated `ensure_body`; requests Exceptions.
        let m = lower("begin\n  x = 1\nensure\n  y = 2\nend\n");
        match &main_body(&m).stmts[0] {
            Stmt::TryCatch { rescues, ensure_body, .. } => {
                assert!(rescues.is_empty(), "ensure-only has no rescue clauses");
                let ens = ensure_body.as_ref().expect("ensure body present");
                assert!(matches!(&ens[0], Stmt::LetBinding { name, .. } if name == "y"));
            }
            other => panic!("expected Stmt::TryCatch, got {:?}", other),
        }
        assert!(m.manifest.contains(semantic_ir::Feature::Exceptions));
    }

    #[test]
    fn ensure_body_preserves_statement_order() {
        // The ensure body keeps its statements in source order.
        let m = lower("begin\n  x = 1\nensure\n  a = 1\n  b = 2\nend\n");
        match &main_body(&m).stmts[0] {
            Stmt::TryCatch { ensure_body, .. } => {
                let ens = ensure_body.as_ref().expect("ensure body present");
                assert_eq!(ens.len(), 2, "both ensure stmts preserved; got {:?}", ens);
                assert!(matches!(&ens[0], Stmt::LetBinding { name, .. } if name == "a"));
                assert!(matches!(&ens[1], Stmt::LetBinding { name, .. } if name == "b"));
            }
            other => panic!("expected Stmt::TryCatch, got {:?}", other),
        }
    }

    #[test]
    fn ensure_only_passes_sir_validator() {
        // E2E: an ensure-only begin (no rescue) validates end-to-end.
        let m = lower("begin\n  x = 1\nensure\n  y = 2\nend\n");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected ensure-only begin: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 16d (FC) — `raise` / `raise Foo` / `raise Foo, "msg"`.
    // -----------------------------------------------------------------

    /// Extract the head `BuiltinCall` from `main`'s trailing block value
    /// (a lone trailing expression lowers to the block `value`, not a
    /// statement).
    fn raise_builtin(m: &semantic_ir::Module) -> (&str, &[Expr], semantic_ir::EffectSet) {
        match &main_body(m).value {
            Expr::BuiltinCall { name, args, effects, .. } => {
                (name.as_str(), args.as_slice(), *effects)
            }
            other => panic!("expected BuiltinCall(raise), got {:?}", other),
        }
    }

    #[test]
    fn raise_bare_lowers_to_builtin_with_throw_divergent() {
        // A bare `raise` (re-raise) lowers to `BuiltinCall("raise", [])`
        // tagged MayThrow + Divergent, and requests `Feature::Exceptions`.
        let m = lower("raise\n");
        let (name, args, effects) = raise_builtin(&m);
        assert_eq!(name, "raise");
        assert!(args.is_empty(), "bare raise has no args; got {:?}", args);
        assert!(effects.contains(Effect::MayThrow), "raise is MayThrow");
        assert!(effects.contains(Effect::Divergent), "raise is Divergent");
        assert!(m.manifest.contains(semantic_ir::Feature::Exceptions));
    }

    #[test]
    fn raise_with_class_lowers_to_builtin() {
        // `raise Foo` → `BuiltinCall("raise", [VarRef(Foo, Const)])`.
        let m = lower("raise Foo\n");
        let (name, args, _effects) = raise_builtin(&m);
        assert_eq!(name, "raise");
        assert_eq!(args.len(), 1, "one exception-class arg; got {:?}", args);
        assert!(matches!(&args[0], Expr::VarRef { name, scope, .. } if name == "Foo" && *scope == Scope::Const));
        assert!(m.manifest.contains(semantic_ir::Feature::Exceptions));
    }

    #[test]
    fn raise_with_class_and_message_lowers_to_builtin() {
        // `raise Foo, "boom"` → `BuiltinCall("raise", [Foo, StrLit])`.
        let m = lower("raise Foo, \"boom\"\n");
        let (name, args, _effects) = raise_builtin(&m);
        assert_eq!(name, "raise");
        assert_eq!(args.len(), 2, "class + message args; got {:?}", args);
        assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "Foo"));
        assert!(matches!(&args[1], Expr::StrLit { value, .. } if value == "boom"));
    }

    #[test]
    fn raise_passes_sir_validator() {
        // E2E: `raise Foo, "boom"` validates end-to-end.
        let m = lower("raise Foo, \"boom\"\n");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected raise: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 16e (FC) — method-level rescue/ensure (def … rescue … end).
    // -----------------------------------------------------------------

    /// Find a hoisted top-level function by name.
    fn func<'a>(m: &'a semantic_ir::Module, name: &str) -> &'a semantic_ir::Function {
        m.functions
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("expected hoisted function `{}`", name))
    }

    #[test]
    fn def_with_method_level_rescue_wraps_body_in_trycatch() {
        // `def f; x = 1; rescue Foo => e; y = e; end` → the method body is
        // a single `Stmt::TryCatch` (no explicit begin); the function
        // value is nil.
        let m = lower("def f\n  x = 1\nrescue Foo => e\n  y = e\nend\n");
        let f = func(&m, "f");
        assert_eq!(f.body.stmts.len(), 1, "method body is one TryCatch; got {:?}", f.body.stmts);
        match &f.body.stmts[0] {
            Stmt::TryCatch { body, rescues, ensure_body, .. } => {
                assert!(!body.is_empty(), "try body has the method statements");
                assert_eq!(rescues.len(), 1);
                assert_eq!(rescues[0].exception_types, vec!["Foo".to_string()]);
                assert_eq!(rescues[0].binding.as_deref(), Some("e"));
                assert!(ensure_body.is_none());
            }
            other => panic!("expected Stmt::TryCatch, got {:?}", other),
        }
        assert!(matches!(f.body.value, Expr::NilLit { .. }), "method value is nil");
        assert!(m.manifest.contains(semantic_ir::Feature::Exceptions));
    }

    #[test]
    fn def_with_method_level_ensure_wraps_body_in_trycatch() {
        // `def f; x = 1; ensure; y = 2; end` → TryCatch with no rescues
        // and a populated ensure body.
        let m = lower("def f\n  x = 1\nensure\n  y = 2\nend\n");
        let f = func(&m, "f");
        match &f.body.stmts[0] {
            Stmt::TryCatch { rescues, ensure_body, .. } => {
                assert!(rescues.is_empty(), "ensure-only: no rescues");
                assert!(ensure_body.is_some(), "ensure body present");
            }
            other => panic!("expected Stmt::TryCatch, got {:?}", other),
        }
    }

    #[test]
    fn def_without_rescue_keeps_plain_body() {
        // Regression: a normal `def` (no rescue/ensure) is unchanged —
        // no TryCatch wraps the body.
        let m = lower("def f(n)\n  x = n + 1\nend\n");
        let f = func(&m, "f");
        assert!(
            !f.body.stmts.iter().any(|s| matches!(s, Stmt::TryCatch { .. })),
            "plain def must not produce a TryCatch"
        );
        assert!(
            f.body.stmts.iter().any(|s| matches!(s, Stmt::LetBinding { name, .. } if name == "x")),
            "plain def keeps its body statements; got {:?}", f.body.stmts
        );
    }

    #[test]
    fn method_level_rescue_passes_sir_validator() {
        // E2E: a method-level rescue/ensure def (with the binding used in
        // the rescue body) validates end-to-end.
        let m = lower("def f\n  x = 1\nrescue StandardError => e\n  y = e\nensure\n  z = 1\nend\n");
        let result = semantic_ir::validate(&m);
        assert!(result.is_ok(), "validator rejected method-level rescue: {:?}", result);
    }

    // -----------------------------------------------------------------
    // Phase 6u — `case … when … else … end`
    // -----------------------------------------------------------------
    //
    // case x
    // when v1, v2 then a
    // when v3     then b
    // else c
    // end
    //
    // lowers to a chained Expr::If with `==`-comparisons (joined by
    // `or` for multi-value whens) and the else block as the chain's
    // final tail.

    #[test]
    fn case_single_when_lowers_to_if_with_case_eq() {
        // M5: `case x; when 1; y = 1; end` — a literal `when` lowers to
        // `case_eq(1, x)` (Ruby case-equality `1 === x`, whose floor is `==`).
        let m = lower("x = 1\ncase x\nwhen 1\n  y = 1\nend\n");
        let b = main_body(&m);
        // Second stmt should be an ExprStmt(If) — the case lowering.
        let if_expr = match &b.stmts[1] {
            Stmt::ExprStmt { expr, .. } => expr,
            other => panic!("expected ExprStmt(If) from case, got {:?}", other),
        };
        let cond = match if_expr {
            Expr::If { cond, .. } => cond,
            other => panic!("expected If from case, got {:?}", other),
        };
        // The condition is BuiltinCall("case_eq", [IntLit(1), VarRef(x)]).
        assert!(
            matches!(cond.as_ref(), Expr::BuiltinCall { name, .. } if name == "case_eq"),
            "expected `case_eq` builtin in when cond, got {:?}",
            cond
        );
    }

    #[test]
    fn case_when_class_lowers_to_is_a_dispatch() {
        // M5: `when Integer` (a bare constant) is a class match → lowers to a
        // `__method__` `is_a?` dispatch, NOT `case_eq`/`==`.
        let m = lower("x = 1\ncase x\nwhen Integer\n  y = 1\nend\n");
        let b = main_body(&m);
        let if_expr = match &b.stmts[1] {
            Stmt::ExprStmt { expr, .. } => expr,
            other => panic!("expected ExprStmt(If), got {:?}", other),
        };
        let cond = match if_expr {
            Expr::If { cond, .. } => cond,
            other => panic!("expected If, got {:?}", other),
        };
        match cond.as_ref() {
            Expr::BuiltinCall { name, args, .. } if name == "__method__" => {
                assert!(
                    matches!(&args[1], Expr::StrLit { value, .. } if value == "is_a?"),
                    "expected is_a? dispatch, got {:?}",
                    args
                );
            }
            other => panic!("expected __method__ is_a? dispatch for `when Integer`, got {:?}", other),
        }
    }

    #[test]
    fn case_with_multi_value_when_lowers_to_or_chain() {
        // M5: `when 1, 2, 3` → `((ceq(1,x) || ceq(2,x)) || ceq(3,x))`.
        let m = lower("x = 1\ncase x\nwhen 1, 2, 3\n  y = 1\nend\n");
        let b = main_body(&m);
        let if_expr = match &b.stmts[1] {
            Stmt::ExprStmt { expr, .. } => expr,
            other => panic!("expected ExprStmt(If), got {:?}", other),
        };
        let cond = match if_expr {
            Expr::If { cond, .. } => cond,
            other => panic!("expected If, got {:?}", other),
        };
        // Outermost should be `or`.  Count `case_eq` and `or` nodes.
        fn count_builtin(e: &Expr, target: &str) -> usize {
            match e {
                Expr::BuiltinCall { name, args, .. } => {
                    let mine = if name == target { 1 } else { 0 };
                    mine + args.iter().map(|a| count_builtin(a, target)).sum::<usize>()
                }
                _ => 0,
            }
        }
        assert_eq!(
            count_builtin(cond.as_ref(), "case_eq"),
            3,
            "expected three `case_eq` comparisons for three when values"
        );
        assert_eq!(
            count_builtin(cond.as_ref(), "or"),
            2,
            "expected two `or` joins for three values (left-fold)"
        );
    }

    #[test]
    fn case_with_else_terminates_chain() {
        // `case x; when 1; a=1; else a=2; end` — else body sits in the
        // chain's outermost else-branch.
        let m = lower("x = 1\ncase x\nwhen 1\n  a = 1\nelse\n  a = 2\nend\n");
        let b = main_body(&m);
        let if_expr = match &b.stmts[1] {
            Stmt::ExprStmt { expr, .. } => expr,
            other => panic!("expected ExprStmt(If), got {:?}", other),
        };
        let else_branch = match if_expr {
            Expr::If { else_branch, .. } => else_branch,
            other => panic!("expected If, got {:?}", other),
        };
        // Else branch's stmts should carry the `a = 2` assignment.
        // (a was first defined in the if's then-branch as a LetBinding;
        // the else-branch sees it as already declared via the shared
        // outer scope snapshot.)
        assert!(
            !else_branch.stmts.is_empty(),
            "expected else block to carry the `a = 2` stmt, got empty"
        );
    }

    #[test]
    fn case_without_else_uses_nil_tail() {
        // No else clause → else-branch is `Block{stmts: [], value: NilLit}`.
        let m = lower("x = 1\ncase x\nwhen 1\n  a = 1\nend\n");
        let b = main_body(&m);
        let if_expr = match &b.stmts[1] {
            Stmt::ExprStmt { expr, .. } => expr,
            other => panic!("expected ExprStmt(If), got {:?}", other),
        };
        let else_branch = match if_expr {
            Expr::If { else_branch, .. } => else_branch,
            other => panic!("expected If, got {:?}", other),
        };
        assert!(
            else_branch.stmts.is_empty(),
            "expected empty else block when no else clause"
        );
        assert!(
            matches!(&else_branch.value, Expr::NilLit { .. }),
            "expected NilLit tail when no else clause, got {:?}",
            else_branch.value
        );
    }

    // -----------------------------------------------------------------
    // Phase 6t — `yield` keyword
    // -----------------------------------------------------------------
    //
    // `yield ...` → ExprStmt(BuiltinCall("yield", lowered_args, PURE)).

    #[test]
    fn bare_yield_lowers_to_yield_builtin_no_args() {
        let m = lower("yield\n");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::ExprStmt { expr: Expr::BuiltinCall { name, args, .. }, .. } => {
                assert_eq!(name, "yield");
                assert_eq!(args.len(), 0, "bare yield should have 0 args");
            }
            other => panic!("expected ExprStmt(BuiltinCall(yield, [])), got {:?}", other),
        }
    }

    #[test]
    fn yield_with_one_arg_lowers_to_builtin_with_one_arg() {
        let m = lower("yield 42\n");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::ExprStmt { expr: Expr::BuiltinCall { name, args, .. }, .. } => {
                assert_eq!(name, "yield");
                assert_eq!(args.len(), 1);
                assert!(matches!(&args[0], Expr::IntLit { value: 42, .. }));
            }
            other => panic!("expected yield builtin call, got {:?}", other),
        }
    }

    #[test]
    fn yield_with_paren_args_lowers_to_two_arg_builtin() {
        let m = lower("yield(1, 2)\n");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::ExprStmt { expr: Expr::BuiltinCall { name, args, .. }, .. } => {
                assert_eq!(name, "yield");
                assert_eq!(args.len(), 2);
                assert!(matches!(&args[0], Expr::IntLit { value: 1, .. }));
                assert!(matches!(&args[1], Expr::IntLit { value: 2, .. }));
            }
            other => panic!("expected yield builtin call with 2 args, got {:?}", other),
        }
    }

    #[test]
    fn yield_with_splat_arg_lowers_with_splat_envelope() {
        // `yield(*arr)` — the splat reuses Phase 6s's BuiltinCall("splat", …)
        // envelope, and yield itself sees that envelope as a single arg.
        let m = lower("arr = [1]\nyield(*arr)\n");
        let b = main_body(&m);
        // Second stmt is the yield.
        match &b.stmts[1] {
            Stmt::ExprStmt { expr: Expr::BuiltinCall { name, args, .. }, .. } => {
                assert_eq!(name, "yield");
                assert_eq!(args.len(), 1);
                match &args[0] {
                    Expr::BuiltinCall { name, args, .. } => {
                        assert_eq!(name, "splat");
                        assert_eq!(args.len(), 1);
                        assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "arr"));
                    }
                    other => panic!("expected splat envelope, got {:?}", other),
                }
            }
            other => panic!("expected yield builtin call, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Phase Q9e — explicit block-param ABI, part 1: a method that
    // `yield`s gains a trailing reserved `__sir_block__` parameter and
    // every in-body `yield` is rewritten to an `IndirectCall` through it.
    // -----------------------------------------------------------------------

    #[test]
    fn def_with_yield_threads_block_param_and_rewrites_yield() {
        let m = lower("def t\n  yield 5\nend\n");
        let t = func(&m, "t");
        // Trailing reserved block parameter appended.
        assert_eq!(
            t.params.last().map(|p| p.name.as_str()),
            Some("__sir_block__"),
            "yielding method must gain a trailing __sir_block__ param"
        );
        // The in-body `yield 5` is now an IndirectCall through that
        // param — NOT a BuiltinCall("yield").
        match &t.body.stmts[0] {
            Stmt::ExprStmt { expr: Expr::IndirectCall { target, args, .. }, .. } => {
                match target.as_ref() {
                    Expr::VarRef { name, scope, .. } => {
                        assert_eq!(name, "__sir_block__");
                        assert_eq!(*scope, Scope::Param);
                    }
                    other => panic!("expected VarRef(__sir_block__, param), got {:?}", other),
                }
                assert_eq!(args.len(), 1);
                assert!(matches!(&args[0], Expr::IntLit { value: 5, .. }));
            }
            other => panic!("expected ExprStmt(IndirectCall(...)), got {:?}", other),
        }
        // And no BuiltinCall("yield") survives anywhere in the body.
        assert!(
            !matches!(
                &t.body.stmts[0],
                Stmt::ExprStmt { expr: Expr::BuiltinCall { name, .. }, .. } if name == "yield"
            ),
            "no BuiltinCall(\"yield\") may remain after threading"
        );
        // The rewritten module still validates (param resolves, the
        // Closures/DynamicTyping features are declared).
        assert!(semantic_ir::validate(&m).is_ok(), "threaded module validates: {:?}", semantic_ir::validate(&m));
    }

    #[test]
    fn def_without_yield_is_unchanged() {
        // A non-yielding method keeps its exact arity — no spurious
        // __sir_block__ param is appended.
        let m = lower("def t\n  5\nend\n");
        let t = func(&m, "t");
        assert!(
            t.params.is_empty(),
            "non-yielding method must not gain a block param, got {:?}",
            t.params
        );
        assert!(matches!(t.body.value, Expr::IntLit { value: 5, .. }));
    }

    #[test]
    fn yield_inside_if_in_def_is_rewritten() {
        // The rewrite descends into control flow: a `yield` guarded by an
        // `if` inside the method body is still threaded.
        let m = lower("def t(x)\n  if x\n    yield 1\n  end\nend\n");
        let t = func(&m, "t");
        // Params: the original `x`, then the trailing block param.
        assert_eq!(t.params.len(), 2);
        assert_eq!(t.params[0].name, "x");
        assert_eq!(t.params[1].name, "__sir_block__");
        // The `yield 1` lives in the `if`'s then-branch and is now an
        // IndirectCall.  Locate the If and inspect its then-branch.
        let if_expr = t
            .body
            .stmts
            .iter()
            .find_map(|s| match s {
                Stmt::ExprStmt { expr: e @ Expr::If { .. }, .. } => Some(e),
                _ => None,
            })
            .or(match &t.body.value {
                e @ Expr::If { .. } => Some(e),
                _ => None,
            })
            .expect("if-expression present in method body");
        if let Expr::If { then_branch, .. } = if_expr {
            let found_indirect = then_branch.stmts.iter().any(|s| {
                matches!(s, Stmt::ExprStmt { expr: Expr::IndirectCall { .. }, .. })
            }) || matches!(then_branch.value, Expr::IndirectCall { .. });
            assert!(found_indirect, "yield inside if must become IndirectCall");
        }
        assert!(semantic_ir::validate(&m).is_ok(), "threaded module validates: {:?}", semantic_ir::validate(&m));
    }

    // (A `yield` lexically inside a block literal — the documented v0
    // cut-line where the rewrite must NOT descend into a `MakeClosure` —
    // cannot be expressed in the current grammar: `yield` is a
    // statement, not an expression, so it never appears inside a `{ … }`
    // brace block.  The guard in `rewrite_yields_in_expr` is therefore
    // defensive; no source-level test can exercise it today.)

    // -----------------------------------------------------------------------
    // Phase Q9f — explicit block-param ABI, part 2: call-site
    // normalization.  Calls to a yielding method get the matching block
    // argument threaded into the trailing slot so arity matches the def.
    // -----------------------------------------------------------------------

    /// Recursively locate the first `DirectCall` to `name` reachable from
    /// a block's statements / value (one finder shared by the Q9f tests).
    fn find_direct_call<'a>(b: &'a semantic_ir::Block, name: &str) -> Option<&'a Expr> {
        fn in_expr<'a>(e: &'a Expr, name: &str) -> Option<&'a Expr> {
            match e {
                Expr::DirectCall { fn_name, args, .. } => {
                    if fn_name == name {
                        return Some(e);
                    }
                    args.iter().find_map(|a| in_expr(a, name))
                }
                Expr::BuiltinCall { args, .. } | Expr::Intrinsic { args, .. } => {
                    args.iter().find_map(|a| in_expr(a, name))
                }
                Expr::IndirectCall { target, args, .. } => in_expr(target, name)
                    .or_else(|| args.iter().find_map(|a| in_expr(a, name))),
                Expr::If { cond, then_branch, else_branch, .. } => in_expr(cond, name)
                    .or_else(|| in_block(then_branch, name))
                    .or_else(|| in_block(else_branch, name)),
                Expr::Block(b) => in_block(b, name),
                Expr::SeqLit { items, .. } => items.iter().find_map(|i| in_expr(i, name)),
                _ => None,
            }
        }
        fn in_stmt<'a>(s: &'a Stmt, name: &str) -> Option<&'a Expr> {
            match s {
                Stmt::LetBinding { value, .. }
                | Stmt::LetStarBinding { value, .. }
                | Stmt::Assign { value, .. }
                | Stmt::ExprStmt { expr: value, .. } => in_expr(value, name),
                _ => None,
            }
        }
        fn in_block<'a>(b: &'a semantic_ir::Block, name: &str) -> Option<&'a Expr> {
            b.stmts
                .iter()
                .find_map(|s| in_stmt(s, name))
                .or_else(|| in_expr(&b.value, name))
        }
        in_block(b, name)
    }

    #[test]
    fn call_to_yielding_method_with_block_keeps_makeclosure() {
        let m = lower("def t\n  yield 5\nend\nt { |x| puts x }\n");
        let call = find_direct_call(main_body(&m), "t").expect("DirectCall to t");
        if let Expr::DirectCall { args, .. } = call {
            assert_eq!(args.len(), 1, "block arg occupies the single trailing slot");
            assert!(
                matches!(args.last(), Some(Expr::MakeClosure { .. })),
                "explicit block stays a MakeClosure, got {:?}",
                args.last()
            );
        }
        // Arity matches the threaded def (1 param: __sir_block__).
        let t = func(&m, "t");
        assert_eq!(t.params.len(), 1);
        assert!(semantic_ir::validate(&m).is_ok(), "validates: {:?}", semantic_ir::validate(&m));
    }

    #[test]
    fn call_to_yielding_method_without_block_appends_nil() {
        // `t(1)` passes no block, so a trailing nil is appended to match
        // the def's threaded arity (params: `a`, `__sir_block__`).
        let m = lower("def t(a)\n  yield a\nend\nt(1)\n");
        let call = find_direct_call(main_body(&m), "t").expect("DirectCall to t");
        if let Expr::DirectCall { args, .. } = call {
            assert_eq!(args.len(), 2, "positional arg + appended nil block slot");
            assert!(matches!(&args[0], Expr::IntLit { value: 1, .. }));
            assert!(
                matches!(args.last(), Some(Expr::NilLit { .. })),
                "no-block call binds nil, got {:?}",
                args.last()
            );
        }
        // Arity matches the threaded def (params: a, __sir_block__).
        assert_eq!(func(&m, "t").params.len(), 2);
        assert!(semantic_ir::validate(&m).is_ok(), "validates: {:?}", semantic_ir::validate(&m));
    }

    #[test]
    fn block_pass_to_yielding_method_unwraps_to_inner() {
        // `foo(&p)` — the block_pass envelope is unwrapped to the proc
        // value `p` itself in the trailing slot.
        let m = lower("def foo\n  yield 5\nend\np = 1\nfoo(&p)\n");
        let call = find_direct_call(main_body(&m), "foo").expect("DirectCall to foo");
        if let Expr::DirectCall { args, .. } = call {
            assert_eq!(args.len(), 1);
            match args.last() {
                Some(Expr::VarRef { name, .. }) => assert_eq!(name, "p"),
                other => panic!("expected unwrapped VarRef(p), got {:?}", other),
            }
            // The block_pass envelope must NOT survive.
            assert!(
                !matches!(args.last(), Some(Expr::BuiltinCall { name, .. }) if name == "block_pass"),
                "block_pass envelope must be unwrapped"
            );
        }
        assert!(semantic_ir::validate(&m).is_ok(), "validates: {:?}", semantic_ir::validate(&m));
    }

    #[test]
    fn call_to_non_block_method_is_unchanged() {
        // `g` does not yield, so its call site keeps its exact args — no
        // spurious trailing nil.
        let m = lower("def g(x)\n  x + 1\nend\ng(1)\n");
        let call = find_direct_call(main_body(&m), "g").expect("DirectCall to g");
        if let Expr::DirectCall { args, .. } = call {
            assert_eq!(args.len(), 1, "non-yielding call keeps its arity");
            assert!(matches!(args.last(), Some(Expr::IntLit { value: 1, .. })));
        }
        assert!(semantic_ir::validate(&m).is_ok(), "validates: {:?}", semantic_ir::validate(&m));
    }

    #[test]
    fn call_before_def_is_threaded() {
        // The pass runs after the whole program is lowered, so a call
        // appearing before the `def` is still threaded (order-independent).
        let m = lower("t { |x| puts x }\ndef t\n  yield 5\nend\n");
        let call = find_direct_call(main_body(&m), "t").expect("DirectCall to t");
        if let Expr::DirectCall { args, .. } = call {
            assert_eq!(args.len(), 1);
            assert!(matches!(args.last(), Some(Expr::MakeClosure { .. })));
        }
        assert!(semantic_ir::validate(&m).is_ok(), "validates: {:?}", semantic_ir::validate(&m));
    }

    // -----------------------------------------------------------------------
    // Phase Q10c — parenless/argless call to a yielding method → DirectCall
    // -----------------------------------------------------------------------

    #[test]
    fn parenless_call_to_yielding_method_becomes_direct_call_with_nil_block() {
        // Bare `t` (no parens/args) referencing a yielding method is a
        // zero-arg call: it must become DirectCall t [NilLit], not a VarRef.
        let m = lower("def t\n  yield 5\nend\nt\n");
        let call = find_direct_call(main_body(&m), "t")
            .expect("bare `t` should lower to a DirectCall");
        if let Expr::DirectCall { args, .. } = call {
            assert_eq!(args.len(), 1, "nil block slot threaded");
            assert!(matches!(args.last(), Some(Expr::NilLit { .. })));
        }
        // And no bare VarRef named `t` survives in main.
        assert!(
            !body_mentions_varref(main_body(&m), "t"),
            "parenless call must not remain a VarRef"
        );
        assert!(semantic_ir::validate(&m).is_ok(), "validates: {:?}", semantic_ir::validate(&m));
    }

    #[test]
    fn local_shadowing_a_method_name_stays_a_varref() {
        // `t = 1` binds a local; the later bare `t` is that variable, NOT a
        // call — it must remain a VarRef even though a method `t` exists.
        let m = lower("def t\n  yield 5\nend\nt = 1\nt\n");
        assert!(
            find_direct_call(main_body(&m), "t").is_none(),
            "a shadowed name must not be rewritten into a DirectCall"
        );
        assert!(
            matches!(&main_body(&m).value, Expr::VarRef { name, .. } if name == "t"),
            "the tail `t` stays a local VarRef, got {:?}",
            main_body(&m).value
        );
        assert!(semantic_ir::validate(&m).is_ok(), "validates: {:?}", semantic_ir::validate(&m));
    }

    #[test]
    fn parenless_reference_to_non_block_method_is_left_alone() {
        // `g` does not take a block, so a bare `g` is not rewritten (it
        // stays whatever the lowerer produced — a VarRef — and no spurious
        // DirectCall is synthesized).
        let m = lower("def g(x)\n  x + 1\nend\ng\n");
        assert!(
            find_direct_call(main_body(&m), "g").is_none(),
            "non-block method's bare ref must not be threaded"
        );
    }

    // -----------------------------------------------------------------------
    // RB1 — trailing block on a receiver/dotted method call
    // (`recv.each { … }` / `recv.each do … end`).  The block is hoisted to
    // a top-level Function and attached as the `__method__` envelope's
    // trailing MakeClosure argument (previously the block was dropped /
    // the whole construct failed to parse).
    // -----------------------------------------------------------------------

    /// The hoisted block function name on a `__method__` call's trailing
    /// `MakeClosure` argument, if present.
    fn method_block_closure(e: &Expr) -> Option<&str> {
        if let Expr::BuiltinCall { name, args, .. } = e {
            if name == "__method__" {
                if let Some(Expr::MakeClosure { fn_name, .. }) = args.last() {
                    return Some(fn_name.as_str());
                }
            }
        }
        None
    }

    #[test]
    fn receiver_method_brace_block_is_hoisted_and_attached() {
        let m = lower("[1, 2].each { |x| puts x }\n");
        let v = &main_body(&m).value;
        let blk = method_block_closure(v)
            .expect("each-with-block must attach a hoisted MakeClosure");
        // The hoisted function exists and carries the block body.
        let bf = m.functions.iter().find(|f| f.name == blk).expect("hoisted block fn");
        assert!(
            !bf.body.stmts.is_empty() || !matches!(bf.body.value, Expr::NilLit { .. }),
            "hoisted block must carry its body"
        );
        assert!(semantic_ir::validate(&m).is_ok(), "validates: {:?}", semantic_ir::validate(&m));
    }

    #[test]
    fn receiver_method_do_end_block_is_hoisted_and_attached() {
        let m = lower("[1, 2].each do |x|\n  puts x\nend\n");
        let v = &main_body(&m).value;
        assert!(
            method_block_closure(v).is_some(),
            "do/end block on a receiver call must attach a MakeClosure, got {:?}",
            v
        );
        assert!(semantic_ir::validate(&m).is_ok(), "validates: {:?}", semantic_ir::validate(&m));
    }

    #[test]
    fn receiver_method_without_block_has_no_closure() {
        // A plain dotted call keeps its `__method__` shape with no trailing
        // closure (the block arm must not fire spuriously).
        let m = lower("[1, 2].length\n");
        assert!(
            method_block_closure(&main_body(&m).value).is_none(),
            "no block ⇒ no MakeClosure on the __method__ envelope"
        );
        assert!(semantic_ir::validate(&m).is_ok());
    }

    // -----------------------------------------------------------------------
    // RB2 — `yield` inside a hoisted block captures the enclosing method's
    // `__sir_block__`.  The enclosing `def` gains the trailing block param,
    // the hoisted block fn declares a `__sir_block__` capture, and the
    // in-block `yield` becomes an `IndirectCall` through that capture.
    // -----------------------------------------------------------------------

    #[test]
    fn yield_inside_block_captures_enclosing_block() {
        // `outer` passes a block to `helper`; that block `yield`s, which in
        // Ruby invokes `outer`'s own block. So `outer` must take a block,
        // and the hoisted block must capture it.
        let m = lower("def helper(x)\n  x + 1\nend\ndef outer\n  helper(2) { yield 99 }\nend\n");
        let outer = func(&m, "outer");
        assert_eq!(
            outer.params.last().map(|p| p.name.as_str()),
            Some("__sir_block__"),
            "outer must gain the trailing block param, got {:?}",
            outer.params
        );
        // The hoisted block fn declares the capture and yields through it.
        let blk = m
            .functions
            .iter()
            .find(|f| f.name.starts_with("__block"))
            .expect("hoisted block fn present");
        assert!(
            blk.captures.iter().any(|c| c.name == "__sir_block__"),
            "block fn must capture __sir_block__, got {:?}",
            blk.captures
        );
        let yields_via_capture = blk.body.stmts.iter().any(|s| matches!(s,
            Stmt::ExprStmt { expr: Expr::IndirectCall { target, .. }, .. }
                if matches!(target.as_ref(),
                    Expr::VarRef { name, scope, .. }
                        if name == "__sir_block__" && *scope == Scope::Capture)))
            || matches!(&blk.body.value, Expr::IndirectCall { target, .. }
                if matches!(target.as_ref(),
                    Expr::VarRef { name, scope, .. }
                        if name == "__sir_block__" && *scope == Scope::Capture));
        assert!(yields_via_capture, "in-block yield must call through the captured block");
        assert!(semantic_ir::validate(&m).is_ok(), "validates: {:?}", semantic_ir::validate(&m));
    }

    #[test]
    fn yield_inside_receiver_block_captures_enclosing_block() {
        // Same, via the RB1 receiver-block path (`recv.each { … }`).
        let m = lower("def outer\n  [1, 2].each { yield 7 }\nend\n");
        let outer = func(&m, "outer");
        assert_eq!(outer.params.last().map(|p| p.name.as_str()), Some("__sir_block__"));
        let blk = m
            .functions
            .iter()
            .find(|f| f.name.starts_with("__block"))
            .expect("hoisted block fn present");
        assert!(blk.captures.iter().any(|c| c.name == "__sir_block__"));
        assert!(semantic_ir::validate(&m).is_ok(), "validates: {:?}", semantic_ir::validate(&m));
    }

    #[test]
    fn nested_block_yield_still_validates() {
        // A block nested inside another block, with the inner block
        // `yield`ing, must NOT emit an invalid cross-level capture. v0
        // cut-line: only a block directly in the method body threads the
        // capture; the nested inner block keeps its raw `yield` (valid
        // SIR), so the module still validates.
        let m = lower("def outer\n  [1].each { [2].each { yield 9 } }\nend\n");
        assert!(
            semantic_ir::validate(&m).is_ok(),
            "nested block yield must still produce a valid module: {:?}",
            semantic_ir::validate(&m)
        );
    }

    #[test]
    fn top_level_block_yield_is_not_captured() {
        // At the top level there is no enclosing method, so a block that
        // `yield`s keeps its raw `yield` (no spurious capture / no dangling
        // __sir_block__ reference).
        let m = lower("def helper(x)\n  x + 1\nend\nhelper(2) { yield 1 }\n");
        let blk = m
            .functions
            .iter()
            .find(|f| f.name.starts_with("__block"))
            .expect("hoisted block fn present");
        assert!(
            blk.captures.is_empty(),
            "top-level block must NOT capture an enclosing block, got {:?}",
            blk.captures
        );
        assert!(semantic_ir::validate(&m).is_ok(), "validates: {:?}", semantic_ir::validate(&m));
    }

    // -----------------------------------------------------------------------
    // Phase Q10b — block_given? → not(null?(__sir_block__))
    // -----------------------------------------------------------------------

    /// Whether a `block_given?`-shaped nil-check —
    /// `not(null?(VarRef __sir_block__))` — appears anywhere in a block.
    fn finds_block_given_check(b: &semantic_ir::Block) -> bool {
        fn is_check(e: &Expr) -> bool {
            if let Expr::BuiltinCall { name, args, .. } = e {
                if name == "not" && args.len() == 1 {
                    if let Expr::BuiltinCall { name: inner, args: ia, .. } = &args[0] {
                        if inner == "null?" && ia.len() == 1 {
                            return matches!(&ia[0],
                                Expr::VarRef { name, scope, .. }
                                    if name == "__sir_block__" && *scope == Scope::Param);
                        }
                    }
                }
            }
            false
        }
        fn in_expr(e: &Expr) -> bool {
            if is_check(e) {
                return true;
            }
            match e {
                Expr::DirectCall { args, .. }
                | Expr::BuiltinCall { args, .. }
                | Expr::Intrinsic { args, .. } => args.iter().any(in_expr),
                Expr::IndirectCall { target, args, .. } => {
                    in_expr(target) || args.iter().any(in_expr)
                }
                Expr::If { cond, then_branch, else_branch, .. } => {
                    in_expr(cond) || in_blk(then_branch) || in_blk(else_branch)
                }
                Expr::Block(b) => in_blk(b),
                Expr::SeqLit { items, .. } => items.iter().any(in_expr),
                Expr::LogicalAnd { lhs, rhs, .. } | Expr::LogicalOr { lhs, rhs, .. } => {
                    in_expr(lhs) || in_expr(rhs)
                }
                _ => false,
            }
        }
        fn in_blk(b: &semantic_ir::Block) -> bool {
            b.stmts.iter().any(|s| match s {
                Stmt::LetBinding { value, .. }
                | Stmt::LetStarBinding { value, .. }
                | Stmt::Assign { value, .. }
                | Stmt::ExprStmt { expr: value, .. } => in_expr(value),
                _ => false,
            }) || in_expr(&b.value)
        }
        in_blk(b)
    }

    /// Whether a `VarRef` named `name` appears anywhere in a block (used to
    /// assert the raw `block_given?` ref was fully rewritten away).
    fn body_mentions_varref(b: &semantic_ir::Block, name: &str) -> bool {
        fn in_expr(e: &Expr, name: &str) -> bool {
            match e {
                Expr::VarRef { name: n, .. } => n == name,
                Expr::DirectCall { args, .. }
                | Expr::BuiltinCall { args, .. }
                | Expr::Intrinsic { args, .. } => args.iter().any(|a| in_expr(a, name)),
                Expr::IndirectCall { target, args, .. } => {
                    in_expr(target, name) || args.iter().any(|a| in_expr(a, name))
                }
                Expr::If { cond, then_branch, else_branch, .. } => {
                    in_expr(cond, name) || in_blk(then_branch, name) || in_blk(else_branch, name)
                }
                Expr::Block(b) => in_blk(b, name),
                Expr::SeqLit { items, .. } => items.iter().any(|i| in_expr(i, name)),
                Expr::LogicalAnd { lhs, rhs, .. } | Expr::LogicalOr { lhs, rhs, .. } => {
                    in_expr(lhs, name) || in_expr(rhs, name)
                }
                _ => false,
            }
        }
        fn in_blk(b: &semantic_ir::Block, name: &str) -> bool {
            b.stmts.iter().any(|s| match s {
                Stmt::LetBinding { value, .. }
                | Stmt::LetStarBinding { value, .. }
                | Stmt::Assign { value, .. }
                | Stmt::ExprStmt { expr: value, .. } => in_expr(value, name),
                _ => false,
            }) || in_expr(&b.value, name)
        }
        in_blk(b, name)
    }

    #[test]
    fn block_given_in_yielding_method_becomes_nil_check() {
        let m = lower("def t\n  if block_given?\n    yield 5\n  end\nend\n");
        let t = func(&m, "t");
        assert_eq!(t.params.last().map(|p| p.name.as_str()), Some("__sir_block__"));
        assert!(
            finds_block_given_check(&t.body),
            "block_given? must become not(null?(__sir_block__))"
        );
        assert!(
            !body_mentions_varref(&t.body, "block_given?"),
            "no raw block_given? VarRef may remain"
        );
        assert!(semantic_ir::validate(&m).is_ok(), "validates: {:?}", semantic_ir::validate(&m));
    }

    #[test]
    fn block_given_alone_threads_block_param() {
        // A method that queries block_given? but never yields must STILL be
        // threaded (detection fires on block_given?, not only yield).
        let m = lower("def t\n  if block_given?\n    1\n  else\n    2\n  end\nend\n");
        let t = func(&m, "t");
        assert_eq!(
            t.params.last().map(|p| p.name.as_str()),
            Some("__sir_block__"),
            "block_given?-only method must still gain __sir_block__"
        );
        assert!(finds_block_given_check(&t.body));
        assert!(semantic_ir::validate(&m).is_ok(), "validates: {:?}", semantic_ir::validate(&m));
    }

    #[test]
    fn method_without_block_given_or_yield_is_unchanged() {
        let m = lower("def g(x)\n  x + 1\nend\n");
        let g = func(&m, "g");
        assert_eq!(g.params.len(), 1, "non-block method keeps its arity");
        assert!(!finds_block_given_check(&g.body));
    }

    // -----------------------------------------------------------------------
    // Phase 22d — `super` keyword lowering
    //
    //   super        → ExprStmt(BuiltinCall("__super__", [method, class, …params]))
    //   super()      → ExprStmt(BuiltinCall("__super__", [method, class]))  (no args)
    //   super(1, 2)  → ExprStmt(BuiltinCall("__super__", [method, class, 1, 2]))
    //
    // Milestone O2 (OOP production) folded the old `super`/`zsuper` markers into
    // the single OOP-runtime builtin `__super__(method_name, class_name, …args)`.
    // The leading two args are the enclosing method + class names (empty strings
    // at the top level, where there is no enclosing class/method — not legal
    // Ruby, but the parser admits it).  Bare `super` (zsuper) forwards the
    // enclosing method's params by reference; `super()` forwards nothing.
    // -----------------------------------------------------------------------

    #[test]
    fn bare_super_lowers_to_super_builtin_forwarding_params() {
        // At the top level there are no params, so bare `super` forwards none:
        // `__super__("", "")` (method + class are empty — no enclosing context).
        // The in-class param-forwarding case is covered by
        // `bare_super_forwards_enclosing_params` in the O2 section.
        //
        // Issue #59 — `super` is now an EXPRESSION (`super_expr` in `factor`),
        // so a bare `super` as the SOLE top-level statement is promoted to the
        // module's tail `value` (like any bare expression), not `stmts[0]`.
        // The lowered SHAPE (`__super__(method, class, …)`) is unchanged.
        let m = lower("super\n");
        let b = main_body(&m);
        match &b.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "__super__", "bare super lowers to __super__");
                assert_eq!(args.len(), 2, "method + class names, no forwarded params");
                assert!(matches!(&args[0], Expr::StrLit { value, .. } if value.is_empty()));
                assert!(matches!(&args[1], Expr::StrLit { value, .. } if value.is_empty()));
            }
            other => panic!("expected tail BuiltinCall(__super__, …), got {:?}", other),
        }
    }

    #[test]
    fn super_empty_parens_lowers_to_super_builtin_no_args() {
        // `super()` forwards NO arguments: `__super__(method, class)` with no
        // trailing operands (at the top level the two names are empty strings).
        // Issue #59 — promoted to the module tail `value` (super is an expr).
        let m = lower("super()\n");
        let b = main_body(&m);
        match &b.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "__super__", "super() lowers to __super__");
                assert_eq!(args.len(), 2, "super() forwards zero extra args");
            }
            other => panic!("expected tail BuiltinCall(__super__, …), got {:?}", other),
        }
    }

    #[test]
    fn super_with_args_lowers_and_passes_validator() {
        // `super(1, 2)` → __super__("", "", IntLit(1), IntLit(2)) at the top
        // level, and the module validates (args are recursively well-formed).
        let m = lower("super(1, 2)\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected super with args: {:?}",
            result
        );
        let b = main_body(&m);
        // Issue #59 — promoted to the module tail `value` (super is an expr).
        match &b.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "__super__");
                assert_eq!(args.len(), 4, "method, class, then the two forwarded args");
                assert!(matches!(&args[2], Expr::IntLit { value: 1, .. }));
                assert!(matches!(&args[3], Expr::IntLit { value: 2, .. }));
            }
            other => panic!("expected tail BuiltinCall(__super__, …), got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Phase 6y — string interpolation lowering
    //
    // Output shapes covered:
    //   "plain"              → StrLit("plain")
    //   "#{x}"               → VarRef("x")
    //   "hi #{name}"         → StrConcat([StrLit("hi "), VarRef("name")])
    //   "sum=#{1+2}"         → StrConcat([StrLit("sum="),
    //                                     BuiltinCall("+", [1, 2])])
    //
    // Phase 20b replaced the v0 `BuiltinCall("string_concat", …)` marker
    // with the first-class `Expr::StrConcat` node (mirroring how 20a
    // replaced the `__interp__` body marker with real lowering).
    //
    // Plus an end-to-end smoke test that an interpolated module passes
    // the SIR validator (proves the StrConcat + StrLit shape is
    // well-formed semantic IR and the manifest declares
    // `StringInterpolation`).
    // -----------------------------------------------------------------------

    #[test]
    fn plain_string_with_no_interp_remains_a_strlit() {
        // Regression: zero-cost path — no interpolation markers means
        // we emit a plain `StrLit`, exactly as we did pre-Phase-6y.
        let m = lower(r#"x = "hello""#);
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::LetBinding { value, .. } => match value {
                Expr::StrLit { value, .. } => assert_eq!(value, "hello"),
                other => panic!("expected plain StrLit, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    }

    #[test]
    fn interpolated_string_with_bare_name_lowers_to_str_concat() {
        // `"hi #{name}"` → StrConcat([StrLit("hi "), VarRef("name")])
        //
        // The interpolated literal is in *trailing-value* position so
        // it sees the prior LetBinding (the validator's parallel-let
        // rule would otherwise reject the cross-binding reference).
        let m = lower(r##"name = "world"
"hi #{name}"
"##);
        let b = main_body(&m);
        // Single LetBinding for `name`; the interpolated string is the
        // block's trailing value expression.
        assert_eq!(b.stmts.len(), 1);
        assert!(matches!(&b.stmts[0], Stmt::LetBinding { name, .. } if name == "name"));
        match &b.value {
            Expr::StrConcat { parts, .. } => {
                assert_eq!(parts.len(), 2, "expected 2 concat segments");
                assert!(matches!(&parts[0], Expr::StrLit { value, .. } if value == "hi "));
                assert!(matches!(&parts[1], Expr::VarRef { name, .. } if name == "name"));
            }
            other => panic!("expected StrConcat, got {:?}", other),
        }
    }

    #[test]
    fn interpolated_string_that_is_only_interp_unwraps_to_a_single_segment() {
        // `"#{name}"` has no literal text — the lowerer should hand back
        // the single segment directly (no `StrConcat` wrapper).
        // Trailing-value position so the VarRef sees the LetBinding.
        let m = lower(r##"name = "world"
"#{name}"
"##);
        let b = main_body(&m);
        assert_eq!(b.stmts.len(), 1);
        assert!(matches!(&b.stmts[0], Stmt::LetBinding { name, .. } if name == "name"));
        assert!(
            matches!(&b.value, Expr::VarRef { name, .. } if name == "name"),
            "expected bare VarRef without concat wrapper, got {:?}",
            b.value
        );
    }

    #[test]
    fn interpolated_string_with_expression_lowers_recursively() {
        // Phase 20a (FC) — `"sum=#{1+2}"` — the interp body `1+2` is a real
        // expression.  Rather than emit the v0 `__interp__` marker carrying
        // the raw body text, the lowerer now re-parses the body and lowers
        // it into genuine SIR: the `+` operator becomes `BuiltinCall("+", …)`.
        // The marker survives only as a fallback for bodies we cannot parse.
        // Phase 20b — the outer concat is now an `Expr::StrConcat` node.
        let m = lower(r##"x = "sum=#{1+2}""##);
        let b = main_body(&m);
        let value = match &b.stmts[0] {
            Stmt::LetBinding { value, .. } => value,
            other => panic!("expected LetBinding, got {:?}", other),
        };
        match value {
            Expr::StrConcat { parts, .. } => {
                assert_eq!(parts.len(), 2);
                assert!(matches!(&parts[0], Expr::StrLit { value, .. } if value == "sum="));
                match &parts[1] {
                    Expr::BuiltinCall { name, args, .. } => {
                        assert_eq!(name, "+", "interp body 1+2 lowers to a real + call");
                        assert_eq!(args.len(), 2);
                        assert!(matches!(args[0], Expr::IntLit { value: 1, .. }));
                        assert!(matches!(args[1], Expr::IntLit { value: 2, .. }));
                    }
                    other => panic!("expected real `+` BuiltinCall, got {:?}", other),
                }
            }
            other => panic!("expected StrConcat, got {:?}", other),
        }
    }

    #[test]
    fn interpolated_string_module_passes_sir_validator() {
        // End-to-end smoke: the module produced from an interpolated
        // string round-trips through the SIR validator.  This catches
        // shape regressions (missing effects, wrong arg ordering,
        // unbound names) that the unit assertions above might miss.
        // Trailing-value placement so the interpolation's VarRef sees
        // the prior LetBinding under the validator's parallel-let rule.
        let m = lower(r##"name = "world"
puts("hi #{name}")
"##);
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected interpolated-string module: {:?}",
            result
        );
    }

    #[test]
    fn adjacent_interps_with_no_literal_lower_to_str_concat_of_refs() {
        // Phase 20b (FC) — `"#{a}#{b}"` has two interpolations and *no*
        // literal text between them.  The concat still has two parts
        // (both `VarRef`s), so it lowers to a two-part `StrConcat` — a
        // concat is well-defined without any `StrLit` segment.
        let m = lower(r##"a = "x"
b = "y"
"#{a}#{b}"
"##);
        let b = main_body(&m);
        match &b.value {
            Expr::StrConcat { parts, .. } => {
                assert_eq!(parts.len(), 2);
                assert!(matches!(&parts[0], Expr::VarRef { name, .. } if name == "a"));
                assert!(matches!(&parts[1], Expr::VarRef { name, .. } if name == "b"));
            }
            other => panic!("expected StrConcat of two VarRefs, got {:?}", other),
        }
    }

    #[test]
    fn str_concat_module_declares_string_interpolation_feature() {
        // Phase 20b (FC) — emitting a `StrConcat` must add
        // `Feature::StringInterpolation` to the module manifest, or the
        // validator would reject the module as using an undeclared
        // feature.  This pins the manifest/observed agreement directly.
        let m = lower(r##"name = "world"
"hi #{name}"
"##);
        assert!(
            m.manifest.contains(semantic_ir::Feature::StringInterpolation),
            "manifest should declare string-interpolation; got {:?}",
            m.manifest
        );
    }

    #[test]
    fn interpolated_string_with_multiple_expr_interps_lowers_each_recursively() {
        // Phase 20a (FC) — `"a#{1}b#{2}c"` carries two expression
        // interpolations interleaved with three literal runs.  Each `#{…}`
        // body re-parses and lowers to a real `IntLit`, producing a
        // five-segment concat (no `__interp__` markers anywhere).
        // Phase 20b — the concat is now an `Expr::StrConcat` node.
        let m = lower(r##"x = "a#{1}b#{2}c""##);
        let b = main_body(&m);
        let value = match &b.stmts[0] {
            Stmt::LetBinding { value, .. } => value,
            other => panic!("expected LetBinding, got {:?}", other),
        };
        match value {
            Expr::StrConcat { parts, .. } => {
                assert_eq!(parts.len(), 5, "a | 1 | b | 2 | c");
                assert!(matches!(&parts[0], Expr::StrLit { value, .. } if value == "a"));
                assert!(matches!(parts[1], Expr::IntLit { value: 1, .. }));
                assert!(matches!(&parts[2], Expr::StrLit { value, .. } if value == "b"));
                assert!(matches!(parts[3], Expr::IntLit { value: 2, .. }));
                assert!(matches!(&parts[4], Expr::StrLit { value, .. } if value == "c"));
            }
            other => panic!("expected StrConcat, got {:?}", other),
        }
    }

    #[test]
    fn interpolated_string_with_binary_var_expr_lowers_recursively() {
        // Phase 20a (FC) — `"#{a + b}"` is a sole interpolation whose body is
        // a binary expression over two names.  It unwraps to the lowered
        // `+` call directly (single-segment unwrap), with both operands as
        // `VarRef`s — proving the recursive lowerer threads the current
        // scope through, not a `__interp__` marker.
        let m = lower(r##"x = "#{a + b}""##);
        let b = main_body(&m);
        let value = match &b.stmts[0] {
            Stmt::LetBinding { value, .. } => value,
            other => panic!("expected LetBinding, got {:?}", other),
        };
        match value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "+", "single interp body unwraps to the real + call");
                assert_eq!(args.len(), 2);
                assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "a"));
                assert!(matches!(&args[1], Expr::VarRef { name, .. } if name == "b"));
            }
            other => panic!("expected real `+` BuiltinCall, got {:?}", other),
        }
    }

    #[test]
    fn interpolated_expression_string_validates_e2e() {
        // Phase 20a (FC) — end-to-end: an arithmetic interpolation lowers to
        // real SIR (`string_concat([StrLit, BuiltinCall("+", …)])`) and that
        // shape round-trips cleanly through the SIR validator.  Guards
        // against effect/arg-shape regressions in the new recursive path.
        let m = lower(r##"puts("sum is #{1 + 2}")"##);
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected expression-interpolation module: {:?}",
            result
        );
    }

    #[test]
    fn deeply_nested_interpolation_terminates_without_stack_overflow() {
        // Phase 20a (FC) DoS guard — the recursive interp lowerer caps
        // re-parsing at MAX_INTERP_DEPTH (8).  Build an interpolation
        // nested *far* beyond that (200 levels) and confirm lowering
        // simply terminates — past the cap the body is kept as the
        // verbatim `__interp__` marker rather than recursing until the
        // thread stack is exhausted.  Before the guard this input would
        // abort the process with an uncatchable stack overflow.
        let mut src = String::from("1");
        for _ in 0..200 {
            // each wrap adds one `"#{ … }"` interpolation level
            src = format!("\"#{{{src}}}\"");
        }
        let code = format!("x = {src}");
        // The assertion is simply that this returns at all (no panic /
        // overflow); we touch the body to ensure a real module came back.
        let m = lower(&code);
        let _ = main_body(&m);
    }

    // -----------------------------------------------------------------------
    // Phase 6z — float / hex / bin / oct numeric literal lowering
    //
    // Shapes covered:
    //   1.5          → FloatLit(1.5)   (+ Feature::Floats)
    //   1e10         → FloatLit(1e10)
    //   1.5e-3       → FloatLit(0.0015)
    //   0x1F         → IntLit(31)
    //   0xDEAD_BEEF  → IntLit(3735928559)
    //   0b1010       → IntLit(10)
    //   0o17         → IntLit(15)
    //   42           → IntLit(42)      (regression — pre-Phase-6z path)
    //   1_000_000    → IntLit(1000000) (underscore separator)
    // -----------------------------------------------------------------------

    #[test]
    fn float_literal_lowers_to_floatlit_and_triggers_floats_feature() {
        // `x = 1.5` — fused single Number token by lexer Phase 4k.
        let m = lower("x = 1.5");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::LetBinding { value, name, .. } => {
                assert_eq!(name, "x");
                match value {
                    Expr::FloatLit { value, .. } => {
                        // Exact comparison is fine for 1.5 (representable
                        // exactly in IEEE-754); we use `==` directly.
                        assert_eq!(*value, 1.5_f64);
                    }
                    other => panic!("expected FloatLit, got {:?}", other),
                }
            }
            other => panic!("expected LetBinding, got {:?}", other),
        }
        // Feature::Floats must be tracked in the module's manifest.
        assert!(
            m.manifest.contains(semantic_ir::Feature::Floats),
            "expected Floats feature in manifest"
        );
    }

    #[test]
    fn float_literal_with_signed_exponent_lowers_correctly() {
        // `x = 1.5e-3` — fractional + signed exponent.  Value is
        // 0.0015 (NOT exactly representable in IEEE-754, so we use a
        // tight tolerance instead of `==`).
        let m = lower("x = 1.5e-3");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::LetBinding {
                value: Expr::FloatLit { value, .. },
                ..
            } => {
                assert!(
                    (*value - 0.0015_f64).abs() < 1e-12,
                    "expected 0.0015, got {}",
                    value
                );
            }
            other => panic!("expected LetBinding(FloatLit), got {:?}", other),
        }
    }

    #[test]
    fn hex_literal_lowers_to_intlit_with_correct_value() {
        // `x = 0xDEAD_BEEF` — hex with underscore separator.  Decimal
        // value is 3,735,928,559.  Triggers the radix-detection
        // branch in `lower_numeric_literal`.
        let m = lower("x = 0xDEAD_BEEF");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::LetBinding {
                value: Expr::IntLit { value, .. },
                ..
            } => {
                assert_eq!(*value, 0xDEAD_BEEF_i64);
            }
            other => panic!("expected LetBinding(IntLit), got {:?}", other),
        }
        // Hex integers must NOT trigger Feature::Floats.
        assert!(
            !m.manifest.contains(semantic_ir::Feature::Floats),
            "hex integer literal should not trigger Floats feature"
        );
    }

    #[test]
    fn binary_literal_lowers_to_intlit() {
        // `x = 0b1010` → IntLit(10).
        let m = lower("x = 0b1010");
        let b = main_body(&m);
        assert!(matches!(
            &b.stmts[0],
            Stmt::LetBinding { value: Expr::IntLit { value: 10, .. }, .. }
        ));
    }

    #[test]
    fn octal_literal_lowers_to_intlit() {
        // `x = 0o17` → IntLit(15).
        let m = lower("x = 0o17");
        let b = main_body(&m);
        assert!(matches!(
            &b.stmts[0],
            Stmt::LetBinding { value: Expr::IntLit { value: 15, .. }, .. }
        ));
    }

    #[test]
    fn float_literal_module_passes_sir_validator() {
        // End-to-end smoke: a module that uses float literals and
        // declares the Floats feature passes the validator.
        let m = lower("x = 1.5\nputs(x)\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected float-using module: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // Phase 7a — backtick command literal lowering
    //
    // Lexer Phase 4m emits `` `ls -la` `` as a `TokenType::String` whose
    // value is the verbatim source including the surrounding backticks
    // (`` `ls -la` ``).  The lowerer detects the leading backtick and
    // emits a marker `BuiltinCall("backtick", [StrLit(body)])` with
    // effects = MayBlock | MayPrint | MayThrow.
    // -----------------------------------------------------------------------

    #[test]
    fn backtick_command_literal_lowers_to_backtick_builtin_call() {
        // `x = `ls -la`` → BuiltinCall("backtick", [StrLit("ls -la")]).
        let m = lower("x = `ls -la`");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::LetBinding { value, name, .. } => {
                assert_eq!(name, "x");
                match value {
                    Expr::BuiltinCall { name, args, .. } => {
                        assert_eq!(name, "backtick");
                        assert_eq!(args.len(), 1);
                        match &args[0] {
                            Expr::StrLit { value, .. } => {
                                assert_eq!(value, "ls -la");
                            }
                            other => panic!("expected StrLit body, got {:?}", other),
                        }
                    }
                    other => panic!("expected backtick BuiltinCall, got {:?}", other),
                }
            }
            other => panic!("expected LetBinding, got {:?}", other),
        }
    }

    #[test]
    fn backtick_command_literal_carries_effect_set() {
        // Backtick spawns a child process — must carry MayBlock,
        // MayPrint, MayThrow.  Verify all three are set on the marker.
        let m = lower("x = `pwd`");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::LetBinding {
                value: Expr::BuiltinCall { effects, .. },
                ..
            } => {
                assert!(effects.contains(Effect::MayBlock), "expected MayBlock");
                assert!(effects.contains(Effect::MayPrint), "expected MayPrint");
                assert!(effects.contains(Effect::MayThrow), "expected MayThrow");
            }
            other => panic!("expected backtick BuiltinCall, got {:?}", other),
        }
    }

    #[test]
    fn empty_backtick_command_literal_lowers_with_empty_body() {
        // `x = ``` (empty body).  The marker still emits a StrLit, with
        // an empty body string.
        let m = lower("x = ``");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::LetBinding {
                value: Expr::BuiltinCall { name, args, .. },
                ..
            } => {
                assert_eq!(name, "backtick");
                assert!(matches!(&args[0], Expr::StrLit { value, .. } if value.is_empty()));
            }
            other => panic!("expected backtick BuiltinCall, got {:?}", other),
        }
    }

    #[test]
    fn backtick_command_literal_triggers_strings_feature() {
        // The synthetic StrLit body triggers `Feature::Strings`.
        let m = lower("x = `whoami`");
        assert!(
            m.manifest.contains(semantic_ir::Feature::Strings),
            "expected Strings feature in manifest"
        );
    }

    #[test]
    fn backtick_command_literal_module_passes_sir_validator() {
        // End-to-end smoke: a module that uses a backtick literal
        // passes the SIR validator.
        let m = lower("x = `echo hi`\nputs(x)\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected backtick-using module: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // Phase 7b — heredoc literal lowering
    //
    // Lexer Phase 3c/4o emits every heredoc as a `TokenType::String` whose
    // value is the canonical `<<TAG\n<body>TAG` form (with `<<~TAG`
    // indent-stripping pre-applied).  The lowerer detects the `<<` prefix
    // and emits `StrLit(body)` — the inner body, tag suffix stripped.
    //
    // Phase 17a (FC): heredocs interpolate like double-quoted strings, so
    // the extracted body is routed through the shared interpolation
    // splitter.  A body with no `#{…}` still lowers to a single `StrLit`
    // (the tests below), while an interpolating body lowers to a
    // `StrConcat` of literal runs + lowered `#{…}` expressions.
    // -----------------------------------------------------------------------

    #[test]
    fn plain_heredoc_lowers_to_strlit_body_only() {
        // `x = <<EOF\nhello\nEOF\n` → StrLit("hello\n").
        let m = lower("x = <<EOF\nhello\nEOF\n");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::LetBinding { value, name, .. } => {
                assert_eq!(name, "x");
                match value {
                    Expr::StrLit { value, .. } => {
                        // Body retains the trailing newline that was
                        // part of the source between `hello` and `EOF`.
                        assert_eq!(value, "hello\n");
                    }
                    other => panic!("expected StrLit, got {:?}", other),
                }
            }
            other => panic!("expected LetBinding, got {:?}", other),
        }
    }

    #[test]
    fn dash_indent_heredoc_lowers_to_strlit_body_only() {
        // `<<-EOF` strips opener prefix `<<-` and closing tag, leaves
        // body intact.  Closing-tag indentation is already stripped by
        // the lexer's canonicalisation.
        let m = lower("x = <<-EOF\nhello\n  EOF\n");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::LetBinding {
                value: Expr::StrLit { value, .. },
                ..
            } => {
                assert_eq!(value, "hello\n");
            }
            other => panic!("expected StrLit, got {:?}", other),
        }
    }

    #[test]
    fn tilde_indent_heredoc_strips_common_leading_whitespace() {
        // `<<~EOF` — common leading whitespace pre-stripped by the
        // lexer.  Our lowerer just emits the body as a StrLit.
        let m = lower("x = <<~EOF\n  hello\n  world\n  EOF\n");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::LetBinding {
                value: Expr::StrLit { value, .. },
                ..
            } => {
                // Lexer stripped the 2-space common indent from each
                // non-empty body line.
                assert_eq!(value, "hello\nworld\n");
            }
            other => panic!("expected StrLit, got {:?}", other),
        }
    }

    #[test]
    fn heredoc_triggers_strings_feature() {
        // The synthetic StrLit body triggers `Feature::Strings`.
        let m = lower("x = <<EOF\nhi\nEOF\n");
        assert!(
            m.manifest.contains(semantic_ir::Feature::Strings),
            "expected Strings feature in manifest"
        );
    }

    #[test]
    fn heredoc_module_passes_sir_validator() {
        // End-to-end smoke: a module that uses a heredoc literal
        // passes the SIR validator.
        let m = lower("x = <<EOF\nhello\nEOF\nputs(x)\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected heredoc-using module: {:?}",
            result
        );
    }

    #[test]
    fn interpolated_heredoc_lowers_body_to_str_concat() {
        // Phase 17a (FC) — a heredoc body interpolates like a
        // double-quoted string.  `<<EOF\nhi #{name}\nEOF` → the body
        // `hi #{name}\n` splits into ["hi ", VarRef(name), "\n"] under a
        // three-part `StrConcat`.  `name` is bound first so it resolves
        // as a local.
        let m = lower("name = \"bob\"\ny = <<EOF\nhi #{name}\nEOF\n");
        let b = main_body(&m);
        // stmts[0] binds `name`; stmts[1] binds `y` to the heredoc.
        let value = match &b.stmts[1] {
            Stmt::LetBinding { name, value, .. } | Stmt::LetStarBinding { name, value, .. }
                if name == "y" =>
            {
                value
            }
            other => panic!("expected LetBinding y, got {:?}", other),
        };
        match value {
            Expr::StrConcat { parts, .. } => {
                assert_eq!(parts.len(), 3, "hi  | name | newline");
                assert!(matches!(&parts[0], Expr::StrLit { value, .. } if value == "hi "));
                assert!(matches!(&parts[1], Expr::VarRef { name, .. } if name == "name"));
                assert!(matches!(&parts[2], Expr::StrLit { value, .. } if value == "\n"));
            }
            other => panic!("expected StrConcat, got {:?}", other),
        }
    }

    #[test]
    fn interpolated_tilde_heredoc_with_expression_lowers_recursively() {
        // Phase 17a (FC) — interpolation works through the `<<~`
        // indent-stripping form too, and the `#{1+2}` body lowers to a
        // real `+` call (via Phase 20a recursive interp lowering).
        let m = lower("x = <<~EOF\n  sum #{1 + 2}\n  EOF\n");
        let b = main_body(&m);
        let value = match &b.stmts[0] {
            Stmt::LetBinding { value, .. } => value,
            other => panic!("expected LetBinding, got {:?}", other),
        };
        match value {
            Expr::StrConcat { parts, .. } => {
                assert_eq!(parts.len(), 3, "sum  | 1+2 | newline");
                assert!(matches!(&parts[0], Expr::StrLit { value, .. } if value == "sum "));
                assert!(
                    matches!(&parts[1], Expr::BuiltinCall { name, .. } if name == "+"),
                    "expected real `+` call, got {:?}", parts[1]
                );
                assert!(matches!(&parts[2], Expr::StrLit { value, .. } if value == "\n"));
            }
            other => panic!("expected StrConcat, got {:?}", other),
        }
    }

    #[test]
    fn interpolated_heredoc_validates_e2e_and_declares_feature() {
        // Phase 17a (FC) — end-to-end: an interpolating heredoc lowers to
        // a `StrConcat`, declares `Feature::StringInterpolation`, and the
        // module round-trips the SIR validator.  The heredoc is the
        // block's trailing value expression (not a `LetBinding` RHS) so
        // its `#{name}` VarRef sees the prior binding under the
        // validator's parallel-let rule — mirroring the double-quoted
        // `interpolated_string_module_passes_sir_validator` test.
        let m = lower("name = \"bob\"\n<<EOF\nhi #{name}\nEOF\n");
        assert!(
            m.manifest.contains(semantic_ir::Feature::StringInterpolation),
            "expected string-interpolation feature; got {:?}",
            m.manifest
        );
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected interpolating-heredoc module: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // Phase 7c — Ruby 3.0 endless method definitions
    //
    // `def foo = expr` and `def foo(x, y) = expr` both hoist to a top-level
    // Function whose Block has `stmts=[]` and `value=<lowered expr>`.
    // -----------------------------------------------------------------------

    #[test]
    fn endless_def_no_params_hoists_to_top_level_function() {
        // `def hello = 1` → top-level Function named "hello" with no
        // params and body value = IntLit(1).
        let m = lower("def hello = 1");
        let f = m
            .functions
            .iter()
            .find(|f| f.name == "hello")
            .expect("expected top-level `hello` Function");
        assert!(f.params.is_empty(), "expected no params");
        assert!(f.body.stmts.is_empty(), "endless def body has no stmts");
        assert!(
            matches!(f.body.value, Expr::IntLit { value: 1, .. }),
            "expected IntLit(1) as body value, got {:?}",
            f.body.value
        );
    }

    #[test]
    fn endless_def_with_params_carries_param_scope() {
        // `def add(x, y) = x + y` — params should bind to Scope::Param
        // inside the body expression.
        let m = lower("def add(x, y) = x + y");
        let f = m
            .functions
            .iter()
            .find(|f| f.name == "add")
            .expect("expected top-level `add` Function");
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.params[0].name, "x");
        assert_eq!(f.params[1].name, "y");
        // Body value should be the `x + y` BuiltinCall with VarRefs
        // both at Scope::Param.
        match &f.body.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "+");
                assert_eq!(args.len(), 2);
                assert!(
                    matches!(&args[0], Expr::VarRef { name, scope, .. }
                        if name == "x" && *scope == Scope::Param),
                    "expected VarRef(x, Param), got {:?}",
                    args[0]
                );
                assert!(
                    matches!(&args[1], Expr::VarRef { name, scope, .. }
                        if name == "y" && *scope == Scope::Param),
                    "expected VarRef(y, Param), got {:?}",
                    args[1]
                );
            }
            other => panic!("expected +-BuiltinCall, got {:?}", other),
        }
    }

    #[test]
    fn endless_def_does_not_emit_main_body_stmt() {
        // The endless def hoists to a top-level Function; the
        // main-body statement stream should NOT contain an
        // assignment / call for it (just the synthetic NilLit
        // ExprStmt placeholder, same as block-bodied def).
        let m = lower("def hello = 1\nputs(hello())");
        // Expect at least two functions: main + hello.
        assert!(
            m.functions.iter().any(|f| f.name == "hello"),
            "expected hello Function"
        );
        let main = m
            .functions
            .iter()
            .find(|f| f.name == "main")
            .expect("expected main");
        // The first stmt is the no-op placeholder for the endless def.
        match &main.body.stmts[0] {
            Stmt::ExprStmt {
                expr: Expr::NilLit { .. },
                ..
            } => {} // OK
            other => panic!("expected NilLit ExprStmt placeholder, got {:?}", other),
        }
    }

    #[test]
    fn endless_def_module_passes_sir_validator() {
        // End-to-end smoke: a module with an endless def passes
        // the validator.  The function body's lowering must produce
        // a well-formed Block (stmts + value).
        let m = lower("def square(n) = n * n\nputs(square(3))\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected endless-def module: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // Phase 7d — case/in pattern matching lowering
    //
    // case_statement walks both when_clause and in_clause subnodes in
    // source order; in_clause pattern dispatch lives in
    // `lower_in_clause_pattern`.
    //
    // Patterns covered here:
    //   literal_pattern  → `==` comparison cond, no bindings
    //   binding_pattern  → `BoolLit(true)` cond + LetBinding prefix
    //   array_pattern    → Phase 13a/13b: fixed-arity elements lower
    //                      structurally (`len(s)==N && s[i]==lit …` +
    //                      element LetBindings), recursing into nested
    //                      array AND hash sub-patterns
    //   hash_pattern     → Phase FC: structural match keyed by symbol
    //                      (`MapGet(s,:k)` + `&&` + per-key LetBindings),
    //                      shorthand `{k:}` binds `k = s[:k]`
    // -----------------------------------------------------------------------

    /// Helper: extract the `Expr::If` from a top-level `case` statement
    /// — Phase 6u/7d lowers case to `Stmt::ExprStmt(Expr::If(...))`.
    fn extract_case_if(b: &semantic_ir::Block) -> &Expr {
        // The case is typically the second statement (first is the
        // `x = ...` LetBinding from the test prelude).
        match &b.stmts[1] {
            Stmt::ExprStmt { expr, .. } => expr,
            other => panic!("expected ExprStmt from case, got {:?}", other),
        }
    }

    #[test]
    fn case_in_literal_pattern_lowers_to_equality_check() {
        // `case x; in 1; "one"; end` — literal pattern emits the same
        // `scrutinee == 1` shape as a `when` clause.
        let m = lower("x = 1\ncase x\nin 1\n  \"one\"\nend\n");
        let b = main_body(&m);
        let if_expr = extract_case_if(b);
        let cond = match if_expr {
            Expr::If { cond, .. } => cond,
            other => panic!("expected If from case, got {:?}", other),
        };
        match cond.as_ref() {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "==");
                assert_eq!(args.len(), 2);
                // args[0] is the scrutinee (VarRef x); args[1] is IntLit(1).
                assert!(matches!(&args[1], Expr::IntLit { value: 1, .. }));
            }
            other => panic!("expected ==-BuiltinCall as cond, got {:?}", other),
        }
    }

    #[test]
    fn case_in_binding_pattern_emits_letbinding_prefix() {
        // `case x; in y; puts(y); end` — binding pattern.  Cond is true;
        // body has a LetBinding(y, scrutinee) prepended; body trailing
        // value is the `puts(y)` call.  Body shape is `puts(y)` (and
        // not bare `y`) because method_call_no_paren would otherwise
        // greedily consume the closing `end` keyword as a one-arg
        // method-call argument — a pre-existing grammar limitation
        // unrelated to Phase 7d.
        let m = lower("x = 1\ncase x\nin y\n  puts(y)\nend\n");
        let b = main_body(&m);
        let if_expr = extract_case_if(b);
        let (cond, then_branch) = match if_expr {
            Expr::If { cond, then_branch, .. } => (cond, then_branch),
            other => panic!("expected If from case, got {:?}", other),
        };
        // Condition is BoolLit(true).
        assert!(
            matches!(cond.as_ref(), Expr::BoolLit { value: true, .. }),
            "expected BoolLit(true), got {:?}",
            cond
        );
        // then_branch.stmts[0] should be LetBinding(y, …).
        let prefix = then_branch
            .stmts
            .first()
            .expect("expected LetBinding prefix stmt");
        match prefix {
            Stmt::LetBinding { name, .. } => {
                assert_eq!(name, "y");
            }
            other => panic!("expected LetBinding(y), got {:?}", other),
        }
        // then_branch.value should be the `puts(y)` BuiltinCall whose
        // single arg is the bound `y`.
        match &then_branch.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "puts");
                assert!(
                    matches!(&args[0], Expr::VarRef { name, .. } if name == "y"),
                    "expected puts(VarRef(y)), got args={:?}",
                    args
                );
            }
            other => panic!("expected puts BuiltinCall, got {:?}", other),
        }
    }

    #[test]
    fn case_in_pin_pattern_lowers_to_equality_with_local() {
        // Phase FC — `in ^expected` lowers to `scrutinee == expected`
        // (equality BuiltinCall over a VarRef to the pinned local), no
        // binding prefix.
        let m = lower("expected = 1\nv = 2\ncase v\nin ^expected\n  puts(1)\nend\n");
        let b = main_body(&m);
        // stmts: [let expected, let v, case]; the case is the last stmt.
        let if_expr = match b.stmts.last() {
            Some(Stmt::ExprStmt { expr, .. }) => expr,
            other => panic!("expected trailing case ExprStmt, got {:?}", other),
        };
        let cond = match if_expr {
            Expr::If { cond, .. } => cond,
            other => panic!("expected If, got {:?}", other),
        };
        match cond.as_ref() {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "==");
                assert!(
                    matches!(&args[1], Expr::VarRef { name, .. } if name == "expected"),
                    "expected RHS VarRef(expected), got {:?}",
                    args[1]
                );
            }
            other => panic!("expected ==-BuiltinCall cond, got {:?}", other),
        }
    }

    #[test]
    fn case_in_class_pattern_lowers_to_is_a_check() {
        // Phase FC — `in Integer(n)` lowers to
        // `is_a?(x, "Integer") && len(x)==1 && …` with `n = x[0]` bound.
        let m = lower("x = 1\ncase x\nin Integer(n)\n  puts(n)\nend\n");
        let b = main_body(&m);
        let (cond, then_branch) = match extract_case_if(b) {
            Expr::If { cond, then_branch, .. } => (cond, then_branch),
            other => panic!("expected If, got {:?}", other),
        };
        let printed = format!("{:?}", cond);
        assert!(
            printed.contains("is_a?") && printed.contains("Integer"),
            "expected an is_a?(_, \"Integer\") check, got {}",
            printed
        );
        // `n = x[0]` binding present in the body.
        let has_n = then_branch.stmts.iter().any(|s| {
            matches!(s, Stmt::LetBinding { name, value, .. }
                if name == "n" && matches!(value, Expr::SeqIndex { .. }))
        });
        assert!(has_n, "expected `let n = x[0]` binding in body");
    }

    #[test]
    fn case_in_pin_and_class_patterns_validate_e2e() {
        // End-to-end: both new pattern forms round-trip the SIR validator.
        let pin = lower("e = 1\nv = 2\ncase v\nin ^e\n  puts(1)\nend\n");
        assert!(
            semantic_ir::validate(&pin).is_ok(),
            "validator rejected pin-pattern module: {:?}",
            semantic_ir::validate(&pin)
        );
        let class = lower("x = 1\ncase x\nin Integer(n)\n  puts(n)\nend\n");
        assert!(
            semantic_ir::validate(&class).is_ok(),
            "validator rejected class-pattern module: {:?}",
            semantic_ir::validate(&class)
        );
    }

    #[test]
    fn case_in_literal_array_pattern_lowers_to_structural_match() {
        // Phase 13a (FC) — `case x; in [1, 2]; "pair"; end` — a
        // fixed-arity all-literal array pattern lowers structurally to
        // `((len(x) == 2) && (x[0] == 1)) && (x[1] == 2)` (an AND-chain
        // of a length check and per-element equality), no longer the
        // `__pattern_match__` marker.
        let m = lower("x = 1\ncase x\nin [1, 2]\n  \"pair\"\nend\n");
        let b = main_body(&m);
        let if_expr = extract_case_if(b);
        let cond = match if_expr {
            Expr::If { cond, .. } => cond,
            other => panic!("expected If, got {:?}", other),
        };
        // Outermost node is the final `&& (x[1] == 2)`.
        let (lhs, rhs) = match cond.as_ref() {
            Expr::LogicalAnd { lhs, rhs, .. } => (lhs, rhs),
            other => panic!("expected LogicalAnd cond, got {:?}", other),
        };
        // rhs: x[1] == 2
        match rhs.as_ref() {
            Expr::BuiltinCall { name, args, .. } if name == "==" => {
                assert!(matches!(&args[0], Expr::SeqIndex { .. }));
                assert!(matches!(&args[1], Expr::IntLit { value: 2, .. }));
            }
            other => panic!("expected `x[1] == 2`, got {:?}", other),
        }
        // lhs: (len(x) == 2) && (x[0] == 1)
        let (len_chk, first) = match lhs.as_ref() {
            Expr::LogicalAnd { lhs, rhs, .. } => (lhs, rhs),
            other => panic!("expected nested LogicalAnd, got {:?}", other),
        };
        match len_chk.as_ref() {
            Expr::BuiltinCall { name, args, .. } if name == "==" => {
                assert!(matches!(&args[0], Expr::SeqLen { .. }), "expected len() check");
                assert!(matches!(&args[1], Expr::IntLit { value: 2, .. }));
            }
            other => panic!("expected `len(x) == 2`, got {:?}", other),
        }
        assert!(
            matches!(first.as_ref(), Expr::BuiltinCall { name, .. } if name == "=="),
            "expected `x[0] == 1`, got {:?}", first
        );
    }

    #[test]
    fn case_in_array_one_splat_lowers_structurally() {
        // Phase FC — `in [a, *rest, b]` desugars to a relaxed length check
        // (`len(x) >= 2`) plus front/back element bindings and a middle
        // slice bind via `__seq_slice__`.  No `__pattern_match__` marker.
        let m = lower("x = 1\ncase x\nin [a, *rest, b]\n  puts(a)\nend\n");
        let b = main_body(&m);
        let if_expr = extract_case_if(b);
        let (cond, then_branch) = match if_expr {
            Expr::If { cond, then_branch, .. } => (cond, then_branch),
            other => panic!("expected If, got {:?}", other),
        };
        let printed = format!("{:?}", cond);
        assert!(
            !printed.contains("__pattern_match__"),
            "expected no marker in one-splat cond, got {}",
            printed
        );
        // Relaxed length check `len(x) >= 2` lives at the root of the AND chain.
        assert!(
            printed.contains("\">=\""),
            "expected a `>=` length check, got {}",
            printed
        );
        // Body binds front `a = x[0]`, back `b = x[len-1]`, and the middle
        // slice `rest = __seq_slice__(x, 1, len-1)`.
        let names: Vec<&str> = then_branch
            .stmts
            .iter()
            .filter_map(|s| match s {
                Stmt::LetBinding { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert!(names.contains(&"a"), "expected front binding a, got {:?}", names);
        assert!(names.contains(&"b"), "expected back binding b, got {:?}", names);
        assert!(names.contains(&"rest"), "expected splat binding rest, got {:?}", names);
        let has_slice = then_branch.stmts.iter().any(|s| matches!(s,
            Stmt::LetBinding { name, value, .. }
                if name == "rest"
                && matches!(value, Expr::BuiltinCall { name, .. } if name == "__seq_slice__")));
        assert!(has_slice, "expected `rest = __seq_slice__(..)` binding");
    }

    #[test]
    fn case_in_array_one_splat_validates() {
        // Phase FC — the desugared one-splat pattern produces well-formed
        // SIR: the module validates end-to-end.
        let m = lower("x = 1\ncase x\nin [a, *rest, b]\n  puts(a)\nend\n");
        assert!(
            semantic_ir::validate(&m).is_ok(),
            "expected one-splat pattern module to validate: {:?}",
            semantic_ir::validate(&m)
        );
    }

    #[test]
    fn case_in_array_anonymous_splat_binds_nothing() {
        // Phase FC — a bare `*` (anonymous splat) binds no slice name; only
        // the fixed front element `a` is bound, and the cond is `len(x) >= 1`.
        let m = lower("x = 1\ncase x\nin [a, *]\n  puts(a)\nend\n");
        let b = main_body(&m);
        let if_expr = extract_case_if(b);
        let then_branch = match if_expr {
            Expr::If { then_branch, .. } => then_branch,
            other => panic!("expected If, got {:?}", other),
        };
        let has_slice = then_branch.stmts.iter().any(|s| matches!(s,
            Stmt::LetBinding { value, .. }
                if matches!(value, Expr::BuiltinCall { name, .. } if name == "__seq_slice__")));
        assert!(!has_slice, "anonymous splat must not bind a slice");
    }

    #[test]
    fn case_in_array_find_pattern_falls_back_to_marker() {
        // Phase FC — a two-splat *find* pattern `[*, x, *]` is not yet a
        // first-class desugaring; it falls back to the `__pattern_match__`
        // marker (documented v0 limitation), which still validates.
        let m = lower("x = 1\ncase x\nin [*, 1, *]\n  \"h\"\nend\n");
        let b = main_body(&m);
        let cond = match extract_case_if(b) {
            Expr::If { cond, .. } => cond,
            other => panic!("expected If, got {:?}", other),
        };
        let printed = format!("{:?}", cond);
        assert!(
            printed.contains("__pattern_match__"),
            "expected find pattern to fall back to marker, got {}",
            printed
        );
        assert!(semantic_ir::validate(&m).is_ok(), "marker module must validate");
    }

    #[test]
    fn case_in_array_pattern_binds_name_elements() {
        // Phase 13a (FC) — `in [a, b]` matches any 2-element sequence and
        // binds `a = x[0]`, `b = x[1]` as prefix LetBindings in the body.
        // Cond is just the length check (no per-element equality).
        let m = lower("x = 1\ncase x\nin [a, b]\n  puts(a)\nend\n");
        let b = main_body(&m);
        let if_expr = extract_case_if(b);
        let (cond, then_branch) = match if_expr {
            Expr::If { cond, then_branch, .. } => (cond, then_branch),
            other => panic!("expected If, got {:?}", other),
        };
        // Cond: len(x) == 2 (no element equality for pure bindings).
        match cond.as_ref() {
            Expr::BuiltinCall { name, args, .. } if name == "==" => {
                assert!(matches!(&args[0], Expr::SeqLen { .. }));
                assert!(matches!(&args[1], Expr::IntLit { value: 2, .. }));
            }
            other => panic!("expected `len(x) == 2` cond, got {:?}", other),
        }
        // Body prefix: let a = x[0]; let b = x[1].
        let names: Vec<&str> = then_branch
            .stmts
            .iter()
            .filter_map(|s| match s {
                Stmt::LetBinding {
                    name,
                    value: Expr::SeqIndex { .. },
                    ..
                } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(names, vec!["a", "b"], "expected element bindings a, b");
    }

    #[test]
    fn case_in_nested_array_pattern_lowers_structurally() {
        // Phase 13b (FC) — a nested array sub-pattern (`[[1], y]`) now
        // lowers structurally: the cond AND-chain contains an inner
        // `len(x[0]) == 1` check (proving recursion into `x[0]`), and the
        // body binds `y = x[1]`.
        let m = lower("x = 1\ncase x\nin [[1], y]\n  puts(y)\nend\n");
        let b = main_body(&m);
        let if_expr = extract_case_if(b);
        let (cond, then_branch) = match if_expr {
            Expr::If { cond, then_branch, .. } => (cond, then_branch),
            other => panic!("expected If, got {:?}", other),
        };
        // No __pattern_match__ marker anywhere in the cond tree.
        let printed = format!("{:?}", cond);
        assert!(
            !printed.contains("__pattern_match__"),
            "expected no marker in nested pattern cond, got {}",
            printed
        );
        // The cond must contain a nested SeqLen over a SeqIndex
        // (`len(x[0])`), which only the recursive lowering produces.
        assert!(
            printed.contains("SeqLen") && printed.contains("SeqIndex"),
            "expected nested SeqLen(SeqIndex(..)) in cond, got {}",
            printed
        );
        // Body binds `y = x[1]`.
        let has_y = then_branch.stmts.iter().any(|s| {
            matches!(s, Stmt::LetBinding { name, value, .. }
                if name == "y" && matches!(value, Expr::SeqIndex { .. }))
        });
        assert!(has_y, "expected `let y = x[1]` binding in body");
    }

    #[test]
    fn case_in_array_with_hash_element_lowers_structurally() {
        // Phase FC — a hash sub-pattern inside an array (`[{a: 1}, 2]`)
        // now lowers structurally rather than falling back to the
        // whole-pattern `__pattern_match__` marker: the cond contains both
        // the array `SeqLen`/`SeqIndex` machinery AND a `MapGet` from the
        // nested hash, and no marker remains.
        let m = lower("x = 1\ncase x\nin [{a: 1}, 2]\n  \"h\"\nend\n");
        let b = main_body(&m);
        let cond = match extract_case_if(b) {
            Expr::If { cond, .. } => cond,
            other => panic!("expected If, got {:?}", other),
        };
        let printed = format!("{:?}", cond);
        assert!(
            !printed.contains("__pattern_match__"),
            "expected no marker (hash-in-array now lowers), got {}",
            printed
        );
        assert!(
            printed.contains("SeqLen") && printed.contains("MapGet"),
            "expected both array (SeqLen) and hash (MapGet) machinery, got {}",
            printed
        );
    }

    #[test]
    fn case_in_literal_array_pattern_validates_e2e() {
        // Phase 13b (FC) — regression: a literal array pattern emits
        // `Expr::LogicalAnd`, so the module observes `Feature::ShortCircuit`
        // and the manifest must declare it (a gap not exercised through
        // the validator by the Phase 13a binding-only E2E test).
        let m = lower("x = 1\ncase x\nin [1, 2]\n  puts(x)\nend\n");
        assert!(
            m.manifest.contains(semantic_ir::Feature::ShortCircuit),
            "expected ShortCircuit feature for literal array pattern; got {:?}",
            m.manifest
        );
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected literal-array-pattern module: {:?}",
            result
        );
    }

    #[test]
    fn case_in_nested_array_pattern_validates_e2e() {
        // Phase 13b (FC) — end-to-end: a nested binding array pattern
        // lowers to real recursive SIR that round-trips the validator.
        let m = lower("x = 1\ncase x\nin [[a], b]\n  puts(a)\nend\n");
        assert!(
            m.manifest.contains(semantic_ir::Feature::Sequences),
            "expected Sequences feature; got {:?}",
            m.manifest
        );
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected nested-array-pattern module: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // Phase 23b — `defined?` operator
    //
    // `defined?(expr)` / `defined? expr` lowers to
    // `BuiltinCall("defined?", [operand])` (PURE).  Works in both
    // expression position (assignment RHS) and statement position.
    // -----------------------------------------------------------------------

    #[test]
    fn defined_with_parens_lowers_to_builtin_call() {
        // `y = defined?(x)` → LetBinding(y, BuiltinCall("defined?", [VarRef x])).
        let m = lower("x = 1\ny = defined?(x)\n");
        let b = main_body(&m);
        let value = match &b.stmts[1] {
            Stmt::LetBinding { name, value, .. } | Stmt::LetStarBinding { name, value, .. }
                if name == "y" =>
            {
                value
            }
            other => panic!("expected LetBinding y, got {:?}", other),
        };
        match value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "defined?");
                assert_eq!(args.len(), 1);
                assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "x"));
            }
            other => panic!("expected BuiltinCall(defined?, [x]), got {:?}", other),
        }
    }

    #[test]
    fn defined_of_literal_lowers_with_literal_operand() {
        // `y = defined?(1)` → operand is an IntLit (no parens-vs-bare diff
        // at the SIR level).
        let m = lower("y = defined?(1)\n");
        let b = main_body(&m);
        let value = match &b.stmts[0] {
            Stmt::LetBinding { value, .. } => value,
            other => panic!("expected LetBinding, got {:?}", other),
        };
        match value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "defined?");
                assert!(matches!(&args[0], Expr::IntLit { value: 1, .. }));
            }
            other => panic!("expected BuiltinCall(defined?, [1]), got {:?}", other),
        }
    }

    #[test]
    fn defined_in_statement_position_is_expr_stmt() {
        // `defined?(x)` as a bare statement → Stmt::ExprStmt wrapping the
        // BuiltinCall (the grammar routes it via the statement-level
        // `defined_expression` alternative, not `method_call`).
        let m = lower("x = 1\ndefined?(x)\n");
        let b = main_body(&m);
        match &b.stmts[1] {
            Stmt::ExprStmt { expr, .. } => match expr {
                Expr::BuiltinCall { name, args, .. } => {
                    assert_eq!(name, "defined?");
                    assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "x"));
                }
                other => panic!("expected BuiltinCall(defined?, …), got {:?}", other),
            },
            other => panic!("expected ExprStmt for bare defined?, got {:?}", other),
        }
    }

    #[test]
    fn defined_expression_validates_e2e() {
        // Phase 23b (FC) — end-to-end: `defined?(x)` in trailing-value
        // position (so the operand VarRef sees the prior binding) lowers
        // to a PURE BuiltinCall that round-trips the SIR validator.
        let m = lower("x = 1\ndefined?(x)\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected defined?-using module: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // Phase 24a — `alias new old` method aliasing
    //
    // `alias new old` lowers to a statement-position
    // `BuiltinCall("alias", [StrLit(new), StrLit(old)])` (PURE).  The two
    // method names are surfaced as string literals (not VarRefs): they are
    // method names, not locals, so they must not be subject to
    // unbound-variable validation.
    // -----------------------------------------------------------------------

    #[test]
    fn alias_lowers_to_builtin_call() {
        // `alias foo bar` → ExprStmt(BuiltinCall("alias", [StrLit, StrLit])).
        let m = lower("alias foo bar\n");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::ExprStmt { expr, .. } => match expr {
                Expr::BuiltinCall { name, args, .. } => {
                    assert_eq!(name, "alias");
                    assert_eq!(args.len(), 2);
                }
                other => panic!("expected BuiltinCall(alias, …), got {:?}", other),
            },
            other => panic!("expected ExprStmt for alias, got {:?}", other),
        }
    }

    #[test]
    fn alias_operands_are_string_literals() {
        // Both operands surface as StrLit carrying the verbatim method
        // names — never VarRefs.
        let m = lower("alias size length\n");
        let b = main_body(&m);
        let args = match &b.stmts[0] {
            Stmt::ExprStmt {
                expr: Expr::BuiltinCall { name, args, .. },
                ..
            } if name == "alias" => args,
            other => panic!("expected alias BuiltinCall, got {:?}", other),
        };
        assert!(
            matches!(&args[0], Expr::StrLit { value, .. } if value == "size"),
            "expected new-name StrLit(\"size\"), got {:?}",
            args[0]
        );
        assert!(
            matches!(&args[1], Expr::StrLit { value, .. } if value == "length"),
            "expected old-name StrLit(\"length\"), got {:?}",
            args[1]
        );
    }

    #[test]
    fn alias_is_pure_and_validates_e2e() {
        // Phase 24a (FC) — end-to-end: a bare `alias` statement lowers to a
        // PURE BuiltinCall over string-literal operands that round-trips
        // the SIR validator (no unbound names, no effect surprises).
        let m = lower("alias foo bar\n");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::ExprStmt {
                expr: Expr::BuiltinCall { effects, .. },
                ..
            } => assert!(
                !effects.contains(Effect::Divergent),
                "alias should be PURE (non-divergent), got {:?}",
                effects
            ),
            other => panic!("expected alias BuiltinCall, got {:?}", other),
        }
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected alias-using module: {:?}",
            result
        );
    }

    #[test]
    fn undef_lowers_to_builtin_call() {
        // `undef foo` → ExprStmt(BuiltinCall("undef", [StrLit])).
        let m = lower("undef foo\n");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::ExprStmt { expr, .. } => match expr {
                Expr::BuiltinCall { name, args, .. } => {
                    assert_eq!(name, "undef");
                    assert_eq!(args.len(), 1);
                }
                other => panic!("expected BuiltinCall(undef, …), got {:?}", other),
            },
            other => panic!("expected ExprStmt for undef, got {:?}", other),
        }
    }

    #[test]
    fn undef_operand_is_string_literal() {
        // The single operand surfaces as a StrLit carrying the verbatim
        // method name — never a VarRef.
        let m = lower("undef obsolete\n");
        let b = main_body(&m);
        let args = match &b.stmts[0] {
            Stmt::ExprStmt {
                expr: Expr::BuiltinCall { name, args, .. },
                ..
            } if name == "undef" => args,
            other => panic!("expected undef BuiltinCall, got {:?}", other),
        };
        assert!(
            matches!(&args[0], Expr::StrLit { value, .. } if value == "obsolete"),
            "expected StrLit(\"obsolete\"), got {:?}",
            args[0]
        );
    }

    #[test]
    fn undef_is_pure_and_validates_e2e() {
        // Phase 24b (FC) — end-to-end: a bare `undef` statement lowers to a
        // PURE BuiltinCall over a string-literal operand that round-trips
        // the SIR validator (no unbound names, no effect surprises).
        let m = lower("undef foo\n");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::ExprStmt {
                expr: Expr::BuiltinCall { effects, .. },
                ..
            } => assert!(
                !effects.contains(Effect::Divergent),
                "undef should be PURE (non-divergent), got {:?}",
                effects
            ),
            other => panic!("expected undef BuiltinCall, got {:?}", other),
        }
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected undef-using module: {:?}",
            result
        );
    }

    #[test]
    fn file_keyword_lowers_to_strlit() {
        // Phase 23a (FC) — `__FILE__` lowers to a compile-time StrLit
        // carrying the lowerer's `file_name` (here the test module name
        // "test"), surfaced as the argument of the enclosing `puts`.
        let m = lower("puts(__FILE__)\n");
        let b = main_body(&m);
        // A lone `puts(...)` is the block's trailing VALUE, not a stmt.
        let args = match &b.value {
            Expr::BuiltinCall { name, args, .. } if name == "puts" => args,
            other => panic!("expected BuiltinCall(puts, ...), got {:?}", other),
        };
        assert!(
            matches!(&args[0], Expr::StrLit { value, .. } if value == "test"),
            "expected `__FILE__` to lower to StrLit(\"test\"), got {:?}",
            args[0]
        );
    }

    #[test]
    fn file_keyword_declares_strings_feature() {
        // Emitting the StrLit means the module must declare the `strings`
        // feature — verify the manifest carries it.
        let m = lower("puts(__FILE__)\n");
        assert!(
            m.manifest.contains(semantic_ir::Feature::Strings),
            "expected Strings feature for `__FILE__`; got {:?}",
            m.manifest
        );
    }

    #[test]
    fn file_keyword_validates_e2e() {
        // End-to-end: a `__FILE__`-using module round-trips the SIR
        // validator (the StrLit is a self-contained literal — no unbound
        // names, no undeclared features).
        let m = lower("puts(__FILE__)\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected `__FILE__`-using module: {:?}",
            result
        );
    }

    #[test]
    fn file_keyword_shadowed_by_local_is_varref() {
        // A `__FILE__` shadowed by a prior local binding keeps the local
        // (mirrors the bare-`raise` shadow guard): the read lowers to a
        // VarRef, NOT the file-name StrLit.
        let m = lower("__FILE__ = 1\nputs(__FILE__)\n");
        let b = main_body(&m);
        // The `__FILE__ = 1` binding is a stmt; the trailing `puts(...)`
        // is the block VALUE, where the shadowed read appears.
        let args = match &b.value {
            Expr::BuiltinCall { name, args, .. } if name == "puts" => args,
            other => panic!("expected BuiltinCall(puts, ...), got {:?}", other),
        };
        assert!(
            matches!(&args[0], Expr::VarRef { name, .. } if name == "__FILE__"),
            "expected shadowed `__FILE__` to stay a VarRef, got {:?}",
            args[0]
        );
    }

    #[test]
    fn line_keyword_lowers_to_intlit() {
        // Phase 23c (FC) — `__LINE__` lowers to a compile-time IntLit
        // carrying the token's 1-based source line, surfaced as the
        // argument of the enclosing `puts`.  A lone `puts(...)` is the
        // block's trailing VALUE, not a stmt.
        let m = lower("puts(__LINE__)\n");
        let b = main_body(&m);
        let args = match &b.value {
            Expr::BuiltinCall { name, args, .. } if name == "puts" => args,
            other => panic!("expected BuiltinCall(puts, ...), got {:?}", other),
        };
        assert!(
            matches!(&args[0], Expr::IntLit { value, .. } if *value == 1),
            "expected `__LINE__` on line 1 to lower to IntLit(1), got {:?}",
            args[0]
        );
    }

    #[test]
    fn line_keyword_tracks_source_line() {
        // The IntLit value is the actual (1-based) line of the `__LINE__`
        // token — here line 2, after a leading statement — proving it is
        // not a hard-coded constant.
        let m = lower("x = 1\nputs(__LINE__)\n");
        let b = main_body(&m);
        let args = match &b.value {
            Expr::BuiltinCall { name, args, .. } if name == "puts" => args,
            other => panic!("expected BuiltinCall(puts, ...), got {:?}", other),
        };
        assert!(
            matches!(&args[0], Expr::IntLit { value, .. } if *value == 2),
            "expected `__LINE__` on line 2 to lower to IntLit(2), got {:?}",
            args[0]
        );
    }

    #[test]
    fn line_keyword_validates_e2e() {
        // End-to-end: a `__LINE__`-using module round-trips the SIR
        // validator (the IntLit is a self-contained literal — no unbound
        // names, no feature requirement since integers are baseline).
        let m = lower("puts(__LINE__)\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected `__LINE__`-using module: {:?}",
            result
        );
    }

    #[test]
    fn line_keyword_shadowed_by_local_is_varref() {
        // A `__LINE__` shadowed by a prior local binding keeps the local
        // (mirrors the `__FILE__` / bare-`raise` shadow guards): the read
        // lowers to a VarRef, NOT the line-number IntLit.
        let m = lower("__LINE__ = 7\nputs(__LINE__)\n");
        let b = main_body(&m);
        let args = match &b.value {
            Expr::BuiltinCall { name, args, .. } if name == "puts" => args,
            other => panic!("expected BuiltinCall(puts, ...), got {:?}", other),
        };
        assert!(
            matches!(&args[0], Expr::VarRef { name, .. } if name == "__LINE__"),
            "expected shadowed `__LINE__` to stay a VarRef, got {:?}",
            args[0]
        );
    }

    #[test]
    fn dir_keyword_lowers_to_strlit() {
        // Phase 23d (FC) — `__dir__` lowers to a compile-time StrLit
        // carrying the directory portion of the lowerer's `file_name`.
        // The test module name is "test" (no path separator), so the
        // directory is the conventional ".".
        let m = lower("puts(__dir__)\n");
        let b = main_body(&m);
        let args = match &b.value {
            Expr::BuiltinCall { name, args, .. } if name == "puts" => args,
            other => panic!("expected BuiltinCall(puts, ...), got {:?}", other),
        };
        assert!(
            matches!(&args[0], Expr::StrLit { value, .. } if value == "."),
            "expected `__dir__` to lower to StrLit(\".\"), got {:?}",
            args[0]
        );
    }

    #[test]
    fn dir_keyword_declares_strings_feature() {
        // Emitting the StrLit means the module must declare the `strings`
        // feature — verify the manifest carries it.
        let m = lower("puts(__dir__)\n");
        assert!(
            m.manifest.contains(semantic_ir::Feature::Strings),
            "expected Strings feature for `__dir__`; got {:?}",
            m.manifest
        );
    }

    #[test]
    fn dir_keyword_validates_e2e() {
        // End-to-end: a `__dir__`-using module round-trips the SIR
        // validator (the StrLit is a self-contained literal).
        let m = lower("puts(__dir__)\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected `__dir__`-using module: {:?}",
            result
        );
    }

    #[test]
    fn dir_keyword_shadowed_by_local_is_varref() {
        // A `__dir__` shadowed by a prior local binding keeps the local
        // (mirrors the sibling shadow guards): the read lowers to a
        // VarRef, NOT the directory StrLit.
        let m = lower("__dir__ = 1\nputs(__dir__)\n");
        let b = main_body(&m);
        let args = match &b.value {
            Expr::BuiltinCall { name, args, .. } if name == "puts" => args,
            other => panic!("expected BuiltinCall(puts, ...), got {:?}", other),
        };
        assert!(
            matches!(&args[0], Expr::VarRef { name, .. } if name == "__dir__"),
            "expected shadowed `__dir__` to stay a VarRef, got {:?}",
            args[0]
        );
    }

    #[test]
    fn using_lowers_to_builtin_call() {
        // Phase 26a (FC) — `using Mod` lowers to a statement-position
        // BuiltinCall("using", [<module>]) rather than an unknown
        // DirectCall.  A lone trailing call is the block's VALUE.
        let m = lower("Foo = 1\nusing Foo\n");
        let b = main_body(&m);
        match &b.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "using");
                assert_eq!(args.len(), 1, "expected exactly the module arg");
            }
            other => panic!("expected BuiltinCall(using, …), got {:?}", other),
        }
    }

    #[test]
    fn using_operand_is_the_module_ref() {
        // The sole argument is the refinement module, lowered through the
        // normal expression path — here a constant reference `Foo`.
        let m = lower("Foo = 1\nusing Foo\n");
        let b = main_body(&m);
        let args = match &b.value {
            Expr::BuiltinCall { name, args, .. } if name == "using" => args,
            other => panic!("expected using BuiltinCall, got {:?}", other),
        };
        assert!(
            matches!(&args[0], Expr::VarRef { name, .. } if name == "Foo"),
            "expected module operand VarRef(\"Foo\"), got {:?}",
            args[0]
        );
    }

    #[test]
    fn using_is_pure_and_validates_e2e() {
        // End-to-end: a `using`-activated module round-trips the SIR
        // validator — the call is no longer an undeclared DirectCall, and
        // the constant operand is declared by the preceding assignment.
        let m = lower("Foo = 1\nusing Foo\n");
        match &main_body(&m).value {
            Expr::BuiltinCall { effects, .. } => assert!(
                !effects.contains(Effect::Divergent),
                "using should be PURE (non-divergent), got {:?}",
                effects
            ),
            other => panic!("expected using BuiltinCall, got {:?}", other),
        }
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected `using`-using module: {:?}",
            result
        );
    }

    #[test]
    fn refine_lowers_to_builtin_call() {
        // Phase 26b (FC) — `refine(Class) do … end` lowers to a
        // statement-position BuiltinCall("refine", [<class>, <closure>])
        // rather than an unknown DirectCall.  A lone trailing call is the
        // block's VALUE.
        let m = lower("String = 1\nrefine(String) do\n  1\nend\n");
        let b = main_body(&m);
        // A block-taking call at statement position lowers to an ExprStmt
        // (not the block's trailing VALUE like a paren-less call does).
        let call = refine_call(b);
        match call {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "refine");
                assert_eq!(
                    args.len(),
                    2,
                    "expected [target class, refinement closure]"
                );
            }
            other => panic!("expected BuiltinCall(refine, …), got {:?}", other),
        }
    }

    /// Locate the `refine` BuiltinCall among a block's ExprStmts.
    fn refine_call(b: &semantic_ir::Block) -> &Expr {
        b.stmts
            .iter()
            .find_map(|s| match s {
                Stmt::ExprStmt {
                    expr: e @ Expr::BuiltinCall { name, .. },
                    ..
                } if name == "refine" => Some(e),
                _ => None,
            })
            .expect("expected a refine BuiltinCall ExprStmt")
    }

    #[test]
    fn refine_block_is_makeclosure_arg() {
        // The refinement body is hoisted to a `MakeClosure` trailing arg
        // (the same shape `lower_method_with_block` uses for any
        // block-taking call).
        let m = lower("String = 1\nrefine(String) do\n  1\nend\n");
        let b = main_body(&m);
        let args = match refine_call(b) {
            Expr::BuiltinCall { args, .. } => args,
            other => panic!("expected refine BuiltinCall, got {:?}", other),
        };
        assert!(
            matches!(&args[1], Expr::MakeClosure { .. }),
            "expected the refinement block to lower to MakeClosure, got {:?}",
            args[1]
        );
    }

    #[test]
    fn refine_is_pure_and_validates_e2e() {
        // End-to-end: a `refine`-using module round-trips the SIR
        // validator — the call is a first-class PURE builtin (not an
        // undeclared DirectCall), the target class is declared by the
        // preceding assignment, and the hoisted refinement closure is a
        // well-formed function.
        let m = lower("String = 1\nrefine(String) do\n  1\nend\n");
        match refine_call(main_body(&m)) {
            Expr::BuiltinCall { effects, .. } => assert!(
                !effects.contains(Effect::Divergent),
                "refine should be PURE (non-divergent), got {:?}",
                effects
            ),
            other => panic!("expected refine BuiltinCall, got {:?}", other),
        }
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected `refine`-using module: {:?}",
            result
        );
    }

    #[test]
    fn case_in_array_pattern_validates_e2e() {
        // Phase 13a (FC) — end-to-end: a binding array pattern lowers to
        // real SIR (`SeqLen`/`SeqIndex` + `LetBinding`) that round-trips
        // the SIR validator, and declares `Feature::Sequences`.
        let m = lower("x = 1\ncase x\nin [a, b]\n  puts(a)\nend\n");
        assert!(
            m.manifest.contains(semantic_ir::Feature::Sequences),
            "expected Sequences feature; got {:?}",
            m.manifest
        );
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected array-pattern module: {:?}",
            result
        );
    }

    #[test]
    fn case_in_hash_pattern_binding_emits_mapget_letbinding() {
        // Phase FC — `in {name: y}` lowers structurally: the body binds
        // `y = x[:name]` via `MapGet`, and no `__pattern_match__` marker
        // remains in the condition.
        let m = lower("x = {name: 1}\ncase x\nin {name: y}\n  puts(y)\nend\n");
        let b = main_body(&m);
        let (cond, then_branch) = match extract_case_if(b) {
            Expr::If { cond, then_branch, .. } => (cond, then_branch),
            other => panic!("expected If, got {:?}", other),
        };
        let printed = format!("{:?}", cond);
        assert!(
            !printed.contains("__pattern_match__"),
            "expected no marker in hash-pattern cond, got {}",
            printed
        );
        let has_y = then_branch.stmts.iter().any(|s| {
            matches!(s, Stmt::LetBinding { name, value, .. }
                if name == "y" && matches!(value, Expr::MapGet { .. }))
        });
        assert!(has_y, "expected `let y = x[:name]` (MapGet) binding in body");
    }

    #[test]
    fn case_in_hash_pattern_literal_emits_equality_on_mapget() {
        // `in {age: 30}` → the cond ANDs in `x[:age] == 30` (an equality
        // BuiltinCall over a MapGet), with no marker.
        let m = lower("x = {age: 1}\ncase x\nin {age: 30}\n  \"m\"\nend\n");
        let b = main_body(&m);
        let cond = match extract_case_if(b) {
            Expr::If { cond, .. } => cond,
            other => panic!("expected If, got {:?}", other),
        };
        let printed = format!("{:?}", cond);
        assert!(
            printed.contains("MapGet"),
            "expected MapGet in cond, got {}",
            printed
        );
        assert!(
            printed.contains("\"==\""),
            "expected an equality check in cond, got {}",
            printed
        );
        assert!(!printed.contains("__pattern_match__"));
    }

    #[test]
    fn case_in_hash_pattern_shorthand_binds() {
        // Ruby 3.1 shorthand `{name:}` now binds `name = x[:name]`
        // (previously a deferred no-op).
        let m = lower("x = {name: 1}\ncase x\nin {name:}\n  puts(name)\nend\n");
        let b = main_body(&m);
        let then_branch = match extract_case_if(b) {
            Expr::If { then_branch, .. } => then_branch,
            other => panic!("expected If, got {:?}", other),
        };
        let has_name = then_branch.stmts.iter().any(|s| {
            matches!(s, Stmt::LetBinding { name, value, .. }
                if name == "name" && matches!(value, Expr::MapGet { .. }))
        });
        assert!(
            has_name,
            "expected shorthand `{{name:}}` to bind name = x[:name]"
        );
    }

    #[test]
    fn case_in_hash_pattern_validates_e2e() {
        // End-to-end: a hash-pattern module declares `Feature::Maps` and
        // round-trips the SIR validator.
        let m = lower("x = {name: 1}\ncase x\nin {name: y}\n  puts(y)\nend\n");
        assert!(
            m.manifest.contains(semantic_ir::Feature::Maps),
            "expected Maps feature for hash pattern; got {:?}",
            m.manifest
        );
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected hash-pattern module: {:?}",
            result
        );
    }

    #[test]
    fn case_in_with_else_clause_emits_else_branch() {
        // `case x; in 1; "one"; else; "other"; end` — else clause is
        // the innermost If's else_branch.
        let m = lower("x = 2\ncase x\nin 1\n  \"one\"\nelse\n  \"other\"\nend\n");
        let b = main_body(&m);
        let if_expr = extract_case_if(b);
        let else_branch = match if_expr {
            Expr::If { else_branch, .. } => else_branch,
            other => panic!("expected If, got {:?}", other),
        };
        assert!(
            matches!(&else_branch.value, Expr::StrLit { value, .. } if value == "other"),
            "expected StrLit(\"other\") in else branch, got {:?}",
            else_branch.value
        );
    }

    // -----------------------------------------------------------------------
    // Phase 7e — Ruby 3.0 rightward assignment `expr => var`
    //
    // Lowers identically to `var = expr` — LetBinding on first sight of
    // the name, Assign on re-bind (with Feature::MutableBindings).
    // -----------------------------------------------------------------------

    #[test]
    fn rightward_assignment_lowers_to_let_binding_on_first_sight() {
        // `1 + 2 => sum` → LetBinding(sum, BuiltinCall("+", [1, 2])).
        let m = lower("1 + 2 => sum");
        let b = main_body(&m);
        match &b.stmts[0] {
            Stmt::LetBinding { name, value, .. } => {
                assert_eq!(name, "sum");
                match value {
                    Expr::BuiltinCall { name, args, .. } => {
                        assert_eq!(name, "+");
                        assert_eq!(args.len(), 2);
                        assert!(matches!(&args[0], Expr::IntLit { value: 1, .. }));
                        assert!(matches!(&args[1], Expr::IntLit { value: 2, .. }));
                    }
                    other => panic!("expected +-BuiltinCall, got {:?}", other),
                }
            }
            other => panic!("expected LetBinding, got {:?}", other),
        }
    }

    #[test]
    fn rightward_assignment_with_literal_lowers_to_int_let_binding() {
        // `42 => x` → LetBinding(x, IntLit(42)).
        let m = lower("42 => x");
        let b = main_body(&m);
        assert!(matches!(
            &b.stmts[0],
            Stmt::LetBinding { name, value: Expr::IntLit { value: 42, .. }, .. } if name == "x"
        ));
    }

    #[test]
    fn rightward_assignment_rebind_emits_assign_with_mutable_bindings_feature() {
        // `x = 1; 2 => x` — second statement is a re-bind, so it must
        // emit Stmt::Assign (not LetBinding) and the module manifest
        // must declare Feature::MutableBindings.
        let m = lower("x = 1\n2 => x\n");
        let b = main_body(&m);
        // stmts[0] = LetBinding(x, 1)
        // stmts[1] = Assign(x, 2) — the rightward re-bind
        match &b.stmts[1] {
            Stmt::Assign { name, value, .. } => {
                assert_eq!(name, "x");
                assert!(matches!(value, Expr::IntLit { value: 2, .. }));
            }
            other => panic!("expected Assign on re-bind, got {:?}", other),
        }
        assert!(
            m.manifest.contains(semantic_ir::Feature::MutableBindings),
            "expected MutableBindings feature in manifest after a re-bind"
        );
    }

    #[test]
    fn rightward_assignment_module_passes_sir_validator() {
        // End-to-end smoke: a module with a rightward-assign statement
        // passes the SIR validator.  Bound name must be visible to
        // subsequent statements.
        let m = lower("1 + 2 => sum\nputs(sum)\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected rightward-assign module: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------
    // Phase 7f — Ruby 3.1 hash value-omitted shorthand `{x:, y:}`.
    //
    // When a hash entry omits its value (`NAME COLON` with no trailing
    // expression), the lowerer must emit a `MapEntry` whose key is
    // `SymLit(name)` (same as the explicit `NAME COLON expression`
    // form) AND whose value is `VarRef(name, scope)`, where `scope`
    // follows the same Param-vs-Local dispatch used by bare-name
    // factor lowering.
    //
    // Five tests below cover (1) basic shorthand emits VarRef value,
    // (2) the value's scope follows current_params, (3) mixed
    // shorthand + explicit forms keep their respective shapes, (4) the
    // existing `{x: 1}` form is unchanged (regression), and (5) an
    // end-to-end validator smoke that exercises name resolution.
    // -----------------------------------------------------------------

    #[test]
    fn hash_value_shorthand_emits_var_ref_value() {
        // `name = "ada"; {name:}` — the shorthand entry's value should
        // be VarRef("name", Local), not the SymLit it would be for the
        // key.  Key is SymLit("name") as usual.
        let m = lower("name = \"ada\"\nh = {name:}\n");
        let main = main_body(&m);
        let entries = main
            .stmts
            .iter()
            .find_map(|s| match s {
                Stmt::LetBinding { name: n, value: Expr::MapLit { entries, .. }, .. }
                | Stmt::LetStarBinding { name: n, value: Expr::MapLit { entries, .. }, .. }
                    if n == "h" =>
                {
                    Some(entries)
                }
                _ => None,
            })
            .expect("expected MapLit bound to h");
        assert_eq!(entries.len(), 1);
        assert!(
            matches!(&entries[0].key, Expr::SymLit { name, .. } if name == "name"),
            "key should be SymLit(\"name\"), got {:?}",
            entries[0].key
        );
        assert!(
            matches!(
                &entries[0].value,
                Expr::VarRef { name, scope, .. }
                    if name == "name" && *scope == Scope::Local
            ),
            "value should be VarRef(\"name\", Local), got {:?}",
            entries[0].value
        );
    }

    #[test]
    fn hash_value_shorthand_inside_method_uses_param_scope() {
        // Inside `def f(x); {x:}; end`, the shorthand value `x` should
        // be VarRef("x", Param) — same scoping rule as bare-name
        // factor references.
        let m = lower("def f(x)\n  {x:}\nend\n");
        // Locate the `f` function definition.
        let f = m
            .functions
            .iter()
            .find(|f| f.name == "f")
            .expect("expected def f/1 in module");
        // The hash literal is the tail expression of f's body — it
        // appears as Block.value (Expr::MapLit) directly because the
        // single-expression method has no preceding statements.
        let entries = match &f.body.value {
            Expr::MapLit { entries, .. } => entries,
            other => panic!("expected MapLit as tail value of f, got {:?}", other),
        };
        assert_eq!(entries.len(), 1);
        assert!(
            matches!(
                &entries[0].value,
                Expr::VarRef { name, scope, .. }
                    if name == "x" && *scope == Scope::Param
            ),
            "value should be VarRef(\"x\", Param) inside method body, got {:?}",
            entries[0].value
        );
    }

    #[test]
    fn hash_value_shorthand_mixed_with_explicit_form() {
        // `name = "ada"; {name:, age: 30}` — first entry's value is
        // VarRef (shorthand), second entry's value is IntLit
        // (explicit `NAME COLON expression`).  Verifies the lowerer
        // correctly dispatches per-entry inside a single hash literal.
        let m = lower("name = \"ada\"\nh = {name:, age: 30}\n");
        let main = main_body(&m);
        let entries = main
            .stmts
            .iter()
            .find_map(|s| match s {
                Stmt::LetBinding { name: n, value: Expr::MapLit { entries, .. }, .. }
                | Stmt::LetStarBinding { name: n, value: Expr::MapLit { entries, .. }, .. }
                    if n == "h" =>
                {
                    Some(entries)
                }
                _ => None,
            })
            .expect("expected MapLit bound to h");
        assert_eq!(entries.len(), 2);
        assert!(
            matches!(
                &entries[0].value,
                Expr::VarRef { name, .. } if name == "name"
            ),
            "first entry value should be VarRef(\"name\"), got {:?}",
            entries[0].value
        );
        assert!(
            matches!(&entries[1].value, Expr::IntLit { value: 30, .. }),
            "second entry value should be IntLit(30), got {:?}",
            entries[1].value
        );
    }

    #[test]
    fn hash_explicit_form_unchanged_after_phase_7f() {
        // Regression: extending hash_entry with `NAME COLON` must NOT
        // change the SIR shape for the existing `{x: 1, y: 2}` form.
        // Each value must still be an IntLit, not a VarRef.
        let m = lower("h = {x: 1, y: 2}\n");
        let main = main_body(&m);
        let entries = main
            .stmts
            .iter()
            .find_map(|s| match s {
                Stmt::LetBinding { value: Expr::MapLit { entries, .. }, .. } => Some(entries),
                _ => None,
            })
            .expect("expected MapLit");
        assert_eq!(entries.len(), 2);
        assert!(matches!(&entries[0].value, Expr::IntLit { value: 1, .. }));
        assert!(matches!(&entries[1].value, Expr::IntLit { value: 2, .. }));
    }

    #[test]
    fn hash_value_shorthand_module_passes_sir_validator() {
        // End-to-end smoke: a module that binds `name`, then builds
        // `{name:}` in tail position, must pass the SIR validator —
        // the VarRef generated for the shorthand value must
        // successfully resolve to the prior LetBinding.
        //
        // We use tail-expression position (not another LetBinding) so
        // that the parallel-let validator sees `name` in scope before
        // the shorthand's RHS is checked — same workaround pattern as
        // the Phase 6y interpolation smoke test.
        let m = lower("name = \"ada\"\nputs({name:})\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected hash-shorthand module: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------
    // Phase 8a (FC) — additional compound-assignment lowering.
    //
    // Six new compound-assign forms are accepted by the lexer/grammar
    // and lowered by reusing the existing `lower_assignment` desugar
    // path: `x %= y` → `x = x % y`, `x **= y` → `x = x ** y`, etc.
    // The lowerer emits `Stmt::Assign` (never LetBinding) because
    // op-assign always reads-then-writes; the validator therefore
    // requires `x` to be pre-declared in the same block.
    //
    // Five tests below cover (1) modulo `%=` emits `Assign` with
    // `BuiltinCall("%")` value, (2) `**=` emits `Assign` with
    // `BuiltinCall("**")` value, (3) `<<=` likewise, (4) the three
    // bitwise compounds `&= |= ^=` all desugar correctly, and
    // (5) a validator E2E smoke that declares `x`, then re-binds via
    // each new compound form in turn.
    // -----------------------------------------------------------------

    #[test]
    fn modulo_assign_desugars_to_assign_with_modulo_builtin() {
        let m = lower("x = 10\nx %= 3\n");
        let b = main_body(&m);
        // stmts[0] = LetBinding(x, 10); stmts[1] = Assign(x, 10 % 3)
        match &b.stmts[1] {
            Stmt::Assign { name, value, .. } => {
                assert_eq!(name, "x");
                match value {
                    Expr::BuiltinCall { name: op, args, .. } => {
                        assert_eq!(op, "%");
                        assert_eq!(args.len(), 2);
                        assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "x"));
                        assert!(matches!(&args[1], Expr::IntLit { value: 3, .. }));
                    }
                    other => panic!("expected BuiltinCall(`%`), got {:?}", other),
                }
            }
            other => panic!("expected Stmt::Assign at stmts[1], got {:?}", other),
        }
        assert!(m.manifest.contains(semantic_ir::Feature::MutableBindings));
    }

    #[test]
    fn power_assign_desugars_to_assign_with_power_builtin() {
        let m = lower("x = 2\nx **= 8\n");
        let b = main_body(&m);
        match &b.stmts[1] {
            Stmt::Assign { value: Expr::BuiltinCall { name, args, .. }, .. } => {
                assert_eq!(name, "**");
                assert_eq!(args.len(), 2);
                assert!(matches!(&args[1], Expr::IntLit { value: 8, .. }));
            }
            other => panic!("expected Assign(BuiltinCall(**)), got {:?}", other),
        }
    }

    #[test]
    fn left_shift_assign_desugars_to_assign_with_lshift_builtin() {
        let m = lower("x = 1\nx <<= 4\n");
        let b = main_body(&m);
        match &b.stmts[1] {
            Stmt::Assign { value: Expr::BuiltinCall { name, args, .. }, .. } => {
                assert_eq!(name, "<<");
                assert_eq!(args.len(), 2);
                assert!(matches!(&args[1], Expr::IntLit { value: 4, .. }));
            }
            other => panic!("expected Assign(BuiltinCall(<<)), got {:?}", other),
        }
    }

    #[test]
    fn bitwise_op_assigns_lower_to_assign_with_bitwise_builtins() {
        // Three statements covering `&=`, `|=`, `^=` — each must
        // desugar to `Stmt::Assign` with the correct builtin name.
        let m = lower("x = 7\nx &= 1\nx |= 2\nx ^= 4\n");
        let b = main_body(&m);
        let expected = ["&", "|", "^"];
        for (i, op) in expected.iter().enumerate() {
            let stmt = &b.stmts[i + 1];
            match stmt {
                Stmt::Assign { value: Expr::BuiltinCall { name, .. }, .. } => {
                    assert_eq!(name, op, "expected `{op}` at stmts[{}]", i + 1);
                }
                other => panic!("expected Assign(BuiltinCall(`{op}`)), got {:?}", other),
            }
        }
    }

    #[test]
    fn compound_assigns_module_passes_sir_validator() {
        // End-to-end smoke: declare `x`, then drive every new
        // compound-assign in turn.  The validator must accept the
        // resulting module — every Assign reads a previously bound
        // `x`, and `MutableBindings` is in the manifest.
        let m = lower("x = 1\nx %= 1\nx **= 1\nx <<= 1\nx &= 1\nx |= 1\nx ^= 1\nputs(x)\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected compound-assign module: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------
    // Phase 8a-2 (FC) — right-shift compound assign `>>=`.
    //
    // The lexer now folds `>>` and `>>=` into single Name tokens,
    // the grammar accepts `>>=` in the assignment rule's alternation,
    // and the lowerer routes it through the same desugar path as
    // `<<=`: `x >>= y` → `x = x >> y` → `Assign(x, BuiltinCall(">>",
    // [VarRef(x), y]))`.
    //
    // Two tests below: shape assertion and validator E2E smoke.
    // -----------------------------------------------------------------

    #[test]
    fn right_shift_assign_desugars_to_assign_with_rshift_builtin() {
        let m = lower("x = 16\nx >>= 2\n");
        let b = main_body(&m);
        match &b.stmts[1] {
            Stmt::Assign { value: Expr::BuiltinCall { name, args, .. }, .. } => {
                assert_eq!(name, ">>");
                assert_eq!(args.len(), 2);
                assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "x"));
                assert!(matches!(&args[1], Expr::IntLit { value: 2, .. }));
            }
            other => panic!("expected Assign(BuiltinCall(>>)), got {:?}", other),
        }
        assert!(m.manifest.contains(semantic_ir::Feature::MutableBindings));
    }

    #[test]
    fn right_shift_assign_module_passes_sir_validator() {
        // End-to-end smoke pairing left- and right-shift compounds.
        let m = lower("x = 1\nx <<= 4\nx >>= 2\nputs(x)\n");
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected shift-assign module: {:?}",
            result
        );
    }

    // ─────────────────────────────────────────────────────────────────
    // Milestone O2 — OOP production.  The Ruby frontend now emits the
    // OOP wiring (method registration, `.new`, `super`, `self`,
    // `attr_accessor`) so object-oriented Ruby executes end to end.
    // These assert the emitted SIR *shape*; the execution-proofs
    // (P1/P2/P3) live in the semantic-ir-to-python crate, which can run
    // the emitted Python through a real interpreter.
    // ─────────────────────────────────────────────────────────────────

    /// Find the single top-level `Stmt::ExprStmt` in `main` whose expression is
    /// a `BuiltinCall` named `builtin`, returning its args.  Panics if absent.
    fn find_builtin_stmt<'a>(
        m: &'a semantic_ir::Module,
        builtin: &str,
    ) -> &'a [Expr] {
        for s in &main_body(m).stmts {
            if let Stmt::ExprStmt { expr: Expr::BuiltinCall { name, args, .. }, .. } = s {
                if name == builtin {
                    return args;
                }
            }
        }
        panic!(
            "no top-level `{}` BuiltinCall in main; stmts = {:?}",
            builtin,
            main_body(m).stmts
        );
    }

    /// Issue #59 — find the `__super__` `BuiltinCall` args in a function body,
    /// searching BOTH the statement list and the tail `value` slot.  `super` is
    /// now an expression, so a method whose sole/last line is `super` puts the
    /// `__super__` call in `body.value`, not `body.stmts`.
    fn find_super_args(f: &semantic_ir::Function) -> Option<&[Expr]> {
        for s in &f.body.stmts {
            if let Stmt::ExprStmt { expr: Expr::BuiltinCall { name, args, .. }, .. } = s {
                if name == "__super__" {
                    return Some(args);
                }
            }
        }
        if let Expr::BuiltinCall { name, args, .. } = &f.body.value {
            if name == "__super__" {
                return Some(args);
            }
        }
        None
    }

    // -----------------------------------------------------------------------
    // Issue #59 — class-method defs (`def self.m`) + class-method calls
    // (`Foo.bar`), plus `super` in expression position.
    // -----------------------------------------------------------------------

    #[test]
    fn def_self_method_registers_as_class_method() {
        // `class Counter; def self.zero; 0; end; end` — the receiver-bearing
        // `def self.zero` must register in the CLASS-method table via
        // `__def_class_method__("Counter", "zero", MakeClosure(Counter__zero_cm))`
        // (NOT `__def_method__`), and hoist under a `_cm`-suffixed name.
        let m = lower("class Counter\n  def self.zero\n    0\n  end\nend\n");
        // Hoisted under the class-method-qualified name.
        assert!(
            m.functions.iter().any(|f| f.name == "Counter__zero_cm"),
            "expected class-method-qualified `Counter__zero_cm`; got {:?}",
            m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        // A `__def_class_method__` registration exists (and NO `__def_method__`
        // for `zero`).
        let regs: Vec<(&str, &str)> = main_body(&m)
            .stmts
            .iter()
            .filter_map(|s| match s {
                Stmt::ExprStmt { expr: Expr::BuiltinCall { name, args, .. }, .. } => {
                    let meth = match args.get(1) {
                        Some(Expr::StrLit { value, .. }) => value.as_str(),
                        _ => "",
                    };
                    Some((name.as_str(), meth))
                }
                _ => None,
            })
            .collect();
        assert!(
            regs.contains(&("__def_class_method__", "zero")),
            "expected __def_class_method__ for `zero`; got {:?}",
            regs
        );
        assert!(
            !regs.iter().any(|(n, meth)| *n == "__def_method__" && *meth == "zero"),
            "class method `zero` must NOT register as an instance method"
        );
    }

    #[test]
    fn class_method_call_lowers_to_class_method_dispatch() {
        // `Counter.zero` — a NON-`new` method call on a CONSTANT receiver is a
        // class-method dispatch: `__class_method__("Counter", "zero")`.
        let m = lower("Counter.zero\n");
        match &main_body(&m).value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "__class_method__");
                assert!(matches!(&args[0], Expr::StrLit { value, .. } if value == "Counter"));
                assert!(matches!(&args[1], Expr::StrLit { value, .. } if value == "zero"));
            }
            other => panic!("expected __class_method__ dispatch, got {:?}", other),
        }
    }

    #[test]
    fn class_method_call_with_args_forwards_them() {
        // `Foo.bar(x)` → `__class_method__("Foo", "bar", VarRef(x))`.
        let m = lower("Foo.bar(1)\n");
        match &main_body(&m).value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "__class_method__");
                assert_eq!(args.len(), 3, "class, method, then the forwarded arg");
                assert!(matches!(&args[2], Expr::IntLit { value: 1, .. }));
            }
            other => panic!("expected __class_method__ dispatch, got {:?}", other),
        }
    }

    #[test]
    fn dot_new_on_const_still_routes_to_new_not_class_method() {
        // Regression guard — `Foo.new` is the implicit constructor class method
        // and must still lower to `__new__`, NOT `__class_method__`.
        let m = lower("Foo.new\n");
        match &main_body(&m).value {
            Expr::BuiltinCall { name, .. } => {
                assert_eq!(name, "__new__", "`.new` must route to __new__");
            }
            other => panic!("expected __new__, got {:?}", other),
        }
    }

    #[test]
    fn instance_method_call_on_local_still_uses_method_envelope() {
        // Regression guard — a method call on a NON-constant receiver (`obj.m`)
        // must still route through `__method__`, not `__class_method__`.
        let m = lower("obj = 1\nobj.foo\n");
        match &main_body(&m).value {
            Expr::BuiltinCall { name, .. } => {
                assert_eq!(name, "__method__", "instance dispatch stays __method__");
            }
            other => panic!("expected __method__, got {:?}", other),
        }
    }

    #[test]
    fn super_as_subexpression_lowers_to_super_builtin() {
        // Issue #59 — `super` used as a SUB-expression (`x = super + 1`) lowers
        // to `+`( __super__(…), 1 ) — the `__super__` sits in expression
        // position inside the binary op.
        let m = lower(
            "class Cat\n  def describe\n    result = super + 1\n    result\n  end\nend\n",
        );
        let f = m
            .functions
            .iter()
            .find(|f| f.name == "Cat__describe")
            .expect("Cat__describe");
        // The `result = super + 1` binding: RHS is `+`(super, 1).
        let has_super_in_binop = f.body.stmts.iter().any(|s| {
            let rhs = match s {
                Stmt::LetBinding { value, .. } => Some(value),
                Stmt::Assign { value, .. } => Some(value),
                _ => None,
            };
            matches!(
                rhs,
                Some(Expr::BuiltinCall { name, args, .. })
                    if name == "+"
                        && matches!(&args[0], Expr::BuiltinCall { name: n, .. } if n == "__super__")
            )
        });
        assert!(
            has_super_in_binop,
            "expected `super + 1` to lower to +(__super__(…), 1); body = {:?}",
            f.body.stmts
        );
    }

    #[test]
    fn def_self_endless_method_registers_as_class_method() {
        // Endless form `def self.zero = 0` also registers as a class method.
        let m = lower("class Counter\n  def self.zero = 0\nend\n");
        assert!(
            m.functions.iter().any(|f| f.name == "Counter__zero_cm"),
            "expected `Counter__zero_cm`; got {:?}",
            m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn class_def_emits_method_registration() {
        // `class Dog; def speak; end; end` produces a `ClassDef` for Dog PLUS a
        // `__def_method__("Dog", "speak", MakeClosure(Dog__speak))` registration
        // right after it, and the method hoists under the class-qualified name.
        let m = lower("class Dog\n  def speak\n    1\n  end\nend\n");

        // The hoisted method is class-qualified (collision-safe).
        assert!(
            m.functions.iter().any(|f| f.name == "Dog__speak"),
            "expected class-qualified hoisted `Dog__speak`; got {:?}",
            m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );

        // main[0] is the ClassDef; a registration follows it.
        let stmts = &main_body(&m).stmts;
        assert!(
            matches!(&stmts[0], Stmt::ClassDef { name, .. } if name == "Dog"),
            "main[0] should be ClassDef(Dog); got {:?}",
            stmts[0]
        );
        let args = find_builtin_stmt(&m, "__def_method__");
        // args = [StrLit("Dog"), StrLit("speak"), MakeClosure{Dog__speak}]
        assert_eq!(args.len(), 3, "def_method takes (class, method, closure)");
        assert!(matches!(&args[0], Expr::StrLit { value, .. } if value == "Dog"));
        assert!(matches!(&args[1], Expr::StrLit { value, .. } if value == "speak"));
        assert!(
            matches!(&args[2], Expr::MakeClosure { fn_name, captures, .. }
                if fn_name == "Dog__speak" && captures.is_empty()),
            "registration closure must name the hoisted fn with no captures; got {:?}",
            args[2]
        );
    }

    #[test]
    fn new_on_constant_lowers_to_new_builtin() {
        // `Dog.new("x")` → `__new__("Dog", StrLit "x")` (not a generic
        // `__method__` dispatch).
        let m = lower("class Dog\n  def initialize(n)\n    @n = n\n  end\nend\nd = Dog.new(\"x\")\n");
        // Find the LetBinding for `d` and inspect its value.
        let value = main_body(&m)
            .stmts
            .iter()
            .find_map(|s| match s {
                Stmt::LetBinding { name, value, .. } if name == "d" => Some(value),
                _ => None,
            })
            .expect("let d = …");
        match value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "__new__", "Foo.new must lower to __new__");
                assert_eq!(args.len(), 2, "__new__(class, arg)");
                assert!(matches!(&args[0], Expr::StrLit { value, .. } if value == "Dog"));
                assert!(matches!(&args[1], Expr::StrLit { value, .. } if value == "x"));
            }
            other => panic!("expected __new__ BuiltinCall, got {:?}", other),
        }
    }

    #[test]
    fn new_chained_method_nests_new_inside_method() {
        // `Foo.new(x).meth` = `__method__(__new__("Foo", x), "meth")` — the
        // receiver of the outer `__method__` is the `__new__` call.
        let m = lower(
            "class Foo\n  def initialize(x)\n    @x = x\n  end\n  def meth\n    @x\n  end\nend\ny = Foo.new(1).meth\n",
        );
        let value = main_body(&m)
            .stmts
            .iter()
            .find_map(|s| match s {
                Stmt::LetBinding { name, value, .. } if name == "y" => Some(value),
                _ => None,
            })
            .expect("let y = …");
        match value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "__method__");
                assert!(
                    matches!(&args[0], Expr::BuiltinCall { name, .. } if name == "__new__"),
                    "outer __method__ receiver must be the __new__ call; got {:?}",
                    args[0]
                );
                assert!(matches!(&args[1], Expr::StrLit { value, .. } if value == "meth"));
            }
            other => panic!("expected __method__ over __new__, got {:?}", other),
        }
    }

    #[test]
    fn super_with_args_threads_method_and_class() {
        // `super(a)` inside `Cat#describe` → `__super__("describe", "Cat", a)`.
        let m = lower(
            "class Cat\n  def describe(a)\n    super(a)\n  end\nend\n",
        );
        let f = m
            .functions
            .iter()
            .find(|f| f.name == "Cat__describe")
            .expect("Cat__describe");
        // Issue #59 — `super(a)` is the method's tail expression, so it lands in
        // `body.value` (super is now an expression, not a statement); helper
        // scans both slots.
        let call = find_super_args(f).expect("__super__ call in body");
        assert_eq!(call.len(), 3, "__super__(method, class, arg)");
        assert!(matches!(&call[0], Expr::StrLit { value, .. } if value == "describe"));
        assert!(matches!(&call[1], Expr::StrLit { value, .. } if value == "Cat"));
        assert!(
            matches!(&call[2], Expr::VarRef { name, scope: Scope::Param, .. } if name == "a"),
            "the explicit super arg `a` must be forwarded; got {:?}",
            call[2]
        );
    }

    #[test]
    fn bare_super_forwards_enclosing_params() {
        // Bare `super` (zsuper) forwards the enclosing method's params by
        // reference: `def m(a, b); super; end` → `__super__("m", "C", a, b)`.
        let m = lower(
            "class C\n  def m(a, b)\n    super\n  end\nend\n",
        );
        let f = m.functions.iter().find(|f| f.name == "C__m").expect("C__m");
        // Issue #59 — bare `super` is the tail expression → `body.value`.
        let call = find_super_args(f).expect("__super__ call");
        assert!(matches!(&call[0], Expr::StrLit { value, .. } if value == "m"));
        assert!(matches!(&call[1], Expr::StrLit { value, .. } if value == "C"));
        // Remaining args are the two params (sorted for determinism: a, b).
        let forwarded: Vec<&str> = call[2..]
            .iter()
            .filter_map(|e| match e {
                Expr::VarRef { name, scope: Scope::Param, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(forwarded, vec!["a", "b"], "bare super forwards all params");
    }

    #[test]
    fn self_lowers_to_self_builtin() {
        // `self` → `__self__()` (was a plain local VarRef "self").
        let m = lower("class C\n  def m\n    self\n  end\nend\n");
        let f = m.functions.iter().find(|f| f.name == "C__m").expect("C__m");
        assert!(
            matches!(&f.body.value, Expr::BuiltinCall { name, args, .. }
                if name == "__self__" && args.is_empty()),
            "method tail `self` must lower to __self__(); got {:?}",
            f.body.value
        );
    }

    #[test]
    fn attr_accessor_expands_to_getter_and_setter() {
        // `attr_accessor :count` synthesizes a getter (`count`) and setter
        // (`count=`) def + registrations — exactly as if hand-written.
        let m = lower("class Counter\n  attr_accessor :count\nend\n");

        // Both accessor functions are hoisted (class-qualified).
        let names: Vec<&String> = m.functions.iter().map(|f| &f.name).collect();
        assert!(
            names.iter().any(|n| *n == "Counter__count"),
            "getter must hoist; got {:?}",
            names
        );
        assert!(
            names.iter().any(|n| *n == "Counter__count_set"),
            "setter (count=) must hoist as Counter__count_set; got {:?}",
            names
        );

        // The getter reads the instance var; the setter writes it.
        let getter = m.functions.iter().find(|f| f.name == "Counter__count").unwrap();
        assert!(
            matches!(&getter.body.value, Expr::VarRef { name, scope: Scope::Instance, .. }
                if name == "@count"),
            "getter body must read @count; got {:?}",
            getter.body.value
        );
        let setter = m.functions.iter().find(|f| f.name == "Counter__count_set").unwrap();
        assert_eq!(setter.params.len(), 1, "setter takes one param");
        assert!(
            setter.body.stmts.iter().any(|s| matches!(s,
                Stmt::Assign { name, scope: Scope::Instance, .. } if name == "@count")),
            "setter must assign @count; got {:?}",
            setter.body.stmts
        );

        // Two registrations follow the ClassDef: one for `count`, one for `count=`.
        let regs: Vec<&str> = main_body(&m)
            .stmts
            .iter()
            .filter_map(|s| match s {
                Stmt::ExprStmt { expr: Expr::BuiltinCall { name, args, .. }, .. }
                    if name == "__def_method__" =>
                {
                    match &args[1] {
                        Expr::StrLit { value, .. } => Some(value.as_str()),
                        _ => None,
                    }
                }
                _ => None,
            })
            .collect();
        assert!(regs.contains(&"count"), "getter registered; got {:?}", regs);
        assert!(regs.contains(&"count="), "setter registered; got {:?}", regs);
    }

    #[test]
    fn attr_reader_expands_to_getter_only() {
        // `attr_reader :x` → getter def only (no setter).
        let m = lower("class C\n  attr_reader :x\nend\n");
        let names: Vec<&String> = m.functions.iter().map(|f| &f.name).collect();
        assert!(names.iter().any(|n| *n == "C__x"), "getter present; got {:?}", names);
        assert!(
            !names.iter().any(|n| *n == "C__x_set"),
            "attr_reader must NOT synthesize a setter; got {:?}",
            names
        );
    }

    #[test]
    fn attr_writer_expands_to_setter_only() {
        // `attr_writer :x` → setter def only (no getter).
        let m = lower("class C\n  attr_writer :x\nend\n");
        let names: Vec<&String> = m.functions.iter().map(|f| &f.name).collect();
        assert!(names.iter().any(|n| *n == "C__x_set"), "setter present; got {:?}", names);
        assert!(
            !names.iter().any(|n| *n == "C__x"),
            "attr_writer must NOT synthesize a getter; got {:?}",
            names
        );
    }

    #[test]
    fn attr_accessor_multiple_symbols_each_expand() {
        // `attr_accessor :a, :b` expands accessors for BOTH symbols.
        let m = lower("class C\n  attr_accessor :a, :b\nend\n");
        let names: Vec<&String> = m.functions.iter().map(|f| &f.name).collect();
        for want in ["C__a", "C__a_set", "C__b", "C__b_set"] {
            assert!(
                names.iter().any(|n| n.as_str() == want),
                "expected accessor `{}`; got {:?}",
                want,
                names
            );
        }
    }

    #[test]
    fn oop_program_validates_end_to_end() {
        // The full P1 shape (class + initialize + method + `.new().m`) lowers and
        // passes the SIR validator — the round-trip the backends consume.
        let m = lower(
            "class Dog\n  def initialize(name)\n    @name = name\n  end\n  \
             def speak\n    \"woof\"\n  end\nend\nd = Dog.new(\"Rex\")\nprint(d.speak)\n",
        );
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected OOP module: {:?}",
            result.errors().collect::<Vec<_>>()
        );
    }

    #[test]
    fn inheritance_and_super_program_validates() {
        // The P2 shape (parent+child, both define `initialize`, child `super`s)
        // now validates: the two `initialize` methods hoist under DISTINCT
        // class-qualified names, so there is no duplicate-function error.
        let m = lower(
            "class Animal\n  def initialize(name)\n    @name = name\n  end\nend\n\
             class Cat < Animal\n  def initialize(name)\n    super(name)\n  end\nend\n\
             c = Cat.new(\"Tom\")\n",
        );
        assert!(
            m.functions.iter().any(|f| f.name == "Animal__initialize"),
            "Animal__initialize present"
        );
        assert!(
            m.functions.iter().any(|f| f.name == "Cat__initialize"),
            "Cat__initialize present (distinct from Animal's)"
        );
        let result = semantic_ir::validate(&m);
        assert!(
            result.is_ok(),
            "validator rejected inheritance module: {:?}",
            result.errors().collect::<Vec<_>>()
        );
    }

    // Regression for a grammar bug: `<`, `>`, `<=`, `>=`, `!=`, `&&`, `||` have
    // no dedicated lexer token type (`classify_op_token` in `ruby-lexer`
    // deliberately leaves every operator lexeme without one on
    // `TokenType::Name` — "the parser dispatches by value"), so `factor`'s
    // bare `NAME` alternative could match one of THEM too. A bare statement
    // like `x > 2` mis-parsed as `method_call_no_paren` — `x` as the callee,
    // the `>` token swallowed whole as if it were an ordinary name-shaped
    // argument — leaving `2` behind as an unrelated second statement, and
    // lowering the malformed "argument" emitted a `DirectCall` to whatever
    // that bare name resolved to (here, `x` itself), which the SIR validator
    // correctly rejected as a call to an unknown function. Fixed by a
    // negative-lookahead in `method_call_no_paren` (`ruby.grammar`) so it
    // fails to match on these operators and `expression_stmt` (`comparison`/
    // `logical_and`/`logical_or`) parses the whole expression correctly
    // instead. `==` was accidentally already immune (its own dedicated
    // `EqualsEquals` token type) — included below as a same-shape control
    // case that must keep passing. (`<=>` is a SEPARATE, pre-existing,
    // out-of-scope gap: it isn't in `comparison`'s operator set at all, so it
    // was never supported as a bare statement or otherwise — not touched
    // here.)
    #[test]
    fn bare_comparison_statement_validates() {
        // The BARE (unwrapped) shape that originally triggered the bug — as
        // opposed to `puts(x > 2)`, where the comparison is nested as a call
        // ARGUMENT and always worked. `==` was accidentally already immune
        // (see the module-level comment above) — included as a same-shape
        // control case that must keep validating.
        for op in ["<", ">", "<=", ">=", "!=", "&&", "||", "=="] {
            let src = format!("x = 3\nx {op} 2\n");
            let m = lower(&src);
            let result = semantic_ir::validate(&m);
            assert!(
                result.is_ok(),
                "bare `x {op} 2` should validate: {:?}",
                result.errors().collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn bare_comparison_in_block_tail_position_validates() {
        // The shape that originally surfaced the bug: a comparison as a
        // block's IMPLICIT RETURN value (`select`/`any?`/etc.'s predicate).
        for op in ["<", ">", "<=", ">=", "!="] {
            let src = format!("puts [1, 2, 3].select {{ |x| x {op} 2 }}\n");
            let m = lower(&src);
            let result = semantic_ir::validate(&m);
            assert!(
                result.is_ok(),
                "`{{ |x| x {op} 2 }}` should validate: {:?}",
                result.errors().collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn bare_comparison_in_def_tail_position_validates() {
        for op in ["<", ">", "<=", ">=", "!=", "&&", "||"] {
            let src = format!("def f(x)\n  x {op} 2\nend\nputs f(3)\n");
            let m = lower(&src);
            let result = semantic_ir::validate(&m);
            assert!(
                result.is_ok(),
                "`def f(x); x {op} 2; end` should validate: {:?}",
                result.errors().collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn bitwise_operators_are_still_cleanly_unsupported_not_mis_parsed() {
        // `**`/`<<`/`>>`/`^`/`&`/`|` have NO binary-operator grammar rule in
        // this Ruby subset at all (only used elsewhere: `**`/`&` as call-arg
        // prefixes, `<<` for singleton-class, `|` for block params, `^` for
        // pin patterns) — so the fix deliberately does NOT guard them: there
        // is no correct fallback parse to preserve for `x << 2`. This pins
        // that they remain UNCHANGED (still lower as separate statements,
        // same as before the fix), so a future contributor doesn't assume
        // this fix silently added bitwise-operator support.
        for op in ["**", "<<", ">>", "^", "&", "|"] {
            let src = format!("x = 3\nx {op} 2\n");
            let m = lower(&src);
            assert_eq!(
                m.functions.iter().find(|f| f.name == "main").unwrap().body.stmts.len(),
                2,
                "`x {op} 2` is not a supported binary expression here; it should \
                 still split into two statements (unchanged), not one: {src:?}"
            );
        }
    }
}
