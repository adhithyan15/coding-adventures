use symbolic_ir::{apply, int, sym, IRNode, ADD, COS, DIV, EXP, MUL, NEG, POW, SIN, SQRT, SUB};

pub const FOURIER: &str = "Fourier";
pub const IFOURIER: &str = "IFourier";
pub const DIRAC_DELTA: &str = "DiracDelta";
pub const UNIT_STEP: &str = "UnitStep";
pub const IMAGINARY_UNIT: &str = "ImaginaryUnit";
pub const PI: &str = "%pi";

pub type EvalFn = dyn Fn(IRNode) -> IRNode;

pub fn fourier_transform(f: IRNode, t: IRNode, omega: IRNode) -> IRNode {
    if let Some((a, b)) = binary_args(&f, ADD) {
        return binary(
            ADD,
            fourier_transform(a.clone(), t.clone(), omega.clone()),
            fourier_transform(b.clone(), t.clone(), omega),
        );
    }
    if let Some((coeff, body)) = extract_coeff_and_fn(&f, &t) {
        if !is_int(&coeff, 1) {
            return binary(MUL, coeff, fourier_transform(body, t, omega));
        }
    }
    forward_lookup(&f, &t, &omega).unwrap_or_else(|| apply(sym(FOURIER), vec![f, t, omega]))
}

pub fn ifourier_transform(f: IRNode, omega: IRNode, t: IRNode) -> IRNode {
    inverse_lookup(&f, &omega, &t).unwrap_or_else(|| apply(sym(IFOURIER), vec![f, omega, t]))
}

pub fn fourier_handler(expr: &IRNode, eval: &EvalFn) -> IRNode {
    let Some(args) = apply_args(expr, FOURIER) else {
        return expr.clone();
    };
    if args.len() != 3
        || !matches!(args[1], IRNode::Symbol(_))
        || !matches!(args[2], IRNode::Symbol(_))
    {
        return expr.clone();
    }
    eval(fourier_transform(
        args[0].clone(),
        args[1].clone(),
        args[2].clone(),
    ))
}

pub fn ifourier_handler(expr: &IRNode, eval: &EvalFn) -> IRNode {
    let Some(args) = apply_args(expr, IFOURIER) else {
        return expr.clone();
    };
    if args.len() != 3
        || !matches!(args[1], IRNode::Symbol(_))
        || !matches!(args[2], IRNode::Symbol(_))
    {
        return expr.clone();
    }
    eval(ifourier_transform(
        args[0].clone(),
        args[1].clone(),
        args[2].clone(),
    ))
}

pub fn build_fourier_handler_table() -> Vec<&'static str> {
    vec![FOURIER, IFOURIER]
}

fn forward_lookup(f: &IRNode, t: &IRNode, omega: &IRNode) -> Option<IRNode> {
    if is_apply_of_var(f, DIRAC_DELTA, t) {
        return Some(int(1));
    }
    if is_one(f) {
        return Some(two_pi_delta(omega.clone()));
    }
    if let Some(a) = match_causal_exp(f, t) {
        return Some(binary(
            DIV,
            int(1),
            binary(ADD, a, binary(MUL, sym(IMAGINARY_UNIT), omega.clone())),
        ));
    }
    if let Some(a) = match_complex_exp(f, t) {
        return Some(two_pi_delta(binary(SUB, omega.clone(), a)));
    }
    if let Some(w) = match_unary_linear(f, SIN, t) {
        let delta_plus = unary(DIRAC_DELTA, binary(ADD, omega.clone(), w.clone()));
        let delta_minus = unary(DIRAC_DELTA, binary(SUB, omega.clone(), w));
        return Some(binary(
            MUL,
            binary(MUL, sym(IMAGINARY_UNIT), sym(PI)),
            binary(SUB, delta_plus, delta_minus),
        ));
    }
    if let Some(w) = match_unary_linear(f, COS, t) {
        let delta_minus = unary(DIRAC_DELTA, binary(SUB, omega.clone(), w.clone()));
        let delta_plus = unary(DIRAC_DELTA, binary(ADD, omega.clone(), w));
        return Some(binary(MUL, sym(PI), binary(ADD, delta_minus, delta_plus)));
    }
    if let Some(a) = match_gaussian(f, t) {
        let scale = unary(SQRT, binary(DIV, sym(PI), a.clone()));
        let exponent = unary(
            NEG,
            binary(
                DIV,
                binary(POW, omega.clone(), int(2)),
                binary(MUL, int(4), a),
            ),
        );
        return Some(binary(MUL, scale, unary(EXP, exponent)));
    }
    if let Some(a) = match_t_causal_exp(f, t) {
        let denom = binary(ADD, a, binary(MUL, sym(IMAGINARY_UNIT), omega.clone()));
        return Some(binary(DIV, int(1), binary(POW, denom, int(2))));
    }
    None
}

