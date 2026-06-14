//! Simple structural substitution.
//!
//! [`subst`]`(value, var, expr)` walks `expr` and replaces every node that
//! is structurally equal to `var` with a clone of `value`.
//!
//! This mirrors the MACSYMA convention:
//! `subst(2, x, expr)` means "replace x with 2 in expr".
//!
//! The search target `var` can be *any* IR node — a symbol, an integer, or
//! even a compound `Apply` expression.  This is more general than purely
//! symbolic substitution:
//!
//! ```text
//! subst(z, x+1, (x+1)*(x+1))  →  z*z
//! ```

use symbolic_ir::{IRApply, IRNode};

/// Replace every occurrence of `var` in `expr` with a clone of `value`.
///
/// Match is structural equality — any sub-tree equal to `var` is replaced.
/// The walk is top-down: if `expr == var` the substitution fires immediately
/// without recursing into `expr`'s children.
///
/// # Examples
///
/// ```rust
/// use symbolic_ir::{apply, int, sym, MUL, POW};
/// use cas_substitution::subst;
///
/// // subst(2, x, x^2) → Pow(2, 2)
/// let expr = apply(sym(POW), vec![sym("x"), int(2)]);
/// assert_eq!(subst(int(2), &sym("x"), expr), apply(sym(POW), vec![int(2), int(2)]));
///
/// // Any sub-expression can be the target:
/// // subst(z, x+1, (x+1)*(x+1)) → z*z
/// let target = apply(sym("Add"), vec![sym("x"), int(1)]);
/// let expr2 = apply(sym(MUL), vec![target.clone(), target.clone()]);
/// assert_eq!(
///     subst(sym("z"), &target, expr2),
///     apply(sym(MUL), vec![sym("z"), sym("z")]),
/// );
/// ```
pub fn subst(value: IRNode, var: &IRNode, expr: IRNode) -> IRNode {
    // Top-down: if the whole node matches `var`, replace immediately.
    if &expr == var {
        return value;
    }

    // Recurse into Apply nodes only; leaves that differ from `var` are
    // returned unchanged.
    match expr {
        IRNode::Apply(apply) => {
            let IRApply { head, args } = *apply;
            let new_head = subst(value.clone(), var, head);
            let new_args: Vec<IRNode> = args
                .into_iter()
                .map(|a| subst(value.clone(), var, a))
                .collect();
            IRNode::Apply(Box::new(IRApply { head: new_head, args: new_args }))
        }
        other => other,
    }
}

/// Apply a sequence of `(var, value)` substitutions in order.
///
/// Each substitution is applied to the result of the previous one.  Rules
/// can interact: the second rule sees the first's output and may rewrite
/// parts of the already-substituted expression.
///
/// ```rust
/// use symbolic_ir::{apply, int, sym, ADD};
/// use cas_substitution::subst_many;
///
/// // subst_many([(x, 2), (y, 3)], x + y) → Add(2, 3)
/// let expr = apply(sym(ADD), vec![sym("x"), sym("y")]);
/// let rules = vec![(sym("x"), int(2)), (sym("y"), int(3))];
/// assert_eq!(subst_many(&rules, expr), apply(sym(ADD), vec![int(2), int(3)]));
/// ```
pub fn subst_many(rules: &[(IRNode, IRNode)], expr: IRNode) -> IRNode {
    let mut out = expr;
    for (var, value) in rules {
        out = subst(value.clone(), var, out);
    }
    out
}
