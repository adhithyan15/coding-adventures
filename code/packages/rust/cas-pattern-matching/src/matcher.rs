//! Structural matcher for IR patterns.
//!
//! ## Algorithm
//!
//! [`match_pattern`] recursively descends both the pattern and the target,
//! comparing them node-by-node.  Five cases:
//!
//! 1. **`Blank()`** — matches any expression; no binding recorded.
//! 2. **`Blank(T)`** — matches any expression whose *effective head* equals
//!    `T`.  Compounds: `IRApply.head.name`.  Literals: type tag
//!    (`"Integer"`, `"Symbol"`, `"Rational"`, `"Float"`, `"String"`).
//! 3. **`Pattern(name, inner)`** — recursively matches `inner` against the
//!    target.  On success, records `name → target` in `Bindings`.  If
//!    `name` was already bound to a different value the match fails.
//! 4. **Compound `IRApply`** — head must match (recursively) and args must
//!    zip pairwise (no sequence wildcards in this release).
//! 5. **Otherwise** — structural equality (`pattern == target`).

use symbolic_ir::{IRApply, IRNode};

use crate::bindings::Bindings;
use crate::nodes::{blank_head_constraint, is_blank, is_pattern, pattern_inner, pattern_name};

/// Try to match `pattern` against `target` starting from `bindings`.
///
/// Returns the updated `Bindings` on success, or `None` if the pattern
/// does not match.  The input `bindings` is consumed; on failure it is
/// lost (callers that need it should `.clone()` before calling).
///
/// # Example
///
/// ```rust
/// use symbolic_ir::{apply, int, sym, ADD};
/// use cas_pattern_matching::{blank, named, match_pattern, Bindings};
///
/// let pat = apply(sym(ADD), vec![named("a", blank()), named("b", blank())]);
/// let target = apply(sym(ADD), vec![int(1), int(2)]);
/// let b = match_pattern(&pat, &target, Bindings::empty()).unwrap();
/// assert_eq!(b.get("a"), Some(&int(1)));
/// assert_eq!(b.get("b"), Some(&int(2)));
/// ```
pub fn match_pattern(
    pattern: &IRNode,
    target: &IRNode,
    bindings: Bindings,
) -> Option<Bindings> {
    // Case 1 & 2: Blank(?) wildcards
    if is_blank(pattern) {
        if let IRNode::Apply(apply) = pattern {
            let head_constraint = blank_head_constraint(apply);
            match head_constraint {
                None => return Some(bindings), // unconstrained — always matches
                Some(constraint) => {
                    if effective_head_name(target) == constraint {
                        return Some(bindings);
                    }
                    return None;
                }
            }
        }
    }

    // Case 3: Pattern(name, inner) — named capture
    if is_pattern(pattern) {
        if let IRNode::Apply(apply) = pattern {
            let name = pattern_name(apply);
            let inner = pattern_inner(apply);
            let sub = match_pattern(inner, target, bindings)?;
            // Consistency check: if name is already bound, it must match.
            if let Some(existing) = sub.get(name) {
                return if existing == target { Some(sub) } else { None };
            }
            return Some(sub.bind(name, target.clone()));
        }
    }

    // Case 4: Compound vs compound
    if let IRNode::Apply(pat_apply) = pattern {
        if let IRNode::Apply(tgt_apply) = target {
            return match_apply(pat_apply, tgt_apply, bindings);
        }
        // Pattern is Apply but target is a leaf — match fails.
        return None;
    }

    // Case 5: Structural equality (leaves and non-Blank/Pattern compounds)
    if pattern == target {
        Some(bindings)
    } else {
        None
    }
}

/// Match two `IRApply` nodes: head must match, then args zip pairwise.
fn match_apply(
    pattern: &IRApply,
    target: &IRApply,
    bindings: Bindings,
) -> Option<Bindings> {
    // Head must match.
    let after_head = match_pattern(&pattern.head, &target.head, bindings)?;

    // Arg count must be equal (no sequence wildcards yet).
    if pattern.args.len() != target.args.len() {
        return None;
    }

    // Pairwise arg matching, threading the bindings.
    let mut cur = after_head;
    for (p_arg, t_arg) in pattern.args.iter().zip(target.args.iter()) {
        cur = match_pattern(p_arg, t_arg, cur)?;
    }
    Some(cur)
}

// ---------------------------------------------------------------------------
// Effective head name
// ---------------------------------------------------------------------------

/// Return the head name used by `Blank(T)` constraints.
///
/// - `Apply(sym, …)` → `sym.name` (or `"Apply"` for non-symbol heads)
/// - `Symbol(_)` → `"Symbol"`
/// - `Integer(_)` → `"Integer"`
/// - `Rational(_, _)` → `"Rational"`
/// - `Float(_)` → `"Float"`
/// - `Str(_)` → `"String"`
pub(crate) fn effective_head_name(node: &IRNode) -> &str {
    match node {
        IRNode::Apply(a) => {
            if let IRNode::Symbol(s) = &a.head {
                s.as_str()
            } else {
                "Apply"
            }
        }
        IRNode::Symbol(_) => "Symbol",
        IRNode::Integer(_) => "Integer",
        IRNode::Rational(_, _) => "Rational",
        IRNode::Float(_) => "Float",
        IRNode::Str(_) => "String",
    }
}
