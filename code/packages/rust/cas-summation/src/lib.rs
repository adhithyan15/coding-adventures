use std::cmp::Ordering;

use symbolic_ir::{apply, int, rat, sym, IRNode, ADD, DIV, EXP, MUL, NEG, POW, SUB};

pub const SUM: &str = "Sum";
pub const PRODUCT: &str = "Product";
pub const GAMMA_FUNC: &str = "GammaFunc";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rational {
    numer: i64,
    denom: i64,
}

impl Rational {
    pub fn new(numer: i64, denom: i64) -> Self {
        assert!(denom != 0, "Rational denominator cannot be zero");
        let (mut numer, mut denom) = if denom < 0 {
            (-numer, -denom)
        } else {
            (numer, denom)
        };
        let g = gcd(numer.unsigned_abs(), denom as u64) as i64;
        numer /= g;
        denom /= g;
        Self { numer, denom }
    }

    pub fn to_ir(self) -> IRNode {
        if self.denom == 1 {
            int(self.numer)
        } else {
            rat(self.numer, self.denom)
        }
    }
}

impl std::ops::Add for Rational {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(
            self.numer * rhs.denom + rhs.numer * self.denom,
            self.denom * rhs.denom,
        )
    }
}

impl std::ops::Sub for Rational {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(
            self.numer * rhs.denom - rhs.numer * self.denom,
            self.denom * rhs.denom,
        )
    }
}

impl std::ops::Mul for Rational {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.numer * rhs.numer, self.denom * rhs.denom)
    }
}

impl std::ops::Div for Rational {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.numer * rhs.denom, self.denom * rhs.numer)
    }
}

impl std::ops::Neg for Rational {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.numer, self.denom)
    }
}

pub fn rational_value(node: &IRNode) -> Option<Rational> {
    match node {
        IRNode::Integer(value) => Some(Rational::new(*value, 1)),
        IRNode::Rational(numer, denom) => Some(Rational::new(*numer, *denom)),
        _ => None,
    }
}

pub fn is_constant_in(node: &IRNode, k: &IRNode) -> bool {
    if node == k {
        return false;
    }
    match node {
        IRNode::Apply(apply_node) => apply_node.args.iter().all(|arg| is_constant_in(arg, k)),
        _ => true,
    }
}

pub fn geometric_sum_ir(
    coeff: IRNode,
    base: IRNode,
    lo: IRNode,
    hi: Option<IRNode>,
    is_infinite: bool,
) -> IRNode {
    let sum_part = if is_infinite {
        let one_minus_base = binary(SUB, int(1), base.clone());
        if matches!(lo, IRNode::Integer(0)) {
            binary(DIV, int(1), one_minus_base)
        } else {
            binary(DIV, binary(POW, base, lo), one_minus_base)
        }
    } else {
        let hi = hi.expect("hi must be provided for finite geometric sums");
        let span_plus_1 = binary(ADD, binary(SUB, hi, lo.clone()), int(1));
        let numerator = binary(SUB, binary(POW, base.clone(), span_plus_1), int(1));
        let denominator = binary(SUB, base.clone(), int(1));
        let ratio_part = binary(DIV, numerator, denominator);
        binary(MUL, binary(POW, base, lo), ratio_part)
    };

    if matches!(coeff, IRNode::Integer(1)) {
        sum_part
    } else {
        binary(MUL, coeff, sum_part)
    }
}

pub fn faulhaber_ir(m: i64, n: IRNode) -> Option<IRNode> {
    Some(match m {
        0 => n,
        1 => binary(DIV, binary(MUL, n.clone(), binary(ADD, n, int(1))), int(2)),
        2 => binary(
            DIV,
            binary(
                MUL,
                n.clone(),
                binary(
                    MUL,
                    binary(ADD, n.clone(), int(1)),
                    binary(ADD, binary(MUL, int(2), n), int(1)),
                ),
            ),
            int(6),
        ),
        3 => {
            let half = binary(DIV, binary(MUL, n.clone(), binary(ADD, n, int(1))), int(2));
            binary(POW, half, int(2))
        }
        4 => {
            let inner = binary(
                SUB,
                binary(
                    ADD,
                    binary(MUL, int(3), binary(POW, n.clone(), int(2))),
                    binary(MUL, int(3), n.clone()),
                ),
                int(1),
            );
            let two_n_plus_1 = binary(ADD, binary(MUL, int(2), n.clone()), int(1));
            binary(
                DIV,
                binary(
                    MUL,
                    n.clone(),
                    binary(
                        MUL,
                        binary(ADD, n, int(1)),
                        binary(MUL, two_n_plus_1, inner),
                    ),
                ),
                int(30),
            )
        }
        5 => {
            let inner = binary(
                SUB,
                binary(
                    ADD,
                    binary(MUL, int(2), binary(POW, n.clone(), int(2))),
                    binary(MUL, int(2), n.clone()),
                ),
                int(1),
            );
            binary(
                DIV,
                binary(
                    MUL,
                    binary(POW, n.clone(), int(2)),
                    binary(MUL, binary(POW, binary(ADD, n, int(1)), int(2)), inner),
                ),
                int(12),
            )
        }
        _ => return None,
    })
}

