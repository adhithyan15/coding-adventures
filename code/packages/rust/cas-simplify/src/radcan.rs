//! Radical canonicalization helpers.

use std::collections::BTreeMap;

use symbolic_ir::{apply, int, rat, sym, IRNode, EXP, LOG, MUL, POW, SQRT};

use crate::assumptions::AssumptionContext;

pub fn radcan(expr: IRNode, ctx: Option<&AssumptionContext>) -> IRNode {
    let IRNode::Apply(apply_node) = expr else {
        return expr;
    };
    let head = apply_node.head;
    let args = apply_node
        .args
        .into_iter()
        .map(|arg| radcan(arg, ctx))
        .collect::<Vec<_>>();
    let node = apply(head, args);
    let IRNode::Apply(apply_node) = node else {
        return node;
    };
    match head_name(&apply_node.head) {
        Some(MUL) => rule_mul(&apply_node.args, ctx),
        Some(SQRT) => rule_sqrt(&apply_node.args, ctx),
        Some(POW) => rule_pow(&apply_node.args),
        Some(EXP) => rule_exp(&apply_node.args),
        Some(LOG) => rule_log(&apply_node.args),
        _ => IRNode::Apply(apply_node),
    }
}

fn rule_mul(args: &[IRNode], ctx: Option<&AssumptionContext>) -> IRNode {
    let mut sqrt_radicands = Vec::new();
    let mut non_sqrt = Vec::new();

    for arg in args {
        if let Some(radicand) = unary_inner(arg, SQRT) {
            sqrt_radicands.push(radicand.clone());
        } else {
            non_sqrt.push(arg.clone());
        }
    }

    let mut args = if sqrt_radicands.len() >= 2 {
        let merged = radcan(apply(sym(SQRT), vec![mul_or_one(sqrt_radicands)]), ctx);
        non_sqrt.push(merged);
        non_sqrt
    } else {
        args.to_vec()
    };

    let mut groups: BTreeMap<(i64, i64), Vec<IRNode>> = BTreeMap::new();
    let mut remaining = Vec::new();
    for arg in args.drain(..) {
        if let Some((numer, denom)) = rational_exponent(&arg) {
            if denom > 1 && (numer, denom) != (1, 2) {
                if let Some(base) = base_of(&arg) {
                    groups.entry((numer, denom)).or_default().push(base.clone());
                    continue;
                }
            }
        }
        remaining.push(arg);
    }

    let mut merged = Vec::new();
    for ((numer, denom), bases) in groups {
        let exp = if denom == 1 {
            int(numer)
        } else {
            rat(numer, denom)
        };
        if bases.len() == 1 {
            remaining.push(apply(sym(POW), vec![bases[0].clone(), exp]));
        } else {
            merged.push(apply(sym(POW), vec![mul_or_one(bases), exp]));
        }
    }

    remaining.extend(merged);
    mul_or_one(remaining)
}

fn rule_sqrt(args: &[IRNode], ctx: Option<&AssumptionContext>) -> IRNode {
    if args.len() != 1 {
        return apply(sym(SQRT), args.to_vec());
    }
    let arg = &args[0];

    if let IRNode::Integer(n) = arg {
        if *n >= 0 {
            if let Some(root) = integer_sqrt(*n) {
                return int(root);
            }
        }
    }

    if is_square_power(arg) {
        if let Some(base) = base_of(arg) {
            let result = abs_or_pos(base, ctx);
            if result != *arg {
                return result;
            }
        }
    }

    if let IRNode::Apply(mul_node) = arg {
        if head_name(&mul_node.head) == Some(MUL) {
            let mut outer = Vec::new();
            let mut inner = Vec::new();
            for factor in &mul_node.args {
                if let Some(extracted) = try_extract_from_sqrt(factor, ctx) {
                    outer.push(extracted);
                } else {
                    inner.push(factor.clone());
                }
            }
            if !outer.is_empty() {
                let outer_prod = mul_or_one(outer);
                let inner_prod = mul_or_one(inner);
                if inner_prod == int(1) {
                    return outer_prod;
                }
                return apply(
                    sym(MUL),
                    vec![outer_prod, apply(sym(SQRT), vec![inner_prod])],
                );
            }
        }
    }

    apply(sym(SQRT), args.to_vec())
}

