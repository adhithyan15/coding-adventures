//! Symbolic `ODE2` solving over `symbolic-ir`.
//!
//! This is a faithful first Rust slice of the Python `cas-ode` package.  The
//! solver is deliberately pure IR-to-IR: where the Python implementation asks
//! the symbolic VM to evaluate an integral, this crate emits an explicit
//! `Integrate(...)` node unless a small structural primitive is needed for
//! recognition, such as exact ODE potentials.

use std::collections::BTreeMap;
use std::ops::{Add as AddOp, Div as DivOp, Mul as MulOp, Neg as NegOp, Sub as SubOp};

use symbolic_ir::{
    apply, int, rat, sym, IRNode, ADD, COS, D, DIV, EQUAL, EXP, INTEGRATE, LOG, MUL, NEG, POW, SIN,
    SUB,
};

pub const ODE2: &str = "ODE2";
pub type Handler = fn(&IRNode) -> IRNode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Frac {
    n: i64,
    d: i64,
}

impl Frac {
    const ZERO: Self = Self { n: 0, d: 1 };
    const ONE: Self = Self { n: 1, d: 1 };

    fn new(n: i64, d: i64) -> Self {
        assert_ne!(d, 0, "zero denominator");
        if n == 0 {
            return Self::ZERO;
        }
        let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
        let g = gcd(n.unsigned_abs(), d.unsigned_abs()) as i64;
        Self { n: n / g, d: d / g }
    }

    fn int(n: i64) -> Self {
        Self { n, d: 1 }
    }

    fn is_zero(self) -> bool {
        self.n == 0
    }

    fn to_ir(self) -> IRNode {
        if self.d == 1 {
            int(self.n)
        } else {
            rat(self.n, self.d)
        }
    }

    fn to_ir_unsigned(self) -> IRNode {
        debug_assert!(self.n >= 0);
        self.to_ir()
    }
}

impl AddOp for Frac {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.n * rhs.d + rhs.n * self.d, self.d * rhs.d)
    }
}

impl SubOp for Frac {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self + (-rhs)
    }
}

impl MulOp for Frac {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.n * rhs.n, self.d * rhs.d)
    }
}

impl DivOp for Frac {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.n * rhs.d, self.d * rhs.n)
    }
}

impl NegOp for Frac {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            n: -self.n,
            d: self.d,
        }
    }
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

fn c() -> IRNode {
    sym("%c")
}

fn c1() -> IRNode {
    sym("%c1")
}

fn c2() -> IRNode {
    sym("%c2")
}

fn zero() -> IRNode {
    int(0)
}

fn one() -> IRNode {
    int(1)
}

fn two() -> IRNode {
    int(2)
}

fn is_int(node: &IRNode, value: i64) -> bool {
    matches!(node, IRNode::Integer(n) if *n == value)
}

#[cfg(test)]
fn head_is(node: &IRNode, head: &str) -> bool {
    matches!(node, IRNode::Apply(app) if app.head == sym(head))
}

fn args_of<'a>(node: &'a IRNode, head: &str) -> Option<&'a [IRNode]> {
    match node {
        IRNode::Apply(app) if app.head == sym(head) => Some(&app.args),
        _ => None,
    }
}

fn binary_args<'a>(node: &'a IRNode, head: &str) -> Option<(&'a IRNode, &'a IRNode)> {
    let args = args_of(node, head)?;
    if args.len() == 2 {
        Some((&args[0], &args[1]))
    } else {
        None
    }
}

fn unary_arg<'a>(node: &'a IRNode, head: &str) -> Option<&'a IRNode> {
    let args = args_of(node, head)?;
    if args.len() == 1 {
        Some(&args[0])
    } else {
        None
    }
}

fn add(a: IRNode, b: IRNode) -> IRNode {
    if is_int(&a, 0) {
        b
    } else if is_int(&b, 0) {
        a
    } else {
        apply(sym(ADD), vec![a, b])
    }
}

fn sub(a: IRNode, b: IRNode) -> IRNode {
    if is_int(&b, 0) {
        a
    } else {
        apply(sym(SUB), vec![a, b])
    }
}

fn neg(a: IRNode) -> IRNode {
    if let IRNode::Integer(n) = a {
        int(-n)
    } else if let Some(inner) = unary_arg(&a, NEG) {
        inner.clone()
    } else {
        apply(sym(NEG), vec![a])
    }
}

fn mul(a: IRNode, b: IRNode) -> IRNode {
    if is_int(&a, 0) || is_int(&b, 0) {
        zero()
    } else if is_int(&a, 1) {
        b
    } else if is_int(&b, 1) {
        a
    } else if is_int(&a, -1) {
        neg(b)
    } else if is_int(&b, -1) {
        neg(a)
    } else {
        apply(sym(MUL), vec![a, b])
    }
}

fn div(a: IRNode, b: IRNode) -> IRNode {
    if is_int(&b, 1) {
        a
    } else {
        apply(sym(DIV), vec![a, b])
    }
}

fn pow(a: IRNode, b: IRNode) -> IRNode {
    if is_int(&b, 0) {
        one()
    } else if is_int(&b, 1) {
        a
    } else {
        apply(sym(POW), vec![a, b])
    }
}

fn exp(a: IRNode) -> IRNode {
    apply(sym(EXP), vec![a])
}

fn sin(a: IRNode) -> IRNode {
    apply(sym(SIN), vec![a])
}

fn cos(a: IRNode) -> IRNode {
    apply(sym(COS), vec![a])
}

fn log(a: IRNode) -> IRNode {
    apply(sym(LOG), vec![a])
}

fn equal(a: IRNode, b: IRNode) -> IRNode {
    apply(sym(EQUAL), vec![a, b])
}

fn deriv(expr: IRNode, var: IRNode) -> IRNode {
    apply(sym(D), vec![expr, var])
}

fn integrate(expr: IRNode, var: IRNode) -> IRNode {
    apply(sym(INTEGRATE), vec![expr, var])
}

fn subst_ir(node: &IRNode, var: &IRNode, replacement: &IRNode) -> IRNode {
    if node == var {
        return replacement.clone();
    }
    match node {
        IRNode::Apply(app) => apply(
            app.head.clone(),
            app.args
                .iter()
                .map(|arg| subst_ir(arg, var, replacement))
                .collect(),
        ),
        _ => node.clone(),
    }
}

fn subst_ratio_ir(node: &IRNode, y: &IRNode, x: &IRNode, v: &IRNode) -> Option<IRNode> {
    if node == y {
        return None;
    }
    if binary_args(node, DIV).is_some_and(|(a, b)| a == y && b == x) {
        return Some(v.clone());
    }
    match node {
        IRNode::Apply(app) => Some(apply(
            app.head.clone(),
            app.args
                .iter()
                .map(|arg| subst_ratio_ir(arg, y, x, v))
                .collect::<Option<Vec<_>>>()?,
        )),
        _ => Some(node.clone()),
    }
}

