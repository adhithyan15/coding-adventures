//! Unit tests for the statement-lowering fixes of issue #18482:
//! augmented, annotated and chained assignment; parameters assigned in
//! the body; a user `def main`; tuples spelled with a trailing comma;
//! `for` over a bound variable; function bodies (falling off the end, the
//! statement before a tail `return`); one-line suites; loop `else:`.
//!
//! Every test here failed before the fix: either the program lowered to
//! SIR that computes something else (a dropped store, a dropped
//! statement, a wrong return value), or a form that must be refused was
//! silently accepted.  The behaviour on real backends is checked end to
//! end in `sir-conformance/tests/python_statement_forms.rs`; these tests
//! pin the IR shape and the refusals.

use python_to_semantic_ir::{compile_source, PythonLowerError};
use semantic_ir::{Block, Expr, Function, Module, Scope, Stmt, Visitor};

// ── helpers ──────────────────────────────────────────────────────────

/// Lower `src`, asserting it lowers *and* passes the SIR validator.
fn lower(src: &str) -> Module {
    let m = compile_source(src, "t").unwrap_or_else(|e| panic!("lowering failed: {e}\n{src}"));
    let report = semantic_ir::validate(&m);
    assert!(
        report.is_ok(),
        "validation failed: {:?}\n{src}",
        report.issues
    );
    m
}

/// Lower `src`, asserting it is refused, and return the error.
fn refuse(src: &str) -> PythonLowerError {
    match compile_source(src, "t") {
        Ok(m) => panic!("expected a refusal, but it lowered:\n{src}\n{m:#?}"),
        Err(e) => e,
    }
}

fn func<'a>(m: &'a Module, name: &str) -> &'a Function {
    m.functions
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| {
            let names: Vec<&str> = m.functions.iter().map(|f| f.name.as_str()).collect();
            panic!("no function `{name}` in {names:?}")
        })
}

fn main_body(m: &Module) -> &Block {
    &func(m, "main").body
}

/// `VarRef { name, scope }` with any span.
fn is_var(e: &Expr, want_name: &str, want_scope: Scope) -> bool {
    matches!(e, Expr::VarRef { name, scope, .. } if name == want_name && *scope == want_scope)
}

fn is_int(e: &Expr, want: i64) -> bool {
    matches!(e, Expr::IntLit { value, .. } if *value == want)
}

/// The `(builtin name, args)` of a `BuiltinCall`, or a panic.
fn builtin(e: &Expr) -> (&str, &[Expr]) {
    match e {
        Expr::BuiltinCall { name, args, .. } => (name.as_str(), args.as_slice()),
        other => panic!("expected a BuiltinCall, got {other:?}"),
    }
}

/// Counts every `SeqLit` in a module.
#[derive(Default)]
struct SeqLits(usize);

impl Visitor for SeqLits {
    fn visit_expr(&mut self, e: &Expr, depth: usize) {
        if matches!(e, Expr::SeqLit { .. }) {
            self.0 += 1;
        }
        semantic_ir::walker::walk_expr_default(self, e, depth);
    }
}

// ── augmented assignment ─────────────────────────────────────────────

/// `x += 2` lowers to the in-place addition builtin `+=`, which backends
/// implement as mutating mutable operands in place and falling back to addition for primitives.
#[test]
fn augmented_add_stores_target_plus_value() {
    let m = lower("x = 1\nx += 2\nprint(x)\n");
    let stmts = &main_body(&m).stmts;
    match &stmts[1] {
        Stmt::Assign {
            name, scope, value, ..
        } => {
            assert_eq!(name, "x");
            assert_eq!(*scope, Scope::Local);
            let (op, args) = builtin(value);
            assert_eq!(op, "+=");
            assert!(
                is_var(&args[0], "x", Scope::Local),
                "reads x first: {args:?}"
            );
            assert!(is_int(&args[1], 2), "{args:?}");
        }
        other => panic!("expected `Assign x`, got {other:?}"),
    }
}

