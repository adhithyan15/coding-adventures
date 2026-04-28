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

use std::collections::HashMap;

use symbolic_ir::{
    IRApply, IRNode, ACOS, ACOSH, ADD, AND, ASIN, ASINH, ASSIGN, ATAN, ATANH, COS, COSH, DEFINE,
    DIV, EQUAL, EXP, GREATER, GREATER_EQUAL, IF, INV, LESS, LESS_EQUAL, LOG, MUL, NEG,
    NOT, NOT_EQUAL, OR, POW, SIN, SINH, SQRT, SUB, TAN, TANH,
};

use crate::backend::Handler;
use crate::vm::VM;

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
    let (numer, denom) = if denom < 0 { (-numer, -denom) } else { (numer, denom) };
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
    if v { true_sym() } else { false_sym() }
}

/// Check if a node is the `True` or `False` symbol.
fn is_truthy(node: &IRNode) -> Option<bool> {
    if let IRNode::Symbol(s) = node {
        if s == "True" { return Some(true); }
        if s == "False" { return Some(false); }
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
        if va.map(|v| v.is_zero()).unwrap_or(false) { return b; }
        if vb.map(|v| v.is_zero()).unwrap_or(false) { return a; }
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
        if vb.map(|v| v.is_zero()).unwrap_or(false) { return a; }
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
        if va.map(|v| v.is_one()).unwrap_or(false) { return b; }
        if vb.map(|v| v.is_one()).unwrap_or(false) { return a; }
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
        if va.map(|v| v.is_zero()).unwrap_or(false) { return IRNode::Integer(0); }
        if vb.map(|v| v.is_one()).unwrap_or(false) { return a; }
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
        if ve.map(|v| v.is_zero()).unwrap_or(false) { return IRNode::Integer(1); }
        // x^1 → x
        if ve.map(|v| v.is_one()).unwrap_or(false) { return base; }
        // 0^n → 0 (n ≠ 0 covered above)
        if vb.map(|v| v.is_zero()).unwrap_or(false) { return IRNode::Integer(0); }
        // 1^n → 1
        if vb.map(|v| v.is_one()).unwrap_or(false) { return IRNode::Integer(1); }
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
            if b == 0 { panic!("0^negative"); }
            let pos = pow_numeric(base, Numeric::Int(-e));
            return make_rat(1, 1) / pos;
        }
    }
    // Rat^Int — exact when exponent is a non-negative integer.
    if let (Numeric::Rat(n, d), Numeric::Int(e)) = (base, exp) {
        if e >= 0 && e <= 30 {
            let eu = e as u32;
            if let (Some(nn), Some(dd)) = ((n as i128).checked_pow(eu), (d as i128).checked_pow(eu)) {
                if nn >= i64::MIN as i128 && nn <= i64::MAX as i128
                    && dd >= 1 && dd <= i64::MAX as i128
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
        single_trig(&expr, "Sin", f64::sin, &[(Numeric::Int(0), IRNode::Integer(0))], simplify)
    })
}

fn cos_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(&expr, "Cos", f64::cos, &[(Numeric::Int(0), IRNode::Integer(1))], simplify)
    })
}

fn tan_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(&expr, "Tan", f64::tan, &[(Numeric::Int(0), IRNode::Integer(0))], simplify)
    })
}

fn exp_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(&expr, "Exp", f64::exp, &[(Numeric::Int(0), IRNode::Integer(1))], simplify)
    })
}

fn log_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(&expr, "Log", f64::ln, &[(Numeric::Int(1), IRNode::Integer(0))], simplify)
    })
}

fn sqrt_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(
            &expr,
            "Sqrt",
            f64::sqrt,
            &[(Numeric::Int(0), IRNode::Integer(0)), (Numeric::Int(1), IRNode::Integer(1))],
            simplify,
        )
    })
}

fn atan_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(&expr, "Atan", f64::atan, &[(Numeric::Int(0), IRNode::Integer(0))], simplify)
    })
}

fn asin_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(&expr, "Asin", f64::asin, &[(Numeric::Int(0), IRNode::Integer(0))], simplify)
    })
}

fn acos_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(&expr, "Acos", f64::acos, &[], simplify)
    })
}

fn sinh_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(&expr, "Sinh", f64::sinh, &[(Numeric::Int(0), IRNode::Integer(0))], simplify)
    })
}

fn cosh_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(&expr, "Cosh", f64::cosh, &[(Numeric::Int(0), IRNode::Integer(1))], simplify)
    })
}

fn tanh_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(&expr, "Tanh", f64::tanh, &[(Numeric::Int(0), IRNode::Integer(0))], simplify)
    })
}

fn asinh_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(&expr, "Asinh", f64::asinh, &[(Numeric::Int(0), IRNode::Integer(0))], simplify)
    })
}

fn acosh_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(&expr, "Acosh", f64::acosh, &[(Numeric::Int(1), IRNode::Integer(0))], simplify)
    })
}

fn atanh_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(&expr, "Atanh", f64::atanh, &[(Numeric::Int(0), IRNode::Integer(0))], simplify)
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

// ---------------------------------------------------------------------------
// Comparison handlers
// ---------------------------------------------------------------------------

fn comparison_handler(
    op: fn(f64, f64) -> bool,
    eq_based: bool,  // true for Equal/NotEqual (structural check)
    is_equal_op: bool,  // true for Equal (not NotEqual)
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
                IRNode::Apply(Box::new(IRApply { head, args: remaining }))
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
                IRNode::Apply(Box::new(IRApply { head, args: remaining }))
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
                IRNode::Apply(Box::new(IRApply { head: expr.head, args: new_args }))
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
        vm.backend.bind(&name, IRNode::Apply(Box::new(expr.clone())));
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
    m.insert(LESS.to_string(), comparison_handler(|a, b| a < b, false, false, simplify));
    m.insert(GREATER.to_string(), comparison_handler(|a, b| a > b, false, false, simplify));
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