fn signed_frac_to_ir(f: Frac) -> IRNode {
    f.to_ir()
}

fn flatten_add(node: &IRNode) -> Vec<IRNode> {
    if let Some((a, b)) = binary_args(node, ADD) {
        let mut out = flatten_add(a);
        out.extend(flatten_add(b));
        return out;
    }
    if let Some((a, b)) = binary_args(node, SUB) {
        let mut out = flatten_add(a);
        out.push(neg(b.clone()));
        return out;
    }
    if let Some(inner) = unary_arg(node, NEG) {
        return flatten_add(inner).into_iter().map(neg).collect();
    }
    vec![node.clone()]
}

fn flatten_product(node: &IRNode) -> (Frac, Vec<IRNode>) {
    if let Some((a, b)) = binary_args(node, MUL) {
        let (ak, mut av) = flatten_product(a);
        let (bk, bv) = flatten_product(b);
        av.extend(bv);
        return (ak * bk, av);
    }
    if let Some(inner) = unary_arg(node, NEG) {
        let (k, v) = flatten_product(inner);
        return (-k, v);
    }
    if let Some(f) = rational_value(node) {
        return (f, vec![]);
    }
    (Frac::ONE, vec![node.clone()])
}

fn rational_value(node: &IRNode) -> Option<Frac> {
    match node {
        IRNode::Integer(n) => Some(Frac::int(*n)),
        IRNode::Rational(n, d) => Some(Frac::new(*n, *d)),
        _ => None,
    }
}

fn is_const_wrt(node: &IRNode, var: &IRNode) -> bool {
    match node {
        IRNode::Symbol(_) => node != var,
        IRNode::Integer(_) | IRNode::Rational(_, _) | IRNode::Float(_) | IRNode::Str(_) => true,
        IRNode::Apply(app) => app.args.iter().all(|arg| is_const_wrt(arg, var)),
    }
}

fn unwrap_neg(node: &IRNode) -> (bool, IRNode) {
    if let Some(inner) = unary_arg(node, NEG) {
        (true, inner.clone())
    } else if let IRNode::Integer(n) = node {
        if *n < 0 {
            (true, int(-*n))
        } else {
            (false, node.clone())
        }
    } else {
        (false, node.clone())
    }
}

fn coeff_base(term: &IRNode) -> Option<(Frac, IRNode)> {
    let (k, factors) = flatten_product(term);
    match factors.len() {
        0 => Some((k, one())),
        1 => Some((k, factors[0].clone())),
        _ => None,
    }
}

#[cfg(test)]
fn contains_symbol(node: &IRNode, needle: &IRNode) -> bool {
    if node == needle {
        return true;
    }
    matches!(node, IRNode::Apply(app) if app.args.iter().any(|arg| contains_symbol(arg, needle)))
}

fn sum_terms(terms: Vec<(IRNode, bool)>) -> IRNode {
    terms.into_iter().fold(zero(), |acc, (node, is_neg)| {
        add(acc, if is_neg { neg(node) } else { node })
    })
}

fn y_prime(y: &IRNode, x: &IRNode) -> IRNode {
    deriv(y.clone(), x.clone())
}

fn y_double(y: &IRNode, x: &IRNode) -> IRNode {
    deriv(y_prime(y, x), x.clone())
}

fn collect_second_order_coeffs(
    expr: &IRNode,
    y: &IRNode,
    x: &IRNode,
) -> Option<(Frac, Frac, Frac)> {
    let yp = y_prime(y, x);
    let yd = y_double(y, x);
    let mut a = Frac::ZERO;
    let mut b = Frac::ZERO;
    let mut ccoef = Frac::ZERO;
    let mut matched = 0;

    for term in flatten_add(expr) {
        let (k, base) = coeff_base(&term)?;
        if base == yd {
            a = a + k;
            matched += 1;
        } else if base == yp {
            b = b + k;
            matched += 1;
        } else if base == *y {
            ccoef = ccoef + k;
            matched += 1;
        } else {
            return None;
        }
    }
    if a.is_zero() || matched < 2 {
        None
    } else {
        Some((a, b, ccoef))
    }
}

fn exact_sqrt_frac(f: Frac) -> Option<Frac> {
    if f.n < 0 {
        return None;
    }
    let rn = exact_sqrt_i64(f.n)?;
    let rd = exact_sqrt_i64(f.d)?;
    Some(Frac::new(rn, rd))
}

fn exact_sqrt_i64(n: i64) -> Option<i64> {
    if n < 0 {
        return None;
    }
    let root = (n as f64).sqrt() as i64;
    for cand in [root - 1, root, root + 1] {
        if cand >= 0 && cand * cand == n {
            return Some(cand);
        }
    }
    None
}

fn exp_r(r: Frac, x: &IRNode) -> IRNode {
    if r.is_zero() {
        one()
    } else if r == Frac::ONE {
        exp(x.clone())
    } else if r == -Frac::ONE {
        exp(neg(x.clone()))
    } else {
        exp(mul(signed_frac_to_ir(r), x.clone()))
    }
}

pub fn solve_second_order_const_coeff(
    a: IRNode,
    b: IRNode,
    ccoef: IRNode,
    y: IRNode,
    x: IRNode,
) -> Option<IRNode> {
    Some(solve_second_order_const_coeff_frac(
        rational_value(&a)?,
        rational_value(&b)?,
        rational_value(&ccoef)?,
        &y,
        &x,
    ))
}

fn solve_second_order_const_coeff_frac(
    a: Frac,
    b: Frac,
    ccoef: Frac,
    y: &IRNode,
    x: &IRNode,
) -> IRNode {
    let disc = b * b - Frac::int(4) * a * ccoef;
    let solution = if disc > Frac::ZERO {
        if let Some(sqrt_disc) = exact_sqrt_frac(disc) {
            let r1 = (-b + sqrt_disc) / (Frac::int(2) * a);
            let r2 = (-b - sqrt_disc) / (Frac::int(2) * a);
            add(mul(c1(), exp_r(r1, x)), mul(c2(), exp_r(r2, x)))
        } else {
            let sqrt_disc = pow(disc.to_ir_unsigned(), rat(1, 2));
            let denom = (Frac::int(2) * a).to_ir();
            let r1 = div(add((-b).to_ir(), sqrt_disc.clone()), denom.clone());
            let r2 = div(sub((-b).to_ir(), sqrt_disc), denom);
            add(
                mul(c1(), exp(mul(r1, x.clone()))),
                mul(c2(), exp(mul(r2, x.clone()))),
            )
        }
    } else if disc == Frac::ZERO {
        let r = (-b) / (Frac::int(2) * a);
        mul(add(c1(), mul(c2(), x.clone())), exp_r(r, x))
    } else {
        let alpha = (-b) / (Frac::int(2) * a);
        let beta_sq = (-disc) / (Frac::int(4) * a * a);
        let beta = exact_sqrt_frac(beta_sq)
            .map(Frac::to_ir)
            .unwrap_or_else(|| pow(beta_sq.to_ir_unsigned(), rat(1, 2)));
        let beta_x = mul(beta, x.clone());
        mul(
            exp_r(alpha, x),
            add(mul(c1(), cos(beta_x.clone())), mul(c2(), sin(beta_x))),
        )
    };
    equal(y.clone(), solution)
}