/// Lowered augmented operators match their binary operator, except `+=` which
/// lowers to the in-place builtin `"+="` to support in-place mutation of collections.
#[test]
fn every_lowered_augmented_operator_matches_its_binary_operator() {
    for op in ["-", "*", "/", "%"] {
        let binary = lower(&format!("x = 7\ny = x {op} 2\n"));
        let augmented = lower(&format!("x = 7\nx {op}= 2\n"));
        let binary_name = match &main_body(&binary).stmts[1] {
            Stmt::LetStarBinding { value, .. } => builtin(value).0.to_string(),
            other => panic!("`{op}`: expected `let* y`, got {other:?}"),
        };
        match &main_body(&augmented).stmts[1] {
            Stmt::Assign { value, .. } => {
                let (name, args) = builtin(value);
                assert_eq!(name, binary_name, "`{op}=` vs `{op}`");
                assert!(is_var(&args[0], "x", Scope::Local));
                assert!(is_int(&args[1], 2));
            }
            other => panic!("`{op}=`: expected `Assign x`, got {other:?}"),
        }
    }
    // `+=` uses `"+="` for in-place semantics
    let add_aug = lower("x = 7\nx += 2\n");
    match &main_body(&add_aug).stmts[1] {
        Stmt::Assign { value, .. } => assert_eq!(builtin(value).0, "+="),
        other => panic!("{other:?}"),
    }
    // The table entry that is not a pass-through spelling.
    let m = lower("x = 7\nx /= 2\n");
    match &main_body(&m).stmts[1] {
        Stmt::Assign { value, .. } => assert_eq!(builtin(value).0, "div_true"),
        other => panic!("{other:?}"),
    }
}

/// The operators whose binary form is not lowered are refused, at the
/// operator, rather than dropped.
#[test]
fn unlowered_augmented_operators_are_refused_with_a_position() {
    for op in ["//=", "**=", "&=", "|=", "^=", "<<=", ">>=", "@="] {
        let err = refuse(&format!("x = 7\nx {op} 2\n"));
        assert!(
            err.message.contains(op) && err.message.contains("unsupported"),
            "`{op}`: {}",
            err.message
        );
        assert_eq!((err.line, err.column), (2, 3), "`{op}`: {err}");
    }
}

/// Python makes an augmented target local to its scope, so reading it
/// first raises `UnboundLocalError`.  A name not bound here is refused.
#[test]
fn augmented_assignment_to_an_unbound_name_is_refused() {
    let err = refuse("y = 1\nx += 1\n");
    assert!(err.message.contains("augmented assignment to `x`"), "{err}");
    assert_eq!(err.line, 2);

    // The classic `UnboundLocalError`: `count` is a `main` local, but the
    // `+=` makes it local to `bump`, so `bump` cannot capture it.
    let err = refuse("count = 0\ndef bump():\n    count += 1\n    return count\n\nprint(bump())\n");
    assert!(
        err.message.contains("augmented assignment to `count`"),
        "{err}"
    );
    assert_eq!(err.line, 3);

    // Likewise a variable of the enclosing function.
    let err =
        refuse("def outer(n):\n    def g():\n        n += 1\n        return n\n    return g()\n");
    assert!(err.message.contains("augmented assignment to `n`"), "{err}");
}

#[test]
fn augmented_subscript_and_foreign_attribute_targets_are_refused() {
    let err = refuse("xs = [1]\nxs[0] += 1\n");
    assert!(err.message.contains("subscript"), "{err}");
    assert_eq!(err.line, 2);

    let err = refuse("x = 1\nx.y += 1\n");
    assert!(err.message.contains("attribute"), "{err}");
    assert_eq!(err.line, 2);
}

/// `self.n += k` reads and writes the instance variable.
#[test]
fn augmented_instance_variable_in_a_method() {
    let m = lower(
        "class C:\n    def __init__(self):\n        self.n = 0\n    def bump(self, k):\n        self.n += k\n        return self.n\n",
    );
    let bump = func(&m, "C__bump");
    match &bump.body.stmts[0] {
        Stmt::Assign {
            name, scope, value, ..
        } => {
            assert_eq!((name.as_str(), *scope), ("@n", Scope::Instance));
            let (op, args) = builtin(value);
            assert_eq!(op, "+=");
            assert!(is_var(&args[0], "@n", Scope::Instance), "{args:?}");
            assert!(is_var(&args[1], "k", Scope::Param), "{args:?}");
        }
        other => panic!("expected `Assign @n`, got {other:?}"),
    }
}

/// A tuple value is a tuple, not its single element.
#[test]
fn augmented_tuple_value_is_refused() {
    let err = refuse("x = 1\nx += 1,\n");
    assert!(err.message.contains("tuple"), "{err}");
}

