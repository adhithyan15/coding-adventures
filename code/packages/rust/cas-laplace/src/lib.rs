use std::collections::BTreeMap;

use symbolic_ir::{
    apply, int, rat, sym, IRNode, ADD, COS, COSH, DIV, EXP, MUL, NEG, POW, SIN, SINH, SUB,
};

pub const LAPLACE: &str = "Laplace";
pub const ILT: &str = "ILT";
pub const DIRAC_DELTA: &str = "DiracDelta";
pub const UNIT_STEP: &str = "UnitStep";

pub type EvalFn = dyn Fn(IRNode) -> IRNode;
pub type Handler = fn(&IRNode, &EvalFn) -> IRNode;

pub fn laplace_transform(f: IRNode, t: IRNode, s: IRNode) -> IRNode {
    if let Some((a, b)) = binary_args(&f, ADD) {
        return binary(
            ADD,
            laplace_transform(a.clone(), t.clone(), s.clone()),
            laplace_transform(b.clone(), t.clone(), s),
        );
    }

    if let Some((coeff, body)) = extract_coeff_and_fn(&f, &t) {
        if !is_int(&coeff, 1) {
            return binary(MUL, coeff, laplace_transform(body, t, s));
        }
    }

    table_lookup(&f, &t, &s).unwrap_or_else(|| apply(sym(LAPLACE), vec![f, t, s]))
}

pub fn inverse_laplace(f: IRNode, s: IRNode, t: IRNode) -> IRNode {
    inverse_lookup(&f, &s, &t).unwrap_or_else(|| apply(sym(ILT), vec![f, s, t]))
}

pub fn laplace_handler(expr: &IRNode, eval: &EvalFn) -> IRNode {
    let Some(args) = apply_args(expr, LAPLACE) else {
        return expr.clone();
    };
    if args.len() != 3
        || !matches!(args[1], IRNode::Symbol(_))
        || !matches!(args[2], IRNode::Symbol(_))
    {
        return expr.clone();
    }
    eval(laplace_transform(
        args[0].clone(),
        args[1].clone(),
        args[2].clone(),
    ))
}

pub fn ilt_handler(expr: &IRNode, eval: &EvalFn) -> IRNode {
    let Some(args) = apply_args(expr, ILT) else {
        return expr.clone();
    };
    if args.len() != 3
        || !matches!(args[1], IRNode::Symbol(_))
        || !matches!(args[2], IRNode::Symbol(_))
    {
        return expr.clone();
    }
    eval(inverse_laplace(
        args[0].clone(),
        args[1].clone(),
        args[2].clone(),
    ))
}

pub fn dirac_delta_handler(expr: &IRNode) -> IRNode {
    let Some(args) = apply_args(expr, DIRAC_DELTA) else {
        return expr.clone();
    };
    if args.len() == 1 && is_int(&args[0], 0) {
        int(1)
    } else {
        expr.clone()
    }
}

pub fn unit_step_handler(expr: &IRNode) -> IRNode {
    let Some(args) = apply_args(expr, UNIT_STEP) else {
        return expr.clone();
    };
    if args.len() != 1 {
        return expr.clone();
    }
    match args[0] {
        IRNode::Integer(n) if n < 0 => int(0),
        IRNode::Integer(0) => rat(1, 2),
        IRNode::Integer(_) => int(1),
        _ => expr.clone(),
    }
}

pub fn build_laplace_handler_table() -> BTreeMap<&'static str, Handler> {
    BTreeMap::from([
        (LAPLACE, laplace_handler as Handler),
        (ILT, ilt_handler as Handler),
        (DIRAC_DELTA, dirac_delta_eval_handler as Handler),
        (UNIT_STEP, unit_step_eval_handler as Handler),
    ])
}

fn dirac_delta_eval_handler(expr: &IRNode, _eval: &EvalFn) -> IRNode {
    dirac_delta_handler(expr)
}

fn unit_step_eval_handler(expr: &IRNode, _eval: &EvalFn) -> IRNode {
    unit_step_handler(expr)
}