fn collect_euler_cauchy_coeffs(
    expr: &IRNode,
    y: &IRNode,
    x: &IRNode,
) -> Option<(Frac, Frac, Frac)> {
    let yp = y_prime(y, x);
    let yd = y_double(y, x);
    let x_sq = pow(x.clone(), two());
    let mut a = Frac::ZERO;
    let mut b = Frac::ZERO;
    let mut ccoef = Frac::ZERO;
    let mut matched = 0;

    for term in flatten_add(expr) {
        let (k, factors) = flatten_product(&term);
        match factors.as_slice() {
            [base] if *base == *y => ccoef = ccoef + k,
            [f1, f2] if (*f1 == x_sq && *f2 == yd) || (*f1 == yd && *f2 == x_sq) => a = a + k,
            [f1, f2] if (*f1 == *x && *f2 == yp) || (*f1 == yp && *f2 == *x) => b = b + k,
            _ => return None,
        }
        matched += 1;
    }
    if a.is_zero() || matched < 2 {
        None
    } else {
        Some((a, b, ccoef))
    }
}

fn x_pow(x: &IRNode, r: Frac) -> IRNode {
    if r.is_zero() {
        one()
    } else if r == Frac::ONE {
        x.clone()
    } else {
        pow(x.clone(), r.to_ir())
    }
}

fn solve_euler_cauchy_frac(a: Frac, b: Frac, ccoef: Frac, y: &IRNode, x: &IRNode) -> IRNode {
    let b_ind = b - a;
    let disc = b_ind * b_ind - Frac::int(4) * a * ccoef;
    let log_x = log(x.clone());
    let solution = if disc > Frac::ZERO {
        if let Some(sqrt_disc) = exact_sqrt_frac(disc) {
            let r1 = (-b_ind + sqrt_disc) / (Frac::int(2) * a);
            let r2 = (-b_ind - sqrt_disc) / (Frac::int(2) * a);
            add(mul(c1(), x_pow(x, r1)), mul(c2(), x_pow(x, r2)))
        } else {
            let sqrt_disc = pow(disc.to_ir_unsigned(), rat(1, 2));
            let denom = (Frac::int(2) * a).to_ir();
            let r1 = div(add((-b_ind).to_ir(), sqrt_disc.clone()), denom.clone());
            let r2 = div(sub((-b_ind).to_ir(), sqrt_disc), denom);
            add(mul(c1(), pow(x.clone(), r1)), mul(c2(), pow(x.clone(), r2)))
        }
    } else if disc == Frac::ZERO {
        let r = (-b_ind) / (Frac::int(2) * a);
        mul(add(c1(), mul(c2(), log_x)), x_pow(x, r))
    } else {
        let alpha = (-b_ind) / (Frac::int(2) * a);
        let beta_sq = (-disc) / (Frac::int(4) * a * a);
        let beta = exact_sqrt_frac(beta_sq)
            .map(Frac::to_ir)
            .unwrap_or_else(|| pow(beta_sq.to_ir_unsigned(), rat(1, 2)));
        let arg = mul(beta, log_x);
        mul(
            x_pow(x, alpha),
            add(mul(c1(), cos(arg.clone())), mul(c2(), sin(arg))),
        )
    };
    equal(y.clone(), solution)
}

fn collect_linear_first_order(expr: &IRNode, y: &IRNode, x: &IRNode) -> Option<(IRNode, IRNode)> {
    let yp = y_prime(y, x);
    let mut yp_coeff = Frac::ZERO;
    let mut p_terms = vec![];
    let mut q_terms = vec![];

    for term in flatten_add(expr) {
        let (is_neg, core) = unwrap_neg(&term);
        if core == yp {
            yp_coeff = yp_coeff + if is_neg { -Frac::ONE } else { Frac::ONE };
            continue;
        }

        let (k, factors) = flatten_product(&core);
        if factors.len() == 1 && factors[0] == yp {
            yp_coeff = yp_coeff + if is_neg { -k } else { k };
            continue;
        }

        if factors.iter().any(|f| f == y) {
            if factors.iter().filter(|f| *f == y).count() != 1 {
                return None;
            }
            let mut coeff = k.to_ir();
            for f in factors.into_iter().filter(|f| f != y) {
                coeff = mul(coeff, f);
            }
            if !is_const_wrt(&coeff, y) {
                return None;
            }
            p_terms.push((coeff, is_neg));
        } else if is_const_wrt(&core, y) {
            q_terms.push((core, !is_neg));
        } else {
            return None;
        }
    }

    if yp_coeff.is_zero() {
        return None;
    }

    let mut p = sum_terms(p_terms);
    let mut q = sum_terms(q_terms);
    if yp_coeff != Frac::ONE {
        p = div(p, yp_coeff.to_ir());
        q = div(q, yp_coeff.to_ir());
    }
    Some((p, q))
}

fn solve_linear_first_order(p: IRNode, q: IRNode, y: &IRNode, x: &IRNode) -> IRNode {
    let int_p = integrate(p, x.clone());
    let mu = exp(int_p);
    let int_mu_q = integrate(mul(mu.clone(), q), x.clone());
    let solution = div(add(int_mu_q, c()), mu);
    equal(y.clone(), solution)
}

fn rhs_from_y_prime_zero_form(expr: &IRNode, y: &IRNode, x: &IRNode) -> Option<IRNode> {
    let yp = y_prime(y, x);
    let mut found = false;
    let mut rhs_terms = vec![];
    for term in flatten_add(expr) {
        let (is_neg, core) = unwrap_neg(&term);
        if core == yp {
            if is_neg {
                return None;
            }
            found = true;
        } else {
            rhs_terms.push(neg(term));
        }
    }
    if !found {
        return None;
    }
    Some(rhs_terms.into_iter().fold(zero(), add))
}