// ── parameters assigned in the body ──────────────────────────────────

/// `n -= 1` inside a loop used to declare a loop-scoped local `n`, so the
/// condition kept reading the parameter: an endless loop.  Now the
/// parameter is copied into an aliased local once, at the top.
#[test]
fn a_parameter_assigned_in_a_loop_is_one_variable() {
    for update in ["n -= 1", "n = n - 1"] {
        let src = format!(
            "def countdown(n):\n    while n > 0:\n        {update}\n    return n\n\nprint(countdown(3))\n"
        );
        let m = lower(&src);
        let f = func(&m, "countdown");
        let alias = match &f.body.stmts[0] {
            Stmt::LetStarBinding { name, value, .. } => {
                assert!(
                    is_var(value, "n", Scope::Param),
                    "copies the param: {value:?}"
                );
                assert_ne!(
                    name, "n",
                    "the copy must not redeclare the param's own name"
                );
                name.clone()
            }
            other => panic!("`{update}`: expected the param-copy prologue, got {other:?}"),
        };
        match &f.body.stmts[1] {
            Stmt::While { cond, body, .. } => {
                let (_, cond_args) = builtin(cond);
                assert!(is_var(&cond_args[0], &alias, Scope::Local), "{cond:?}");
                match &body.stmts[..] {
                    [Stmt::Assign { name, scope, .. }] => {
                        assert_eq!((name, *scope), (&alias, Scope::Local), "`{update}`");
                    }
                    other => panic!("`{update}`: expected one Assign, got {other:?}"),
                }
            }
            other => panic!("expected the while loop, got {other:?}"),
        }
        assert!(is_var(&f.body.value, &alias, Scope::Local));
    }
}

/// A closure built after the parameter is assigned threads the copy.
#[test]
fn a_capture_of_an_assigned_parameter_threads_its_copy() {
    let m = lower("def outer(n):\n    n += 1\n    def g():\n        return n\n    return g()\n");
    let outer = func(&m, "outer");
    let alias = match &outer.body.stmts[0] {
        Stmt::LetStarBinding { name, .. } => name.clone(),
        other => panic!("{other:?}"),
    };
    let mut closures = Closures::default();
    closures.visit_function(outer);
    assert!(
        closures
            .0
            .iter()
            .any(|(f, caps)| f == "g" && caps.len() == 1 && is_var(&caps[0], &alias, Scope::Local)),
        "{:?}",
        closures.0
    );
}

/// Every `MakeClosure` in a function: `(target, capture values)`.
#[derive(Default)]
struct Closures(Vec<(String, Vec<Expr>)>);

impl Visitor for Closures {
    fn visit_expr(&mut self, e: &Expr, depth: usize) {
        if let Expr::MakeClosure {
            fn_name, captures, ..
        } = e
        {
            self.0.push((
                fn_name.clone(),
                captures.iter().map(|c| c.value.clone()).collect(),
            ));
        }
        semantic_ir::walker::walk_expr_default(self, e, depth);
    }
}

