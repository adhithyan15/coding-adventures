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
        // `bar` was hoisted to top-level Functions.
        assert!(
            m.functions.iter().any(|f| f.name == "bar"),
            "expected hoisted `bar` function; got functions {:?}",
            m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        // The class statement itself produced a ClassDef in main.
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
        // into `ClassDef.body` as a `LetBinding`, while `def bar` is
        // still hoisted to a top-level Function.
        let m = lower("class Foo\n  MAX = 10\n  def bar\n  end\nend\n");
        assert!(
            m.functions.iter().any(|f| f.name == "bar"),
            "expected hoisted `bar`; got {:?}",
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
                    Stmt::LetBinding { name, value, .. } => {
                        assert_eq!(name, "MAX");
                        assert!(matches!(value, Expr::IntLit { value: 10, .. }));
                    }
                    other => panic!("expected LetBinding MAX, got {:?}", other),
                }
            }
            other => panic!("expected Stmt::ClassDef, got {:?}", other),
        }
    }

    #[test]
    fn class_body_preserves_multiple_statements_in_source_order() {
        // Two constant assignments must appear in `body` in the same
        // order they were written.
        let m = lower("class Cfg\n  A = 1\n  B = 2\nend\n");
        let main = main_body(&m);
        match &main.stmts[0] {
            Stmt::ClassDef { body, .. } => {
                assert_eq!(body.len(), 2, "both assignments preserved; got {:?}", body);
                let names: Vec<&str> = body
                    .iter()
                    .map(|s| match s {
                        Stmt::LetBinding { name, .. } => name.as_str(),
                        other => panic!("expected LetBinding, got {:?}", other),
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
        let count = |needle: &str| m.functions.iter().filter(|f| f.name == needle).count();
        assert_eq!(count("o"), 1, "`o` hoisted exactly once");
        assert_eq!(count("i"), 1, "`i` hoisted exactly once");
        // Outer's body carries the nested Inner ClassDef.
        let main = main_body(&m);
        match &main.stmts[0] {
            Stmt::ClassDef { name, body, .. } => {
                assert_eq!(name, "Outer");
                assert_eq!(body.len(), 1, "Inner class is the only body stmt; got {:?}", body);
                match &body[0] {
                    Stmt::ClassDef { name, body, .. } => {
                        assert_eq!(name, "Inner");
                        assert!(body.is_empty(), "Inner is method-only → empty body");
                    }
                    other => panic!("expected nested ClassDef Inner, got {:?}", other),
                }
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
        assert!(
            m.functions.iter().any(|f| f.name == "meow"),
            "expected hoisted `meow`; got {:?}",
            m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        let main = main_body(&m);
        match &main.stmts[0] {
            Stmt::ClassDef { name, superclass, body, .. } => {
                assert_eq!(name, "Cat");
                assert_eq!(superclass.as_deref(), Some("Animal"));
                assert_eq!(body.len(), 1, "only `LEGS = 4` stays in body; got {:?}", body);
                assert!(matches!(&body[0], Stmt::LetBinding { name, .. } if name == "LEGS"));
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

    #[test]
    fn module_still_lowers_to_nil_no_op_in_phase_14a() {
        // Phase 14a covers `class` only.  `module M; end` continues
        // to lower to the Phase 6f NilLit no-op until Phase 14d adds
        // a `Stmt::ModuleDef` analog.  This test pins the contract so
        // a future refactor that accidentally merges class/module
        // arms again is caught.
        let m = lower("module M\nend\n");
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
            Stmt::LetBinding { name, value, .. } => {
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
            Stmt::LetBinding { name, value, .. } => {
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
            Stmt::LetBinding { name, value, .. } => {
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
            Stmt::LetBinding { name, value, .. } => {
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
        assert!(matches!(&body.stmts[4], Stmt::LetBinding { name, .. } if name == "a"));
        // Stmt[5] binds `b` to SeqLit of 2 items (the middle 2 temps).
        match &body.stmts[5] {
            Stmt::LetBinding { name, value, .. } => {
                assert_eq!(name, "b");
                match value {
                    Expr::SeqLit { items, .. } => assert_eq!(items.len(), 2),
                    other => panic!("expected SeqLit, got {:?}", other),
                }
            }
            other => panic!("expected LetBinding(b, SeqLit), got {:?}", other),
        }
        // Stmt[6] binds `c` to the last temp.
        assert!(matches!(&body.stmts[6], Stmt::LetBinding { name, .. } if name == "c"));
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
            Stmt::LetBinding { name, value, .. } => {
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
            Stmt::LetBinding { name, value, .. } => {
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
                Stmt::LetBinding { value: Expr::SeqIndex { index, .. }, .. } => {
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
            if let Stmt::LetBinding { value: Expr::SeqIndex { seq, .. }, .. } =
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
            Stmt::LetBinding { name, .. } => assert_eq!(name, "b"),
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
                assert_eq!(*scope, Scope::Local, "v0 puts ivars on Scope::Local");
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
                assert_eq!(*scope, Scope::Local, "v0 puts cvars on Scope::Local");
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
    // Phase 6v — `begin … rescue … ensure … end`
    // -----------------------------------------------------------------
    //
    // SIR has no try/catch primitive.  v0 lowering is lossy: body,
    // rescue, and ensure stmts emit inline with marker BuiltinCalls
    // (`__rescue_marker__`, `__ensure_marker__`) bracketing the
    // rescue/ensure sections.

    #[test]
    fn begin_without_rescue_lowers_body_inline() {
        // `begin x = 1 end` → just LetBinding(x, 1).  No markers.
        let m = lower("begin\n  x = 1\nend\n");
        let b = main_body(&m);
        assert_eq!(b.stmts.len(), 1);
        assert!(matches!(&b.stmts[0], Stmt::LetBinding { name, .. } if name == "x"));
    }

    #[test]
    fn begin_with_rescue_emits_rescue_marker() {
        let m = lower(
            "begin\n  x = 1\nrescue StandardError => e\n  y = 2\nend\n",
        );
        let b = main_body(&m);
        // Stmts: LetBinding(x,1), ExprStmt(BuiltinCall(__rescue_marker__, ["StandardError", "e"])), LetBinding(y,2).
        assert_eq!(b.stmts.len(), 3);
        let marker = match &b.stmts[1] {
            Stmt::ExprStmt { expr, .. } => expr,
            other => panic!("expected ExprStmt(rescue marker), got {:?}", other),
        };
        match marker {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "__rescue_marker__");
                assert_eq!(args.len(), 2);
                assert!(
                    matches!(&args[0], Expr::StrLit { value, .. } if value == "StandardError")
                );
                assert!(
                    matches!(&args[1], Expr::StrLit { value, .. } if value == "e")
                );
            }
            other => panic!("expected __rescue_marker__ builtin, got {:?}", other),
        }
    }

    #[test]
    fn begin_with_ensure_emits_ensure_marker() {
        let m = lower("begin\n  x = 1\nensure\n  y = 2\nend\n");
        let b = main_body(&m);
        // Stmts: LetBinding(x,1), ExprStmt(BuiltinCall(__ensure_marker__, [])), LetBinding(y,2).
        assert_eq!(b.stmts.len(), 3);
        assert!(matches!(
            &b.stmts[1],
            Stmt::ExprStmt {
                expr: Expr::BuiltinCall { name, args, .. },
                ..
            } if name == "__ensure_marker__" && args.is_empty()
        ));
    }

    #[test]
    fn begin_with_rescue_and_ensure_emits_both_markers_in_order() {
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
        // Stmts in order:
        //   0: LetBinding(x, 1)
        //   1: ExprStmt(__rescue_marker__("StandardError", "e"))
        //   2: LetBinding(y, 2)
        //   3: ExprStmt(__ensure_marker__())
        //   4: LetBinding(z, 3)
        assert_eq!(b.stmts.len(), 5);
        assert!(matches!(
            &b.stmts[1],
            Stmt::ExprStmt {
                expr: Expr::BuiltinCall { name, .. },
                ..
            } if name == "__rescue_marker__"
        ));
        assert!(matches!(
            &b.stmts[3],
            Stmt::ExprStmt {
                expr: Expr::BuiltinCall { name, .. },
                ..
            } if name == "__ensure_marker__"
        ));
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
    fn case_single_when_lowers_to_if_with_eq() {
        // `case x; when 1; y = 1; end` — one when, no else.
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
        // The condition is BuiltinCall("==", [VarRef(x), IntLit(1)]).
        assert!(
            matches!(cond.as_ref(), Expr::BuiltinCall { name, .. } if name == "=="),
            "expected `==` builtin in when cond, got {:?}",
            cond
        );
    }

    #[test]
    fn case_with_multi_value_when_lowers_to_or_chain() {
        // `when 1, 2, 3` → cond is `(((x==1) || (x==2)) || (x==3))`.
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
        // Outermost should be `or`.  Count `==` and `or` nodes.
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
            count_builtin(cond.as_ref(), "=="),
            3,
            "expected three `==` comparisons for three when values"
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
    // Phase 6y — string interpolation lowering
    //
    // Output shapes covered:
    //   "plain"              → StrLit("plain")
    //   "#{x}"               → VarRef("x")
    //   "hi #{name}"         → BuiltinCall("string_concat",
    //                            [StrLit("hi "), VarRef("name")])
    //   "sum=#{1+2}"         → BuiltinCall("string_concat",
    //                            [StrLit("sum="),
    //                             BuiltinCall("__interp__", [StrLit("1+2")])])
    //
    // Plus an end-to-end smoke test that an interpolated module passes
    // the SIR validator (proves the BuiltinCall + StrLit shape is
    // well-formed semantic IR).
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
    fn interpolated_string_with_bare_name_lowers_to_string_concat() {
        // `"hi #{name}"` → string_concat(StrLit("hi "), VarRef("name"))
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
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "string_concat");
                assert_eq!(args.len(), 2, "expected 2 concat segments");
                assert!(matches!(&args[0], Expr::StrLit { value, .. } if value == "hi "));
                assert!(matches!(&args[1], Expr::VarRef { name, .. } if name == "name"));
            }
            other => panic!("expected string_concat BuiltinCall, got {:?}", other),
        }
    }

    #[test]
    fn interpolated_string_that_is_only_interp_unwraps_to_a_single_segment() {
        // `"#{name}"` has no literal text — the lowerer should hand back
        // the single segment directly (no `string_concat` wrapper).
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
    fn interpolated_string_with_expression_uses_interp_marker() {
        // `"sum=#{1+2}"` — the interp body `1+2` is not a bare name, so
        // it must lower as the v0 `__interp__` marker carrying the raw
        // body text.  Same marker pattern as Phase 6v rescue/ensure.
        let m = lower(r##"x = "sum=#{1+2}""##);
        let b = main_body(&m);
        let value = match &b.stmts[0] {
            Stmt::LetBinding { value, .. } => value,
            other => panic!("expected LetBinding, got {:?}", other),
        };
        match value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "string_concat");
                assert_eq!(args.len(), 2);
                assert!(matches!(&args[0], Expr::StrLit { value, .. } if value == "sum="));
                match &args[1] {
                    Expr::BuiltinCall { name, args, .. } => {
                        assert_eq!(name, "__interp__");
                        assert_eq!(args.len(), 1);
                        assert!(
                            matches!(&args[0], Expr::StrLit { value, .. } if value == "1+2"),
                            "expected the raw interp body preserved in the marker"
                        );
                    }
                    other => panic!("expected __interp__ marker, got {:?}", other),
                }
            }
            other => panic!("expected string_concat, got {:?}", other),
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
                    &args[0]
                );
                assert!(
                    matches!(&args[1], Expr::VarRef { name, scope, .. }
                        if name == "y" && *scope == Scope::Param),
                    "expected VarRef(y, Param), got {:?}",
                    &args[1]
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
    //   array_pattern    → `__pattern_match__` marker, no bindings
    //   hash_pattern     → `__pattern_match__` marker, no bindings
    // -----------------------------------------------------------------------

    /// Helper: extract the `Expr::If` from a top-level `case` statement
    /// — Phase 6u/7d lowers case to `Stmt::ExprStmt(Expr::If(...))`.
    fn extract_case_if<'a>(b: &'a semantic_ir::Block) -> &'a Expr {
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
    fn case_in_array_pattern_lowers_to_pattern_match_marker() {
        // `case x; in [1, 2]; "pair"; end` — array pattern lowers as
        // `BuiltinCall("__pattern_match__", [scrut, StrLit("<raw>")])`.
        let m = lower("x = 1\ncase x\nin [1, 2]\n  \"pair\"\nend\n");
        let b = main_body(&m);
        let if_expr = extract_case_if(b);
        let cond = match if_expr {
            Expr::If { cond, .. } => cond,
            other => panic!("expected If, got {:?}", other),
        };
        match cond.as_ref() {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "__pattern_match__");
                assert_eq!(args.len(), 2);
                // args[1] is StrLit with the raw pattern text.
                match &args[1] {
                    Expr::StrLit { value, .. } => {
                        assert!(
                            value.contains('1') && value.contains('2'),
                            "expected raw text to contain `1` and `2`, got `{}`",
                            value
                        );
                    }
                    other => panic!("expected StrLit raw pattern, got {:?}", other),
                }
            }
            other => panic!("expected __pattern_match__ BuiltinCall, got {:?}", other),
        }
    }

    #[test]
    fn case_in_hash_pattern_lowers_to_pattern_match_marker() {
        // `case x; in {name: y}; "match"; end` — hash pattern uses the
        // same `__pattern_match__` marker as array.
        let m = lower("x = 1\ncase x\nin {name: y}\n  \"match\"\nend\n");
        let b = main_body(&m);
        let if_expr = extract_case_if(b);
        let cond = match if_expr {
            Expr::If { cond, .. } => cond,
            other => panic!("expected If, got {:?}", other),
        };
        match cond.as_ref() {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "__pattern_match__");
                assert_eq!(args.len(), 2);
                assert!(matches!(&args[1], Expr::StrLit { .. }));
            }
            other => panic!("expected __pattern_match__ BuiltinCall, got {:?}", other),
        }
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
}