fn try_separable(expr: &IRNode, y: &IRNode, x: &IRNode) -> Option<IRNode> {
    let rhs = rhs_from_y_prime_zero_form(expr, y, x)?;
    if is_const_wrt(&rhs, y) {
        return Some(equal(y.clone(), add(integrate(rhs, x.clone()), c())));
    }
    if is_const_wrt(&rhs, x) {
        if let Some((k, base)) = coeff_base(&rhs) {
            if base == *y {
                return Some(solve_linear_first_order((-k).to_ir(), zero(), y, x));
            }
        }
        return Some(equal(
            integrate(div(one(), rhs), y.clone()),
            add(x.clone(), c()),
        ));
    }
    if let Some((a, b)) = binary_args(&rhs, MUL) {
        for (fx, gy) in [(a, b), (b, a)] {
            if is_const_wrt(fx, y) && is_const_wrt(gy, x) {
                return Some(equal(
                    integrate(div(one(), gy.clone()), y.clone()),
                    add(integrate(fx.clone(), x.clone()), c()),
                ));
            }
        }
    }
    None
}

fn try_homogeneous_type(expr: &IRNode, y: &IRNode, x: &IRNode) -> Option<IRNode> {
    let rhs = rhs_from_y_prime_zero_form(expr, y, x)?;
    if is_const_wrt(&rhs, y) {
        return None;
    }

    let v = sym("_hom_v");
    let f_v = fold_numeric(subst_ratio_ir(&rhs, y, x, &v)?);
    if !is_const_wrt(&f_v, x) {
        return None;
    }

    if f_v == v {
        return Some(equal(y.clone(), mul(c(), x.clone())));
    }

    let denom_v = if let Some(alpha) = extract_linear_coeff(&f_v, &v) {
        mul((alpha - Frac::ONE).to_ir(), v.clone())
    } else {
        fold_numeric(sub(f_v, v.clone()))
    };
    let integrand_v = fold_numeric(div(one(), denom_v));
    let h_v = integrate_basic(&integrand_v, &v);
    let y_over_x = div(y.clone(), x.clone());
    let h_yx = subst_ir(&h_v, &v, &y_over_x);
    Some(equal(h_yx, add(log(x.clone()), c())))
}

fn pow_y_exponent(node: &IRNode, y: &IRNode) -> Option<i64> {
    let (base, exp) = binary_args(node, POW)?;
    if base == y {
        if let IRNode::Integer(n) = exp {
            if *n != 0 && *n != 1 {
                return Some(*n);
            }
        }
    }
    None
}

fn try_bernoulli(expr: &IRNode, y: &IRNode, x: &IRNode) -> Option<IRNode> {
    let yp = y_prime(y, x);
    let mut yp_seen = false;
    let mut p_terms = vec![];
    let mut q_terms = vec![];
    let mut power = None;

    for term in flatten_add(expr) {
        let (is_neg, core) = unwrap_neg(&term);
        if core == yp {
            if is_neg {
                return None;
            }
            yp_seen = true;
            continue;
        }

        let (k, factors) = flatten_product(&core);
        if factors.len() == 1 && factors[0] == y_prime(y, x) {
            if k != Frac::ONE || is_neg {
                return None;
            }
            yp_seen = true;
            continue;
        }

        let mut y_pow = None;
        let mut rest = vec![];
        for factor in factors {
            if let Some(n) = pow_y_exponent(&factor, y) {
                y_pow = Some(n);
            } else {
                rest.push(factor);
            }
        }
        if let Some(n) = y_pow {
            if power.is_some_and(|seen| seen != n) {
                return None;
            }
            power = Some(n);
            let coeff = rest.into_iter().fold(k.to_ir(), mul);
            if !is_const_wrt(&coeff, y) {
                return None;
            }
            q_terms.push((coeff, !is_neg));
            continue;
        }

        let (k2, factors2) = flatten_product(&core);
        if factors2.iter().filter(|f| *f == y).count() == 1 {
            let coeff = factors2
                .into_iter()
                .filter(|f| f != y)
                .fold(k2.to_ir(), mul);
            if !is_const_wrt(&coeff, y) {
                return None;
            }
            p_terms.push((coeff, is_neg));
        } else {
            return None;
        }
    }

    let n = power?;
    if !yp_seen {
        return None;
    }
    let one_minus_n = Frac::int(1 - n);
    let new_p = mul(one_minus_n.to_ir(), sum_terms(p_terms));
    let new_q = mul(one_minus_n.to_ir(), sum_terms(q_terms));
    let v_mu = exp(integrate(new_p.clone(), x.clone()));
    let v_sol = div(
        add(integrate(mul(v_mu.clone(), new_q), x.clone()), c()),
        v_mu,
    );
    Some(equal(
        y.clone(),
        pow(v_sol, (Frac::ONE / one_minus_n).to_ir()),
    ))
}

fn derivative(node: &IRNode, var: &IRNode) -> IRNode {
    if node == var {
        return one();
    }
    match node {
        IRNode::Integer(_) | IRNode::Rational(_, _) | IRNode::Float(_) | IRNode::Str(_) => zero(),
        IRNode::Symbol(_) => zero(),
        IRNode::Apply(_) => {
            if let Some((a, b)) = binary_args(node, ADD) {
                add(derivative(a, var), derivative(b, var))
            } else if let Some((a, b)) = binary_args(node, SUB) {
                sub(derivative(a, var), derivative(b, var))
            } else if let Some(inner) = unary_arg(node, NEG) {
                neg(derivative(inner, var))
            } else if let Some((a, b)) = binary_args(node, MUL) {
                add(
                    mul(derivative(a, var), b.clone()),
                    mul(a.clone(), derivative(b, var)),
                )
            } else if let Some((a, b)) = binary_args(node, DIV) {
                div(
                    sub(
                        mul(derivative(a, var), b.clone()),
                        mul(a.clone(), derivative(b, var)),
                    ),
                    pow(b.clone(), two()),
                )
            } else if let Some((base, exponent)) = binary_args(node, POW) {
                if let Some(n) = rational_value(exponent) {
                    let new_exp = n - Frac::ONE;
                    mul(
                        mul(n.to_ir(), pow(base.clone(), new_exp.to_ir())),
                        derivative(base, var),
                    )
                } else {
                    deriv(node.clone(), var.clone())
                }
            } else if let Some(arg) = unary_arg(node, EXP) {
                mul(exp(arg.clone()), derivative(arg, var))
            } else if let Some(arg) = unary_arg(node, SIN) {
                mul(cos(arg.clone()), derivative(arg, var))
            } else if let Some(arg) = unary_arg(node, COS) {
                neg(mul(sin(arg.clone()), derivative(arg, var)))
            } else if let Some(arg) = unary_arg(node, LOG) {
                div(derivative(arg, var), arg.clone())
            } else {
                deriv(node.clone(), var.clone())
            }
        }
    }
}

