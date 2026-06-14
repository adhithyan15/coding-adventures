//! Canonical-form normalization pass.
//!
//! Four transformations, all purely structural:
//!
//! 1. **Flatten** — `Add(a, Add(b, c))` → `Add(a, b, c)` (same for `Mul`).
//! 2. **Sort** — args of commutative heads are sorted by a stable key,
//!    so `Add(c, a, b)` → `Add(a, b, c)`.
//! 3. **Singleton drop** — `Add(x)` → `x`, `Mul(x)` → `x`.
//! 4. **Empty container** — `Add()` → `0`, `Mul()` → `1`.
//!
//! Idempotent: `canonical(canonical(x)) == canonical(x)`.

use symbolic_ir::{IRApply, IRNode, ADD, MUL};

/// Heads that are commutative and should be flattened and sorted.
fn is_commutative_flat(name: &str) -> bool {
    name == ADD || name == MUL
}

/// The sort-key rank for an IR node.
///
/// ```text
/// Integer  → 0
/// Rational → 1
/// Float    → 2
/// Symbol   → 3
/// Apply    → 4
/// Str      → 5
/// ```
fn rank(node: &IRNode) -> u8 {
    match node {
        IRNode::Integer(_) => 0,
        IRNode::Rational(_, _) => 1,
        IRNode::Float(_) => 2,
        IRNode::Symbol(_) => 3,
        IRNode::Apply(_) => 4,
        IRNode::Str(_) => 5,
    }
}

/// Stable total order for IR nodes: (rank, display_string).
fn sort_key(node: &IRNode) -> (u8, String) {
    (rank(node), format!("{node:?}"))
}

/// Recursively normalize `node` into canonical form.
pub fn canonical(node: IRNode) -> IRNode {
    match node {
        IRNode::Apply(apply) => canonical_apply(*apply),
        other => other,
    }
}

fn canonical_apply(node: IRApply) -> IRNode {
    // Recursively canonicalize children.
    let new_head = canonical(node.head);
    let new_args: Vec<IRNode> = node.args.into_iter().map(canonical).collect();

    // Only flatten/sort for commutative associative heads.
    if let IRNode::Symbol(ref name) = new_head {
        if is_commutative_flat(name.as_str()) {
            let head_name = name.as_str();

            // 1. Flatten nested same-head children.
            let flat: Vec<IRNode> = new_args
                .into_iter()
                .flat_map(|a| flatten_one(a, head_name))
                .collect();

            // 2. Sort.
            let mut sorted = flat;
            sorted.sort_by(|a, b| sort_key(a).cmp(&sort_key(b)));

            // 3. Empty container.
            if sorted.is_empty() {
                return if head_name == ADD {
                    IRNode::Integer(0)
                } else {
                    IRNode::Integer(1)
                };
            }

            // 4. Singleton drop.
            if sorted.len() == 1 {
                return sorted.remove(0);
            }

            return IRNode::Apply(Box::new(IRApply {
                head: new_head,
                args: sorted,
            }));
        }
    }

    IRNode::Apply(Box::new(IRApply {
        head: new_head,
        args: new_args,
    }))
}

/// If `node` is `Apply(head_name, ...)`, yield its args; otherwise yield `node`.
fn flatten_one(node: IRNode, head_name: &str) -> Vec<IRNode> {
    if let IRNode::Apply(ref apply) = node {
        if let IRNode::Symbol(ref h) = apply.head {
            if h.as_str() == head_name {
                // Consume the node and return its (already-canonicalized) args.
                if let IRNode::Apply(boxed) = node {
                    return boxed.args;
                }
            }
        }
    }
    vec![node]
}
