//! Shared head handlers for both strict and symbolic backends.
//!
//! Every handler has the signature `fn(&mut VM, IRApply) -> IRNode`.
//! The `simplify` flag controls the behaviour when an operation can't
//! fold numerically:
//!
//! - `simplify = false` (**strict**): `panic!` with a type-error message.
//! - `simplify = true` (**symbolic**): apply identity/zero laws and return
//!   the expression unmodified for anything that can't be reduced further.
//!
//! ## Numeric representation
//!
//! Intermediate arithmetic uses the [`Numeric`] enum, which preserves
//! exactness:
//!
//! - `Int(i64)` — exact integer (checked arithmetic; overflows to Float)
//! - `Rat(i64, i64)` — exact fraction in lowest terms
//! - `Float(f64)` — floating-point (any Float operand poisons the result)
//!
//! [`to_numeric`] converts an `IRNode` to `Numeric` (returns `None` for
//! non-numeric nodes); [`from_numeric`] converts back to `IRNode`,
//! collapsing `Rat(n, 1)` to `Int(n)`.

use std::collections::{HashMap, HashSet};

use cas_factor::factor_integer_polynomial;
use symbolic_ir::{
    IRApply, IRNode, ACOS, ACOSH, ADD, AND, ASIN, ASINH, ASSIGN, ATAN, ATANH, COS, COSH, COTH,
    CSCH, D, DEFINE, DIV, EQUAL, EXP, GREATER, GREATER_EQUAL, IF, INTEGRATE, INV, LESS, LESS_EQUAL,
    LOG, MUL, NEG, NOT, NOT_EQUAL, OR, POW, SECH, SIN, SINH, SQRT, SUB, TAN, TANH,
};

use crate::backend::{handler_fn, Handler};
use crate::vm::VM;

const FACTOR: &str = "Factor";

// ---------------------------------------------------------------------------
// Numeric intermediate value
// ---------------------------------------------------------------------------

/// Exact-or-float intermediate arithmetic type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Numeric {
    Int(i64),
    /// Fraction in lowest terms: sign in numer, denom > 0.
    Rat(i64, i64),
    Float(f64),
}

impl Numeric {
    /// Convert to `f64` for numeric comparison or transcendental functions.
    pub fn to_f64(self) -> f64 {
        match self {
            Numeric::Int(n) => n as f64,
            Numeric::Rat(n, d) => n as f64 / d as f64,
            Numeric::Float(f) => f,
        }
    }

    /// Is this value == 0?
    pub fn is_zero(self) -> bool {
        match self {
            Numeric::Int(n) => n == 0,
            Numeric::Rat(n, _) => n == 0,
            Numeric::Float(f) => f == 0.0,
        }
    }

    /// Is this value == 1?
    pub fn is_one(self) -> bool {
        match self {
            Numeric::Int(n) => n == 1,
            Numeric::Rat(n, d) => n == d,
            Numeric::Float(f) => f == 1.0,
        }
    }
}

/// Build a `Numeric::Rat` in lowest terms, collapsing to `Int` when denom==1.
fn make_rat(numer: i64, denom: i64) -> Numeric {
    debug_assert_ne!(denom, 0);
    let (numer, denom) = if denom < 0 {
        (-numer, -denom)
    } else {
        (numer, denom)
    };
    let g = gcd(numer.unsigned_abs(), denom.unsigned_abs()) as i64;
    let (n, d) = (numer / g, denom / g);
    if d == 1 {
        Numeric::Int(n)
    } else {
        Numeric::Rat(n, d)
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

impl std::ops::Add for Numeric {
    type Output = Numeric;
    fn add(self, rhs: Numeric) -> Numeric {
        match (self, rhs) {
            (Numeric::Int(a), Numeric::Int(b)) => match a.checked_add(b) {
                Some(c) => Numeric::Int(c),
                None => Numeric::Float(a as f64 + b as f64),
            },
            (Numeric::Rat(n1, d1), Numeric::Rat(n2, d2)) => {
                // n1/d1 + n2/d2 = (n1*d2 + n2*d1) / (d1*d2)
                let numer = n1.saturating_mul(d2).saturating_add(n2.saturating_mul(d1));
                let denom = d1.saturating_mul(d2);
                if denom == 0 {
                    Numeric::Float(self.to_f64() + rhs.to_f64())
                } else {
                    make_rat(numer, denom)
                }
            }
            (Numeric::Int(a), Numeric::Rat(n, d)) | (Numeric::Rat(n, d), Numeric::Int(a)) => {
                make_rat(a.saturating_mul(d).saturating_add(n), d)
            }
            _ => Numeric::Float(self.to_f64() + rhs.to_f64()),
        }
    }
}

impl std::ops::Sub for Numeric {
    type Output = Numeric;
    fn sub(self, rhs: Numeric) -> Numeric {
        match (self, rhs) {
            (Numeric::Int(a), Numeric::Int(b)) => match a.checked_sub(b) {
                Some(c) => Numeric::Int(c),
                None => Numeric::Float(a as f64 - b as f64),
            },
            (Numeric::Rat(n1, d1), Numeric::Rat(n2, d2)) => {
                let numer = n1.saturating_mul(d2).saturating_sub(n2.saturating_mul(d1));
                let denom = d1.saturating_mul(d2);
                if denom == 0 {
                    Numeric::Float(self.to_f64() - rhs.to_f64())
                } else {
                    make_rat(numer, denom)
                }
            }
            (Numeric::Int(a), Numeric::Rat(n, d)) => {
                make_rat(a.saturating_mul(d).saturating_sub(n), d)
            }
            (Numeric::Rat(n, d), Numeric::Int(b)) => {
                make_rat(n.saturating_sub(b.saturating_mul(d)), d)
            }
            _ => Numeric::Float(self.to_f64() - rhs.to_f64()),
        }
    }
}

impl std::ops::Mul for Numeric {
    type Output = Numeric;
    fn mul(self, rhs: Numeric) -> Numeric {
        match (self, rhs) {
            (Numeric::Int(a), Numeric::Int(b)) => match a.checked_mul(b) {
                Some(c) => Numeric::Int(c),
                None => Numeric::Float(a as f64 * b as f64),
            },
            (Numeric::Rat(n1, d1), Numeric::Rat(n2, d2)) => {
                make_rat(n1.saturating_mul(n2), d1.saturating_mul(d2))
            }
            (Numeric::Int(a), Numeric::Rat(n, d)) | (Numeric::Rat(n, d), Numeric::Int(a)) => {
                make_rat(a.saturating_mul(n), d)
            }
            _ => Numeric::Float(self.to_f64() * rhs.to_f64()),
        }
    }
}

impl std::ops::Div for Numeric {
    type Output = Numeric;
    fn div(self, rhs: Numeric) -> Numeric {
        // a / b  →  a * (1/b)
        match rhs {
            Numeric::Int(b) => self * make_rat(1, b),
            Numeric::Rat(n, d) => self * make_rat(d, n),
            Numeric::Float(f) => Numeric::Float(self.to_f64() / f),
        }
    }
}

impl std::ops::Neg for Numeric {
    type Output = Numeric;
    fn neg(self) -> Numeric {
        match self {
            Numeric::Int(n) => Numeric::Int(-n),
            Numeric::Rat(n, d) => Numeric::Rat(-n, d),
            Numeric::Float(f) => Numeric::Float(-f),
        }
    }
}

// ---------------------------------------------------------------------------
// to_numeric / from_numeric
// ---------------------------------------------------------------------------

/// Convert an `IRNode` to `Numeric`.  Returns `None` for non-numeric nodes.
pub fn to_numeric(node: &IRNode) -> Option<Numeric> {
    match node {
        IRNode::Integer(n) => Some(Numeric::Int(*n)),
        IRNode::Rational(n, d) => Some(Numeric::Rat(*n, *d)),
        IRNode::Float(f) => Some(Numeric::Float(*f)),
        _ => None,
    }
}

/// Convert a `Numeric` back to the most compact `IRNode` representation.
///
/// - `Int(n)` → `IRNode::Integer(n)`
/// - `Rat(n, 1)` → `IRNode::Integer(n)` (collapsed)
/// - `Rat(n, d)` → `IRNode::Rational(n, d)`
/// - `Float(f)` → `IRNode::Float(f)`
pub fn from_numeric(v: Numeric) -> IRNode {
    match v {
        Numeric::Int(n) => IRNode::Integer(n),
        Numeric::Rat(n, d) => {
            // make_rat already reduced; just map to IRNode
            if d == 1 {
                IRNode::Integer(n)
            } else {
                IRNode::Rational(n, d)
            }
        }
        Numeric::Float(f) => IRNode::Float(f),
    }
}

// ---------------------------------------------------------------------------
// Booleans
// ---------------------------------------------------------------------------

/// The `True` symbol.
pub fn true_sym() -> IRNode {
    IRNode::Symbol("True".to_string())
}

/// The `False` symbol.
pub fn false_sym() -> IRNode {
    IRNode::Symbol("False".to_string())
}

/// Convert a bool to `True`/`False` IR node.
fn bool_node(v: bool) -> IRNode {
    if v {
        true_sym()
    } else {
        false_sym()
    }
}

/// Check if a node is the `True` or `False` symbol.
fn is_truthy(node: &IRNode) -> Option<bool> {
    if let IRNode::Symbol(s) = node {
        if s == "True" {
            return Some(true);
        }
        if s == "False" {
            return Some(false);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Arithmetic handlers
// ---------------------------------------------------------------------------

fn add_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        let (a, b) = match binary_args(&expr) {
            Some(p) => p,
            None => return IRNode::Apply(Box::new(expr)),
        };
        let va = to_numeric(&a);
        let vb = to_numeric(&b);
        if let (Some(va), Some(vb)) = (va, vb) {
            return from_numeric(va + vb);
        }
        if !simplify {
            panic!("Add requires numeric arguments: {expr}");
        }
        // x + 0 → x, 0 + x → x
        if va.map(|v| v.is_zero()).unwrap_or(false) {
            return b;
        }
        if vb.map(|v| v.is_zero()).unwrap_or(false) {
            return a;
        }
        IRNode::Apply(Box::new(expr))
    })
}

fn sub_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        let (a, b) = match binary_args(&expr) {
            Some(p) => p,
            None => return IRNode::Apply(Box::new(expr)),
        };
        let va = to_numeric(&a);
        let vb = to_numeric(&b);
        if let (Some(va), Some(vb)) = (va, vb) {
            return from_numeric(va - vb);
        }
        if !simplify {
            panic!("Sub requires numeric arguments: {expr}");
        }
        // x - 0 → x
        if vb.map(|v| v.is_zero()).unwrap_or(false) {
            return a;
        }
        IRNode::Apply(Box::new(expr))
    })
}