fn integrate_basic(node: &IRNode, var: &IRNode) -> IRNode {
    if is_const_wrt(node, var) {
        return mul(node.clone(), var.clone());
    }
    if node == var {
        return div(pow(var.clone(), two()), two());
    }
    if let Some((a, b)) = binary_args(node, ADD) {
        return add(integrate_basic(a, var), integrate_basic(b, var));
    }
    if let Some((a, b)) = binary_args(node, SUB) {
        return sub(integrate_basic(a, var), integrate_basic(b, var));
    }
    if let Some(inner) = unary_arg(node, NEG) {
        return neg(integrate_basic(inner, var));
    }
    if let Some((base, exponent)) = binary_args(node, POW) {
        if base == var {
            if let Some(n) = rational_value(exponent) {
                if n != -Frac::ONE {
                    let next = n + Frac::ONE;
                    return div(pow(var.clone(), next.to_ir()), next.to_ir());
                }
            }
        }
    }
    if let Some((k, factors)) = {
        let (k, factors) = flatten_product(node);
        if factors.len() >= 2 {
            Some((k, factors))
        } else {
            None
        }
    } {
        let mut const_part = k.to_ir();
        let mut var_part = one();
        for f in factors {
            if is_const_wrt(&f, var) {
                const_part = mul(const_part, f);
            } else {
                var_part = mul(var_part, f);
            }
        }
        if !is_int(&const_part, 1) {
            return mul(const_part, integrate_basic(&var_part, var));
        }
    }
    if let Some(arg) = unary_arg(node, EXP) {
        if let Some(alpha) = extract_linear_coeff(arg, var) {
            return div(exp(arg.clone()), alpha.to_ir());
        }
    }
    if let Some(arg) = unary_arg(node, SIN) {
        if let Some(alpha) = extract_linear_coeff(arg, var) {
            return div(neg(cos(arg.clone())), alpha.to_ir());
        }
    }
    if let Some(arg) = unary_arg(node, COS) {
        if let Some(alpha) = extract_linear_coeff(arg, var) {
            return div(sin(arg.clone()), alpha.to_ir());
        }
    }
    if let Some((a, b)) = binary_args(node, DIV) {
        if is_const_wrt(a, var) {
            if let Some(alpha) = extract_linear_coeff(b, var) {
                return mul(div(a.clone(), alpha.to_ir()), log(var.clone()));
            }
        }
        if is_int(a, 1) && b == var {
            return log(var.clone());
        }
    }
    integrate(node.clone(), var.clone())
}

fn fold_numeric(node: IRNode) -> IRNode {
    match node {
        IRNode::Apply(app) => {
            let args: Vec<_> = app.args.into_iter().map(fold_numeric).collect();
            if app.head == sym(ADD) && args.len() == 2 {
                if let (Some(a), Some(b)) = (rational_value(&args[0]), rational_value(&args[1])) {
                    return (a + b).to_ir();
                }
                return add(args[0].clone(), args[1].clone());
            }
            if app.head == sym(SUB) && args.len() == 2 {
                if let (Some(a), Some(b)) = (rational_value(&args[0]), rational_value(&args[1])) {
                    return (a - b).to_ir();
                }
                return sub(args[0].clone(), args[1].clone());
            }
            if app.head == sym(MUL) && args.len() == 2 {
                if let (Some(a), Some(b)) = (rational_value(&args[0]), rational_value(&args[1])) {
                    return (a * b).to_ir();
                }
                return mul(args[0].clone(), args[1].clone());
            }
            if app.head == sym(DIV) && args.len() == 2 {
                if let (Some(a), Some(b)) = (rational_value(&args[0]), rational_value(&args[1])) {
                    return (a / b).to_ir();
                }
                return div(args[0].clone(), args[1].clone());
            }
            IRNode::Apply(Box::new(symbolic_ir::IRApply {
                head: app.head,
                args,
            }))
        }
        other => other,
    }
}

fn try_exact(expr: &IRNode, y: &IRNode, x: &IRNode) -> Option<IRNode> {
    let yp = y_prime(y, x);
    let mut m_parts = vec![];
    let mut n_parts = vec![];
    for term in flatten_add(expr) {
        let (is_neg, core) = unwrap_neg(&term);
        let (k, factors) = flatten_product(&core);
        if factors.iter().any(|f| f == &yp) {
            let coeff = factors
                .into_iter()
                .filter(|f| f != &yp)
                .fold(k.to_ir(), mul);
            n_parts.push((coeff, is_neg));
        } else {
            m_parts.push((core, is_neg));
        }
    }
    if n_parts.is_empty() {
        return None;
    }
    let m_expr = sum_terms(m_parts);
    let n_expr = sum_terms(n_parts);
    if fold_numeric(derivative(&m_expr, y)) != fold_numeric(derivative(&n_expr, x)) {
        return None;
    }
    let f = integrate_basic(&m_expr, x);
    let g_prime = fold_numeric(sub(n_expr, derivative(&f, y)));
    let g = integrate_basic(&g_prime, y);
    Some(equal(fold_numeric(add(f, g)), c()))
}

fn collect_second_order_nonhom(
    expr: &IRNode,
    y: &IRNode,
    x: &IRNode,
) -> Option<(Frac, Frac, Frac, IRNode)> {
    let yp = y_prime(y, x);
    let yd = y_double(y, x);
    let mut a = Frac::ZERO;
    let mut b = Frac::ZERO;
    let mut ccoef = Frac::ZERO;
    let mut forcing = vec![];

    for term in flatten_add(expr) {
        let (k, base) = coeff_base(&term)?;
        if base == yd {
            a = a + k;
        } else if base == yp {
            b = b + k;
        } else if base == *y {
            ccoef = ccoef + k;
        } else if is_const_wrt(&term, y) {
            forcing.push(neg(term));
        } else {
            return None;
        }
    }
    if a.is_zero() || forcing.is_empty() {
        None
    } else {
        Some((a, b, ccoef, forcing.into_iter().fold(zero(), add)))
    }
}

