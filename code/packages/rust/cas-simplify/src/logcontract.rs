//! Log contraction and expansion helpers.

use symbolic_ir::{apply, sym, IRNode, ADD, DIV, LOG, MUL, POW, SUB};

use crate::assumptions::AssumptionContext;

pub fn logcontract(expr: IRNode) -> IRNode {
    let IRNode::Apply(apply_node) = expr else {
        return expr;
    };
    let head = apply_node.head;
    let args = apply_node.args.into_iter().map(logcontract).collect();
    let node = apply(head, args);
    let IRNode::Apply(apply_node) = node else {
        return node;
    };
    match head_name(&apply_node.head) {
        Some(ADD) => contract_add(&apply_node.args),
        Some(SUB) => contract_sub(&apply_node.args),
        Some(MUL) => contract_mul(&apply_node.args),
        _ => IRNode::Apply(apply_node),
    }
}

pub fn logexpand(expr: IRNode, ctx: Option<&AssumptionContext>) -> IRNode {
    let _ = ctx;
    let IRNode::Apply(apply_node) = expr else {
        return expr;
    };
    let head = apply_node.head;
    let args = apply_node
        .args
        .into_iter()
        .map(|arg| logexpand(arg, ctx))
        .collect();
    let node = apply(head, args);
    let IRNode::Apply(apply_node) = node else {
        return node;
    };
    if head_name(&apply_node.head) == Some(LOG) {
        expand_log(&apply_node.args)
    } else {
        IRNode::Apply(apply_node)
    }
}

fn contract_add(args: &[IRNode]) -> IRNode {
    let mut log_args = Vec::new();
    let mut other = Vec::new();
    for arg in args {
        if let Some(inner) = log_inner(arg) {
            log_args.push(inner.clone());
        } else {
            other.push(arg.clone());
        }
    }
    if log_args.len() < 2 {
        return apply(sym(ADD), args.to_vec());
    }
    let merged = apply(sym(LOG), vec![apply(sym(MUL), log_args)]);
    if other.is_empty() {
        merged
    } else {
        other.push(merged);
        apply(sym(ADD), other)
    }
}

fn contract_sub(args: &[IRNode]) -> IRNode {
    if args.len() == 2 {
        if let (Some(lhs), Some(rhs)) = (log_inner(&args[0]), log_inner(&args[1])) {
            return apply(
                sym(LOG),
                vec![apply(sym(DIV), vec![lhs.clone(), rhs.clone()])],
            );
        }
    }
    apply(sym(SUB), args.to_vec())
}

fn contract_mul(args: &[IRNode]) -> IRNode {
    let mut log_indices = Vec::new();
    let mut numeric_indices = Vec::new();
    let mut other_count = 0;

    for (index, arg) in args.iter().enumerate() {
        if log_inner(arg).is_some() {
            log_indices.push(index);
        } else if matches!(arg, IRNode::Integer(_) | IRNode::Rational(_, _)) {
            numeric_indices.push(index);
        } else {
            other_count += 1;
        }
    }

    if log_indices.len() != 1 || numeric_indices.is_empty() || other_count != 0 {
        return apply(sym(MUL), args.to_vec());
    }

    let log_node = &args[log_indices[0]];
    let coeff_args = numeric_indices
        .into_iter()
        .map(|index| args[index].clone())
        .collect::<Vec<_>>();
    let coeff = match coeff_args.as_slice() {
        [only] => only.clone(),
        _ => apply(sym(MUL), coeff_args),
    };

    apply(
        sym(LOG),
        vec![apply(
            sym(POW),
            vec![
                log_inner(log_node)
                    .expect("log index points to log")
                    .clone(),
                coeff,
            ],
        )],
    )
}

fn expand_log(args: &[IRNode]) -> IRNode {
    if args.len() != 1 {
        return apply(sym(LOG), args.to_vec());
    }
    let arg = &args[0];
    if let IRNode::Apply(apply_node) = arg {
        match head_name(&apply_node.head) {
            Some(POW) if apply_node.args.len() == 2 => {
                let exp = &apply_node.args[1];
                if matches!(exp, IRNode::Integer(_) | IRNode::Rational(_, _)) {
                    return apply(
                        sym(MUL),
                        vec![
                            exp.clone(),
                            apply(sym(LOG), vec![apply_node.args[0].clone()]),
                        ],
                    );
                }
            }
            Some(MUL) if apply_node.args.len() >= 2 => {
                let mut terms = apply_node
                    .args
                    .iter()
                    .map(|arg| apply(sym(LOG), vec![arg.clone()]));
                let first = terms.next().expect("len checked");
                return terms.fold(first, |acc, term| apply(sym(ADD), vec![acc, term]));
            }
            Some(DIV) if apply_node.args.len() == 2 => {
                return apply(
                    sym(SUB),
                    vec![
                        apply(sym(LOG), vec![apply_node.args[0].clone()]),
                        apply(sym(LOG), vec![apply_node.args[1].clone()]),
                    ],
                );
            }
            _ => {}
        }
    }
    apply(sym(LOG), args.to_vec())
}

fn log_inner(node: &IRNode) -> Option<&IRNode> {
    let IRNode::Apply(apply_node) = node else {
        return None;
    };
    if head_name(&apply_node.head) == Some(LOG) && apply_node.args.len() == 1 {
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
