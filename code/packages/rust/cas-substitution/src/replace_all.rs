//! Pattern-aware `replace_all` — Mathematica's `/.` operator.
//!
//! Walks an IR tree and applies a single `Rule(lhs, rhs)` everywhere a
//! match is found.  Differs from [`crate::subst`] in two ways:
//!
//! 1. The search target is a *pattern* (which may include named
//!    [`cas_pattern_matching::blank`] wildcards), not a literal expression.
//! 2. `replace_all` is **single-pass** — once a node is rewritten, the
//!    replacement is not searched again.  For repeated application use
//!    [`cas_pattern_matching::rewrite`].
//!
//! ## Walk order
//!
//! Top-down: at each node we try the rule first.  If it fires, we return
//! the replacement without recursing into it.  If it doesn't fire, we
//! recurse into the head and args.
//!
//! This matches the Python reference implementation and Mathematica's
//! `ReplaceAll[expr, rule]` semantics.

use cas_pattern_matching::apply_rule;
use symbolic_ir::{IRApply, IRNode};

/// Apply `rule` to every position in `expr` where it matches (top-down,
/// single-pass).
///
/// `rule` must be an `IRNode::Apply(Rule, [lhs, rhs])` as produced by
/// [`cas_pattern_matching::rule`].
///
/// # Examples
///
/// ```rust
/// use symbolic_ir::{apply, int, sym, MUL, POW};
/// use cas_pattern_matching::{blank, named, rule};
/// use cas_substitution::replace_all;
///
/// // Rule: Pow(a_, 2) → Mul(a_, a_)
/// let r = rule(
///     apply(sym(POW), vec![named("a", blank()), int(2)]),
///     apply(sym(MUL), vec![named("a", blank()), named("a", blank())]),
/// );
/// let expr = apply(sym(POW), vec![sym("y"), int(2)]);
/// assert_eq!(replace_all(expr, &r), apply(sym(MUL), vec![sym("y"), sym("y")]));
/// ```
pub fn replace_all(expr: IRNode, rule: &IRNode) -> IRNode {
    // Try the rule at the current node first.
    if let Some(replacement) = apply_rule(rule, &expr) {
        // Rule fired — return the replacement without recursing into it
        // (single-pass semantics).
        return replacement;
    }

    // Rule didn't fire here; recurse into children.
    match expr {
        IRNode::Apply(apply) => {
            let IRApply { head, args } = *apply;
            let new_head = replace_all(head, rule);
            let new_args: Vec<IRNode> = args.into_iter().map(|a| replace_all(a, rule)).collect();
            IRNode::Apply(Box::new(IRApply { head: new_head, args: new_args }))
        }
        other => other,
    }
}

/// Apply each rule once across `expr`, sequentially.
///
/// The order of rules matters — later rules see the output of earlier ones.
/// For fixed-point application of all rules simultaneously, use
/// [`cas_pattern_matching::rewrite`].
///
/// # Examples
///
/// ```rust
/// use symbolic_ir::{apply, int, sym, MUL, ADD};
/// use cas_pattern_matching::{blank, named, rule};
/// use cas_substitution::replace_all_many;
///
/// // Rule 1: Add(x_, 0) → x_
/// let r1 = rule(
///     apply(sym(ADD), vec![named("x", blank()), int(0)]),
///     named("x", blank()),
/// );
/// // Rule 2: Mul(x_, 1) → x_
/// let r2 = rule(
///     apply(sym(MUL), vec![named("x", blank()), int(1)]),
///     named("x", blank()),
/// );
/// // Apply both to Mul(Add(z, 0), 1) → z
/// let inner = apply(sym(ADD), vec![sym("z"), int(0)]);
/// let expr = apply(sym(MUL), vec![inner, int(1)]);
/// assert_eq!(replace_all_many(expr, &[r1, r2]), sym("z"));
/// ```
pub fn replace_all_many(expr: IRNode, rules: &[IRNode]) -> IRNode {
    let mut out = expr;
    for rule in rules {
        out = replace_all(out, rule);
    }
    out
}