fn extract_linear_coeff(arg: &IRNode, x: &IRNode) -> Option<Frac> {
    if arg == x {
        return Some(Frac::ONE);
    }
    if let Some(inner) = unary_arg(arg, NEG) {
        return extract_linear_coeff(inner, x).map(NegOp::neg);
    }
    let (k, factors) = flatten_product(arg);
    if factors.len() == 1 && factors[0] == *x {
        return Some(k);
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Forcing {
    Const(Frac),
    Poly(Vec<Frac>),
    Exp(Frac),
    Sin(Frac),
    Cos(Frac),
    ExpSin(Frac, Frac),
    ExpCos(Frac, Frac),
}

fn polynomial_forcing(f: &IRNode, x: &IRNode) -> Option<Vec<Frac>> {
    let mut coeffs = [Frac::ZERO; 3];
    let mut seen = false;
    for term in flatten_add(f) {
        let (is_neg, core) = unwrap_neg(&term);
        let sign = if is_neg { -Frac::ONE } else { Frac::ONE };
        let (k, factors) = flatten_product(&core);
        let degree = match factors.as_slice() {
            [] => 0,
            [base] if base == x => 1,
            [base] if binary_args(base, POW).is_some_and(|(b, e)| b == x && is_int(e, 2)) => 2,
            _ => return None,
        };
        coeffs[degree] = coeffs[degree] + sign * k;
        seen = true;
    }
    if !seen {
        return None;
    }
    let max = coeffs.iter().rposition(|c| !c.is_zero()).unwrap_or(0);
    Some(coeffs[..=max].to_vec())
}

fn classify_forcing(f: &IRNode, x: &IRNode) -> Option<Forcing> {
    if is_const_wrt(f, x) {
        return rational_value(f).map(Forcing::Const);
    }
    if let Some(arg) = unary_arg(f, EXP) {
        return extract_linear_coeff(arg, x).map(Forcing::Exp);
    }
    if let Some(arg) = unary_arg(f, SIN) {
        return extract_linear_coeff(arg, x)
            .filter(|b| *b > Frac::ZERO)
            .map(Forcing::Sin);
    }
    if let Some(arg) = unary_arg(f, COS) {
        return extract_linear_coeff(arg, x)
            .filter(|b| *b > Frac::ZERO)
            .map(Forcing::Cos);
    }
    if let Some((left, right)) = binary_args(f, MUL) {
        for (exp_part, trig_part) in [(left, right), (right, left)] {
            if let Some(exp_arg) = unary_arg(exp_part, EXP) {
                let alpha = extract_linear_coeff(exp_arg, x)?;
                if let Some(trig_arg) = unary_arg(trig_part, SIN) {
                    let beta = extract_linear_coeff(trig_arg, x)?;
                    if beta > Frac::ZERO {
                        return Some(Forcing::ExpSin(alpha, beta));
                    }
                }
                if let Some(trig_arg) = unary_arg(trig_part, COS) {
                    let beta = extract_linear_coeff(trig_arg, x)?;
                    if beta > Frac::ZERO {
                        return Some(Forcing::ExpCos(alpha, beta));
                    }
                }
            }
        }
    }
    polynomial_forcing(f, x).map(Forcing::Poly)
}

fn char_poly_at(a: Frac, b: Frac, ccoef: Frac, r: Frac) -> Frac {
    a * r * r + b * r + ccoef
}

fn compute_particular(
    a: Frac,
    b: Frac,
    ccoef: Frac,
    forcing: Forcing,
    x: &IRNode,
) -> Option<IRNode> {
    match forcing {
        Forcing::Const(k) => {
            if !ccoef.is_zero() {
                Some((k / ccoef).to_ir())
            } else if !b.is_zero() {
                Some(mul((k / b).to_ir(), x.clone()))
            } else {
                Some(mul((k / (Frac::int(2) * a)).to_ir(), pow(x.clone(), two())))
            }
        }
        Forcing::Poly(coeffs) if coeffs.len() == 1 => {
            compute_particular(a, b, ccoef, Forcing::Const(coeffs[0]), x)
        }
        Forcing::Poly(coeffs) if coeffs.len() == 2 && !ccoef.is_zero() => {
            let k0 = coeffs[0];
            let k1 = coeffs[1];
            let a1 = k1 / ccoef;
            let a0 = (k0 - b * a1) / ccoef;
            Some(add(a0.to_ir(), mul(a1.to_ir(), x.clone())))
        }
        Forcing::Poly(coeffs) if coeffs.len() == 3 && !ccoef.is_zero() => {
            let k0 = coeffs[0];
            let k1 = coeffs[1];
            let k2 = coeffs[2];
            let a2 = k2 / ccoef;
            let a1 = (k1 - Frac::int(2) * b * a2) / ccoef;
            let a0 = (k0 - Frac::int(2) * a * a2 - b * a1) / ccoef;
            Some(add(
                add(a0.to_ir(), mul(a1.to_ir(), x.clone())),
                mul(a2.to_ir(), pow(x.clone(), two())),
            ))
        }
        Forcing::Poly(_) => None,
        Forcing::Exp(alpha) => {
            let char_val = char_poly_at(a, b, ccoef, alpha);
            let exp_part = exp(mul(alpha.to_ir(), x.clone()));
            if !char_val.is_zero() {
                Some(mul((Frac::ONE / char_val).to_ir(), exp_part))
            } else {
                let char_prime = Frac::int(2) * a * alpha + b;
                if !char_prime.is_zero() {
                    Some(mul(
                        mul((Frac::ONE / char_prime).to_ir(), x.clone()),
                        exp_part,
                    ))
                } else {
                    Some(mul(
                        mul(
                            (Frac::ONE / (Frac::int(2) * a)).to_ir(),
                            pow(x.clone(), two()),
                        ),
                        exp_part,
                    ))
                }
            }
        }
        Forcing::Sin(beta) | Forcing::Cos(beta) => {
            let is_cos = matches!(forcing, Forcing::Cos(_));
            let p = ccoef - a * beta * beta;
            let q = b * beta;
            let det = p * p + q * q;
            if det.is_zero() {
                return None;
            }
            let (acos, bsin) = if is_cos {
                (p / det, q / det)
            } else {
                (-q / det, p / det)
            };
            let arg = mul(beta.to_ir(), x.clone());
            Some(add(
                mul(acos.to_ir(), cos(arg.clone())),
                mul(bsin.to_ir(), sin(arg)),
            ))
        }
        Forcing::ExpSin(alpha, beta) | Forcing::ExpCos(alpha, beta) => {
            let is_cos = matches!(forcing, Forcing::ExpCos(_, _));
            let b_eff = Frac::int(2) * a * alpha + b;
            let c_eff = char_poly_at(a, b, ccoef, alpha);
            let p = c_eff - a * beta * beta;
            let q = b_eff * beta;
            let det = p * p + q * q;
            if det.is_zero() {
                return None;
            }
            let (acos, bsin) = if is_cos {
                (p / det, q / det)
            } else {
                (-q / det, p / det)
            };
            let arg = mul(beta.to_ir(), x.clone());
            let trig = add(
                mul(acos.to_ir(), cos(arg.clone())),
                mul(bsin.to_ir(), sin(arg)),
            );
            Some(mul(exp(mul(alpha.to_ir(), x.clone())), trig))
        }
    }
}

fn try_second_order_nonhom(expr: &IRNode, y: &IRNode, x: &IRNode) -> Option<IRNode> {
    let (a, b, ccoef, f) = collect_second_order_nonhom(expr, y, x)?;
    let forcing = classify_forcing(&f, x)?;
    let yp = compute_particular(a, b, ccoef, forcing, x)?;
    let hom = solve_second_order_const_coeff_frac(a, b, ccoef, y, x);
    let IRNode::Apply(app) = hom else {
        return None;
    };
    let yh = app.args.get(1)?.clone();
    Some(equal(y.clone(), add(yh, yp)))
}

fn vop_integrand_pair(
    a: Frac,
    b: Frac,
    ccoef: Frac,
    f: IRNode,
    x: &IRNode,
) -> Option<(IRNode, IRNode, IRNode, IRNode)> {
    let disc = b * b - Frac::int(4) * a * ccoef;
    if disc > Frac::ZERO {
        let sqrt_disc = exact_sqrt_frac(disc)?;
        let r1 = (-b + sqrt_disc) / (Frac::int(2) * a);
        let r2 = (-b - sqrt_disc) / (Frac::int(2) * a);
        let y1 = exp_r(r1, x);
        let y2 = exp_r(r2, x);
        let u1 = mul(div(mul(f.clone(), exp_r(-r1, x)), (r1 - r2).to_ir()), one());
        let u2 = div(mul(f, exp_r(-r2, x)), (r2 - r1).to_ir());
        Some((u1, u2, y1, y2))
    } else if disc == Frac::ZERO {
        let r = (-b) / (Frac::int(2) * a);
        let e_rx = exp_r(r, x);
        let y1 = e_rx.clone();
        let y2 = mul(x.clone(), e_rx);
        let e_neg = exp_r(-r, x);
        Some((
            neg(mul(mul(x.clone(), e_neg.clone()), f.clone())),
            mul(e_neg, f),
            y1,
            y2,
        ))
    } else {
        let alpha = (-b) / (Frac::int(2) * a);
        let beta_sq = (-disc) / (Frac::int(4) * a * a);
        let beta = exact_sqrt_frac(beta_sq)?;
        let beta_x = mul(beta.to_ir(), x.clone());
        let e_alpha = exp_r(alpha, x);
        let e_neg_alpha = exp_r(-alpha, x);
        let y1 = mul(e_alpha.clone(), cos(beta_x.clone()));
        let y2 = mul(e_alpha, sin(beta_x.clone()));
        let inv_beta = (Frac::ONE / beta).to_ir();
        let u1 = neg(mul(
            mul(
                mul(inv_beta.clone(), e_neg_alpha.clone()),
                sin(beta_x.clone()),
            ),
            f.clone(),
        ));
        let u2 = mul(mul(mul(inv_beta, e_neg_alpha), cos(beta_x)), f);
        Some((u1, u2, y1, y2))
    }
}

fn try_vop(expr: &IRNode, y: &IRNode, x: &IRNode) -> Option<IRNode> {
    let (a, b, ccoef, f) = collect_second_order_nonhom(expr, y, x)?;
    let (u1p, u2p, y1, y2) = vop_integrand_pair(a, b, ccoef, f, x)?;
    let yp = add(
        mul(y1, integrate(u1p, x.clone())),
        mul(y2, integrate(u2p, x.clone())),
    );
    let hom = solve_second_order_const_coeff_frac(a, b, ccoef, y, x);
    let IRNode::Apply(app) = hom else {
        return None;
    };
    let yh = app.args.get(1)?.clone();
    Some(equal(y.clone(), add(yh, yp)))
}

/// Solve a zero-form ODE expression for `y` as a function of `x`.
pub fn solve_ode(expr: IRNode, y: IRNode, x: IRNode) -> Option<IRNode> {
    if let Some(result) = try_second_order_nonhom(&expr, &y, &x) {
        return Some(result);
    }
    if let Some(result) = try_vop(&expr, &y, &x) {
        return Some(result);
    }
    if let Some((a, b, ccoef)) = collect_second_order_coeffs(&expr, &y, &x) {
        return Some(solve_second_order_const_coeff_frac(a, b, ccoef, &y, &x));
    }
    if let Some((a, b, ccoef)) = collect_euler_cauchy_coeffs(&expr, &y, &x) {
        return Some(solve_euler_cauchy_frac(a, b, ccoef, &y, &x));
    }
    if let Some(result) = try_bernoulli(&expr, &y, &x) {
        return Some(result);
    }
    if let Some((p, q)) = collect_linear_first_order(&expr, &y, &x) {
        return Some(solve_linear_first_order(p, q, &y, &x));
    }
    if let Some(result) = try_separable(&expr, &y, &x) {
        return Some(result);
    }
    if let Some(result) = try_homogeneous_type(&expr, &y, &x) {
        return Some(result);
    }
    try_exact(&expr, &y, &x)
}

/// Evaluate an `ODE2(eqn, y, x)` IR node, or return it unchanged on fallthrough.
pub fn ode2_handler(expr: &IRNode) -> IRNode {
    let Some(args) = args_of(expr, ODE2) else {
        return expr.clone();
    };
    if args.len() != 3 {
        return expr.clone();
    }
    let y = args[1].clone();
    let x = args[2].clone();
    if !matches!(y, IRNode::Symbol(_)) || !matches!(x, IRNode::Symbol(_)) {
        return expr.clone();
    }
    let zero_form = if let Some((lhs, rhs)) = binary_args(&args[0], EQUAL) {
        sub(lhs.clone(), rhs.clone())
    } else {
        args[0].clone()
    };
    solve_ode(zero_form, y, x).unwrap_or_else(|| expr.clone())
}

pub fn build_ode_handler_table() -> BTreeMap<&'static str, Handler> {
    BTreeMap::from([(ODE2, ode2_handler as Handler)])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn x() -> IRNode {
        sym("x")
    }

    fn y() -> IRNode {
        sym("y")
    }

    fn yp() -> IRNode {
        deriv(y(), x())
    }

    fn y_over_x() -> IRNode {
        div(y(), x())
    }

    fn yd() -> IRNode {
        deriv(yp(), x())
    }

    fn assert_equal_y(result: IRNode) -> IRNode {
        let IRNode::Apply(app) = result else {
            panic!("expected Equal");
        };
        assert_eq!(app.head, sym(EQUAL));
        assert_eq!(app.args[0], y());
        app.args[1].clone()
    }

    #[test]
    fn handler_table_names_ode2() {
        let table = build_ode_handler_table();
        assert!(table.contains_key(ODE2));

        let ode = apply(sym(ODE2), vec![yp(), y(), x()]);
        assert!(head_is(&table[ODE2](&ode), EQUAL));
    }

    #[test]
    fn first_order_linear_uses_integrating_factor_shape() {
        let expr = add(yp(), mul(int(2), y()));
        let solution = assert_equal_y(solve_ode(expr, y(), x()).unwrap());
        assert!(contains_symbol(&solution, &sym("%c")));
        assert!(head_is(&solution, DIV));
        assert!(format!("{solution}").contains("Integrate(2, x)"));
    }

    #[test]
    fn separable_returns_implicit_integrals() {
        let rhs = mul(x(), sin(y()));
        let expr = sub(yp(), rhs);
        let result = solve_ode(expr, y(), x()).unwrap();
        assert!(format!("{result}").contains("Integrate(Div(1, Sin(y)), y)"));
        assert!(format!("{result}").contains("Integrate(x, x)"));
    }

    #[test]
    fn subst_ratio_replaces_only_exact_y_over_x() {
        let v = sym("v");
        assert_eq!(subst_ratio_ir(&y_over_x(), &y(), &x(), &v), Some(v.clone()));

        let squared = pow(y_over_x(), int(2));
        assert_eq!(
            subst_ratio_ir(&squared, &y(), &x(), &v),
            Some(pow(v, int(2)))
        );
        assert_eq!(subst_ratio_ir(&int(3), &y(), &x(), &sym("v")), Some(int(3)));
    }

    #[test]
    fn subst_ratio_rejects_bare_or_nested_y_outside_ratio() {
        let v = sym("v");
        assert_eq!(subst_ratio_ir(&y(), &y(), &x(), &v), None);
        assert_eq!(subst_ratio_ir(&mul(y(), x()), &y(), &x(), &v), None);
        assert_eq!(
            subst_ratio_ir(&div(add(y(), x()), x()), &y(), &x(), &v),
            None
        );
    }

    #[test]
    fn homogeneous_type_degenerate_y_prime_equals_y_over_x() {
        let expr = sub(yp(), y_over_x());
        let solution = assert_equal_y(solve_ode(expr, y(), x()).unwrap());
        assert_eq!(solution, mul(sym("%c"), x()));
    }

    #[test]
    fn homogeneous_type_ratio_squared_returns_symbolic_implicit_solution() {
        let expr = sub(yp(), pow(y_over_x(), int(2)));
        let result = solve_ode(expr, y(), x()).unwrap();
        assert!(head_is(&result, EQUAL));

        let result_str = format!("{result}");
        assert!(result_str.contains("Integrate"));
        assert!(result_str.contains("Pow(Div(y, x), 2)"));
        assert!(result_str.contains("Log(x)"));
        assert!(result_str.contains("%c"));
    }

    #[test]
    fn homogeneous_type_transcendental_ratio_keeps_symbolic_integral() {
        let expr = sub(yp(), exp(y_over_x()));
        let result = solve_ode(expr, y(), x()).unwrap();
        let result_str = format!("{result}");
        assert!(result_str.contains("Integrate"));
        assert!(result_str.contains("Exp(Div(y, x))"));
        assert!(result_str.contains("Log(x)"));
        assert!(result_str.contains("%c"));
    }

    #[test]
    fn homogeneous_type_ratio_squared_plus_ratio_back_substitutes_ratio() {
        let ratio = y_over_x();
        let rhs = add(pow(ratio.clone(), int(2)), ratio);
        let result = solve_ode(sub(yp(), rhs), y(), x()).unwrap();
        let result_str = format!("{result}");
        assert!(result_str.contains("Integrate"));
        assert!(result_str.contains("Div(y, x)"));
        assert!(result_str.contains("Log(x)"));
    }

    #[test]
    fn homogeneous_type_two_times_ratio_uses_basic_log_primitive() {
        let result = solve_ode(sub(yp(), mul(int(2), y_over_x())), y(), x()).unwrap();
        let result_str = format!("{result}");
        assert!(result_str.contains("Log(Div(y, x))"));
        assert!(result_str.contains("Log(x)"));
        assert!(result_str.contains("%c"));
    }

    #[test]
    fn homogeneous_type_falls_through_for_non_ratio_y_forms() {
        assert!(try_homogeneous_type(&sub(yp(), sin(x())), &y(), &x()).is_none());
        assert!(try_homogeneous_type(&sub(yp(), add(y(), x())), &y(), &x()).is_none());
        assert!(try_homogeneous_type(&sub(yp(), mul(y(), x())), &y(), &x()).is_none());
        assert!(try_homogeneous_type(&add(y(), x()), &y(), &x()).is_none());
    }

    #[test]
    fn ode2_dispatches_homogeneous_type_degenerate_case() {
        let eqn = apply(sym(EQUAL), vec![yp(), y_over_x()]);
        let ode = apply(sym(ODE2), vec![eqn, y(), x()]);
        let solution = assert_equal_y(ode2_handler(&ode));
        assert_eq!(solution, mul(sym("%c"), x()));
    }

    #[test]
    fn bernoulli_returns_power_back_substitution() {
        let expr = add(sub(yp(), y()), pow(y(), int(2)));
        let solution = assert_equal_y(solve_ode(expr, y(), x()).unwrap());
        assert!(head_is(&solution, POW));
        assert!(format!("{solution}").contains("Integrate"));
    }

    #[test]
    fn exact_ode_returns_implicit_potential() {
        let m = add(mul(int(2), mul(x(), y())), int(1));
        let n = add(pow(x(), int(2)), int(1));
        let expr = add(m, mul(n, yp()));
        let result = solve_ode(expr, y(), x()).unwrap();
        assert!(format!("{result}").contains("%c"));
        assert!(format!("{result}").contains("Pow(x, 2)"));
        assert!(format!("{result}").contains("y"));
    }

    #[test]
    fn second_order_homogeneous_distinct_roots() {
        let expr = sub(yd(), y());
        let solution = assert_equal_y(solve_ode(expr, y(), x()).unwrap());
        assert!(format!("{solution}").contains("Exp(x)"));
        assert!(format!("{solution}").contains("Exp(Neg(x))"));
    }

    #[test]
    fn second_order_nonhom_constant_forcing() {
        let expr = sub(add(yd(), y()), int(3));
        let solution = assert_equal_y(solve_ode(expr, y(), x()).unwrap());
        assert!(format!("{solution}").contains("3"));
        assert!(format!("{solution}").contains("%c1"));
    }

    #[test]
    fn second_order_nonhom_vop_fallback_keeps_symbolic_integrals() {
        let forcing = log(x());
        let expr = sub(sub(yd(), y()), forcing);
        let solution = assert_equal_y(solve_ode(expr, y(), x()).unwrap());
        assert!(format!("{solution}").contains("Integrate"));
        assert!(format!("{solution}").contains("Log(x)"));
    }

    #[test]
    fn euler_cauchy_repeated_root() {
        let expr = add(sub(mul(pow(x(), int(2)), yd()), mul(x(), yp())), y());
        let solution = assert_equal_y(solve_ode(expr, y(), x()).unwrap());
        assert!(format!("{solution}").contains("Log(x)"));
    }

    #[test]
    fn ode2_handler_normalizes_equal_and_falls_through() {
        let eqn = apply(sym(EQUAL), vec![yp(), mul(int(2), y())]);
        let ode = apply(sym(ODE2), vec![eqn, y(), x()]);
        assert!(head_is(&ode2_handler(&ode), EQUAL));

        let unsupported = apply(sym(ODE2), vec![apply(sym("Mystery"), vec![x()]), y(), x()]);
        assert_eq!(ode2_handler(&unsupported), unsupported);
    }
}
