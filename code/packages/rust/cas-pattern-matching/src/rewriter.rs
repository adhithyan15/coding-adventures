//! Apply rewrite rules to IR trees.
//!
//! Two operations:
//!
//! - [`apply_rule`] — try one rule at the *root* of an expression.
//!   Returns the rewritten IR or `None` if the rule didn't match.
//! - [`rewrite`] — recursively apply a list of rules until no rule fires
//!   anywhere in the tree (or `max_iterations` runs out).
//!
//! Rules are applied **bottom-up** (post-order): children are rewritten
//! first, then rules are tried at the current node.  This converges faster
//! than top-down for algebraic identity rules.
//!
//! ## Cycle detection
//!
//! [`rewrite`] bounds total rule-firing iterations at `max_iterations`
//! (default 100) and returns [`Err(RewriteCycleError)`] if the bound is
//! hit.  Well-behaved rules converge in 2–5 passes.

use symbolic_ir::{IRApply, IRNode};

use crate::bindings::Bindings;
use crate::matcher::match_pattern;
use crate::nodes::{is_pattern, is_rule, pattern_name};

/// Returned by [`rewrite`] when the rules do not converge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteCycleError {
    pub max_iterations: usize,
}

impl std::fmt::Display for RewriteCycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "rewrite did not converge within {} iterations",
            self.max_iterations
        )
    }
}

impl std::error::Error for RewriteCycleError {}

// ---------------------------------------------------------------------------
// Single-rule application
// ---------------------------------------------------------------------------

/// Try `rule` against `expr`.  Returns the rewritten IR or `None`.
///
/// `rule` must be a `Rule(lhs, rhs)` or `RuleDelayed(lhs, rhs)` node.
///
/// # Example
///
/// ```rust
/// use cas_pattern_matching::{blank, named, rule, apply_rule};
/// use symbolic_ir::{apply, int, sym, ADD};
///
/// // x_ + 0  →  x_  (RHS uses Pattern node to reference the capture)
/// let x_pat = named("x", blank());
/// let r = rule(
///     apply(sym(ADD), vec![x_pat.clone(), int(0)]),
///     x_pat.clone(),
/// );
/// let target = apply(sym(ADD), vec![int(5), int(0)]);
/// assert_eq!(apply_rule(&r, &target), Some(int(5)));
/// ```
pub fn apply_rule(rule: &IRNode, expr: &IRNode) -> Option<IRNode> {
    if !is_rule(rule) {
        panic!("apply_rule expected Rule/RuleDelayed, got {rule}");
    }
    let (lhs, rhs) = if let IRNode::Apply(a) = rule {
        (&a.args[0], &a.args[1])
    } else {
        unreachable!()
    };
    let bindings = match_pattern(lhs, expr, Bindings::empty())?;
    Some(substitute(rhs, &bindings))
}

/// Structural substitution: replace `Pattern(name, _)` references with
/// their captured value from `bindings`.
///
/// Used to expand the RHS of a fired rule.
pub fn substitute(template: &IRNode, bindings: &Bindings) -> IRNode {
    if is_pattern(template) {
        if let IRNode::Apply(a) = template {
            let name = pattern_name(a);
            if let Some(value) = bindings.get(name) {
                return value.clone();
            }
        }
        return template.clone();
    }

    if let IRNode::Apply(a) = template {
        let new_head = substitute(&a.head, bindings);
        let new_args: Vec<IRNode> = a.args.iter().map(|arg| substitute(arg, bindings)).collect();
        return IRNode::Apply(Box::new(IRApply {
            head: new_head,
            args: new_args,
        }));
    }

    template.clone()
}

// ---------------------------------------------------------------------------
// Bottom-up fixed-point rewrite
// ---------------------------------------------------------------------------

/// Apply `rules` to `expr` until no rule fires anywhere in the tree.
///
/// Walks bottom-up (post-order).  If any rule fires at a node, the result
/// is re-walked from that point.  Returns the fixed-point IR.
///
/// Returns [`Err(RewriteCycleError)`] if more than `max_iterations` total
/// rule applications occur without reaching a fixed point.
///
/// # Example
///
/// ```rust
/// use cas_pattern_matching::{blank, named, rule, rewrite};
/// use symbolic_ir::{apply, int, sym, ADD};
///
/// // x_ + 0  →  x_  (applied anywhere in the tree)
/// let x_pat = named("x", blank());
/// let r = rule(
///     apply(sym(ADD), vec![x_pat.clone(), int(0)]),
///     x_pat.clone(),
/// );
/// // Add(Add(z, 0), 0)  →  z
/// let expr = apply(sym(ADD), vec![
///     apply(sym(ADD), vec![sym("z"), int(0)]),
///     int(0),
/// ]);
/// let result = rewrite(expr, &[r], 100).unwrap();
/// assert_eq!(result, sym("z"));
/// ```
pub fn rewrite(
    expr: IRNode,
    rules: &[IRNode],
    max_iterations: usize,
) -> Result<IRNode, RewriteCycleError> {
    let mut counter = 0usize;

    fn walk(
        node: IRNode,
        rules: &[IRNode],
        counter: &mut usize,
        max: usize,
    ) -> Result<IRNode, RewriteCycleError> {
        // 1. Rewrite children first (bottom-up).
        let current = if let IRNode::Apply(a) = node {
            let new_head = walk(a.head, rules, counter, max)?;
            let mut new_args = Vec::with_capacity(a.args.len());
            for arg in a.args {
                new_args.push(walk(arg, rules, counter, max)?);
            }
            IRNode::Apply(Box::new(IRApply {
                head: new_head,
                args: new_args,
            }))
        } else {
            node
        };

        // 2. Try rules at this position until none fire.
        let mut cur = current;
        loop {
            let mut fired = false;
            for rule in rules {
                if let Some(replacement) = apply_rule(rule, &cur) {
                    if replacement != cur {
                        *counter += 1;
                        if *counter > max {
                            return Err(RewriteCycleError {
                                max_iterations: max,
                            });
                        }
                        // Re-walk the replacement so its sub-parts also get rewritten.
                        cur = walk(replacement, rules, counter, max)?;
                        fired = true;
                        break;
                    }
                }
            }
            if !fired {
                return Ok(cur);
            }
        }
    }

    walk(expr, rules, &mut counter, max_iterations)
}
