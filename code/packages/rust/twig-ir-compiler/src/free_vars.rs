//! Free-variable analysis for Twig lambdas.
//!
//! When a `(lambda ...)` appears inside another function it may reference
//! names that are *neither* its own parameters *nor* top-level
//! `define`-bound globals.  Those names need to be **captured** at
//! closure-construction time and stored inside the closure object so
//! they remain available when the closure is later applied.
//!
//! [`free_vars`] computes that set in stable order — the order is
//! significant because the IR compiler emits the captures both as the
//! leading parameters of the gensym'd `IIRFunction` the lambda compiles
//! to *and* as the leading operands of the `alloc_closure` instruction
//! at the lambda's source position.  If the order drifted, a closure
//! would read its captures from the wrong slots.
//!
//! The traversal is purely structural — a single "currently-bound" set
//! threaded down recursive calls.  It walks the parse tree through the
//! borrowed views in [`crate::cst`] (LANG84: no typed AST on the IIR
//! path).  Output uses an [`indexmap`-like] pattern (a `Vec` paired with
//! a hash check) so insertion order is preserved without any third-party
//! dependency.

use std::collections::HashSet;

use crate::cst::{ExprKind, ExprNode, FormKind, LambdaView, ProgramView};

/// Maximum AST depth the walker will descend before bailing out.
///
/// The parser and `check_cst` already enforce their own depth caps, so
/// this is a belt-and-braces guard for hand-built trees.
/// Hitting this limit returns an empty capture list — safe-ish but
/// unusual; callers that build trees deeper than this should bound
/// depth themselves before calling `free_vars`.
const MAX_WALK_DEPTH: usize = 256;

/// Compute the free variables of `lam`, in stable insertion order.
///
/// `globals` holds *all* names that count as already-bound at any
/// position — top-level `define`-introduced names plus the builtin
/// vocabulary (`+`, `cons`, `null?`, …).  The IR compiler builds and
/// passes that set; this module doesn't hardcode the builtin list.
///
/// # Determinism
///
/// Free-variable order is the order names are *first encountered* by a
/// left-to-right walk of the lambda body.  Two compiles of the same
/// source produce identical closure shapes — important for caching
/// closure-handle layouts and for golden-test stability.
pub(crate) fn free_vars(lam: LambdaView<'_>, globals: &HashSet<String>) -> Vec<String> {
    let mut bound: HashSet<String> = lam.params().map(|p| p.name.to_string()).collect();
    let mut found: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for e in lam.body().iter() {
        walk(e, &mut bound, globals, &mut found, &mut seen, 0);
    }
    found
}

/// Determine which of `value_globals` are **captured by a lambda** anywhere in
/// the program — i.e. referenced inside some `(lambda …)` body rather than only
/// at the top level of `main`.
///
/// This is the safety analysis behind the TW2 "top-level value `define`"
/// optimisation.  A value `define` whose name is *not* captured is read only
/// from `main`, so the compiler can keep its value in a typed register (a
/// `const` / `mov` that the code-gen backends accept) and drop the dynamic
/// `global_set` / `global_get` pair.  A captured one MUST stay on the host
/// global table, because the closure it is read from compiles to a *separate*
/// `IIRFunction` that has no access to `main`'s registers.
///
/// The analysis reuses [`free_vars`]: at every lambda encountered, the free
/// variables (computed against an *empty* global set, so every non-local name
/// counts) that happen to be value-globals are exactly the ones that lambda
/// captures.  Over-approximation is safe here — a name wrongly flagged as
/// captured merely stays on the (correct, if slower) global-table path.
pub(crate) fn lambda_captured_globals(
    program: ProgramView<'_>,
    value_globals: &HashSet<String>,
) -> HashSet<String> {
    let mut captured = HashSet::new();
    for form in program.forms() {
        match form {
            // A define's right-hand side: function sugar is a lambda in its
            // own right; a value define is an ordinary expression.
            FormKind::Define(d) => match (d.lambda(), d.value()) {
                (Some(lam), None) => capture_lambda(lam, value_globals, &mut captured),
                (_, Some(e)) => collect_lambda_captures(e, value_globals, &mut captured, 0),
                (None, None) => {}
            },
            FormKind::Expr(e) => collect_lambda_captures(e, value_globals, &mut captured, 0),
            // RecordDef / UnionDef synthesise constructor / accessor functions,
            // but those reference only their own parameters — never a user
            // value-global — so they capture nothing.  TypeAlias is erased.
            _ => {}
        }
    }
    captured
}