fn mul_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        let (a, b) = match binary_args(&expr) {
            Some(p) => p,
            None => return IRNode::Apply(Box::new(expr)),
        };
        let va = to_numeric(&a);
        let vb = to_numeric(&b);
        if let (Some(va), Some(vb)) = (va, vb) {
            return from_numeric(va * vb);
        }
        if !simplify {
            panic!("Mul requires numeric arguments: {expr}");
        }
        // 0 * x → 0, x * 0 → 0
        if va.map(|v| v.is_zero()).unwrap_or(false) || vb.map(|v| v.is_zero()).unwrap_or(false) {
            return IRNode::Integer(0);
        }
        // 1 * x → x, x * 1 → x
        if va.map(|v| v.is_one()).unwrap_or(false) {
            return b;
        }
        if vb.map(|v| v.is_one()).unwrap_or(false) {
            return a;
        }
        IRNode::Apply(Box::new(expr))
    })
}

fn div_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        let (a, b) = match binary_args(&expr) {
            Some(p) => p,
            None => return IRNode::Apply(Box::new(expr)),
        };
        let va = to_numeric(&a);
        let vb = to_numeric(&b);
        if let (Some(va), Some(vb)) = (va, vb) {
            if vb.is_zero() {
                panic!("division by zero: {expr}");
            }
            return from_numeric(va / vb);
        }
        if !simplify {
            panic!("Div requires numeric arguments: {expr}");
        }
        if va.map(|v| v.is_zero()).unwrap_or(false) {
            return IRNode::Integer(0);
        }
        if vb.map(|v| v.is_one()).unwrap_or(false) {
            return a;
        }
        IRNode::Apply(Box::new(expr))
    })
}

fn pow_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        let (base, exp) = match binary_args(&expr) {
            Some(p) => p,
            None => return IRNode::Apply(Box::new(expr)),
        };
        let vb = to_numeric(&base);
        let ve = to_numeric(&exp);
        if let (Some(vb), Some(ve)) = (vb, ve) {
            return from_numeric(pow_numeric(vb, ve));
        }
        if !simplify {
            panic!("Pow requires numeric arguments: {expr}");
        }
        // x^0 → 1
        if ve.map(|v| v.is_zero()).unwrap_or(false) {
            return IRNode::Integer(1);
        }
        // x^1 → x
        if ve.map(|v| v.is_one()).unwrap_or(false) {
            return base;
        }
        // 0^n → 0 (n ≠ 0 covered above)
        if vb.map(|v| v.is_zero()).unwrap_or(false) {
            return IRNode::Integer(0);
        }
        // 1^n → 1
        if vb.map(|v| v.is_one()).unwrap_or(false) {
            return IRNode::Integer(1);
        }
        IRNode::Apply(Box::new(expr))
    })
}

fn neg_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 1 {
            return IRNode::Apply(Box::new(expr));
        }
        let a = expr.args[0].clone();
        if let Some(va) = to_numeric(&a) {
            return from_numeric(-va);
        }
        if !simplify {
            panic!("Neg requires a numeric argument: {expr}");
        }
        // -(-x) → x
        if let IRNode::Apply(ref inner) = a {
            if let IRNode::Symbol(ref s) = inner.head {
                if s == "Neg" && inner.args.len() == 1 {
                    return inner.args[0].clone();
                }
            }
        }
        IRNode::Apply(Box::new(expr))
    })
}

