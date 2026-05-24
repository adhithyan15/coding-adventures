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
            Stmt::LetBinding { value: Expr::BuiltinCall { name, args, .. }, .. } => {
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
            Stmt::LetBinding { name, value, .. } => {
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
            Stmt::LetBinding { value, .. } => value,
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
            Stmt::LetBinding { value, .. } => value,
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
                    &args[2]
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
                    &args[2]
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
}