pub fn poly_sum_ir(m: i64, coeff: Rational, lo_val: i64, hi: IRNode) -> Option<IRNode> {
    let s_hi = faulhaber_ir(m, hi)?;
    let lo_minus_1 = lo_val - 1;
    let s_lo = if lo_minus_1 <= 0 {
        None
    } else {
        faulhaber_ir(m, int(lo_minus_1))
    };
    let mut diff = match s_lo {
        Some(s_lo) => binary(SUB, s_hi, s_lo),
        None => s_hi,
    };
    if lo_val == 0 && m == 0 {
        diff = binary(ADD, diff, int(1));
    }
    if coeff == Rational::new(1, 1) {
        Some(diff)
    } else {
        Some(binary(MUL, coeff.to_ir(), diff))
    }
}

pub fn try_special_infinite(f: &IRNode, k: &IRNode, lo: &IRNode) -> Option<IRNode> {
    if is_int(lo, 1) && match_inv_k_pow(f, k, 2) {
        return Some(binary(DIV, binary(POW, sym("%pi"), int(2)), int(6)));
    }
    if is_int(lo, 1) && match_inv_k_pow(f, k, 4) {
        return Some(binary(DIV, binary(POW, sym("%pi"), int(4)), int(90)));
    }
    if is_int(lo, 0) && match_leibniz(f, k) {
        return Some(binary(DIV, sym("%pi"), int(4)));
    }
    if is_int(lo, 0) && match_inv_factorial(f, k) {
        return Some(sym("%e"));
    }
    if is_int(lo, 0) {
        if let Some(x) = match_exp_series(f, k) {
            return Some(unary(EXP, x));
        }
    }
    None
}

pub fn evaluate_sum<E>(f: IRNode, k: IRNode, lo: IRNode, hi: IRNode, mut eval_fn: E) -> IRNode
where
    E: FnMut(IRNode) -> IRNode,
{
    let inf_upper = is_inf(&hi);

    if is_constant_in(&f, &k) {
        let count = binary(ADD, binary(SUB, hi.clone(), lo.clone()), int(1));
        return eval_fn(binary(MUL, f, count));
    }

    if let Some((coeff, base)) = try_geometric(&f, &k) {
        let raw = geometric_sum_ir(coeff, base, lo.clone(), Some(hi.clone()), inf_upper);
        return eval_fn(raw);
    }

    if let Some((coeff, m)) = try_power_of_k(&f, &k) {
        if let IRNode::Integer(lo_int) = lo {
            if lo_int >= 0 && !inf_upper {
                if let Some(raw) = poly_sum_ir(m, coeff, lo_int, hi.clone()) {
                    return eval_fn(raw);
                }
            }
        }
    }

    if inf_upper {
        if let Some(raw) = try_special_infinite(&f, &k, &lo) {
            return eval_fn(raw);
        }
    }

    if let (IRNode::Integer(lo_int), IRNode::Integer(hi_int)) = (&lo, &hi) {
        if (0..=999).contains(&(hi_int - lo_int)) {
            let mut total = Rational::new(0, 1);
            let mut ok = true;
            for value in *lo_int..=*hi_int {
                let term = substitute(&f, &k, &int(value));
                let evaluated = eval_fn(term);
                if let Some(r) = rational_value(&evaluated) {
                    total = total + r;
                } else {
                    ok = false;
                    break;
                }
            }
            if ok {
                return total.to_ir();
            }
        }
    }

    apply(sym(SUM), vec![f, k, lo, hi])
}