fn inv_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 1 {
            return IRNode::Apply(Box::new(expr));
        }
        let a = expr.args[0].clone();
        if let Some(va) = to_numeric(&a) {
            if va.is_zero() {
                panic!("inverse of zero: {expr}");
            }
            return from_numeric(make_rat(1, 1) / va);
        }
        if !simplify {
            panic!("Inv requires a numeric argument: {expr}");
        }
        IRNode::Apply(Box::new(expr))
    })
}

/// Raise a numeric base to a numeric exponent, preserving exactness.
///
/// `Rat^Int` stays exact; anything involving Float goes to Float.
fn pow_numeric(base: Numeric, exp: Numeric) -> Numeric {
    // Int^Int (small positive exponent) — stay exact.
    if let (Numeric::Int(b), Numeric::Int(e)) = (base, exp) {
        if e >= 0 && e <= 62 {
            // b^e fits in i64 when |b| <= 1 or e <= ~19 for b==2
            // Use checked_pow to avoid overflow.
            if let Some(result) = (b as i128).checked_pow(e as u32) {
                if result >= i64::MIN as i128 && result <= i64::MAX as i128 {
                    return Numeric::Int(result as i64);
                }
            }
        }
        if e < 0 {
            // b^(-n) = 1/b^n
            if b == 0 {
                panic!("0^negative");
            }
            let pos = pow_numeric(base, Numeric::Int(-e));
            return make_rat(1, 1) / pos;
        }
    }
    // Rat^Int — exact when exponent is a non-negative integer.
    if let (Numeric::Rat(n, d), Numeric::Int(e)) = (base, exp) {
        if e >= 0 && e <= 30 {
            let eu = e as u32;
            if let (Some(nn), Some(dd)) = ((n as i128).checked_pow(eu), (d as i128).checked_pow(eu))
            {
                if nn >= i64::MIN as i128
                    && nn <= i64::MAX as i128
                    && dd >= 1
                    && dd <= i64::MAX as i128
                {
                    return make_rat(nn as i64, dd as i64);
                }
            }
        }
    }
    // Fall back to f64.
    Numeric::Float(base.to_f64().powf(exp.to_f64()))
}

// ---------------------------------------------------------------------------
// Elementary function handlers
// ---------------------------------------------------------------------------

/// Build a handler for a single-argument transcendental function.
///
/// `exact_cases`: a list of `(input_value, output_node)` pairs that are
/// folded before going to the floating-point implementation.  Only
/// integer / rational inputs are checked.
#[allow(dead_code)]
fn elementary_handler(
    name: &'static str,
    f: fn(f64) -> f64,
    exact_cases: &'static [(Numeric, IRNode)],
    simplify: bool,
) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 1 {
            return IRNode::Apply(Box::new(expr));
        }
        let a = expr.args[0].clone();
        if let Some(va) = to_numeric(&a) {
            for (input, output) in exact_cases {
                if va == *input {
                    return output.clone();
                }
            }
            return IRNode::Float(f(va.to_f64()));
        }
        if !simplify {
            panic!("{name} requires a numeric argument: {expr}");
        }
        IRNode::Apply(Box::new(expr))
    })
}

// We can't store `IRNode` in a `&'static` slice, so we pre-compute them
// via lazy_static.  Instead, we hard-code the numeric values and
// reconstruct IRNode::Integer/Float as needed.

fn sin_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(
            &expr,
            "Sin",
            f64::sin,
            &[(Numeric::Int(0), IRNode::Integer(0))],
            simplify,
        )
    })
}

fn cos_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(
            &expr,
            "Cos",
            f64::cos,
            &[(Numeric::Int(0), IRNode::Integer(1))],
            simplify,
        )
    })
}

fn tan_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(
            &expr,
            "Tan",
            f64::tan,
            &[(Numeric::Int(0), IRNode::Integer(0))],
            simplify,
        )
    })
}

fn exp_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(
            &expr,
            "Exp",
            f64::exp,
            &[(Numeric::Int(0), IRNode::Integer(1))],
            simplify,
        )
    })
}

fn log_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(
            &expr,
            "Log",
            f64::ln,
            &[(Numeric::Int(1), IRNode::Integer(0))],
            simplify,
        )
    })
}

fn sqrt_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(
            &expr,
            "Sqrt",
            f64::sqrt,
            &[
                (Numeric::Int(0), IRNode::Integer(0)),
                (Numeric::Int(1), IRNode::Integer(1)),
            ],
            simplify,
        )
    })
}

fn atan_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(
            &expr,
            "Atan",
            f64::atan,
            &[(Numeric::Int(0), IRNode::Integer(0))],
            simplify,
        )
    })
}

fn asin_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(
            &expr,
            "Asin",
            f64::asin,
            &[(Numeric::Int(0), IRNode::Integer(0))],
            simplify,
        )
    })
}

fn acos_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(&expr, "Acos", f64::acos, &[], simplify)
    })
}

fn sinh_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(
            &expr,
            "Sinh",
            f64::sinh,
            &[(Numeric::Int(0), IRNode::Integer(0))],
            simplify,
        )
    })
}

fn cosh_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(
            &expr,
            "Cosh",
            f64::cosh,
            &[(Numeric::Int(0), IRNode::Integer(1))],
            simplify,
        )
    })
}

fn tanh_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(
            &expr,
            "Tanh",
            f64::tanh,
            &[(Numeric::Int(0), IRNode::Integer(0))],
            simplify,
        )
    })
}

fn coth_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_recip_hyp(&expr, COTH, |x| x.cosh() / x.sinh(), true, &[], simplify)
    })
}

fn sech_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_recip_hyp(
            &expr,
            SECH,
            |x| 1.0 / x.cosh(),
            false,
            &[(Numeric::Int(0), IRNode::Integer(1))],
            simplify,
        )
    })
}

fn csch_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_recip_hyp(&expr, CSCH, |x| 1.0 / x.sinh(), true, &[], simplify)
    })
}

fn asinh_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(
            &expr,
            "Asinh",
            f64::asinh,
            &[(Numeric::Int(0), IRNode::Integer(0))],
            simplify,
        )
    })
}

fn acosh_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(
            &expr,
            "Acosh",
            f64::acosh,
            &[(Numeric::Int(1), IRNode::Integer(0))],
            simplify,
        )
    })
}

fn atanh_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(
            &expr,
            "Atanh",
            f64::atanh,
            &[(Numeric::Int(0), IRNode::Integer(0))],
            simplify,
        )
    })
}

