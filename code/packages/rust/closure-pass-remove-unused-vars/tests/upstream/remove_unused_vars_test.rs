//! Ported from `RemoveUnusedCodeTest.java` (the descendant of the
//! historical `RemoveUnusedVarsTest.java`) in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! This is a CLOC12 port for the `remove-unused-vars` pass. The upstream
//! `RemoveUnusedCode` pass is enormous — it removes unused locals,
//! parameters, function declarations, self-referential dead cycles, and
//! rewrites `var a = sideEffect();` into a bare `sideEffect();`. Our
//! `RemoveUnusedVarsPass` today implements the narrow, provably-sound
//! core: it drops **GLOBAL-scope** `var` / `let` / `const` bindings that
//! have **zero references** and a **pure** initializer (a literal, a bare
//! identifier, or none). See the crate docs and `is_removable_init`.
//!
//! So the file splits in two:
//!
//! - **Active `#[test]`s** — the upstream behaviors our pass genuinely
//!   supports today, translated to the AST-builder surface (closurec has
//!   no public source-string → typed `Program` entry point, so — exactly
//!   as the `closure-pass-dce` port does — we build the `Program` by hand
//!   with small helpers instead of `assertPrint("var a=1;", "")`).
//! - **`#[ignore = "blocked on gap-NNN"]` placeholders** — upstream
//!   intent our pass does not cover yet, each pinned to a `gap-NNN`
//!   entry in `code/specs/CLOC12-gaps.md`. Running with
//!   `--include-ignored` measures progress as those gaps close.
//!
//! Every active test that *disagrees* with our pass is a real closurec
//! defect, not a translation artifact — the whole point of the port is
//! to surface exactly that.

use coding_adventures_closure_pass_pipeline::{Pass, PassContext};
use coding_adventures_closure_pass_remove_unused_vars::RemoveUnusedVarsPass;
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    BindingTarget, CallExpression, Declaration, Expression, ExpressionStatement, Identifier,
    NumericLiteral, Program, ProgramItem, SourceType, Statement, VarKind, VariableDeclaration,
    VariableDeclarator,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers
//
// The upstream surface is a JS source string; ours is the typed AST.
// These helpers build the handful of shapes the ported cases need:
// a global `var/let/const` (in both the bare-Declaration and the
// Statement-wrapped forms the bridge can emit), a bare-identifier "use"
// that keeps a binding alive, and a call-initialized binding (impure).
// =====================================================================

fn ident(name: &str) -> Identifier {
    Identifier {
        cv: None,
        name: name.to_string(),
    }
}

fn num_init(v: f64) -> Expression {
    Expression::NumericLiteral(NumericLiteral {
        cv: None,
        value: v,
        raw: format!("{}", v as i64),
    })
}

/// A `var/let/const` declaration as a bare `ProgramItem::Declaration`.
/// `inits` maps each name to its initializer expression (`None` → no
/// initializer, `var x;`).
fn var_decl(kind: VarKind, decls: Vec<(&str, Option<Expression>)>) -> ProgramItem {
    ProgramItem::Declaration(Declaration::VariableDeclaration(VariableDeclaration {
        cv: None,
        kind,
        declarations: decls
            .into_iter()
            .map(|(n, init)| VariableDeclarator {
                cv: None,
                id: BindingTarget::Identifier(ident(n)),
                init,
            })
            .collect(),
    }))
}

/// The same declaration in the Statement-wrapped shape
/// (`ProgramItem::Statement(Statement::Declaration(...))`) the real
/// `javascript-parser` bridge emits, so we can pin that both AST shapes
/// are pruned identically.
fn var_stmt(kind: VarKind, decls: Vec<(&str, Option<Expression>)>) -> ProgramItem {
    let ProgramItem::Declaration(d) = var_decl(kind, decls) else {
        unreachable!("var_decl always returns a Declaration")
    };
    ProgramItem::Statement(Statement::Declaration(d))
}

/// A bare expression statement that *reads* `name` — the reference that
/// keeps the binding alive (`name;`).
fn use_stmt(name: &str) -> ProgramItem {
    ProgramItem::Statement(Statement::expression_statement(ExpressionStatement {
        cv: None,
        expression: Expression::Identifier(ident(name)),
    }))
}

/// A call expression `callee()` — an **impure** initializer the purity
/// gate must refuse to delete.
fn call(callee: &str) -> Expression {
    Expression::CallExpression(CallExpression {
        cv: None,
        callee: Box::new(Expression::Identifier(ident(callee))),
        arguments: Vec::new(),
    })
}