fn try_extract_from_sqrt(factor: &IRNode, ctx: Option<&AssumptionContext>) -> Option<IRNode> {
    if is_square_power(factor) {
        let base = base_of(factor)?;
        if matches!(base, IRNode::Integer(n) if *n > 0) {
            return Some(base.clone());
        }
        if let IRNode::Symbol(name) = base {
            if ctx.is_some_and(|ctx| ctx.is_positive(name) == Some(true)) {
                return Some(base.clone());
            }
        }
    }

    if let IRNode::Integer(n) = factor {
        if *n > 0 {
            return integer_sqrt(*n).map(int);
        }
    }
    None
}

fn rule_pow(args: &[IRNode]) -> IRNode {
    if args.len() == 2 && args[1] == int(2) {
        if let Some(inner) = unary_inner(&args[0], SQRT) {
            return inner.clone();
        }
    }
    apply(sym(POW), args.to_vec())
}

fn rule_exp(args: &[IRNode]) -> IRNode {
    if args.len() == 1 {
        if let Some(inner) = unary_inner(&args[0], LOG) {
            return inner.clone();
        }
    }
    apply(sym(EXP), args.to_vec())
}

fn rule_log(args: &[IRNode]) -> IRNode {
    if args.len() == 1 {
        if let Some(inner) = unary_inner(&args[0], EXP) {
            return inner.clone();
        }
    }
    apply(sym(LOG), args.to_vec())
}

fn is_square_power(node: &IRNode) -> bool {
    matches!(
        node,
        IRNode::Apply(apply_node)
            if head_name(&apply_node.head) == Some(POW)
                && apply_node.args.len() == 2
                && apply_node.args[1] == int(2)
    )
}

fn abs_or_pos(base: &IRNode, ctx: Option<&AssumptionContext>) -> IRNode {
    if matches!(base, IRNode::Integer(n) if *n > 0) {
        return base.clone();
    }
    if let IRNode::Symbol(name) = base {
        if ctx.is_some_and(|ctx| ctx.is_positive(name) == Some(true)) {
            return base.clone();
        }
    }
    apply(sym(SQRT), vec![apply(sym(POW), vec![base.clone(), int(2)])])
}

fn rational_exponent(node: &IRNode) -> Option<(i64, i64)> {
    let IRNode::Apply(apply_node) = node else {
        return None;
    };
    if head_name(&apply_node.head) != Some(POW) || apply_node.args.len() != 2 {
        return None;
    }
    match apply_node.args[1] {
        IRNode::Integer(n) => Some((n, 1)),
        IRNode::Rational(n, d) => Some((n, d)),
        _ => None,
    }
}

fn base_of(node: &IRNode) -> Option<&IRNode> {
    let IRNode::Apply(apply_node) = node else {
        return None;
    };
    if head_name(&apply_node.head) == Some(POW) && apply_node.args.len() == 2 {
        Some(&apply_node.args[0])
    } else {
        None
    }
}

fn integer_sqrt(n: i64) -> Option<i64> {
    if n < 0 {
        return None;
    }
    let mut root = (n as f64).sqrt() as i64;
    while root.saturating_mul(root) > n {
        root -= 1;
    }
    while (root + 1).saturating_mul(root + 1) <= n {
        root += 1;
    }
    (root * root == n).then_some(root)
}

fn mul_or_one(nodes: Vec<IRNode>) -> IRNode {
    match nodes.as_slice() {
        [] => int(1),
        [only] => only.clone(),
        _ => apply(sym(MUL), nodes),
    }
}

fn unary_inner<'a>(node: &'a IRNode, head: &str) -> Option<&'a IRNode> {
    let IRNode::Apply(apply_node) = node else {
        return None;
    };
    if head_name(&apply_node.head) == Some(head) && apply_node.args.len() == 1 {
        Some(&apply_node.args[0])
    } else {
        None
    }
}

fn head_name(node: &IRNode) -> Option<&str> {
    match node {
        IRNode::Symbol(name) => Some(name),
        _ => None,
    }
}