/// Common single-argument transcendental handler body.
fn single_trig(
    expr: &IRApply,
    name: &str,
    f: fn(f64) -> f64,
    exact_cases: &[(Numeric, IRNode)],
    simplify: bool,
) -> IRNode {
    if expr.args.len() != 1 {
        return IRNode::Apply(Box::new(expr.clone()));
    }
    let a = &expr.args[0];
    if let Some(va) = to_numeric(a) {
        for (input, output) in exact_cases {
            if va == *input {
                return output.clone();
            }
        }
        return IRNode::Float(f(va.to_f64()));
    }
    if !simplify {
        panic!("{name} requires a numeric argument: {expr}");
    }
    IRNode::Apply(Box::new(expr.clone()))
}

fn single_recip_hyp(
    expr: &IRApply,
    name: &str,
    f: fn(f64) -> f64,
    undefined_at_zero: bool,
    exact_cases: &[(Numeric, IRNode)],
    simplify: bool,
) -> IRNode {
    if expr.args.len() != 1 {
        return IRNode::Apply(Box::new(expr.clone()));
    }
    let a = &expr.args[0];
    if let Some(va) = to_numeric(a) {
        if undefined_at_zero && va.is_zero() {
            panic!("{name} undefined at zero: {expr}");
        }
        for (input, output) in exact_cases {
            if va == *input {
                return output.clone();
            }
        }
        return IRNode::Float(f(va.to_f64()));
    }
    if !simplify {
        panic!("{name} requires a numeric argument: {expr}");
    }
    IRNode::Apply(Box::new(expr.clone()))
}

// ---------------------------------------------------------------------------
// Comparison handlers
// ---------------------------------------------------------------------------

fn comparison_handler(
    op: fn(f64, f64) -> bool,
    eq_based: bool,    // true for Equal/NotEqual (structural check)
    is_equal_op: bool, // true for Equal (not NotEqual)
    simplify: bool,
) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        let (a, b) = match binary_args(&expr) {
            Some(p) => p,
            None => return IRNode::Apply(Box::new(expr)),
        };
        let va = to_numeric(&a);
        let vb = to_numeric(&b);
        if let (Some(va), Some(vb)) = (va, vb) {
            return bool_node(op(va.to_f64(), vb.to_f64()));
        }
        // Structural equality: x == x → True
        if eq_based && a == b {
            return bool_node(is_equal_op);
        }
        if !simplify {
            panic!("comparison requires numeric arguments: {expr}");
        }
        IRNode::Apply(Box::new(expr))
    })
}

// ---------------------------------------------------------------------------
// Logic handlers
// ---------------------------------------------------------------------------

fn and_handler(_simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        let mut remaining: Vec<IRNode> = Vec::new();
        for a in &expr.args {
            match is_truthy(a) {
                Some(false) => return false_sym(),
                Some(true) => {} // identity, skip
                None => remaining.push(a.clone()),
            }
        }
        match remaining.len() {
            0 => true_sym(),
            1 => remaining.remove(0),
            _ => {
                let head = IRNode::Symbol(AND.to_string());
                IRNode::Apply(Box::new(IRApply {
                    head,
                    args: remaining,
                }))
            }
        }
    })
}

fn or_handler(_simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        let mut remaining: Vec<IRNode> = Vec::new();
        for a in &expr.args {
            match is_truthy(a) {
                Some(true) => return true_sym(),
                Some(false) => {}
                None => remaining.push(a.clone()),
            }
        }
        match remaining.len() {
            0 => false_sym(),
            1 => remaining.remove(0),
            _ => {
                let head = IRNode::Symbol(OR.to_string());
                IRNode::Apply(Box::new(IRApply {
                    head,
                    args: remaining,
                }))
            }
        }
    })
}

fn not_handler(_simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 1 {
            return IRNode::Apply(Box::new(expr));
        }
        match is_truthy(&expr.args[0]) {
            Some(true) => false_sym(),
            Some(false) => true_sym(),
            None => IRNode::Apply(Box::new(expr)),
        }
    })
}

// ---------------------------------------------------------------------------
// If handler — held head: args are NOT pre-evaluated
// ---------------------------------------------------------------------------

fn if_handler(_simplify: bool) -> Handler {
    std::sync::Arc::new(move |vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() < 2 || expr.args.len() > 3 {
            panic!("If expects 2 or 3 arguments, got {}", expr.args.len());
        }
        let predicate = vm.eval(expr.args[0].clone());
        match is_truthy(&predicate) {
            Some(true) => vm.eval(expr.args[1].clone()),
            Some(false) => {
                if expr.args.len() == 3 {
                    vm.eval(expr.args[2].clone())
                } else {
                    false_sym()
                }
            }
            None => {
                // Predicate didn't reduce — rebuild the expression.
                let mut new_args = vec![predicate];
                new_args.extend(expr.args[1..].iter().cloned());
                IRNode::Apply(Box::new(IRApply {
                    head: expr.head,
                    args: new_args,
                }))
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Assign / Define — binding forms
// ---------------------------------------------------------------------------

fn assign_handler(_simplify: bool) -> Handler {
    std::sync::Arc::new(move |vm: &mut VM, expr: IRApply| -> IRNode {
        let (lhs, rhs) = match binary_args(&expr) {
            Some(p) => p,
            None => return IRNode::Apply(Box::new(expr)),
        };
        let name = match &lhs {
            IRNode::Symbol(s) => s.clone(),
            _ => panic!("Assign lhs must be a symbol, got {lhs}"),
        };
        let value = vm.eval(rhs);
        vm.backend.bind(&name, value.clone());
        value
    })
}

fn define_handler(_simplify: bool) -> Handler {
    std::sync::Arc::new(move |vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 3 {
            return IRNode::Apply(Box::new(expr));
        }
        let name = match &expr.args[0] {
            IRNode::Symbol(s) => s.clone(),
            n => panic!("Define name must be a symbol, got {n}"),
        };
        // Store the entire Define(...) record under the name so the VM's
        // function-call path can find and apply it.
        vm.backend
            .bind(&name, IRNode::Apply(Box::new(expr.clone())));
        IRNode::Symbol(name)
    })
}

// ---------------------------------------------------------------------------
// List — passthrough
// ---------------------------------------------------------------------------

fn list_handler(_simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        IRNode::Apply(Box::new(expr))
    })
}

// ---------------------------------------------------------------------------
// Symbolic differentiation
// ---------------------------------------------------------------------------

fn derivative_handler() -> Handler {
    std::sync::Arc::new(move |vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 2 {
            panic!("D expects 2 arguments, got {}", expr.args.len());
        }

        let f = expr.args[0].clone();
        let x = match &expr.args[1] {
            IRNode::Symbol(s) => s.clone(),
            _ => return IRNode::Apply(Box::new(expr)),
        };

        let result = diff(&f, &x);
        let original = apply_node(D, vec![f, IRNode::Symbol(x)]);
        if result == original {
            result
        } else {
            vm.eval(result)
        }
    })
}