/// Union the value-globals one lambda captures into `captured`.
fn capture_lambda(lam: LambdaView<'_>, value_globals: &HashSet<String>, captured: &mut HashSet<String>) {
    for name in free_vars(lam, &HashSet::new()) {
        if value_globals.contains(&name) {
            captured.insert(name);
        }
    }
}

/// Recurse through `expr` looking for lambdas; union each lambda's captured
/// value-globals into `captured`.  `free_vars` already descends into nested
/// lambdas, so once a lambda is found we do not recurse into it again here.
fn collect_lambda_captures(
    expr: ExprNode<'_>,
    value_globals: &HashSet<String>,
    captured: &mut HashSet<String>,
    depth: usize,
) {
    // Same belt-and-braces depth guard as `walk`: a hand-built AST (one that
    // reached the public `compile_ast` entry without the parser's
    // paren-depth pre-scan) must not overflow the stack here before
    // the main pass's `MAX_COMPILE_DEPTH` guard would fire.  Bailing early only
    // stops accumulating more captures; the main pass still produces the hard
    // depth error, and stopping early can only *under*-collect — which here is
    // safe because such a deep tree is rejected downstream anyway.
    if depth > MAX_WALK_DEPTH {
        return;
    }
    let depth = depth + 1;
    match expr.kind() {
        ExprKind::Lambda(l) => capture_lambda(l, value_globals, captured),
        ExprKind::If(i) => {
            collect_lambda_captures(i.cond, value_globals, captured, depth);
            collect_lambda_captures(i.then_branch, value_globals, captured, depth);
            collect_lambda_captures(i.else_branch, value_globals, captured, depth);
        }
        ExprKind::Begin(exprs) => {
            for e in exprs.iter() {
                collect_lambda_captures(e, value_globals, captured, depth);
            }
        }
        ExprKind::Let(l) | ExprKind::LetStar(l) => {
            for (_, rhs) in l.bindings() {
                collect_lambda_captures(rhs, value_globals, captured, depth);
            }
            for e in l.body().iter() {
                collect_lambda_captures(e, value_globals, captured, depth);
            }
        }
        ExprKind::Apply(a) => {
            if let Some(f) = a.fn_expr() {
                collect_lambda_captures(f, value_globals, captured, depth);
            }
            for e in a.args().iter() {
                collect_lambda_captures(e, value_globals, captured, depth);
            }
        }
        ExprKind::Match(m) => {
            if let Some(s) = m.scrutinee() {
                collect_lambda_captures(s, value_globals, captured, depth);
            }
            for arm in m.arms() {
                for e in arm.body().iter() {
                    collect_lambda_captures(e, value_globals, captured, depth);
                }
            }
        }
        // Atoms — no nested lambdas.
        ExprKind::IntLit(_) | ExprKind::BoolLit(_) | ExprKind::NilLit | ExprKind::SymLit(_)
        | ExprKind::StrLit(_) | ExprKind::VarRef(_) => {}
    }
}