fn table_lookup(f: &IRNode, t: &IRNode, s: &IRNode) -> Option<IRNode> {
    if is_one(f) {
        return Some(binary(DIV, int(1), s.clone()));
    }
    if let Some(n) = match_power_of_t(f, t) {
        return Some(binary(
            DIV,
            int(factorial(n)),
            binary(POW, s.clone(), int(n + 1)),
        ));
    }
    if let Some(a) = match_unary_linear(f, EXP, t) {
        return Some(binary(DIV, int(1), binary(SUB, s.clone(), a)));
    }
    if let Some(w) = match_unary_linear(f, SIN, t) {
        return Some(binary(DIV, w.clone(), sum_s_sq_param_sq(s, &w)));
    }
    if let Some(w) = match_unary_linear(f, COS, t) {
        return Some(binary(DIV, s.clone(), sum_s_sq_param_sq(s, &w)));
    }
    if let Some(a) = match_unary_linear(f, SINH, t) {
        return Some(binary(DIV, a.clone(), sub_s_sq_param_sq(s, &a)));
    }
    if let Some(a) = match_unary_linear(f, COSH, t) {
        return Some(binary(DIV, s.clone(), sub_s_sq_param_sq(s, &a)));
    }
    if is_apply_of_var(f, DIRAC_DELTA, t) {
        return Some(int(1));
    }
    if is_apply_of_var(f, UNIT_STEP, t) {
        return Some(binary(DIV, int(1), s.clone()));
    }
    if let Some((a, trig_head, w)) = match_exp_times_trig(f, t) {
        let shifted = binary(SUB, s.clone(), a);
        let denom = binary(
            ADD,
            binary(POW, shifted.clone(), int(2)),
            binary(POW, w.clone(), int(2)),
        );
        return Some(if trig_head == SIN {
            binary(DIV, w, denom)
        } else {
            binary(DIV, shifted, denom)
        });
    }
    if let Some((n, a)) = match_t_power_times_exp(f, t) {
        let shifted = binary(SUB, s.clone(), a);
        return Some(binary(
            DIV,
            int(factorial(n)),
            binary(POW, shifted, int(n + 1)),
        ));
    }
    if let Some((trig_head, w)) = match_t_times_trig(f, t) {
        let denom_base = sum_s_sq_param_sq(s, &w);
        let denom = binary(POW, denom_base, int(2));
        return Some(if trig_head == SIN {
            binary(DIV, binary(MUL, int(2), binary(MUL, w, s.clone())), denom)
        } else {
            binary(
                DIV,
                binary(SUB, binary(POW, s.clone(), int(2)), binary(POW, w, int(2))),
                denom,
            )
        });
    }
    None
}

fn inverse_lookup(f: &IRNode, s: &IRNode, t: &IRNode) -> Option<IRNode> {
    let (num, den) = binary_args(f, DIV)?;
    if is_int(num, 1) && same(den, s) {
        return Some(unary(UNIT_STEP, t.clone()));
    }
    if is_int(num, 1) {
        if let Some(a) = match_s_minus_a(den, s) {
            return Some(unary(EXP, binary(MUL, a, t.clone())));
        }
        if let Some(n) = match_pow_of(den, s) {
            if n >= 2 {
                let power = if n == 2 {
                    t.clone()
                } else {
                    binary(POW, t.clone(), int(n - 1))
                };
                let body = if n == 2 {
                    power
                } else {
                    binary(DIV, power, int(factorial(n - 1)))
                };
                return Some(body);
            }
        }
    }
    if let Some(w) = match_s_sq_plus_param_sq(den, s) {
        if same(num, &w) {
            return Some(unary(SIN, binary(MUL, w, t.clone())));
        }
        if same(num, s) {
            return Some(unary(COS, binary(MUL, w, t.clone())));
        }
    }
    if let Some(a) = match_s_sq_minus_param_sq(den, s) {
        if same(num, &a) {
            return Some(unary(SINH, binary(MUL, a, t.clone())));
        }
        if same(num, s) {
            return Some(unary(COSH, binary(MUL, a, t.clone())));
        }
    }
    None
}

fn match_exp_times_trig(f: &IRNode, t: &IRNode) -> Option<(IRNode, &'static str, IRNode)> {
    let (a, b) = binary_args(f, MUL)?;
    for (exp_node, trig_node) in [(a, b), (b, a)] {
        let shift = match_unary_linear(exp_node, EXP, t)?;
        if let Some(w) = match_unary_linear(trig_node, SIN, t) {
            return Some((shift, SIN, w));
        }
        if let Some(w) = match_unary_linear(trig_node, COS, t) {
            return Some((shift, COS, w));
        }
    }
    None
}

fn match_t_power_times_exp(f: &IRNode, t: &IRNode) -> Option<(i64, IRNode)> {
    let (a, b) = binary_args(f, MUL)?;
    for (power_node, exp_node) in [(a, b), (b, a)] {
        let n = match_power_of_t(power_node, t)?;
        let shift = match_unary_linear(exp_node, EXP, t)?;
        return Some((n, shift));
    }
    None
}

fn match_t_times_trig(f: &IRNode, t: &IRNode) -> Option<(&'static str, IRNode)> {
    let (a, b) = binary_args(f, MUL)?;
    for (left, right) in [(a, b), (b, a)] {
        if !same(left, t) {
            continue;
        }
        if let Some(w) = match_unary_linear(right, SIN, t) {
            return Some((SIN, w));
        }
        if let Some(w) = match_unary_linear(right, COS, t) {
            return Some((COS, w));
        }
    }
    None
}

fn match_power_of_t(f: &IRNode, t: &IRNode) -> Option<i64> {
    if same(f, t) {
        return Some(1);
    }
    let (base, exp) = binary_args(f, POW)?;
    if same(base, t) {
        if let IRNode::Integer(n) = exp {
            if *n >= 1 {
                return Some(*n);
            }
        }
    }
    None
}

fn match_unary_linear(f: &IRNode, head: &str, t: &IRNode) -> Option<IRNode> {
    let args = apply_args(f, head)?;
    if args.len() == 1 {
        extract_linear_arg(&args[0], t)
    } else {
        None
    }
}