fn integrate_handler() -> Handler {
    std::sync::Arc::new(move |vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 2 {
            panic!("Integrate expects 2 arguments, got {}", expr.args.len());
        }

        let f = expr.args[0].clone();
        let x = match &expr.args[1] {
            IRNode::Symbol(s) => s.clone(),
            _ => return IRNode::Apply(Box::new(expr)),
        };

        let result = integrate(&f, &x);
        let original = apply_node(INTEGRATE, vec![f, IRNode::Symbol(x)]);
        if result == original {
            result
        } else {
            vm.eval(result)
        }
    })
}

fn integrate(f: &IRNode, x: &str) -> IRNode {
    if !depends_on(f, x) {
        return apply_node(MUL, vec![f.clone(), IRNode::Symbol(x.to_string())]);
    }

    if f == &IRNode::Symbol(x.to_string()) {
        return apply_node(
            MUL,
            vec![
                IRNode::Rational(1, 2),
                apply_node(POW, vec![IRNode::Symbol(x.to_string()), IRNode::Integer(2)]),
            ],
        );
    }

    let IRNode::Apply(apply) = f else {
        return apply_node(INTEGRATE, vec![f.clone(), IRNode::Symbol(x.to_string())]);
    };

    let IRNode::Symbol(head) = &apply.head else {
        return apply_node(INTEGRATE, vec![f.clone(), IRNode::Symbol(x.to_string())]);
    };

    match (head.as_str(), apply.args.as_slice()) {
        (ADD, [a, b]) => apply_node(ADD, vec![integrate(a, x), integrate(b, x)]),
        (SUB, [a, b]) => apply_node(SUB, vec![integrate(a, x), integrate(b, x)]),
        (NEG, [a]) => apply_node(NEG, vec![integrate(a, x)]),
        (MUL, [a, b]) if !depends_on(a, x) => apply_node(MUL, vec![a.clone(), integrate(b, x)]),
        (MUL, [a, b]) if !depends_on(b, x) => apply_node(MUL, vec![b.clone(), integrate(a, x)]),
        (DIV, [c, denom]) if denom == &IRNode::Symbol(x.to_string()) && !depends_on(c, x) => {
            apply_node(MUL, vec![c.clone(), apply_node(LOG, vec![denom.clone()])])
        }
        (POW, [base, exponent]) if base == &IRNode::Symbol(x.to_string()) => {
            integrate_power_of_x(exponent, x)
        }
        (POW, [base, exponent]) if exponent == &IRNode::Symbol(x.to_string()) => {
            if !depends_on(base, x) {
                apply_node(
                    DIV,
                    vec![
                        apply_node(POW, vec![base.clone(), exponent.clone()]),
                        apply_node(LOG, vec![base.clone()]),
                    ],
                )
            } else {
                apply_node(INTEGRATE, vec![f.clone(), IRNode::Symbol(x.to_string())])
            }
        }
        (SIN, [inner]) if inner == &IRNode::Symbol(x.to_string()) => {
            apply_node(NEG, vec![apply_node(COS, vec![inner.clone()])])
        }
        (COS, [inner]) if inner == &IRNode::Symbol(x.to_string()) => {
            apply_node(SIN, vec![inner.clone()])
        }
        (EXP, [inner]) if inner == &IRNode::Symbol(x.to_string()) => {
            apply_node(EXP, vec![inner.clone()])
        }
        (LOG, [inner]) if inner == &IRNode::Symbol(x.to_string()) => apply_node(
            SUB,
            vec![
                apply_node(
                    MUL,
                    vec![inner.clone(), apply_node(LOG, vec![inner.clone()])],
                ),
                inner.clone(),
            ],
        ),
        (SQRT, [inner]) if inner == &IRNode::Symbol(x.to_string()) => apply_node(
            MUL,
            vec![
                IRNode::Rational(2, 3),
                apply_node(POW, vec![inner.clone(), IRNode::Rational(3, 2)]),
            ],
        ),
        _ => apply_node(INTEGRATE, vec![f.clone(), IRNode::Symbol(x.to_string())]),
    }
}

fn integrate_power_of_x(exponent: &IRNode, x: &str) -> IRNode {
    let Some(n) = to_numeric(exponent) else {
        return apply_node(
            INTEGRATE,
            vec![
                apply_node(POW, vec![IRNode::Symbol(x.to_string()), exponent.clone()]),
                IRNode::Symbol(x.to_string()),
            ],
        );
    };

    if n == Numeric::Int(-1) {
        return apply_node(LOG, vec![IRNode::Symbol(x.to_string())]);
    }

    let next = n + Numeric::Int(1);
    if next.is_zero() {
        return apply_node(LOG, vec![IRNode::Symbol(x.to_string())]);
    }

    apply_node(
        MUL,
        vec![
            from_numeric(Numeric::Int(1) / next),
            apply_node(POW, vec![IRNode::Symbol(x.to_string()), from_numeric(next)]),
        ],
    )
}