/// A parameter the body never assigns is not copied.
#[test]
fn a_parameter_that_is_only_read_is_not_copied() {
    let m = lower("def f(a, b):\n    b = b + a\n    return a + b\n");
    let f = func(&m, "f");
    let copies: Vec<&str> = f
        .body
        .stmts
        .iter()
        .filter_map(|s| match s {
            Stmt::LetStarBinding {
                value:
                    Expr::VarRef {
                        name,
                        scope: Scope::Param,
                        ..
                    },
                ..
            } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(copies, ["b"]);
}

/// The alias is fresh: it does not take a name the program spells.
#[test]
fn the_parameter_copy_avoids_names_in_the_source() {
    let m = lower("def f(n):\n    __param_n_0 = 1\n    n += __param_n_0\n    return n\n");
    let f = func(&m, "f");
    match &f.body.stmts[0] {
        Stmt::LetStarBinding { name, .. } => assert_eq!(name, "__param_n_1"),
        other => panic!("{other:?}"),
    }
}

/// Inside a method, a store into `self` would be invisible to every
/// later read of `self` (which is the receiver), so it is refused.
#[test]
fn assignment_to_self_in_a_method_is_refused() {
    let err = refuse("class C:\n    def m(self):\n        self = 1\n        return 0\n");
    assert!(err.message.contains("`self`"), "{err}");
    assert_eq!(err.line, 3);
}

// ── annotated assignment ─────────────────────────────────────────────

/// `x: int = 5` used to lower to the statement `x` (refused: `x` unbound).
#[test]
fn annotated_assignment_stores_the_value() {
    let m = lower("x: int = 5\nx\n");
    let body = main_body(&m);
    match &body.stmts[0] {
        Stmt::LetStarBinding { name, value, .. } => {
            assert_eq!(name, "x");
            assert!(is_int(value, 5));
        }
        other => panic!("expected `let* x = 5`, got {other:?}"),
    }
    // A second annotated assignment re-binds.
    let m = lower("x = 1\nx: int = 2\n");
    assert!(
        matches!(&main_body(&m).stmts[1], Stmt::Assign { name, value, .. }
        if name == "x" && is_int(value, 2))
    );
}

/// `x: int` binds nothing; it is no statement at all.
#[test]
fn annotation_without_a_value_is_a_no_op() {
    let m = lower("x = 1\nx: int\nx\n");
    let body = main_body(&m);
    assert_eq!(body.stmts.len(), 1, "{:?}", body.stmts);
    assert!(is_var(&body.value, "x", Scope::Local));

    // And it does not bind: a read is still unresolved (Python: NameError).
    let err = refuse("y: int\nprint(y)\n");
    assert!(err.message.contains("unresolved name `y`"), "{err}");
}

/// `self.x: int` would evaluate `self`; only a bare name is accepted
/// without a value.
#[test]
fn annotation_without_a_value_on_a_non_name_is_refused() {
    let err = refuse("class C:\n    def m(self):\n        self.x: int\n        return 0\n");
    assert!(err.message.contains("annotation"), "{err}");
}

/// The annotation is never evaluated, so its names are not captures.
#[test]
fn annotation_names_are_not_captured() {
    let m =
        lower("def outer(T):\n    def g():\n        y: T = 1\n        return y\n    return g()\n");
    assert!(
        func(&m, "g").captures.is_empty(),
        "{:?}",
        func(&m, "g").captures
    );
}

// ── chained assignment ───────────────────────────────────────────────

/// `a = b = 0` used to lower to `a = b`.
#[test]
fn chained_assignment_stores_into_every_target_left_to_right() {
    let m = lower("a = b = 0\nprint(a + b)\n");
    let stmts = &main_body(&m).stmts;
    let tmp = match &stmts[0] {
        Stmt::LetStarBinding { name, value, .. } => {
            assert!(is_int(value, 0));
            name.clone()
        }
        other => panic!("expected the value's temporary, got {other:?}"),
    };
    for (stmt, want) in stmts[1..3].iter().zip(["a", "b"]) {
        match stmt {
            Stmt::LetStarBinding { name, value, .. } => {
                assert_eq!(name, want);
                assert!(is_var(value, &tmp, Scope::Local), "{value:?}");
            }
            other => panic!("expected `let* {want}`, got {other:?}"),
        }
    }
}

/// The value is evaluated once, so both names hold the same list.
#[test]
fn chained_assignment_evaluates_the_value_once() {
    let m = lower("a = b = c = []\n");
    let mut lits = SeqLits::default();
    lits.visit_module(&m);
    assert_eq!(lits.0, 1);
}

/// Targets of every kind: re-bound locals, subscripts, `self.x`.
#[test]
fn chained_assignment_mixes_target_kinds() {
    let m = lower("xs = [0]\nn = 1\nn = xs[0] = m = 4\n");
    let stmts = &main_body(&m).stmts;
    assert!(matches!(&stmts[3], Stmt::Assign { name, .. } if name == "n"));
    assert!(matches!(&stmts[4], Stmt::SeqSet { .. }), "{:?}", stmts[4]);
    assert!(matches!(&stmts[5], Stmt::LetStarBinding { name, .. } if name == "m"));

    let m = lower("class C:\n    def __init__(self):\n        self.a = self.b = 0\n");
    let init = func(&m, "C____init__");
    let names: Vec<&str> = init
        .body
        .stmts
        .iter()
        .filter_map(|s| match s {
            Stmt::Assign {
                name,
                scope: Scope::Instance,
                ..
            } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(names, ["@a", "@b"]);
}

/// The temporary never takes a name the program spells.
#[test]
fn the_chain_temporary_avoids_names_in_the_source() {
    let m = lower("__chain_0 = 5\na = b = 1\nprint(__chain_0)\n");
    match &main_body(&m).stmts[1] {
        Stmt::LetStarBinding { name, .. } => assert_eq!(name, "__chain_1"),
        other => panic!("{other:?}"),
    }
}

/// Every chained target is bound in its function, so none is captured.
#[test]
fn chained_targets_are_bound_not_captured() {
    let m = lower(
        "def outer(a):\n    def g():\n        a = b = 1\n        return a + b\n    return g()\n",
    );
    assert!(func(&m, "g").captures.is_empty());
}

#[test]
fn chained_assignment_to_a_tuple_target_is_refused() {
    let err = refuse("a = b, c = 1, 2\n");
    assert!(err.message.contains("tuple"), "{err}");
}

// ── tuples spelled with a trailing comma ─────────────────────────────

/// A trailing comma makes a one-element tuple.  Each of these used to
/// lower as the bare element.
#[test]
fn trailing_comma_tuples_are_refused() {
    for src in [
        "x = 1,\n",
        "def f():\n    return 1,\n",
        "for x, in [[1]]:\n    print(x)\n",
        "xs = [1]\nprint(xs[0,])\n",
        "xs = [1]\nxs[0,] = 2\n",
    ] {
        let err = refuse(src);
        assert!(err.message.contains("tuple"), "{src:?}: {err}");
    }
}

// ── `for` over an already-bound variable ─────────────────────────────

/// Python's loop stores into the existing variable, which keeps the last
/// item.  The IR loop binds a temporary and stores it into the variable.
#[test]
fn a_for_loop_over_a_bound_local_stores_into_it() {
    let m = lower("i = 10\nfor i in range(3):\n    i\nprint(i)\n");
    match &main_body(&m).stmts[1] {
        Stmt::ForRange { var, body, .. } => {
            assert_ne!(var, "i");
            match &body.stmts[0] {
                Stmt::Assign {
                    name, scope, value, ..
                } => {
                    assert_eq!((name.as_str(), *scope), ("i", Scope::Local));
                    assert!(is_var(value, var, Scope::Local));
                }
                other => panic!("expected `Assign i`, got {other:?}"),
            }
        }
        other => panic!("expected a ForRange, got {other:?}"),
    }
    // A fresh loop variable still binds directly.
    let m = lower("for j in range(3):\n    j\n");
    assert!(matches!(&main_body(&m).stmts[0], Stmt::ForRange { var, .. } if var == "j"));
}

#[test]
fn a_for_each_over_a_parameter_stores_into_its_copy() {
    let m = lower("def last(x, xs):\n    for x in xs:\n        x\n    return x\n");
    let f = func(&m, "last");
    let alias = match &f.body.stmts[0] {
        Stmt::LetStarBinding { name, .. } => name.clone(),
        other => panic!("{other:?}"),
    };
    match &f.body.stmts[1] {
        Stmt::ForEach { body, .. } => {
            assert!(matches!(&body.stmts[0], Stmt::Assign { name, .. } if *name == alias));
        }
        other => panic!("{other:?}"),
    }
    assert!(is_var(&f.body.value, &alias, Scope::Local));
}

// ── function bodies: statements before and at the end ───────────────

/// Is `e` a value that is always `None`: `NilLit`, or an `If` whose
/// branches both end in such a value (a tail `if` with no `return`)?
fn always_none(e: &Expr) -> bool {
    match e {
        Expr::NilLit { .. } => true,
        Expr::If {
            then_branch,
            else_branch,
            ..
        } => always_none(&then_branch.value) && always_none(&else_branch.value),
        _ => false,
    }
}

/// A function that falls off its end returns `None`, whatever its last
/// statement is.  These returned the expression, the docstring, and the
/// nested closure.
#[test]
fn falling_off_the_end_returns_none() {
    for (src, name) in [
        ("def f(x):\n    x + 1\n", "f"),
        ("def f():\n    \"doc\"\n", "f"),
        ("def f():\n    def g():\n        return 1\n", "f"),
        ("def f(x):\n    if x:\n        x + 1\n", "f"),
    ] {
        let m = lower(src);
        let f = func(&m, name);
        assert!(
            always_none(&f.body.value),
            "{src:?} returns {:?}",
            f.body.value
        );
    }
}

/// The statement just before a tail `return` is kept.  It used to become
/// the block's value, which the `return` then replaced.
#[test]
fn the_statement_before_a_tail_return_is_kept() {
    // An expression statement (a call) before `return`.
    let m = lower("def g(x):\n    return x\n\ndef f(x):\n    g(x)\n    return 1\n");
    let f = func(&m, "f");
    assert!(
        matches!(&f.body.stmts[..], [Stmt::ExprStmt { expr: Expr::DirectCall { fn_name, .. }, .. }] if fn_name == "g"),
        "{:?}",
        f.body.stmts
    );
    // An `if` statement before `return`, and before a tail `if`.
    for tail in [
        "return n",
        "if n > 0:\n        return 1\n    else:\n        return 2",
    ] {
        let src = format!("def f(n):\n    if n > 10:\n        n = 10\n    {tail}\n");
        let m = lower(&src);
        let f = func(&m, "f");
        assert!(
            f.body.stmts.iter().any(|s| matches!(
                s,
                Stmt::ExprStmt {
                    expr: Expr::If { .. },
                    ..
                }
            )),
            "{tail:?}: the `if` was dropped: {:?}",
            f.body.stmts
        );
    }
}

// ── a user `def main` ────────────────────────────────────────────────

/// `def main` clashed with the synthesised module body.  It is escaped,
/// and calls and closure values follow it.
#[test]
fn a_user_main_is_escaped() {
    let m = lower("def main():\n    return 42\n\nprint(main())\nh = main\n");
    let names: Vec<&str> = m.functions.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names.iter().filter(|n| **n == "main").count(),
        1,
        "{names:?}"
    );
    assert!(names.contains(&"__py_main"), "{names:?}");
    assert!(func(&m, "__py_main").params.is_empty());

    let main = main_body(&m);
    let print_arg = match &main.stmts[0] {
        Stmt::ExprStmt { expr, .. } => builtin(expr).1.last().expect("an argument"),
        other => panic!("{other:?}"),
    };
    assert!(
        matches!(print_arg, Expr::DirectCall { fn_name, .. } if fn_name == "__py_main"),
        "{print_arg:?}"
    );
    assert!(
        matches!(&main.stmts[1], Stmt::LetStarBinding { value: Expr::MakeClosure { fn_name, .. }, .. } if fn_name == "__py_main")
    );
}

/// The escape is one-to-one: a def that spells the escaped name is
/// escaped too, so the two never meet.
#[test]
fn the_main_escape_is_one_to_one() {
    let m = lower(
        "def main():\n    return 1\n\ndef __py_main():\n    return 2\n\ndef other():\n    return 3\n\nprint(main() + __py_main() + other())\n",
    );
    let mut names: Vec<&str> = m.functions.iter().map(|f| f.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["__py___py_main", "__py_main", "main", "other"]);
}

/// A capturing nested `main` is escaped at its closure sites too.
#[test]
fn a_nested_main_is_escaped() {
    let m = lower("def outer(n):\n    def main():\n        return n\n    return main()\n");
    assert_eq!(func(&m, "__py_main").captures.len(), 1);
}

/// Mutual recursion through `main` is still detected (the call graph is
/// keyed by the escaped names, like the calls in it).
#[test]
fn mutual_recursion_through_main_is_detected() {
    let m = lower(
        "def main(n):\n    if n == 0:\n        return 0\n    else:\n        return helper(n - 1)\n\ndef helper(n):\n    return main(n)\n\nprint(main(3))\n",
    );
    assert!(m.manifest.contains(semantic_ir::Feature::MutualRecursion));
}

// ── one-line suites ──────────────────────────────────────────────────

/// `suite = simple_stmt | NEWLINE INDENT { statement } DEDENT`.  A body
/// written on the header's line is a bare `simple_stmt`, with no
/// `statement` wrapper, and the suite lowerers kept only `statement`
/// children: the whole body was dropped.  `def f(x): return x + 1`
/// returned `None`, and `if c: y = 1` did nothing.
#[test]
fn a_one_line_function_body_is_lowered() {
    let m = lower("def f(x): return x + 1\n");
    let (op, args) = builtin(&func(&m, "f").body.value);
    assert_eq!(op, "+");
    assert!(is_var(&args[0], "x", Scope::Param));
}

#[test]
fn one_line_control_flow_bodies_are_lowered() {
    let m = lower("x = 0\nif x == 0: x = 5\nwhile x > 3: x -= 1\nfor i in range(2): x += i\n");
    let stmts = &main_body(&m).stmts;
    match &stmts[1] {
        Stmt::ExprStmt {
            expr: Expr::If { then_branch, .. },
            ..
        } => assert!(
            matches!(&then_branch.stmts[..], [Stmt::Assign { name, .. }] if name == "x"),
            "{then_branch:?}"
        ),
        other => panic!("expected the if, got {other:?}"),
    }
    for (i, what) in [(2, "while"), (3, "for")] {
        let body = match &stmts[i] {
            Stmt::While { body, .. } | Stmt::ForRange { body, .. } => body,
            other => panic!("expected the {what} loop, got {other:?}"),
        };
        assert!(
            matches!(&body.stmts[..], [Stmt::Assign { name, .. }] if name == "x"),
            "{what}: {body:?}"
        );
    }
}

/// A one-line class body is checked like any other: `pass` is accepted,
/// anything but a `def` is refused (it used to be dropped).
#[test]
fn a_one_line_class_body_is_checked() {
    lower("class C: pass\n");
    let err = refuse("class C: x = 1\n");
    assert!(err.message.contains("class body"), "{err}");
}

// ── loop `else:` clauses ─────────────────────────────────────────────

/// A loop's `else:` suite was silently dropped (only the first suite was
/// lowered).  It is refused.
#[test]
fn a_loop_else_clause_is_refused() {
    for src in [
        "x = 0\nwhile x < 3:\n    x += 1\nelse:\n    x = 100\nprint(x)\n",
        "x = 0\nfor i in range(3):\n    x += i\nelse:\n    x = 100\nprint(x)\n",
    ] {
        let err = refuse(src);
        assert!(err.message.contains("else"), "{src:?}: {err}");
        assert_eq!(err.line, 2, "{src:?}: {err}");
    }
}

// ── `pass` (review F1) ───────────────────────────────────────────────

/// `pass` is a statement that does nothing.  It was refused, in every
/// position; one-line `def f(): pass` had been accepted only because
/// one-line bodies used to be dropped, so fixing those made it a
/// refusal.  It now lowers to no statement.
#[test]
fn pass_lowers_to_nothing_everywhere() {
    for src in [
        "def f(): pass\n",
        "def f():\n    pass\n",
        "x = 1\nif x: pass\n",
        "x = 1\nif x:\n    pass\nelse:\n    pass\n",
        "x = 1\nif x == 0:\n    x = 2\nelse: pass\n",
        "for x in [1]: pass\n",
        "for i in range(3):\n    pass\n",
        "x = 0\nwhile x > 0: pass\n",
        "class C:\n    def m(self): pass\n",
    ] {
        lower(src);
    }
    // A body that is only `pass` has no statements and returns None.
    let m = lower("def f(): pass\n");
    let f = func(&m, "f");
    assert!(f.body.stmts.is_empty(), "{:?}", f.body.stmts);
    assert!(matches!(f.body.value, Expr::NilLit { .. }));
    // `pass` between statements adds nothing.
    let m = lower("x = 1\npass\nx = 2\n");
    assert_eq!(main_body(&m).stmts.len(), 2);
}

// ── `for self in …` in a method (review F3) ──────────────────────────

/// A bare `self` inside a method always reads the receiver, so a loop
/// that binds `self` would be invisible to the body.  Refused, like
/// `self = …`.
#[test]
fn for_self_in_a_method_is_refused() {
    let err = refuse(
        "class C:\n    def m(self):\n        for self in [1, 2]:\n            t = self\n        return 0\n",
    );
    assert!(err.message.contains("`for self in"), "{err}");
    assert_eq!(err.line, 3);
    // Outside a method, `self` is an ordinary name.
    lower("for self in [1, 2]:\n    t = self\n");
}

// ── variables spelled with the escape marker (review F4) ─────────────

/// A variable spelled `__py_main` used to keep that name in the IR, the
/// same name the escaped `def main` gets, so on every backend the local
/// shadowed the function.  Variables starting with the marker are
/// escaped too, one to one.
#[test]
fn a_variable_spelled_like_an_escaped_function_is_escaped() {
    let m =
        lower("def main(): return 1\ndef g():\n    __py_main = 5\n    return main() + __py_main\n");
    let g = func(&m, "g");
    match &g.body.stmts[0] {
        Stmt::LetStarBinding { name, .. } => assert_eq!(name, "__py___py_main"),
        other => panic!("{other:?}"),
    }
    let (_, args) = builtin(&g.body.value);
    assert!(
        matches!(&args[0], Expr::DirectCall { fn_name, .. } if fn_name == "__py_main"),
        "{args:?}"
    );
    assert!(is_var(&args[1], "__py___py_main", Scope::Local), "{args:?}");
}

/// Every place a variable name reaches the IR is escaped the same way:
/// parameters, keyword arguments, captures, loop variables.
#[test]
fn escaped_variable_names_agree_across_params_keywords_and_captures() {
    let m = lower(
        "def f(__py_a, *, __py_k=1):\n    def g():\n        return __py_a + __py_k\n    return g()\n\nprint(f(1, __py_k=2))\nfor __py_i in range(2):\n    t = __py_i\n",
    );
    let f = func(&m, "f");
    let params: Vec<&str> = f.params.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(params, ["__py___py_a", "__py___py_k"]);
    let caps: Vec<&str> = func(&m, "g")
        .captures
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(caps, ["__py___py_a", "__py___py_k"]);
    let main = main_body(&m);
    let print_arg = match &main.stmts[0] {
        Stmt::ExprStmt { expr, .. } => builtin(expr).1.last().expect("an argument"),
        other => panic!("{other:?}"),
    };
    match print_arg {
        Expr::DirectCall { args, .. } => assert!(
            matches!(&args[1], Expr::KeywordArg { name, .. } if name == "__py___py_k"),
            "{args:?}"
        ),
        other => panic!("{other:?}"),
    }
    assert!(matches!(&main.stmts[1], Stmt::ForRange { var, .. } if var == "__py___py_i"));
}

// ── loop bodies are statements only (review, found with F2) ──────────

/// A loop body's trailing expression statement used to become the body
/// block's value, which the C and Ruby backends never emit: `for i in
/// range(3): print(i)` printed nothing there.  A loop body now has no
/// value; every statement is a statement.
#[test]
fn a_loop_body_has_no_value() {
    let m = lower("for i in range(3):\n    print(i)\nx = 0\nwhile x < 2:\n    x += 1\n    if x == 2:\n        x\nfor y in [1]:\n    y\n");
    for stmt in &main_body(&m).stmts {
        let body = match stmt {
            Stmt::ForRange { body, .. } | Stmt::While { body, .. } | Stmt::ForEach { body, .. } => {
                body
            }
            _ => continue,
        };
        assert!(matches!(body.value, Expr::NilLit { .. }), "{body:?}");
        assert!(
            matches!(body.stmts.last(), Some(Stmt::ExprStmt { .. })),
            "the trailing expression is kept as a statement: {body:?}"
        );
    }
}

// ── synthesised function names avoid source names (#18482 re-review) ─

/// A synthesised lambda used to be `__lambda_0` even when the program had a
/// local of that name, which shadowed the function on every backend that
/// shares one namespace (JavaScript: `TypeError: __lambda_0 is not a
/// function`).  The lambda now skips every name spelled in the source.
#[test]
fn a_synthesised_lambda_skips_names_in_the_source() {
    let m = lower("def g():\n    __lambda_0 = 5\n    f = lambda: 1\n    return f() + __lambda_0\n");
    let names: Vec<&str> = m.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(!names.contains(&"__lambda_0"), "{names:?}");
    assert!(names.contains(&"__lambda_1"), "{names:?}");
}

/// Likewise a hoisted method, `{Class}__{method}`, skips a source name of
/// the same spelling (and a second method that would hoist to it).
#[test]
fn a_hoisted_method_skips_names_in_the_source() {
    let m = lower(
        "class C:\n    def m(self):\n        return 1\n\nC__m = 5\nclass A:\n    def b__c(self):\n        return 2\n\nclass A__b:\n    def c(self):\n        return 3\n",
    );
    let mut names: Vec<&str> = m.functions.iter().map(|f| f.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["A__b__c", "A__b__c_1", "C__m_1", "main"]);
}