fn extract_linear_arg(arg: &IRNode, t: &IRNode) -> Option<IRNode> {
    if same(arg, t) {
        return Some(int(1));
    }
    if let Some((a, b)) = binary_args(arg, MUL) {
        if same(a, t) && is_constant(b, t) {
            return Some(b.clone());
        }
        if same(b, t) && is_constant(a, t) {
            return Some(a.clone());
        }
    }
    if let Some(args) = apply_args(arg, NEG) {
        if args.len() == 1 {
            return extract_linear_arg(&args[0], t).map(negate);
        }
    }
    None
}

fn extract_coeff_and_fn(node: &IRNode, t: &IRNode) -> Option<(IRNode, IRNode)> {
    let (a, b) = binary_args(node, MUL)?;
    if is_constant(a, t) {
        return Some((a.clone(), b.clone()));
    }
    if is_constant(b, t) {
        return Some((b.clone(), a.clone()));
    }
    None
}

fn is_constant(node: &IRNode, var: &IRNode) -> bool {
    if same(node, var) {
        return false;
    }
    match node {
        IRNode::Apply(apply_node) => apply_node.args.iter().all(|arg| is_constant(arg, var)),
        _ => true,
    }
}

fn match_s_minus_a(node: &IRNode, s: &IRNode) -> Option<IRNode> {
    let (left, right) = binary_args(node, SUB)?;
    if same(left, s) {
        Some(right.clone())
    } else {
        None
    }
}

fn match_pow_of(node: &IRNode, base: &IRNode) -> Option<i64> {
    let (pow_base, exp) = binary_args(node, POW)?;
    if same(pow_base, base) {
        if let IRNode::Integer(n) = exp {
            return Some(*n);
        }
    }
    None
}

fn match_s_sq_plus_param_sq(node: &IRNode, s: &IRNode) -> Option<IRNode> {
    let (a, b) = binary_args(node, ADD)?;
    match_s_sq_param_sq(a, b, s).or_else(|| match_s_sq_param_sq(b, a, s))
}

fn match_s_sq_minus_param_sq(node: &IRNode, s: &IRNode) -> Option<IRNode> {
    let (a, b) = binary_args(node, SUB)?;
    match_s_sq_param_sq(a, b, s)
}

fn match_s_sq_param_sq(s_sq: &IRNode, param_sq: &IRNode, s: &IRNode) -> Option<IRNode> {
    if !matches!(match_pow_of(s_sq, s), Some(2)) {
        return None;
    }
    if let Some(param) = sqrt_param(param_sq) {
        return Some(param);
    }
    None
}

fn sqrt_param(node: &IRNode) -> Option<IRNode> {
    if let Some((base, exp)) = binary_args(node, POW) {
        if is_int(exp, 2) {
            return Some(base.clone());
        }
    }
    if let IRNode::Integer(n) = node {
        if *n >= 0 {
            let root = (*n as f64).sqrt() as i64;
            if root * root == *n {
                return Some(int(root));
            }
        }
    }
    None
}

fn is_apply_of_var(node: &IRNode, head: &str, var: &IRNode) -> bool {
    matches!(apply_args(node, head), Some(args) if args.len() == 1 && same(&args[0], var))
}

fn sum_s_sq_param_sq(s: &IRNode, param: &IRNode) -> IRNode {
    binary(
        ADD,
        binary(POW, s.clone(), int(2)),
        binary(POW, param.clone(), int(2)),
    )
}

fn sub_s_sq_param_sq(s: &IRNode, param: &IRNode) -> IRNode {
    binary(
        SUB,
        binary(POW, s.clone(), int(2)),
        binary(POW, param.clone(), int(2)),
    )
}

fn apply_args<'a>(node: &'a IRNode, head: &str) -> Option<&'a [IRNode]> {
    match node {
        IRNode::Apply(apply_node) if apply_node.head == sym(head) => Some(&apply_node.args),
        _ => None,
    }
}

fn binary_args<'a>(node: &'a IRNode, head: &str) -> Option<(&'a IRNode, &'a IRNode)> {
    let args = apply_args(node, head)?;
    if args.len() == 2 {
        Some((&args[0], &args[1]))
    } else {
        None
    }
}

fn binary(head: &str, a: IRNode, b: IRNode) -> IRNode {
    apply(sym(head), vec![a, b])
}

fn unary(head: &str, arg: IRNode) -> IRNode {
    apply(sym(head), vec![arg])
}

fn negate(node: IRNode) -> IRNode {
    match node {
        IRNode::Integer(n) => int(-n),
        _ => unary(NEG, node),
    }
}

fn same(a: &IRNode, b: &IRNode) -> bool {
    a == b
}

fn is_int(node: &IRNode, value: i64) -> bool {
    matches!(node, IRNode::Integer(n) if *n == value)
}

fn is_one(node: &IRNode) -> bool {
    is_int(node, 1) || matches!(node, IRNode::Rational(n, d) if *n == *d)
}

fn factorial(n: i64) -> i64 {
    (1..=n).product()
}