fn walk(
    expr: ExprNode<'_>,
    bound: &mut HashSet<String>,
    globals: &HashSet<String>,
    found: &mut Vec<String>,
    seen: &mut HashSet<String>,
    depth: usize,
) {
    // Depth bound: silently stop descending if we exceed the cap.
    // The compiler's own depth cap fires first on real inputs; this
    // is for hand-built trees that bypass the parser.
    if depth > MAX_WALK_DEPTH {
        return;
    }
    let depth = depth + 1;

    /// Bind `names` for the duration of `body`, then unbind exactly the
    /// ones that were not already bound (so an inner binder that shadows
    /// an outer one does not unbind the outer one on the way out).
    fn with_bound<'n>(
        bound: &mut HashSet<String>,
        names: impl Iterator<Item = &'n str>,
        body: impl FnOnce(&mut HashSet<String>),
    ) {
        let mut added: Vec<String> = Vec::new();
        for n in names {
            if bound.insert(n.to_string()) {
                added.push(n.to_string());
            }
        }
        body(bound);
        for n in added {
            bound.remove(&n);
        }
    }

    match expr.kind() {
        // ------------------------------------------------------------------
        // Atoms with no embedded names
        // ------------------------------------------------------------------
        // LANG51: StrLit is also a leaf — no variable names to capture.
        ExprKind::IntLit(_) | ExprKind::BoolLit(_) | ExprKind::NilLit | ExprKind::SymLit(_)
        | ExprKind::StrLit(_) => {}

        // ------------------------------------------------------------------
        // Variable reference — the only place a name actually becomes "free"
        // ------------------------------------------------------------------
        // A reference is free iff it is not currently bound (in some
        // enclosing lambda/let) and not a global.  Builtins are passed in
        // via `globals` so we don't special-case them here.
        ExprKind::VarRef(name) => {
            if !bound.contains(name) && !globals.contains(name) && !seen.contains(name) {
                seen.insert(name.to_string());
                found.push(name.to_string());
            }
        }

        ExprKind::If(i) => {
            walk(i.cond, bound, globals, found, seen, depth);
            walk(i.then_branch, bound, globals, found, seen, depth);
            walk(i.else_branch, bound, globals, found, seen, depth);
        }

        ExprKind::Begin(exprs) => {
            for e in exprs.iter() {
                walk(e, bound, globals, found, seen, depth);
            }
        }

        // ------------------------------------------------------------------
        // Let — Scheme semantics: bindings see the OUTER scope
        // ------------------------------------------------------------------
        // The body sees the outer bound-set extended with the binding
        // names.  We restore the outer set on the way out so subsequent
        // peers don't think they're shadowed.
        ExprKind::Let(l) => {
            for (_, rhs) in l.bindings() {
                walk(rhs, bound, globals, found, seen, depth);
            }
            with_bound(bound, l.bindings().map(|(n, _)| n), |bound| {
                for e in l.body().iter() {
                    walk(e, bound, globals, found, seen, depth);
                }
            });
        }

        // LANG52: let* — each RHS is in scope of all prior bindings, so we
        // bind incrementally as we walk.
        ExprKind::LetStar(l) => {
            let mut added: Vec<String> = Vec::new();
            for (n, rhs) in l.bindings() {
                // The RHS is walked BEFORE the name is bound — same as the
                // compiler: each binding sees all *prior* names, but not itself.
                walk(rhs, bound, globals, found, seen, depth);
                if bound.insert(n.to_string()) {
                    added.push(n.to_string());
                }
            }
            for e in l.body().iter() {
                walk(e, bound, globals, found, seen, depth);
            }
            for n in added {
                bound.remove(&n);
            }
        }

        // ------------------------------------------------------------------
        // Inner lambda — params are bound for its body
        // ------------------------------------------------------------------
        // Free names inside the inner lambda that aren't its params and
        // aren't ours (the outer lambda's captures so far) bubble up as
        // captures of the outer lambda too.
        ExprKind::Lambda(inner) => {
            with_bound(bound, inner.params().map(|p| p.name), |bound| {
                for e in inner.body().iter() {
                    walk(e, bound, globals, found, seen, depth);
                }
            });
        }

        ExprKind::Apply(a) => {
            if let Some(f) = a.fn_expr() {
                walk(f, bound, globals, found, seen, depth);
            }
            for e in a.args().iter() {
                walk(e, bound, globals, found, seen, depth);
            }
        }

        // TW05-A / LANG48 — match expression free-variable analysis.
        //
        // The scrutinee is evaluated in the outer scope.  Each arm's
        // body is evaluated with the arm's bindings added to `bound`.
        ExprKind::Match(m) => {
            if let Some(s) = m.scrutinee() {
                walk(s, bound, globals, found, seen, depth);
            }
            for arm in m.arms() {
                with_bound(bound, arm.pat().bound_names(), |bound| {
                    for e in arm.body().iter() {
                        walk(e, bound, globals, found, seen, depth);
                    }
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::ProgramView;
    use parser::grammar_parser::GrammarASTNode;
    use twig_parser::parse_to_ast;

    /// Parse `src` (a single top-level `(lambda ...)`) to its CST.
    fn cst(src: &str) -> GrammarASTNode {
        parse_to_ast(src).unwrap()
    }

    /// The lambda view of the only top-level form of `tree`.
    fn lam_in(tree: &GrammarASTNode) -> LambdaView<'_> {
        match ProgramView(tree).forms().next() {
            Some(FormKind::Expr(e)) => match e.kind() {
                ExprKind::Lambda(l) => l,
                _ => panic!("expected a top-level lambda"),
            },
            _ => panic!("expected a top-level expression"),
        }
    }

    fn globals(words: &[&str]) -> HashSet<String> {
        words.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_free_vars_when_only_params_referenced() {
        let t = cst("(lambda (x) (+ x x))");
        let l = lam_in(&t);
        let g = globals(&["+"]);
        assert!(free_vars(l, &g).is_empty());
    }

    #[test]
    fn captures_are_free() {
        // n is captured from the surrounding scope
        let t = cst("(lambda (x) (+ x n))");
        let l = lam_in(&t);
        let g = globals(&["+"]);
        assert_eq!(free_vars(l, &g), vec!["n".to_string()]);
    }

    #[test]
    fn globals_are_not_free() {
        // `add` is in globals → not captured
        let t = cst("(lambda (x) (add x 1))");
        let l = lam_in(&t);
        let g = globals(&["add"]);
        assert!(free_vars(l, &g).is_empty());
    }

    #[test]
    fn duplicate_references_appear_once_in_stable_order() {
        let t = cst("(lambda (x) (+ a (+ a (+ b a))))");
        let l = lam_in(&t);
        let g = globals(&["+"]);
        assert_eq!(free_vars(l, &g), vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn let_binding_shadows_capture() {
        let t = cst("(lambda (x) (let ((y 1)) (+ x y)))");
        let l = lam_in(&t);
        let g = globals(&["+"]);
        assert!(free_vars(l, &g).is_empty(), "y is bound by let, not captured");
    }

    #[test]
    fn let_rhs_uses_outer_scope() {
        // y appears on the RHS of a let — that RHS is in the outer
        // scope, so y is captured.
        let t = cst("(lambda (x) (let ((q y)) (+ x q)))");
        let l = lam_in(&t);
        let g = globals(&["+"]);
        assert_eq!(free_vars(l, &g), vec!["y".to_string()]);
    }

    #[test]
    fn nested_lambda_captures_bubble_up() {
        // Inner lambda captures `n`; outer lambda has no `n` — so `n`
        // bubbles up as outer's capture.
        let t = cst("(lambda () (lambda (x) (+ x n)))");
        let l = lam_in(&t);
        let g = globals(&["+"]);
        assert_eq!(free_vars(l, &g), vec!["n".to_string()]);
    }

    #[test]
    fn nested_lambda_param_does_not_bubble_up() {
        // Inner lambda has its own `n` — outer must NOT capture `n`.
        let t = cst("(lambda () (lambda (n) (+ n 1)))");
        let l = lam_in(&t);
        let g = globals(&["+"]);
        assert!(free_vars(l, &g).is_empty());
    }

    #[test]
    fn if_branches_all_walked() {
        let t = cst("(lambda () (if a b c))");
        let l = lam_in(&t);
        let g = globals(&[]);
        assert_eq!(
            free_vars(l, &g),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn begin_walked() {
        let t = cst("(lambda () (begin a b))");
        let l = lam_in(&t);
        let g = globals(&[]);
        assert_eq!(free_vars(l, &g), vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn quoted_symbols_are_not_var_refs() {
        // 'foo is a SymLit, not a VarRef — never captured.
        let t = cst("(lambda () 'foo)");
        let l = lam_in(&t);
        let g = globals(&[]);
        assert!(free_vars(l, &g).is_empty());
    }
}