fn diff(f: &IRNode, x: &str) -> IRNode {
    if !depends_on(f, x) {
        return IRNode::Integer(0);
    }

    if f == &IRNode::Symbol(x.to_string()) {
        return IRNode::Integer(1);
    }

    let IRNode::Apply(apply) = f else {
        return apply_node(D, vec![f.clone(), IRNode::Symbol(x.to_string())]);
    };

    let IRNode::Symbol(head) = &apply.head else {
        return apply_node(D, vec![f.clone(), IRNode::Symbol(x.to_string())]);
    };

    match (head.as_str(), apply.args.as_slice()) {
        (ADD, [a, b]) => apply_node(ADD, vec![diff(a, x), diff(b, x)]),
        (SUB, [a, b]) => apply_node(SUB, vec![diff(a, x), diff(b, x)]),
        (NEG, [a]) => apply_node(NEG, vec![diff(a, x)]),
        (MUL, [a, b]) => apply_node(
            ADD,
            vec![
                apply_node(MUL, vec![diff(a, x), b.clone()]),
                apply_node(MUL, vec![a.clone(), diff(b, x)]),
            ],
        ),
        (DIV, [a, b]) => apply_node(
            DIV,
            vec![
                apply_node(
                    SUB,
                    vec![
                        apply_node(MUL, vec![diff(a, x), b.clone()]),
                        apply_node(MUL, vec![a.clone(), diff(b, x)]),
                    ],
                ),
                apply_node(POW, vec![b.clone(), IRNode::Integer(2)]),
            ],
        ),
        (POW, [base, exponent]) => diff_pow(f, base, exponent, x),
        (SIN, [inner]) => chain(COS, inner, x),
        (COS, [inner]) => apply_node(
            MUL,
            vec![
                apply_node(NEG, vec![apply_node(SIN, vec![inner.clone()])]),
                diff(inner, x),
            ],
        ),
        (TAN, [inner]) => apply_node(
            DIV,
            vec![
                diff(inner, x),
                apply_node(
                    POW,
                    vec![apply_node(COS, vec![inner.clone()]), IRNode::Integer(2)],
                ),
            ],
        ),
        (EXP, [inner]) => chain(EXP, inner, x),
        (LOG, [inner]) => apply_node(DIV, vec![diff(inner, x), inner.clone()]),
        (SQRT, [inner]) => apply_node(
            DIV,
            vec![
                diff(inner, x),
                apply_node(
                    MUL,
                    vec![IRNode::Integer(2), apply_node(SQRT, vec![inner.clone()])],
                ),
            ],
        ),
        (SINH, [inner]) => chain(COSH, inner, x),
        (COSH, [inner]) => chain(SINH, inner, x),
        (TANH, [inner]) => apply_node(
            DIV,
            vec![
                diff(inner, x),
                apply_node(
                    POW,
                    vec![apply_node(COSH, vec![inner.clone()]), IRNode::Integer(2)],
                ),
            ],
        ),
        (ASINH, [inner]) => {
            let denom = apply_node(
                SQRT,
                vec![apply_node(
                    ADD,
                    vec![
                        apply_node(POW, vec![inner.clone(), IRNode::Integer(2)]),
                        IRNode::Integer(1),
                    ],
                )],
            );
            apply_node(DIV, vec![diff(inner, x), denom])
        }
        (ACOSH, [inner]) => {
            let denom = apply_node(
                SQRT,
                vec![apply_node(
                    SUB,
                    vec![
                        apply_node(POW, vec![inner.clone(), IRNode::Integer(2)]),
                        IRNode::Integer(1),
                    ],
                )],
            );
            apply_node(DIV, vec![diff(inner, x), denom])
        }
        (ATANH, [inner]) => {
            let denom = apply_node(
                SUB,
                vec![
                    IRNode::Integer(1),
                    apply_node(POW, vec![inner.clone(), IRNode::Integer(2)]),
                ],
            );
            apply_node(DIV, vec![diff(inner, x), denom])
        }
        (COTH, [inner]) => {
            let denom = apply_node(
                POW,
                vec![apply_node(SINH, vec![inner.clone()]), IRNode::Integer(2)],
            );
            apply_node(NEG, vec![apply_node(DIV, vec![diff(inner, x), denom])])
        }
        (SECH, [inner]) => {
            let numer = apply_node(
                MUL,
                vec![diff(inner, x), apply_node(SINH, vec![inner.clone()])],
            );
            let denom = apply_node(
                POW,
                vec![apply_node(COSH, vec![inner.clone()]), IRNode::Integer(2)],
            );
            apply_node(NEG, vec![apply_node(DIV, vec![numer, denom])])
        }
        (CSCH, [inner]) => {
            let numer = apply_node(
                MUL,
                vec![diff(inner, x), apply_node(COSH, vec![inner.clone()])],
            );
            let denom = apply_node(
                POW,
                vec![apply_node(SINH, vec![inner.clone()]), IRNode::Integer(2)],
            );
            apply_node(NEG, vec![apply_node(DIV, vec![numer, denom])])
        }
        _ => apply_node(D, vec![f.clone(), IRNode::Symbol(x.to_string())]),
    }
}

fn diff_pow(f: &IRNode, base: &IRNode, exponent: &IRNode, x: &str) -> IRNode {
    let base_depends = depends_on(base, x);
    let exp_depends = depends_on(exponent, x);

    if !exp_depends {
        return apply_node(
            MUL,
            vec![
                apply_node(
                    MUL,
                    vec![
                        exponent.clone(),
                        apply_node(
                            POW,
                            vec![
                                base.clone(),
                                apply_node(SUB, vec![exponent.clone(), IRNode::Integer(1)]),
                            ],
                        ),
                    ],
                ),
                diff(base, x),
            ],
        );
    }

    if !base_depends {
        return apply_node(
            MUL,
            vec![
                apply_node(MUL, vec![f.clone(), apply_node(LOG, vec![base.clone()])]),
                diff(exponent, x),
            ],
        );
    }

    diff(
        &apply_node(
            EXP,
            vec![apply_node(
                MUL,
                vec![exponent.clone(), apply_node(LOG, vec![base.clone()])],
            )],
        ),
        x,
    )
}

fn chain(head: &str, inner: &IRNode, x: &str) -> IRNode {
    apply_node(
        MUL,
        vec![apply_node(head, vec![inner.clone()]), diff(inner, x)],
    )
}

fn depends_on(node: &IRNode, var: &str) -> bool {
    match node {
        IRNode::Symbol(s) => s == var,
        IRNode::Apply(apply) => {
            depends_on(&apply.head, var) || apply.args.iter().any(|arg| depends_on(arg, var))
        }
        IRNode::Integer(_) | IRNode::Rational(_, _) | IRNode::Float(_) | IRNode::Str(_) => false,
    }
}

fn apply_node(head: &str, args: Vec<IRNode>) -> IRNode {
    IRNode::Apply(Box::new(IRApply {
        head: IRNode::Symbol(head.to_string()),
        args,
    }))
}

// ---------------------------------------------------------------------------
// Factor
// ---------------------------------------------------------------------------

fn factor_handler(vm: &mut VM, expr: IRApply) -> IRNode {
    let fallback = IRNode::Apply(Box::new(expr.clone()));
    if expr.args.len() != 1 {
        return fallback;
    }

    let input = &expr.args[0];
    if let Some(variable) = find_single_variable(input) {
        if let Some(coeffs) = ir_to_integer_poly(input, &variable) {
            let (content, factors) = factor_integer_polynomial(&coeffs);
            if factors.is_empty() {
                return input.clone();
            }
            if factors.len() == 1 && factors[0].1 == 1 && content == 1 && factors[0].0 == coeffs {
                return fallback;
            }
            return factor_result_to_ir(content, factors, &variable);
        }
    }

    if let Some(rewritten) = factor_common_symbolic_term(input) {
        return vm.eval(rewritten);
    }

    fallback
}

fn factor_common_symbolic_term(node: &IRNode) -> Option<IRNode> {
    let terms = additive_terms(node)?;
    if terms.len() < 2 {
        return None;
    }

    let mut common = term_factor_powers(&terms[0]);
    for term in &terms[1..] {
        let powers = term_factor_powers(term);
        common.retain(|base, exponent| {
            if let Some(other) = powers.get(base) {
                *exponent = (*exponent).min(*other);
                *exponent > 0
            } else {
                false
            }
        });
        if common.is_empty() {
            return None;
        }
    }

    let common_factor = powers_to_ir(&common);
    let residual_terms: Vec<IRNode> = terms
        .iter()
        .map(|term| remove_common_factor(term, &common))
        .collect();
    let residual = add_nodes(residual_terms);
    Some(apply_node(
        MUL,
        vec![common_factor, apply_node(FACTOR, vec![residual])],
    ))
}

fn additive_terms(node: &IRNode) -> Option<Vec<IRNode>> {
    match node {
        IRNode::Apply(apply) if is_head_name(&apply.head, ADD) => {
            let mut terms = Vec::new();
            for arg in &apply.args {
                collect_additive_terms(arg, &mut terms);
            }
            Some(terms)
        }
        IRNode::Apply(apply) if is_head_name(&apply.head, SUB) && apply.args.len() == 2 => {
            let mut terms = Vec::new();
            collect_additive_terms(&apply.args[0], &mut terms);
            terms.push(negate_node(apply.args[1].clone()));
            Some(terms)
        }
        _ => None,
    }
}

