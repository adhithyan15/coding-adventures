//! Sentinel head names and constructor helpers for pattern nodes.
//!
//! Patterns live *inside* the standard `IRNode` tree rather than as separate
//! types.  That keeps the matcher uniform — every pattern is just an IR tree
//! — and avoids duplicating the hashing/equality machinery the IR already
//! provides.
//!
//! ## Sentinel heads
//!
//! | Constant | Head name | Meaning |
//! |----------|-----------|---------|
//! | `BLANK` | `"Blank"` | Wildcard |
//! | `PATTERN` | `"Pattern"` | Named capture |
//! | `RULE` | `"Rule"` | Immediate substitution rule |
//! | `RULE_DELAYED` | `"RuleDelayed"` | Delayed substitution rule |

use symbolic_ir::{apply, sym, IRApply, IRNode};

// ---------------------------------------------------------------------------
// Sentinel head name constants
// ---------------------------------------------------------------------------

pub const BLANK: &str = "Blank";
pub const PATTERN: &str = "Pattern";
pub const RULE: &str = "Rule";
pub const RULE_DELAYED: &str = "RuleDelayed";

// ---------------------------------------------------------------------------
// Constructor helpers
// ---------------------------------------------------------------------------

/// Build an unconstrained wildcard `Blank()` — matches any expression.
///
/// ```rust
/// use cas_pattern_matching::blank;
/// use symbolic_ir::{int, IRNode};
/// use cas_pattern_matching::{match_pattern, Bindings};
///
/// // Blank() matches anything
/// let b = blank();
/// assert!(match_pattern(&b, &int(42), Bindings::empty()).is_some());
/// ```
pub fn blank() -> IRNode {
    apply(sym(BLANK), vec![])
}

/// Build a type-constrained wildcard `Blank(head_name)`.
///
/// Matches any expression whose *effective head* equals `head_name`.
/// For compounds that is `Apply.head.name`; for literals it is the
/// type tag (`"Integer"`, `"Symbol"`, etc.).
///
/// ```rust
/// use cas_pattern_matching::blank_typed;
/// use symbolic_ir::{int, sym, IRNode};
/// use cas_pattern_matching::{match_pattern, Bindings};
///
/// let b_int = blank_typed("Integer");
/// assert!(match_pattern(&b_int, &int(42), Bindings::empty()).is_some());
/// assert!(match_pattern(&b_int, &sym("x"), Bindings::empty()).is_none());
/// ```
pub fn blank_typed(head_name: &str) -> IRNode {
    apply(sym(BLANK), vec![sym(head_name)])
}

/// Build a named pattern `Pattern(name, inner)`.
///
/// When `inner` matches a target, the binding `name → target` is recorded
/// in the [`Bindings`] map.  If `name` was already bound to a different
/// target, the match fails.
///
/// ```rust
/// use cas_pattern_matching::{blank, named, match_pattern, Bindings};
/// use symbolic_ir::int;
///
/// let p = named("x", blank());
/// let b = match_pattern(&p, &int(7), Bindings::empty()).unwrap();
/// assert_eq!(b.get("x"), Some(&int(7)));
/// ```
pub fn named(name: &str, inner: IRNode) -> IRNode {
    apply(sym(PATTERN), vec![sym(name), inner])
}

/// Build an immediate-substitution rule `Rule(lhs, rhs)`.
///
/// ```rust
/// use cas_pattern_matching::{blank, named, rule, apply_rule};
/// use symbolic_ir::{apply, int, sym, ADD};
///
/// // x_ + 0  →  x_   (RHS references capture via Pattern node, not bare Symbol)
/// let x_pat = named("x", blank());
/// let lhs = apply(sym(ADD), vec![x_pat.clone(), int(0)]);
/// let rhs = x_pat.clone();  // RHS must also use Pattern("x", …)
/// let r = rule(lhs, rhs);
///
/// let target = apply(sym(ADD), vec![int(5), int(0)]);
/// assert_eq!(apply_rule(&r, &target).unwrap(), int(5));
/// ```
pub fn rule(lhs: IRNode, rhs: IRNode) -> IRNode {
    apply(sym(RULE), vec![lhs, rhs])
}

/// Build a delayed-substitution rule `RuleDelayed(lhs, rhs)`.
///
/// Identical semantics to [`rule`] for pattern matching; reserved
/// separately for future passes that distinguish evaluated vs. held RHSes.
pub fn rule_delayed(lhs: IRNode, rhs: IRNode) -> IRNode {
    apply(sym(RULE_DELAYED), vec![lhs, rhs])
}

// ---------------------------------------------------------------------------
// Inspection helpers
// ---------------------------------------------------------------------------

/// True if `node` is a `Blank(…)` apply.
pub fn is_blank(node: &IRNode) -> bool {
    is_head(node, BLANK)
}

/// True if `node` is a `Pattern(…)` apply.
pub fn is_pattern(node: &IRNode) -> bool {
    is_head(node, PATTERN)
}

/// True if `node` is a `Rule(lhs, rhs)` or `RuleDelayed(lhs, rhs)` apply
/// with exactly 2 arguments.
pub fn is_rule(node: &IRNode) -> bool {
    if let IRNode::Apply(a) = node {
        if let IRNode::Symbol(s) = &a.head {
            return (s == RULE || s == RULE_DELAYED) && a.args.len() == 2;
        }
    }
    false
}

/// Extract the optional head constraint from a `Blank(T)` node.
///
/// Returns `Some("Integer")` for `Blank("Integer")` and `None` for `Blank()`.
pub fn blank_head_constraint(node: &IRApply) -> Option<&str> {
    if node.args.is_empty() {
        return None;
    }
    if let IRNode::Symbol(s) = &node.args[0] {
        Some(s.as_str())
    } else {
        None
    }
}

/// Extract the bound name from a `Pattern(name, inner)` node.
pub fn pattern_name(node: &IRApply) -> &str {
    if let IRNode::Symbol(s) = &node.args[0] {
        s.as_str()
    } else {
        panic!("Pattern name must be a Symbol, got {:?}", node.args[0])
    }
}

/// Extract the inner pattern from a `Pattern(name, inner)` node.
pub fn pattern_inner(node: &IRApply) -> &IRNode {
    &node.args[1]
}

// ---------------------------------------------------------------------------
// Internal helper
// ---------------------------------------------------------------------------

fn is_head(node: &IRNode, head_name: &str) -> bool {
    if let IRNode::Apply(a) = node {
        if let IRNode::Symbol(s) = &a.head {
            return s == head_name;
        }
    }
    false
}