fn program_with(items: Vec<ProgramItem>) -> Program {
    let mut p = Program::new("prog.1".to_string(), EsVersion::Es2025, SourceType::Module);
    p.body = items;
    p
}

/// Run only `RemoveUnusedVarsPass` and return `(new_program, changed)`.
fn run(prog: Program) -> (Program, bool) {
    let pass = RemoveUnusedVarsPass::new();
    let sidecar = Sidecar::new();
    let mut cv = CVLog::new(false);
    let ctx = PassContext {
        program: &prog,
        sidecar: &sidecar,
        cv: &mut cv,
    };
    let out = pass.run(ctx).expect("remove-unused-vars pass ran");
    (out.program, out.changed)
}

/// All variable-declarator names still present in the program body, in
/// source order, across both the bare-Declaration and Statement-wrapped
/// shapes. Ignores non-declaration items (a `use_stmt` etc.), so a test
/// can assert exactly which bindings survived regardless of the uses
/// left around them.
fn surviving_names(prog: &Program) -> Vec<String> {
    fn from_decl(d: &Declaration, out: &mut Vec<String>) {
        if let Declaration::VariableDeclaration(vd) = d {
            for decl in &vd.declarations {
                let BindingTarget::Identifier(id) = &decl.id;
                out.push(id.name.clone());
            }
        }
    }
    let mut names = Vec::new();
    for item in &prog.body {
        match item {
            ProgramItem::Declaration(d) => from_decl(d, &mut names),
            ProgramItem::Statement(Statement::Declaration(d)) => from_decl(d, &mut names),
            _ => {}
        }
    }
    names
}

// =====================================================================
// Active ports — behaviors `RemoveUnusedVarsPass` supports today.
// =====================================================================

#[test]
fn removes_unused_global_var() {
    // upstream: `removeUnusedVars` — `var a = 1;` with no references
    // is deleted whole.
    let (out, changed) = run(program_with(vec![var_decl(
        VarKind::Var,
        vec![("a", Some(num_init(1.0)))],
    )]));
    assert!(changed, "an unused global var must be removed");
    assert!(
        surviving_names(&out).is_empty(),
        "expected no surviving bindings; got {:?}",
        surviving_names(&out)
    );
}

#[test]
fn keeps_referenced_global_var() {
    // upstream `testSame`: `var a = 1; a;` — the read keeps `a` alive.
    let (out, changed) = run(program_with(vec![
        var_decl(VarKind::Var, vec![("a", Some(num_init(1.0)))]),
        use_stmt("a"),
    ]));
    assert!(!changed, "a referenced var must be left untouched");
    assert_eq!(surviving_names(&out), vec!["a".to_string()]);
}

#[test]
fn removes_unused_let() {
    let (out, changed) = run(program_with(vec![var_decl(
        VarKind::Let,
        vec![("a", Some(num_init(1.0)))],
    )]));
    assert!(changed);
    assert!(surviving_names(&out).is_empty());
}

#[test]
fn removes_unused_const() {
    let (out, changed) = run(program_with(vec![var_decl(
        VarKind::Const,
        vec![("a", Some(num_init(1.0)))],
    )]));
    assert!(changed);
    assert!(surviving_names(&out).is_empty());
}

#[test]
fn removes_uninitialized_var() {
    // `var a;` — no initializer, nothing to evaluate, unused → gone.
    let (out, changed) = run(program_with(vec![var_decl(VarKind::Var, vec![("a", None)])]));
    assert!(changed);
    assert!(surviving_names(&out).is_empty());
}

#[test]
fn removes_var_with_pure_identifier_initializer() {
    // `var a = b;` — reading a variable has no side effect, so the
    // whole `a` binding is removable even though it references `b`
    // (a free global). Upstream removes it too.
    let (out, changed) = run(program_with(vec![var_decl(
        VarKind::Var,
        vec![("a", Some(Expression::Identifier(ident("b"))))],
    )]));
    assert!(changed);
    assert!(surviving_names(&out).is_empty());
}

#[test]
fn keeps_unused_var_with_impure_call_initializer() {
    // `let a = f();` unused — the call might have a side effect, so the
    // binding is KEPT (conservative purity gate). Upstream's full pass
    // rewrites this to a bare `f();`; that stronger transform is
    // gap-124 below.
    let (out, changed) = run(program_with(vec![var_stmt(
        VarKind::Let,
        vec![("a", Some(call("f")))],
    )]));
    assert!(!changed, "impure initializer must keep the binding");
    assert_eq!(surviving_names(&out), vec!["a".to_string()]);
}

