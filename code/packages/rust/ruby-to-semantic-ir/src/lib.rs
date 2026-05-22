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
        let greet = m.functions.iter().find(|f| f.name == "greet");
        assert!(
            greet.is_some(),
            "expected `greet` to be hoisted to top-level functions, got {:?}",
            m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        // `main` is still present (no top-level statements were lost).
        assert!(m.functions.iter().any(|f| f.name == "main"));
    }

    #[test]
    fn empty_class_lowers_cleanly() {
        // `class Foo; end` with no body should produce a module that
        // still has `main` and no extra user functions.
        let m = lower("class Foo\nend\n");
        // Only `main` exists.
        assert_eq!(m.functions.len(), 1);
        assert_eq!(m.functions[0].name, "main");
        // The class declaration lowered to a no-op stmt in main.
        let main = main_body(&m);
        assert_eq!(main.stmts.len(), 1);
        assert!(matches!(
            &main.stmts[0],
            Stmt::ExprStmt { expr: Expr::NilLit { .. }, .. }
        ));
    }

    #[test]
    fn module_with_def_hoists_def_to_top_level() {
        let m = lower("module M\n  def helper\n  end\nend\n");
        assert!(m.functions.iter().any(|f| f.name == "helper"));
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