pub fn evaluate_product<E>(f: IRNode, k: IRNode, lo: IRNode, hi: IRNode, mut eval_fn: E) -> IRNode
where
    E: FnMut(IRNode) -> IRNode,
{
    if let Some(raw) = evaluate_product_expr(f.clone(), k.clone(), lo.clone(), hi.clone()) {
        return eval_fn(raw);
    }

    if let (IRNode::Integer(lo_int), IRNode::Integer(hi_int)) = (&lo, &hi) {
        if (0..=20).contains(&(hi_int - lo_int)) {
            let mut total = Rational::new(1, 1);
            let mut ok = true;
            for value in *lo_int..=*hi_int {
                let term = substitute(&f, &k, &int(value));
                let evaluated = eval_fn(term);
                if let Some(r) = rational_value(&evaluated) {
                    total = total * r;
                } else {
                    ok = false;
                    break;
                }
            }
            if ok {
                return total.to_ir();
            }
        }
    }

    apply(sym(PRODUCT), vec![f, k, lo, hi])
}

pub fn evaluate_product_expr(f: IRNode, k: IRNode, lo: IRNode, hi: IRNode) -> Option<IRNode> {
    if is_constant_in(&f, &k) {
        let span = binary(ADD, binary(SUB, hi, lo), int(1));
        return Some(binary(POW, f, span));
    }
    if is_int(&lo, 1) && f == k {
        return Some(gamma(hi));
    }
    if is_int(&lo, 1) {
        if let Some(coeff) = split_linear_coeff(&f, &k) {
            if coeff == Rational::new(1, 1) {
                return Some(gamma(hi));
            }
            return Some(binary(
                MUL,
                binary(POW, coeff.to_ir(), hi.clone()),
                gamma(hi),
            ));
        }
    }
    None
}

fn try_geometric(f: &IRNode, k: &IRNode) -> Option<(IRNode, IRNode)> {
    if let IRNode::Apply(node) = f {
        if head_is(&node.head, POW)
            && node.args.len() == 2
            && node.args[1] == *k
            && is_constant_in(&node.args[0], k)
            && node.args[0] != *k
        {
            return Some((int(1), node.args[0].clone()));
        }
        if head_is(&node.head, MUL) && node.args.len() == 2 {
            for (coeff, pow) in [
                (&node.args[0], &node.args[1]),
                (&node.args[1], &node.args[0]),
            ] {
                if let IRNode::Apply(pow_node) = pow {
                    if head_is(&pow_node.head, POW)
                        && pow_node.args.len() == 2
                        && pow_node.args[1] == *k
                        && pow_node.args[0] != *k
                        && is_constant_in(&pow_node.args[0], k)
                        && is_constant_in(coeff, k)
                    {
                        return Some((coeff.clone(), pow_node.args[0].clone()));
                    }
                }
            }
        }
    }
    None
}