fn collect_additive_terms(node: &IRNode, terms: &mut Vec<IRNode>) {
    match node {
        IRNode::Apply(apply) if is_head_name(&apply.head, ADD) => {
            for arg in &apply.args {
                collect_additive_terms(arg, terms);
            }
        }
        IRNode::Apply(apply) if is_head_name(&apply.head, SUB) && apply.args.len() == 2 => {
            collect_additive_terms(&apply.args[0], terms);
            terms.push(negate_node(apply.args[1].clone()));
        }
        other => terms.push(other.clone()),
    }
}

fn negate_node(node: IRNode) -> IRNode {
    match node {
        IRNode::Integer(value) => IRNode::Integer(-value),
        IRNode::Apply(apply) if is_head_name(&apply.head, MUL) => {
            let mut args = Vec::with_capacity(apply.args.len() + 1);
            args.push(IRNode::Integer(-1));
            args.extend(apply.args);
            apply_node(MUL, args)
        }
        other => apply_node(MUL, vec![IRNode::Integer(-1), other]),
    }
}

fn term_factor_powers(term: &IRNode) -> HashMap<IRNode, usize> {
    let mut powers = HashMap::new();
    for factor in multiplicative_factors(term) {
        if let Some((base, exponent)) = factor_base_power(factor) {
            *powers.entry(base).or_insert(0) += exponent;
        }
    }
    powers
}

fn multiplicative_factors(node: &IRNode) -> Vec<IRNode> {
    match node {
        IRNode::Apply(apply) if is_head_name(&apply.head, MUL) => {
            apply.args.iter().flat_map(multiplicative_factors).collect()
        }
        other => vec![other.clone()],
    }
}

fn factor_base_power(factor: IRNode) -> Option<(IRNode, usize)> {
    match factor {
        IRNode::Integer(_) | IRNode::Rational(_, _) | IRNode::Float(_) => None,
        IRNode::Symbol(name) if name.starts_with('%') => None,
        IRNode::Apply(apply) if is_head_name(&apply.head, POW) && apply.args.len() == 2 => {
            if let IRNode::Integer(exponent) = apply.args[1] {
                if exponent > 0 && !is_numeric_or_protected(&apply.args[0]) {
                    return Some((apply.args[0].clone(), exponent as usize));
                }
            }
            Some((IRNode::Apply(apply), 1))
        }
        other if is_numeric_or_protected(&other) => None,
        other => Some((other, 1)),
    }
}

fn is_numeric_or_protected(node: &IRNode) -> bool {
    match node {
        IRNode::Integer(_) | IRNode::Rational(_, _) | IRNode::Float(_) => true,
        IRNode::Symbol(name) => name.starts_with('%'),
        _ => false,
    }
}

fn remove_common_factor(term: &IRNode, common: &HashMap<IRNode, usize>) -> IRNode {
    let mut remaining = common.clone();
    let mut pieces = Vec::new();
    for factor in multiplicative_factors(term) {
        let Some((base, exponent)) = factor_base_power(factor.clone()) else {
            pieces.push(factor);
            continue;
        };
        let Some(common_exponent) = remaining.get_mut(&base) else {
            pieces.push(factor);
            continue;
        };

        if exponent > *common_exponent {
            pieces.push(power_to_ir(base.clone(), exponent - *common_exponent));
            *common_exponent = 0;
        } else {
            *common_exponent -= exponent;
        }
    }
    multiply_nodes(pieces)
}

fn powers_to_ir(powers: &HashMap<IRNode, usize>) -> IRNode {
    let mut pieces: Vec<IRNode> = powers
        .iter()
        .map(|(base, exponent)| power_to_ir(base.clone(), *exponent))
        .collect();
    pieces.sort_by_key(|piece| piece.to_string());
    multiply_nodes(pieces)
}

fn power_to_ir(base: IRNode, exponent: usize) -> IRNode {
    if exponent == 1 {
        base
    } else {
        apply_node(POW, vec![base, IRNode::Integer(exponent as i64)])
    }
}

fn find_single_variable(node: &IRNode) -> Option<String> {
    let mut variables = HashSet::new();
    collect_variables(node, &mut variables);
    if variables.len() == 1 {
        variables.into_iter().next()
    } else {
        None
    }
}

fn collect_variables(node: &IRNode, variables: &mut HashSet<String>) {
    match node {
        IRNode::Symbol(name) if !name.starts_with('%') => {
            variables.insert(name.clone());
        }
        IRNode::Apply(apply) => {
            for arg in &apply.args {
                collect_variables(arg, variables);
            }
        }
        _ => {}
    }
}

fn ir_to_integer_poly(node: &IRNode, variable: &str) -> Option<Vec<i64>> {
    match node {
        IRNode::Integer(value) => Some(vec![*value]),
        IRNode::Symbol(name) if name == variable => Some(vec![0, 1]),
        IRNode::Apply(apply) if is_head_name(&apply.head, ADD) => {
            apply.args.iter().try_fold(vec![0], |acc, arg| {
                Some(poly_add(&acc, &ir_to_integer_poly(arg, variable)?))
            })
        }
        IRNode::Apply(apply) if is_head_name(&apply.head, SUB) && apply.args.len() == 2 => {
            let a = ir_to_integer_poly(&apply.args[0], variable)?;
            let b = ir_to_integer_poly(&apply.args[1], variable)?;
            Some(poly_sub(&a, &b))
        }
        IRNode::Apply(apply) if is_head_name(&apply.head, MUL) => {
            apply.args.iter().try_fold(vec![1], |acc, arg| {
                Some(poly_mul(&acc, &ir_to_integer_poly(arg, variable)?))
            })
        }
        IRNode::Apply(apply) if is_head_name(&apply.head, POW) && apply.args.len() == 2 => {
            let IRNode::Integer(exp) = apply.args[1] else {
                return None;
            };
            if !(0..=32).contains(&exp) {
                return None;
            }
            let base = ir_to_integer_poly(&apply.args[0], variable)?;
            Some(poly_pow(&base, exp as usize))
        }
        _ => None,
    }
}

fn factor_result_to_ir(content: i64, factors: Vec<(Vec<i64>, usize)>, variable: &str) -> IRNode {
    let mut pieces = Vec::new();
    if content != 1 {
        pieces.push(IRNode::Integer(content));
    }
    for (coeffs, multiplicity) in factors {
        let factor = poly_to_ir(&coeffs, variable);
        if multiplicity == 1 {
            pieces.push(factor);
        } else {
            pieces.push(apply_node(
                POW,
                vec![factor, IRNode::Integer(multiplicity as i64)],
            ));
        }
    }
    multiply_nodes(pieces)
}

