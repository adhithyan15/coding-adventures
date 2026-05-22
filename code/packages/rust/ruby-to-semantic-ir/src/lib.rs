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
    use semantic_ir::{Effect, Expr, Scope, Stmt};

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
        let names: Vec<&str> = b
            .stmts
            .iter()
            .filter_map(|s| match s {
                Stmt::LetBinding { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(names, vec!["x", "y", "z"]);
        // The third RHS references x and y as locals.
        if let Stmt::LetBinding { value, .. } = &b.stmts[2] {
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
}