fn try_power_of_k(f: &IRNode, k: &IRNode) -> Option<(Rational, i64)> {
    if f == k {
        return Some((Rational::new(1, 1), 1));
    }
    if let IRNode::Apply(node) = f {
        if head_is(&node.head, POW) && node.args.len() == 2 && node.args[0] == *k {
            if let IRNode::Integer(m) = node.args[1] {
                if (0..=5).contains(&m) {
                    return Some((Rational::new(1, 1), m));
                }
            }
        }
        if head_is(&node.head, MUL) && node.args.len() == 2 {
            for (coeff, other) in [
                (&node.args[0], &node.args[1]),
                (&node.args[1], &node.args[0]),
            ] {
                if let Some(c) = rational_value(coeff) {
                    if other == k {
                        return Some((c, 1));
                    }
                    if let IRNode::Apply(other_node) = other {
                        if head_is(&other_node.head, POW)
                            && other_node.args.len() == 2
                            && other_node.args[0] == *k
                        {
                            if let IRNode::Integer(m) = other_node.args[1] {
                                if (0..=5).contains(&m) {
                                    return Some((c, m));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn split_linear_coeff(f: &IRNode, k: &IRNode) -> Option<Rational> {
    if f == k {
        return Some(Rational::new(1, 1));
    }
    if let IRNode::Apply(node) = f {
        if head_is(&node.head, MUL) && node.args.len() == 2 {
            let a = rational_value(&node.args[0]);
            let b = rational_value(&node.args[1]);
            if let Some(coeff) = a {
                if node.args[1] == *k {
                    return Some(coeff);
                }
            }
            if let Some(coeff) = b {
                if node.args[0] == *k {
                    return Some(coeff);
                }
            }
        }
    }
    None
}

fn match_inv_k_pow(f: &IRNode, k: &IRNode, exp: i64) -> bool {
    if let IRNode::Apply(node) = f {
        if head_is(&node.head, DIV) && node.args.len() == 2 && is_int(&node.args[0], 1) {
            if let IRNode::Apply(denom) = &node.args[1] {
                return head_is(&denom.head, POW)
                    && denom.args.len() == 2
                    && denom.args[0] == *k
                    && is_int(&denom.args[1], exp);
            }
        }
    }
    false
}

fn match_leibniz(f: &IRNode, k: &IRNode) -> bool {
    let IRNode::Apply(node) = f else {
        return false;
    };
    if !head_is(&node.head, DIV) || node.args.len() != 2 {
        return false;
    }
    let numerator = &node.args[0];
    let denominator = &node.args[1];
    let num_ok = match numerator {
        IRNode::Apply(num) if head_is(&num.head, POW) && num.args.len() == 2 => {
            num.args[1] == *k && (is_int(&num.args[0], -1) || is_neg_one(&num.args[0]))
        }
        _ => false,
    };
    if !num_ok {
        return false;
    }
    let IRNode::Apply(denom) = denominator else {
        return false;
    };
    if !head_is(&denom.head, ADD) || denom.args.len() != 2 {
        return false;
    }
    let (a, b) = (&denom.args[0], &denom.args[1]);
    is_two_k_plus_one(a, b, k) || is_two_k_plus_one(b, a, k)
}

fn match_inv_factorial(f: &IRNode, k: &IRNode) -> bool {
    let IRNode::Apply(node) = f else {
        return false;
    };
    if !head_is(&node.head, DIV) || node.args.len() != 2 || !is_int(&node.args[0], 1) {
        return false;
    }
    match_gamma_k_plus_one(&node.args[1], k)
}

fn match_exp_series(f: &IRNode, k: &IRNode) -> Option<IRNode> {
    let IRNode::Apply(node) = f else {
        return None;
    };
    if !head_is(&node.head, DIV) || node.args.len() != 2 {
        return None;
    }
    let IRNode::Apply(numer) = &node.args[0] else {
        return None;
    };
    if !head_is(&numer.head, POW) || numer.args.len() != 2 || numer.args[1] != *k {
        return None;
    }
    if numer.args[0] == *k || !match_gamma_k_plus_one(&node.args[1], k) {
        return None;
    }
    Some(numer.args[0].clone())
}

fn match_gamma_k_plus_one(node: &IRNode, k: &IRNode) -> bool {
    let IRNode::Apply(gamma_node) = node else {
        return false;
    };
    if !head_is(&gamma_node.head, GAMMA_FUNC) || gamma_node.args.len() != 1 {
        return false;
    }
    let IRNode::Apply(arg) = &gamma_node.args[0] else {
        return false;
    };
    head_is(&arg.head, ADD) && arg.args.len() == 2 && arg.args[0] == *k && is_int(&arg.args[1], 1)
}

fn substitute(node: &IRNode, from: &IRNode, to: &IRNode) -> IRNode {
    if node == from {
        return to.clone();
    }
    match node {
        IRNode::Apply(apply_node) => apply(
            apply_node.head.clone(),
            apply_node
                .args
                .iter()
                .map(|arg| substitute(arg, from, to))
                .collect(),
        ),
        _ => node.clone(),
    }
}

fn binary(head: &str, a: IRNode, b: IRNode) -> IRNode {
    apply(sym(head), vec![a, b])
}

fn unary(head: &str, a: IRNode) -> IRNode {
    apply(sym(head), vec![a])
}

fn gamma(n: IRNode) -> IRNode {
    unary(GAMMA_FUNC, binary(ADD, n, int(1)))
}

fn head_is(node: &IRNode, name: &str) -> bool {
    matches!(node, IRNode::Symbol(actual) if actual == name)
}

fn is_int(node: &IRNode, value: i64) -> bool {
    matches!(node, IRNode::Integer(actual) if *actual == value)
}

fn is_inf(node: &IRNode) -> bool {
    matches!(node, IRNode::Symbol(name) if name == "inf" || name == "%inf")
}

fn is_neg_one(node: &IRNode) -> bool {
    matches!(node, IRNode::Apply(apply_node)
        if head_is(&apply_node.head, NEG)
            && apply_node.args.len() == 1
            && is_int(&apply_node.args[0], 1))
}

fn is_two_k_plus_one(two_k: &IRNode, one: &IRNode, k: &IRNode) -> bool {
    if !is_int(one, 1) {
        return false;
    }
    matches!(two_k, IRNode::Apply(node)
        if head_is(&node.head, MUL)
            && node.args.len() == 2
            && is_int(&node.args[0], 2)
            && node.args[1] == *k)
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    match a.cmp(&0) {
        Ordering::Equal => 1,
        Ordering::Greater => a,
        Ordering::Less => unreachable!(),
    }
}
