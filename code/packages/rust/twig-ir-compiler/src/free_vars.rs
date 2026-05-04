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
//! to *and* as the leading arguments to `call_builtin "make_closure"`
//! at the lambda's source position.  If the order drifted, a closure
//! would read its captures from the wrong slots.
//!
//! The traversal is purely structural — a single "currently-bound" set
//! threaded down recursive calls.  Output uses an [`indexmap`-like]
//! pattern (a `Vec` paired with a hash check) so insertion order is
//! preserved without any third-party dependency.

use std::collections::HashSet;

use twig_parser::{Apply, Begin, Expr, If, Lambda, Let};

/// Maximum AST depth the walker will descend before bailing out.
///
/// The parser already enforces its own depth cap so this is a
/// belt-and-braces guard for callers that hand-build an `Expr` tree.
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
pub fn free_vars(lam: &Lambda, globals: &HashSet<String>) -> Vec<String> {
    let mut bound: HashSet<String> = lam.params.iter().cloned().collect();
    let mut found: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for e in &lam.body {
        walk(e, &mut bound, globals, &mut found, &mut seen, 0);
    }
    found
}

fn walk(
    expr: &Expr,
    bound: &mut HashSet<String>,
    globals: &HashSet<String>,
    found: &mut Vec<String>,
    seen: &mut HashSet<String>,
    depth: usize,
) {
    // Depth bound: silently stop descending if we exceed the cap.
    // The compiler's own depth cap fires first on real inputs; this
    // is for hand-built ASTs that bypass the parser.
    if depth > MAX_WALK_DEPTH {
        return;
    }
    let depth = depth + 1;
    match expr {
        // ------------------------------------------------------------------
        // Atoms with no embedded names
        // ------------------------------------------------------------------
        Expr::IntLit(_) | Expr::BoolLit(_) | Expr::NilLit(_) | Expr::SymLit(_) => {}

        // ------------------------------------------------------------------
        // Variable reference — the only place a name actually becomes "free"
        // ------------------------------------------------------------------
        // A reference is free iff it is not currently bound (in some
        // enclosing lambda/let) and not a global.  Builtins are passed in
        // via `globals` so we don't special-case them here.
        Expr::VarRef(v) => {
            if !bound.contains(&v.name) && !globals.contains(&v.name) && !seen.contains(&v.name) {
                seen.insert(v.name.clone());
                found.push(v.name.clone());
            }
        }

        Expr::If(If { cond, then_branch, else_branch, .. }) => {
            walk(cond, bound, globals, found, seen, depth);
            walk(then_branch, bound, globals, found, seen, depth);
            walk(else_branch, bound, globals, found, seen, depth);
        }

        Expr::Begin(Begin { exprs, .. }) => {
            for e in exprs {
                walk(e, bound, globals, found, seen, depth);
            }
        }

        // ------------------------------------------------------------------
        // Let — Scheme semantics: bindings see the OUTER scope
        // ------------------------------------------------------------------
        // The body sees the outer bound-set extended with the binding
        // names.  We restore the outer set on the way out so subsequent
        // peers don't think they're shadowed.
        Expr::Let(Let { bindings, body, .. }) => {
            for (_, rhs) in bindings {
                walk(rhs, bound, globals, found, seen, depth);
            }
            let mut added: Vec<String> = Vec::new();
            for (n, _) in bindings {
                if bound.insert(n.clone()) {
                    added.push(n.clone());
                }
            }
            for e in body {
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
        Expr::Lambda(inner) => {
            let mut added: Vec<String> = Vec::new();
            for p in &inner.params {
                if bound.insert(p.clone()) {
                    added.push(p.clone());
                }
            }
            for e in &inner.body {
                walk(e, bound, globals, found, seen, depth);
            }
            for n in added {
                bound.remove(&n);
            }
        }

        Expr::Apply(Apply { fn_expr, args, .. }) => {
            walk(fn_expr, bound, globals, found, seen, depth);
            for a in args {
                walk(a, bound, globals, found, seen, depth);
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
    use twig_parser::{parse, Form};

    fn lam_in(src: &str) -> Lambda {
        let p = parse(src).unwrap();
        // Helper accepts `(lambda ...)` as the single top-level expression.
        match p.forms.into_iter().next().unwrap() {
            Form::Expr(Expr::Lambda(l)) => l,
            other => panic!("expected top-level lambda, got {other:?}"),
        }
    }

    fn globals(words: &[&str]) -> HashSet<String> {
        words.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_free_vars_when_only_params_referenced() {
        let l = lam_in("(lambda (x) (+ x x))");
        let g = globals(&["+"]);
        assert!(free_vars(&l, &g).is_empty());
    }

    #[test]
    fn captures_are_free() {
        // n is captured from the surrounding scope
        let l = lam_in("(lambda (x) (+ x n))");
        let g = globals(&["+"]);
        assert_eq!(free_vars(&l, &g), vec!["n".to_string()]);
    }

    #[test]
    fn globals_are_not_free() {
        // `add` is in globals → not captured
        let l = lam_in("(lambda (x) (add x 1))");
        let g = globals(&["add"]);
        assert!(free_vars(&l, &g).is_empty());
    }

    #[test]
    fn duplicate_references_appear_once_in_stable_order() {
        let l = lam_in("(lambda (x) (+ a (+ a (+ b a))))");
        let g = globals(&["+"]);
        assert_eq!(free_vars(&l, &g), vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn let_binding_shadows_capture() {
        let l = lam_in("(lambda (x) (let ((y 1)) (+ x y)))");
        let g = globals(&["+"]);
        assert!(free_vars(&l, &g).is_empty(), "y is bound by let, not captured");
    }

    #[test]
    fn let_rhs_uses_outer_scope() {
        // y appears on the RHS of a let — that RHS is in the outer
        // scope, so y is captured.
        let l = lam_in("(lambda (x) (let ((q y)) (+ x q)))");
        let g = globals(&["+"]);
        assert_eq!(free_vars(&l, &g), vec!["y".to_string()]);
    }

    #[test]
    fn nested_lambda_captures_bubble_up() {
        // Inner lambda captures `n`; outer lambda has no `n` — so `n`
        // bubbles up as outer's capture.
        let l = lam_in("(lambda () (lambda (x) (+ x n)))");
        let g = globals(&["+"]);
        assert_eq!(free_vars(&l, &g), vec!["n".to_string()]);
    }

    #[test]
    fn nested_lambda_param_does_not_bubble_up() {
        // Inner lambda has its own `n` — outer must NOT capture `n`.
        let l = lam_in("(lambda () (lambda (n) (+ n 1)))");
        let g = globals(&["+"]);
        assert!(free_vars(&l, &g).is_empty());
    }

    #[test]
    fn if_branches_all_walked() {
        let l = lam_in("(lambda () (if a b c))");
        let g = globals(&[]);
        assert_eq!(
            free_vars(&l, &g),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn begin_walked() {
        let l = lam_in("(lambda () (begin a b))");
        let g = globals(&[]);
        assert_eq!(free_vars(&l, &g), vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn quoted_symbols_are_not_var_refs() {
        // 'foo is a SymLit, not a VarRef — never captured.
        let l = lam_in("(lambda () 'foo)");
        let g = globals(&[]);
        assert!(free_vars(&l, &g).is_empty());
    }
}