fn poly_to_ir(coeffs: &[i64], variable: &str) -> IRNode {
    let terms: Vec<IRNode> = coeffs
        .iter()
        .enumerate()
        .filter_map(|(degree, coeff)| {
            if *coeff == 0 {
                None
            } else {
                Some(monomial_to_ir(*coeff, degree, variable))
            }
        })
        .collect();
    if terms.is_empty() {
        IRNode::Integer(0)
    } else {
        add_nodes(terms)
    }
}

fn monomial_to_ir(coeff: i64, degree: usize, variable: &str) -> IRNode {
    if degree == 0 {
        return IRNode::Integer(coeff);
    }
    let power = if degree == 1 {
        IRNode::Symbol(variable.to_string())
    } else {
        apply_node(
            POW,
            vec![
                IRNode::Symbol(variable.to_string()),
                IRNode::Integer(degree as i64),
            ],
        )
    };
    match coeff {
        1 => power,
        -1 => apply_node(MUL, vec![IRNode::Integer(-1), power]),
        _ => apply_node(MUL, vec![IRNode::Integer(coeff), power]),
    }
}

fn add_nodes(nodes: Vec<IRNode>) -> IRNode {
    if nodes.len() == 1 {
        nodes.into_iter().next().unwrap()
    } else {
        apply_node(ADD, nodes)
    }
}

fn multiply_nodes(nodes: Vec<IRNode>) -> IRNode {
    if nodes.is_empty() {
        IRNode::Integer(1)
    } else if nodes.len() == 1 {
        nodes.into_iter().next().unwrap()
    } else {
        apply_node(MUL, nodes)
    }
}

fn poly_add(a: &[i64], b: &[i64]) -> Vec<i64> {
    let len = a.len().max(b.len());
    let mut out = vec![0; len];
    for i in 0..len {
        out[i] = a.get(i).copied().unwrap_or(0) + b.get(i).copied().unwrap_or(0);
    }
    trim_poly(out)
}

fn poly_sub(a: &[i64], b: &[i64]) -> Vec<i64> {
    let len = a.len().max(b.len());
    let mut out = vec![0; len];
    for i in 0..len {
        out[i] = a.get(i).copied().unwrap_or(0) - b.get(i).copied().unwrap_or(0);
    }
    trim_poly(out)
}

fn poly_mul(a: &[i64], b: &[i64]) -> Vec<i64> {
    let mut out = vec![0; a.len() + b.len() - 1];
    for (i, ca) in a.iter().enumerate() {
        for (j, cb) in b.iter().enumerate() {
            out[i + j] += ca * cb;
        }
    }
    trim_poly(out)
}

fn poly_pow(base: &[i64], exp: usize) -> Vec<i64> {
    let mut out = vec![1];
    for _ in 0..exp {
        out = poly_mul(&out, base);
    }
    out
}

fn trim_poly(mut poly: Vec<i64>) -> Vec<i64> {
    while poly.len() > 1 && poly.last() == Some(&0) {
        poly.pop();
    }
    poly
}

fn is_head_name(head: &IRNode, expected: &str) -> bool {
    matches!(head, IRNode::Symbol(name) if name == expected)
}

// ---------------------------------------------------------------------------
// Build handler table
// ---------------------------------------------------------------------------

/// Produce the full handler table for a backend.
///
/// `simplify = false` → numeric-only evaluator (StrictBackend).
/// `simplify = true`  → symbolic evaluator with algebraic identities.
pub fn build_handler_table(simplify: bool) -> HashMap<String, Handler> {
    let mut m: HashMap<String, Handler> = HashMap::new();
    m.insert(ADD.to_string(), add_handler(simplify));
    m.insert(SUB.to_string(), sub_handler(simplify));
    m.insert(MUL.to_string(), mul_handler(simplify));
    m.insert(DIV.to_string(), div_handler(simplify));
    m.insert(POW.to_string(), pow_handler(simplify));
    m.insert(NEG.to_string(), neg_handler(simplify));
    m.insert(INV.to_string(), inv_handler(simplify));
    m.insert(SIN.to_string(), sin_handler(simplify));
    m.insert(COS.to_string(), cos_handler(simplify));
    m.insert(TAN.to_string(), tan_handler(simplify));
    m.insert(EXP.to_string(), exp_handler(simplify));
    m.insert(LOG.to_string(), log_handler(simplify));
    m.insert(SQRT.to_string(), sqrt_handler(simplify));
    m.insert(ATAN.to_string(), atan_handler(simplify));
    m.insert(ASIN.to_string(), asin_handler(simplify));
    m.insert(ACOS.to_string(), acos_handler(simplify));
    m.insert(SINH.to_string(), sinh_handler(simplify));
    m.insert(COSH.to_string(), cosh_handler(simplify));
    m.insert(TANH.to_string(), tanh_handler(simplify));
    m.insert(COTH.to_string(), coth_handler(simplify));
    m.insert(SECH.to_string(), sech_handler(simplify));
    m.insert(CSCH.to_string(), csch_handler(simplify));
    m.insert(ASINH.to_string(), asinh_handler(simplify));
    m.insert(ACOSH.to_string(), acosh_handler(simplify));
    m.insert(ATANH.to_string(), atanh_handler(simplify));
    m.insert(
        EQUAL.to_string(),
        comparison_handler(|a, b| a == b, true, true, simplify),
    );
    m.insert(
        NOT_EQUAL.to_string(),
        comparison_handler(|a, b| a != b, true, false, simplify),
    );
    m.insert(
        LESS.to_string(),
        comparison_handler(|a, b| a < b, false, false, simplify),
    );
    m.insert(
        GREATER.to_string(),
        comparison_handler(|a, b| a > b, false, false, simplify),
    );
    m.insert(
        LESS_EQUAL.to_string(),
        comparison_handler(|a, b| a <= b, false, false, simplify),
    );
    m.insert(
        GREATER_EQUAL.to_string(),
        comparison_handler(|a, b| a >= b, false, false, simplify),
    );
    m.insert(AND.to_string(), and_handler(simplify));
    m.insert(OR.to_string(), or_handler(simplify));
    m.insert(NOT.to_string(), not_handler(simplify));
    m.insert(IF.to_string(), if_handler(simplify));
    m.insert(ASSIGN.to_string(), assign_handler(simplify));
    m.insert(DEFINE.to_string(), define_handler(simplify));
    m.insert("List".to_string(), list_handler(simplify));
    if simplify {
        m.insert(D.to_string(), derivative_handler());
        m.insert(INTEGRATE.to_string(), integrate_handler());
        m.insert(FACTOR.to_string(), handler_fn(factor_handler));
    }
    m
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract `(a, b)` from a binary `IRApply`.  Returns `None` (leaving the
/// expr unchanged) if the argument count is wrong.
fn binary_args(expr: &IRApply) -> Option<(IRNode, IRNode)> {
    if expr.args.len() == 2 {
        Some((expr.args[0].clone(), expr.args[1].clone()))
    } else {
        None
    }
}