fn inverse_lookup(f: &IRNode, omega: &IRNode, t: &IRNode) -> Option<IRNode> {
    if is_int(f, 1) {
        return Some(unary(DIRAC_DELTA, t.clone()));
    }
    if is_apply_of_var(f, DIRAC_DELTA, omega) {
        return Some(binary(DIV, int(1), binary(MUL, int(2), sym(PI))));
    }
    if let Some(delta_arg) = match_two_pi_delta(f) {
        if same(&delta_arg, omega) {
            return Some(int(1));
        }
        if let Some(a) = match_omega_minus_a(&delta_arg, omega) {
            return Some(unary(
                EXP,
                binary(MUL, binary(MUL, sym(IMAGINARY_UNIT), a), t.clone()),
            ));
        }
    }
    if let Some(a) = match_causal_denom(f, omega, false) {
        return Some(binary(
            MUL,
            unary(EXP, unary(NEG, binary(MUL, a, t.clone()))),
            unary(UNIT_STEP, t.clone()),
        ));
    }
    if let Some(a) = match_causal_denom(f, omega, true) {
        return Some(binary(
            MUL,
            t.clone(),
            binary(
                MUL,
                unary(EXP, unary(NEG, binary(MUL, a, t.clone()))),
                unary(UNIT_STEP, t.clone()),
            ),
        ));
    }
    None
}

fn match_causal_exp(f: &IRNode, t: &IRNode) -> Option<IRNode> {
    let args = apply_args(f, EXP)?;
    if args.len() != 1 {
        return None;
    }
    let neg_args = apply_args(&args[0], NEG)?;
    if neg_args.len() == 1 {
        extract_linear_arg(&neg_args[0], t)
    } else {
        None
    }
}

fn match_complex_exp(f: &IRNode, t: &IRNode) -> Option<IRNode> {
    let args = apply_args(f, EXP)?;
    if args.len() == 1 {
        match_i_a_t(&args[0], t)
    } else {
        None
    }
}

fn match_i_a_t(node: &IRNode, t: &IRNode) -> Option<IRNode> {
    let (a, b) = binary_args(node, MUL)?;
    for (left, right) in [(a, b), (b, a)] {
        if same(right, t) {
            if same(left, &sym(IMAGINARY_UNIT)) {
                return Some(int(1));
            }
            if let Some((x, y)) = binary_args(left, MUL) {
                if same(x, &sym(IMAGINARY_UNIT)) && is_constant(y, t) {
                    return Some(y.clone());
                }
                if same(y, &sym(IMAGINARY_UNIT)) && is_constant(x, t) {
                    return Some(x.clone());
                }
            }
        }
    }
    None
}