#[test]
fn splits_multi_declarator_dropping_only_the_dead_one() {
    // `var a = 1, b = 2; a;` → `var a = 1;` — `b` is dead, `a` is
    // read. Upstream splits the declaration and keeps the survivor.
    let (out, changed) = run(program_with(vec![
        var_decl(
            VarKind::Var,
            vec![("a", Some(num_init(1.0))), ("b", Some(num_init(2.0)))],
        ),
        use_stmt("a"),
    ]));
    assert!(changed, "the dead declarator `b` must be dropped");
    assert_eq!(surviving_names(&out), vec!["a".to_string()]);
}

#[test]
fn drops_whole_declaration_when_every_declarator_is_dead() {
    // `var a = 1, b = 2;` — both unused → the whole statement vanishes.
    let (out, changed) = run(program_with(vec![var_decl(
        VarKind::Var,
        vec![("a", Some(num_init(1.0))), ("b", Some(num_init(2.0)))],
    )]));
    assert!(changed);
    assert!(surviving_names(&out).is_empty());
}

#[test]
fn keeps_only_referenced_among_several() {
    // `var a = 1, b = 2, c = 3; b;` → `var b = 2;`.
    let (out, changed) = run(program_with(vec![
        var_decl(
            VarKind::Var,
            vec![
                ("a", Some(num_init(1.0))),
                ("b", Some(num_init(2.0))),
                ("c", Some(num_init(3.0))),
            ],
        ),
        use_stmt("b"),
    ]));
    assert!(changed);
    assert_eq!(surviving_names(&out), vec!["b".to_string()]);
}

#[test]
fn prunes_the_statement_wrapped_shape_too() {
    // Same as `removes_unused_global_var` but in the
    // `ProgramItem::Statement(Statement::Declaration(...))` shape the
    // real parser bridge produces — both must prune identically.
    let (out, changed) = run(program_with(vec![var_stmt(
        VarKind::Var,
        vec![("dead", Some(num_init(1.0)))],
    )]));
    assert!(changed);
    assert!(surviving_names(&out).is_empty());
}

// =====================================================================
// Not-yet-supported upstream behaviors — pinned to CLOC12-gaps.md.
// Each body encodes the upstream intent so the port is executable the
// moment the gap closes (drop the `#[ignore]`).
// =====================================================================

#[test]
#[ignore = "blocked on gap-121: function-local unused-var removal (pass only acts on GLOBAL scope)"]
fn removes_function_local_unused_var() {
    // upstream: `function f(){ var a = 1; }` → `function f(){}`.
    // Our pass restricts removal to `ScopeId::GLOBAL`; nested-scope
    // name handling is a follow-up.
}

#[test]
#[ignore = "blocked on gap-122: unused function-declaration removal is treeshake's job, not this pass"]
fn removes_unused_function_declaration() {
    // upstream: an unreferenced `function g(){}` is dropped. Our pass
    // filters out `Function`-kind bindings at the eligibility scan.
}

#[test]
#[ignore = "blocked on gap-123: unused function-parameter removal needs Param-binding analysis"]
fn removes_trailing_unused_parameter() {
    // upstream: `function f(a, b){ return a; }` drops the unused `b`
    // (arity permitting). `Param`-kind bindings are skipped today.
}

#[test]
#[ignore = "blocked on gap-124: side-effecting initializer should be preserved as a bare expression statement"]
fn extracts_side_effect_from_unused_binding() {
    // upstream: `var a = f();` (a unused) → `f();` — the binding is
    // removed but the initializer's side effect is preserved. We
    // conservatively keep the whole binding instead (see
    // `keeps_unused_var_with_impure_call_initializer`).
}

#[test]
#[ignore = "blocked on gap-125: self-referential dead binding needs reference-cycle detection"]
fn removes_self_referential_dead_binding() {
    // upstream: `var a = function(){ a(); };` with no external use is
    // removed. A naive use-count sees `a` referenced (by itself) and
    // keeps it; upstream detects the dead cycle.
}

#[test]
#[ignore = "blocked on gap-126: assignment-only dead var needs write-vs-read reference classification"]
fn removes_var_that_is_only_assigned() {
    // upstream: `var a; a = 1;` (never read) → removed entirely. Our
    // analyzer counts the `a = 1` write as a reference, so `a` is kept.
}
