//! Trig/hyperbolic exponentialization and De Moivre decomposition.

use symbolic_ir::{
    apply, int, sym, IRNode, ADD, COS, COSH, DIV, EXP, MUL, NEG, SIN, SINH, SUB, TAN, TANH,
};

const IMAGINARY_UNIT: &str = "ImaginaryUnit";

pub fn exponentialize(expr: IRNode) -> IRNode {
    let IRNode::Apply(apply_node) = expr else {
        return expr;
    };
    let head = apply_node.head;
    let args = apply_node.args.into_iter().map(exponentialize).collect();
    let node = apply(head, args);
    let IRNode::Apply(apply_node) = node else {
        return node;
    };
    if apply_node.args.len() != 1 {
        return IRNode::Apply(apply_node);
    }
    let x = apply_node.args[0].clone();
    match head_name(&apply_node.head) {
        Some(SIN) => sin_exp(x),
        Some(COS) => cos_exp(x),
        Some(TAN) => tan_exp(x),
        Some(SINH) => sinh_exp(x),
        Some(COSH) => cosh_exp(x),
        Some(TANH) => tanh_exp(x),
        _ => IRNode::Apply(apply_node),
    }
}

pub fn demoivre(expr: IRNode) -> IRNode {
    let IRNode::Apply(apply_node) = expr else {
        return expr;
    };
    let head = apply_node.head;
    let args = apply_node.args.into_iter().map(demoivre).collect();
    let node = apply(head, args);
    let IRNode::Apply(apply_node) = node else {
        return node;
    };
    if head_name(&apply_node.head) != Some(EXP) || apply_node.args.len() != 1 {
        return IRNode::Apply(apply_node);
    }

    let (real, imag) = split_real_imag(&apply_node.args[0]);
    let Some(imag) = imag else {
        return IRNode::Apply(apply_node);
    };

    let trig_sum = apply(
        sym(ADD),
        vec![
            apply(sym(COS), vec![imag.clone()]),
            apply(
                sym(MUL),
                vec![imaginary_unit(), apply(sym(SIN), vec![imag])],
            ),
        ],
    );

    if let Some(real) = real {
        apply(sym(MUL), vec![apply(sym(EXP), vec![real]), trig_sum])
    } else {
        trig_sum
    }
}

fn ix(x: IRNode) -> IRNode {
    apply(sym(MUL), vec![imaginary_unit(), x])
}

fn neg_ix(x: IRNode) -> IRNode {
    apply(sym(MUL), vec![imaginary_unit(), apply(sym(NEG), vec![x])])
}

fn sin_exp(x: IRNode) -> IRNode {
    let e_pos = apply(sym(EXP), vec![ix(x.clone())]);
    let e_neg = apply(sym(EXP), vec![neg_ix(x)]);
    apply(
        sym(DIV),
        vec![
            apply(sym(SUB), vec![e_pos, e_neg]),
            apply(sym(MUL), vec![int(2), imaginary_unit()]),
        ],
    )
}

fn cos_exp(x: IRNode) -> IRNode {
    let e_pos = apply(sym(EXP), vec![ix(x.clone())]);
    let e_neg = apply(sym(EXP), vec![neg_ix(x)]);
    apply(sym(DIV), vec![apply(sym(ADD), vec![e_pos, e_neg]), int(2)])
}

fn tan_exp(x: IRNode) -> IRNode {
    let e_pos = apply(sym(EXP), vec![ix(x.clone())]);
    let e_neg = apply(sym(EXP), vec![neg_ix(x)]);
    apply(
        sym(DIV),
        vec![
            apply(
                sym(MUL),
                vec![
                    apply(sym(NEG), vec![imaginary_unit()]),
                    apply(sym(SUB), vec![e_pos.clone(), e_neg.clone()]),
                ],
            ),
            apply(sym(ADD), vec![e_pos, e_neg]),
        ],
    )
}

fn sinh_exp(x: IRNode) -> IRNode {
    let e_pos = apply(sym(EXP), vec![x.clone()]);
    let e_neg = apply(sym(EXP), vec![apply(sym(NEG), vec![x])]);
    apply(sym(DIV), vec![apply(sym(SUB), vec![e_pos, e_neg]), int(2)])
}

fn cosh_exp(x: IRNode) -> IRNode {
    let e_pos = apply(sym(EXP), vec![x.clone()]);
    let e_neg = apply(sym(EXP), vec![apply(sym(NEG), vec![x])]);
    apply(sym(DIV), vec![apply(sym(ADD), vec![e_pos, e_neg]), int(2)])
}

fn tanh_exp(x: IRNode) -> IRNode {
    let e_pos = apply(sym(EXP), vec![x.clone()]);
    let e_neg = apply(sym(EXP), vec![apply(sym(NEG), vec![x])]);
    apply(
        sym(DIV),
        vec![
            apply(sym(SUB), vec![e_pos.clone(), e_neg.clone()]),
            apply(sym(ADD), vec![e_pos, e_neg]),
        ],
    )
}

fn split_real_imag(arg: &IRNode) -> (Option<IRNode>, Option<IRNode>) {
    if *arg == imaginary_unit() {
        return (None, Some(int(1)));
    }

    if let IRNode::Apply(apply_node) = arg {
        if head_name(&apply_node.head) == Some(MUL) {
            if let Some(coeff) = extract_i_from_mul(&apply_node.args) {
                return (None, Some(coeff));
            }
        }

        if head_name(&apply_node.head) == Some(ADD) {
            let mut real_terms = Vec::new();
            let mut imag_coeff = None;
            for term in &apply_node.args {
                let i_part = if *term == imaginary_unit() {
                    Some(int(1))
                } else if let IRNode::Apply(term_apply) = term {
                    if head_name(&term_apply.head) == Some(MUL) {
                        extract_i_from_mul(&term_apply.args)
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(coeff) = i_part {
                    if imag_coeff.is_some() {
                        return (Some(arg.clone()), None);
                    }
                    imag_coeff = Some(coeff);
                } else {
                    real_terms.push(term.clone());
                }
            }

            let Some(imag_coeff) = imag_coeff else {
                return (Some(arg.clone()), None);
            };
            let real = match real_terms.as_slice() {
                [] => None,
                [only] => Some(only.clone()),
                _ => Some(apply(sym(ADD), real_terms)),
            };
            return (real, Some(imag_coeff));
        }
    }

    (Some(arg.clone()), None)
}

fn extract_i_from_mul(args: &[IRNode]) -> Option<IRNode> {
    let i_count = args.iter().filter(|arg| **arg == imaginary_unit()).count();
    if i_count != 1 {
        return None;
    }
    let rest = args
        .iter()
        .filter(|arg| **arg != imaginary_unit())
        .cloned()
        .collect::<Vec<_>>();
    match rest.as_slice() {
        [] => Some(int(1)),
        [only] => Some(only.clone()),
        _ => Some(apply(sym(MUL), rest)),
    }
}

fn imaginary_unit() -> IRNode {
    sym(IMAGINARY_UNIT)
}

fn head_name(node: &IRNode) -> Option<&str> {
    match node {
        IRNode::Symbol(name) => Some(name),
        _ => None,
    }
}