fn match_gaussian(f: &IRNode, t: &IRNode) -> Option<IRNode> {
    let args = apply_args(f, EXP)?;
    let neg_args = (args.len() == 1)
        .then(|| apply_args(&args[0], NEG))
        .flatten()?;
    if neg_args.len() != 1 {
        return None;
    }
    let inner = &neg_args[0];
    if let Some((base, exp)) = binary_args(inner, POW) {
        if same(base, t) && is_int(exp, 2) {
            return Some(int(1));
        }
    }
    let (a, b) = binary_args(inner, MUL)?;
    for (coeff, pow) in [(a, b), (b, a)] {
        if is_constant(coeff, t) {
            if let Some((base, exp)) = binary_args(pow, POW) {
                if same(base, t) && is_int(exp, 2) {
                    return Some(coeff.clone());
                }
            }
        }
    }
    None
}

fn match_t_causal_exp(f: &IRNode, t: &IRNode) -> Option<IRNode> {
    let (a, b) = binary_args(f, MUL)?;
    for (left, right) in [(a, b), (b, a)] {
        if same(left, t) {
            if let Some(coeff) = match_causal_exp(right, t) {
                return Some(coeff);
            }
        }
    }
    None
}

fn match_causal_denom(f: &IRNode, omega: &IRNode, squared: bool) -> Option<IRNode> {
    let (num, den) = binary_args(f, DIV)?;
    if !is_int(num, 1) {
        return None;
    }
    let denom = if squared {
        let (base, exp) = binary_args(den, POW)?;
        if is_int(exp, 2) {
            base
        } else {
            return None;
        }
    } else {
        den
    };
    let (a, iomega) = binary_args(denom, ADD)?;
    if is_i_omega(iomega, omega) {
        Some(a.clone())
    } else if is_i_omega(a, omega) {
        Some(iomega.clone())
    } else {
        None
    }
}

fn match_two_pi_delta(f: &IRNode) -> Option<IRNode> {
    let (a, b) = binary_args(f, MUL)?;
    for (left, right) in [(a, b), (b, a)] {
        if is_two_pi(left) {
            let args = apply_args(right, DIRAC_DELTA)?;
            if args.len() == 1 {
                return Some(args[0].clone());
            }
        }
    }
    None
}

fn match_omega_minus_a(node: &IRNode, omega: &IRNode) -> Option<IRNode> {
    let (left, right) = binary_args(node, SUB)?;
    if same(left, omega) {
        Some(right.clone())
    } else {
        None
    }
}

fn is_i_omega(node: &IRNode, omega: &IRNode) -> bool {
    let Some((a, b)) = binary_args(node, MUL) else {
        return false;
    };
    (same(a, &sym(IMAGINARY_UNIT)) && same(b, omega))
        || (same(b, &sym(IMAGINARY_UNIT)) && same(a, omega))
}

fn is_two_pi(node: &IRNode) -> bool {
    let Some((a, b)) = binary_args(node, MUL) else {
        return false;
    };
    (is_int(a, 2) && same(b, &sym(PI))) || (is_int(b, 2) && same(a, &sym(PI)))
}

fn two_pi_delta(arg: IRNode) -> IRNode {
    binary(MUL, binary(MUL, int(2), sym(PI)), unary(DIRAC_DELTA, arg))
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
    let (a, b) = binary_args(arg, MUL)?;
    if same(a, t) && is_constant(b, t) {
        Some(b.clone())
    } else if same(b, t) && is_constant(a, t) {
        Some(a.clone())
    } else {
        None
    }
}

fn is_apply_of_var(node: &IRNode, head: &str, var: &IRNode) -> bool {
    matches!(apply_args(node, head), Some(args) if args.len() == 1 && same(&args[0], var))
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

fn same(a: &IRNode, b: &IRNode) -> bool {
    a == b
}

fn is_int(node: &IRNode, value: i64) -> bool {
    matches!(node, IRNode::Integer(n) if *n == value)
}

fn is_one(node: &IRNode) -> bool {
    is_int(node, 1) || matches!(node, IRNode::Rational(n, d) if *n == *d)
}
