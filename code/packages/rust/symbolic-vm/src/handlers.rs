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

/// Phase 47 (Rust port): walk a nested `Add` tree and append every
/// non-`Add` leaf to `out`.  Used by the `Add` handler's flattening
/// path.
fn flatten_add_leaves(node: &IRNode, out: &mut Vec<IRNode>) {
    match node {
        IRNode::Apply(apply_node) if matches!(&apply_node.head, IRNode::Symbol(s) if s == ADD) => {
            for arg in &apply_node.args {
                flatten_add_leaves(arg, out);
            }
        }
        _ => out.push(node.clone()),
    }
}

fn add_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        // Phase 47: nested-Add flattening for the symbolic backend.
        // When either binary Add operand is itself an Add(...) apply,
        // gather every non-Add leaf, sum the numeric literals once,
        // and rebuild a left-associated chain.  This makes Add
        // canonical for any consumer that compares trees structurally
        // — most importantly the cas-summation telescope detector.
        if simplify && expr.args.len() == 2 {
            let a_is_add = matches!(
                &expr.args[0],
                IRNode::Apply(a) if matches!(&a.head, IRNode::Symbol(s) if s == ADD)
            );
            let b_is_add = matches!(
                &expr.args[1],
                IRNode::Apply(a) if matches!(&a.head, IRNode::Symbol(s) if s == ADD)
            );
            if a_is_add || b_is_add {
                let mut leaves: Vec<IRNode> = Vec::new();
                flatten_add_leaves(&expr.args[0], &mut leaves);
                flatten_add_leaves(&expr.args[1], &mut leaves);
                // Re-evaluation guard: only rebuild when flattening
                // actually changed the operand list (saves a needless
                // round-trip when neither side was nested).
                let rebuilt = leaves.len() != 2
                    || leaves.iter().any(|leaf| {
                        matches!(
                            leaf,
                            IRNode::Apply(a) if matches!(&a.head, IRNode::Symbol(s) if s == ADD)
                        )
                    });
                if rebuilt {
                    let mut lit_acc: Option<Numeric> = None;
                    let mut non_literals: Vec<IRNode> = Vec::new();
                    for leaf in leaves {
                        match to_numeric(&leaf) {
                            Some(n) => {
                                lit_acc = Some(match lit_acc {
                                    Some(acc) => acc + n,
                                    None => n,
                                });
                            }
                            None => non_literals.push(leaf),
                        }
                    }
                    if non_literals.is_empty() {
                        let total = lit_acc.unwrap_or(Numeric::Int(0));
                        return from_numeric(total);
                    }
                    let lit_is_zero = match lit_acc {
                        None => true,
                        Some(n) => n.is_zero(),
                    };
                    let mut final_args = non_literals;
                    if !lit_is_zero {
                        final_args.push(from_numeric(lit_acc.unwrap()));
                    }
                    if final_args.len() == 1 {
                        return final_args.into_iter().next().unwrap();
                    }
                    // Left-associate the chain.
                    let mut iter = final_args.into_iter();
                    let mut out = iter.next().unwrap();
                    for nxt in iter {
                        out = IRNode::Apply(Box::new(IRApply {
                            head: IRNode::Symbol(ADD.to_string()),
                            args: vec![out, nxt],
                        }));
                    }
                    return out;
                }
            }
        }

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

// ---------------------------------------------------------------------------
// Phase 29–33: algebraic helpers
// ---------------------------------------------------------------------------

/// Reduced fraction `(p, q)` with `q > 0` and `gcd(|p|, q) = 1`.
type Frac = (i64, i64);

fn frac_gcd(mut a: i64, mut b: i64) -> i64 {
    a = a.abs();
    b = b.abs();
    while b != 0 { let t = b; b = a % b; a = t; }
    if a == 0 { 1 } else { a }
}

/// Normalise `p/q` to lowest terms with `q > 0`.
/// Returns `None` if `q == 0`.
fn frac_make(p: i64, q: i64) -> Option<Frac> {
    if q == 0 { return None; }
    let (p, q) = if q < 0 { (-p, -q) } else { (p, q) };
    let g = frac_gcd(p.abs(), q);
    Some((p / g, q / g))
}

/// `(p/q) mod m`, result in `[0, m)`.
fn frac_mod(p: i64, q: i64, m: i64) -> Option<Frac> {
    // (p/q) mod m = (p mod (m·q)) / q
    let mq = m.checked_mul(q)?;
    let mut r = p % mq;
    if r < 0 { r = r.checked_add(mq)?; }
    frac_make(r, q)
}

/// Try to extract a plain `i64/i64` rational from an IR numeric node.
fn frac_from_ir(node: &IRNode) -> Option<Frac> {
    match node {
        IRNode::Integer(n) => Some((*n, 1)),
        IRNode::Rational(p, q) => frac_make(*p, *q),
        _ => None,
    }
}

/// Phase 33: if `arg = q·%pi` for a rational `q` with denominator in
/// `{1,2,3,4,6}`, return `q` as `(numer, denom)`.  Otherwise `None`.
///
/// Strategy 1: float ≈ q·π (for backends that evaluate %pi → float).
/// Strategy 2: structural match on `%pi`, `Neg(%pi)`, `Mul(n,%pi)`,
///   `Div(%pi,n)`, `Div(Mul(n,%pi),d)`.
fn try_pi_multiple(arg: &IRNode) -> Option<Frac> {
    // Strategy 1: float value ≈ q·π
    if let IRNode::Float(v) = arg {
        let qf = v / std::f64::consts::PI;
        for d in [1i64, 2, 3, 4, 6] {
            let p_cand = (qf * d as f64).round() as i64;
            if (qf * d as f64 - p_cand as f64).abs() < 1e-9 {
                return frac_make(p_cand, d);
            }
        }
        return None;
    }
    // Strategy 2: structural match
    let pi_sym = IRNode::Symbol("%pi".to_string());
    if *arg == pi_sym { return frac_make(1, 1); }
    let IRNode::Apply(apply) = arg else { return None; };

    // Neg(anything) — recurse and negate
    if apply.head == IRNode::Symbol(NEG.to_string()) && apply.args.len() == 1 {
        let inner = try_pi_multiple(&apply.args[0])?;
        return frac_make(-inner.0, inner.1);
    }

    // Mul(n, %pi) or Mul(%pi, n)
    if apply.head == IRNode::Symbol(MUL.to_string()) && apply.args.len() == 2 {
        let (a, b) = (&apply.args[0], &apply.args[1]);
        if *b == pi_sym { return frac_from_ir(a); }
        if *a == pi_sym { return frac_from_ir(b); }
    }

    // Div(%pi, n) or Div(Mul(n,%pi), d)
    if apply.head == IRNode::Symbol(DIV.to_string()) && apply.args.len() == 2 {
        let (num, den) = (&apply.args[0], &apply.args[1]);
        let df = frac_from_ir(den)?;
        if df.0 == 0 { return None; }
        // Div(%pi, n) → 1/df = (df.1, df.0)
        if *num == pi_sym {
            return frac_make(df.1, df.0);
        }
        // Div(Mul(n,%pi), d) or Div(Mul(%pi,n), d)
        if let IRNode::Apply(mul_apply) = num {
            if mul_apply.head == IRNode::Symbol(MUL.to_string()) && mul_apply.args.len() == 2 {
                let (ma, mb) = (&mul_apply.args[0], &mul_apply.args[1]);
                let coeff_node = if *mb == pi_sym { Some(ma) }
                                 else if *ma == pi_sym { Some(mb) }
                                 else { None }?;
                let nf = frac_from_ir(coeff_node)?;
                // nf / df = (nf.0 * df.1) / (nf.1 * df.0)
                return frac_make(nf.0.checked_mul(df.1)?, nf.1.checked_mul(df.0)?);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Phase 33: exact algebraic IR values for sin/cos/tan tables
// ---------------------------------------------------------------------------

/// `Div(Sqrt(n), d)` helper — produces the IR node `√n / d`.
fn p33_sqrt_over(n: i64, d: i64) -> IRNode {
    apply_node(DIV, vec![
        apply_node(SQRT, vec![IRNode::Integer(n)]),
        IRNode::Integer(d),
    ])
}

/// `Neg(expr)` helper.
fn p33_neg(v: IRNode) -> IRNode { apply_node(NEG, vec![v]) }

/// sin(q·π) for `(q.0, q.1)` in canonical form with denominator dividing 12.
/// Returns `None` if the key is not in the table (including q=1/2 period).
fn sin_pi_table(p: i64, q: i64) -> Option<IRNode> {
    // Entries for q ∈ [0/1, 11/6]  (16 entries, period 2).
    // Key: (p, q) in reduced form after (p mod 2q)/q reduction.
    match (p, q) {
        (0, 1)  => Some(IRNode::Integer(0)),
        (1, 6)  => Some(IRNode::Rational(1, 2)),
        (1, 4)  => Some(p33_sqrt_over(2, 2)),
        (1, 3)  => Some(p33_sqrt_over(3, 2)),
        (1, 2)  => Some(IRNode::Integer(1)),
        (2, 3)  => Some(p33_sqrt_over(3, 2)),
        (3, 4)  => Some(p33_sqrt_over(2, 2)),
        (5, 6)  => Some(IRNode::Rational(1, 2)),
        (1, 1)  => Some(IRNode::Integer(0)),
        (7, 6)  => Some(IRNode::Rational(-1, 2)),
        (5, 4)  => Some(p33_neg(p33_sqrt_over(2, 2))),
        (4, 3)  => Some(p33_neg(p33_sqrt_over(3, 2))),
        (3, 2)  => Some(IRNode::Integer(-1)),
        (5, 3)  => Some(p33_neg(p33_sqrt_over(3, 2))),
        (7, 4)  => Some(p33_neg(p33_sqrt_over(2, 2))),
        (11, 6) => Some(IRNode::Rational(-1, 2)),
        _       => None,
    }
}

/// cos(q·π) — same key convention as `sin_pi_table`.
fn cos_pi_table(p: i64, q: i64) -> Option<IRNode> {
    match (p, q) {
        (0, 1)  => Some(IRNode::Integer(1)),
        (1, 6)  => Some(p33_sqrt_over(3, 2)),
        (1, 4)  => Some(p33_sqrt_over(2, 2)),
        (1, 3)  => Some(IRNode::Rational(1, 2)),
        (1, 2)  => Some(IRNode::Integer(0)),
        (2, 3)  => Some(IRNode::Rational(-1, 2)),
        (3, 4)  => Some(p33_neg(p33_sqrt_over(2, 2))),
        (5, 6)  => Some(p33_neg(p33_sqrt_over(3, 2))),
        (1, 1)  => Some(IRNode::Integer(-1)),
        (7, 6)  => Some(p33_neg(p33_sqrt_over(3, 2))),
        (5, 4)  => Some(p33_neg(p33_sqrt_over(2, 2))),
        (4, 3)  => Some(IRNode::Rational(-1, 2)),
        (3, 2)  => Some(IRNode::Integer(0)),
        (5, 3)  => Some(IRNode::Rational(1, 2)),
        (7, 4)  => Some(p33_sqrt_over(2, 2)),
        (11, 6) => Some(p33_sqrt_over(3, 2)),
        _       => None,
    }
}

/// tan(q·π) — period π, key is `q mod 1` (denominator divides 6).
/// q = 1/2 is omitted (undefined); returns `None` for that case too.
fn tan_pi_table(p: i64, q: i64) -> Option<IRNode> {
    match (p, q) {
        (0, 1)  => Some(IRNode::Integer(0)),
        (1, 6)  => Some(apply_node(DIV, vec![
                       apply_node(SQRT, vec![IRNode::Integer(3)]),
                       IRNode::Integer(3),
                   ])),
        (1, 4)  => Some(IRNode::Integer(1)),
        (1, 3)  => Some(apply_node(SQRT, vec![IRNode::Integer(3)])),
        (2, 3)  => Some(p33_neg(apply_node(SQRT, vec![IRNode::Integer(3)]))),
        (3, 4)  => Some(IRNode::Integer(-1)),
        (5, 6)  => Some(p33_neg(apply_node(DIV, vec![
                       apply_node(SQRT, vec![IRNode::Integer(3)]),
                       IRNode::Integer(3),
                   ]))),
        _       => None,
    }
}

// ---------------------------------------------------------------------------
// Phase 29–33: Enhanced elementary handlers
// ---------------------------------------------------------------------------

/// Phase 29: Abs — idempotency, negation-strip, Mul(-1,x)-strip, even-power.
fn abs_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 1 { return IRNode::Apply(Box::new(expr)); }
        let inner = &expr.args[0];
        // Numeric fold: abs(n) = |n|.
        if let Some(v) = to_numeric(inner) {
            let f = v.to_f64().abs();
            // Try to preserve exact integer/rational form.
            match inner {
                IRNode::Integer(n) => return IRNode::Integer(n.abs()),
                IRNode::Rational(p, q) => return IRNode::Rational(p.abs(), *q),
                _ => return IRNode::Float(f),
            }
        }
        if !simplify { panic!("Abs requires a numeric argument: {expr}"); }
        // Rule 4a: abs(abs(x)) = abs(x)
        if let IRNode::Apply(inner_apply) = inner {
            if inner_apply.head == IRNode::Symbol("Abs".to_string()) {
                return inner.clone();
            }
            // Rule 4b: abs(-x) = abs(x)
            if inner_apply.head == IRNode::Symbol(NEG.to_string()) && inner_apply.args.len() == 1 {
                return vm.eval(apply_node("Abs", vec![inner_apply.args[0].clone()]));
            }
            // Rule 4c: abs(Mul(-1, x)) = abs(x)
            if inner_apply.head == IRNode::Symbol(MUL.to_string()) && inner_apply.args.len() == 2 {
                if inner_apply.args[0] == IRNode::Integer(-1) {
                    return vm.eval(apply_node("Abs", vec![inner_apply.args[1].clone()]));
                }
            }
            // Rule 4d: abs(x^{2k}) = x^{2k}  (even power ≥ 0 always)
            if inner_apply.head == IRNode::Symbol(POW.to_string()) && inner_apply.args.len() == 2 {
                if let IRNode::Integer(n) = &inner_apply.args[1] {
                    if *n >= 2 && n % 2 == 0 { return inner.clone(); }
                }
            }
        }
        IRNode::Apply(Box::new(expr))
    })
}

/// Phase 29: Sqrt — perfect-square fold, even-power algebraic rewrite.
fn sqrt_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 1 { return IRNode::Apply(Box::new(expr)); }
        let arg = &expr.args[0];
        if let Some(va) = to_numeric(arg) {
            if va == Numeric::Int(0) { return IRNode::Integer(0); }
            if va == Numeric::Int(1) { return IRNode::Integer(1); }
            let result = va.to_f64().sqrt();
            // Perfect-square detection: round(√n)² == n → return integer.
            let int_result = result.round() as i64;
            if (int_result as f64 * int_result as f64 - va.to_f64()).abs() < 1e-9 {
                return IRNode::Integer(int_result);
            }
            return IRNode::Float(result);
        }
        if !simplify { panic!("Sqrt requires a numeric argument: {expr}"); }
        // sqrt(x^{2k}) — split on parity of k.
        if let IRNode::Apply(inner) = arg {
            if inner.head == IRNode::Symbol(POW.to_string()) && inner.args.len() == 2 {
                let base = &inner.args[0];
                if let IRNode::Integer(n) = &inner.args[1] {
                    if *n > 0 && n % 2 == 0 {
                        let k = n / 2;
                        if k % 2 == 0 {
                            // k even → x^k ≥ 0 always, e.g. sqrt(x^4) = x^2.
                            return apply_node(POW, vec![base.clone(), IRNode::Integer(k)]);
                        }
                        // k odd → |x^k|
                        let inner_expr = if k == 1 {
                            base.clone()
                        } else {
                            apply_node(POW, vec![base.clone(), IRNode::Integer(k)])
                        };
                        return vm.eval(apply_node("Abs", vec![inner_expr]));
                    }
                }
            }
        }
        IRNode::Apply(Box::new(expr))
    })
}

/// Phase 30: Log — special value log(1)=0, log(exp(x))=x cancellation.
fn log_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 1 { return IRNode::Apply(Box::new(expr)); }
        let arg = &expr.args[0];
        if let Some(va) = to_numeric(arg) {
            if va == Numeric::Int(1) { return IRNode::Integer(0); }
            let f = va.to_f64();
            if f <= 0.0 { return IRNode::Apply(Box::new(expr)); } // undefined for non-positive
            return IRNode::Float(f.ln());
        }
        if !simplify { panic!("Log requires a numeric argument: {expr}"); }
        // log(exp(x)) = x  (structural cancellation).
        if let IRNode::Apply(inner) = arg {
            if inner.head == IRNode::Symbol(EXP.to_string()) && inner.args.len() == 1 {
                return inner.args[0].clone();
            }
        }
        // Note: log(x^n) = n·log(x) requires assumption context; skipped here.
        IRNode::Apply(Box::new(expr))
    })
}

/// Phase 30: Exp — special value exp(0)=1, exp(log(x))=x, exp(n·log(x))=x^n.
fn exp_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 1 { return IRNode::Apply(Box::new(expr)); }
        let arg = &expr.args[0];
        if let Some(va) = to_numeric(arg) {
            if va == Numeric::Int(0) { return IRNode::Integer(1); }
            return IRNode::Float(va.to_f64().exp());
        }
        if !simplify { panic!("Exp requires a numeric argument: {expr}"); }
        if let IRNode::Apply(inner) = arg {
            // exp(log(x)) = x
            if inner.head == IRNode::Symbol(LOG.to_string()) && inner.args.len() == 1 {
                return inner.args[0].clone();
            }
            // exp(n·log(x)) = x^n  — handles both Mul(n, log(x)) and Mul(log(x), n).
            if inner.head == IRNode::Symbol(MUL.to_string()) && inner.args.len() == 2 {
                let (a, b) = (&inner.args[0], &inner.args[1]);
                let is_log_a = matches!(a, IRNode::Apply(ap) if ap.head == IRNode::Symbol(LOG.to_string()) && ap.args.len() == 1);
                let is_log_b = matches!(b, IRNode::Apply(ap) if ap.head == IRNode::Symbol(LOG.to_string()) && ap.args.len() == 1);
                if is_log_a {
                    if let IRNode::Apply(log_ap) = a { return apply_node(POW, vec![log_ap.args[0].clone(), b.clone()]); }
                }
                if is_log_b {
                    if let IRNode::Apply(log_ap) = b { return apply_node(POW, vec![log_ap.args[0].clone(), a.clone()]); }
                }
            }
        }
        IRNode::Apply(Box::new(expr))
    })
}

/// Phase 31+33: Sin — odd symmetry, arc-cancellation, π-multiple exact values.
fn sin_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 1 { return IRNode::Apply(Box::new(expr)); }
        let arg = &expr.args[0];
        // Rule 4 (Phase 33): π-multiple exact values — checked first.
        if let Some((p, q)) = try_pi_multiple(arg) {
            if let Some((rp, rq)) = frac_mod(p, q, 2) {
                if let Some(val) = sin_pi_table(rp, rq) { return val; }
            }
        }
        if let Some(va) = to_numeric(arg) {
            if va == Numeric::Int(0) { return IRNode::Integer(0); }
            return IRNode::Float(va.to_f64().sin());
        }
        if !simplify { panic!("Sin requires a numeric argument: {expr}"); }
        if let IRNode::Apply(inner) = arg {
            // Rule 2 (Phase 31): odd symmetry — sin(-x) = -sin(x).
            if inner.head == IRNode::Symbol(NEG.to_string()) && inner.args.len() == 1 {
                let inner_sin = vm.eval(apply_node(SIN, vec![inner.args[0].clone()]));
                return apply_node(NEG, vec![inner_sin]);
            }
            // Rule 3 (Phase 31): arc-cancellation — sin(asin(x)) = x.
            if inner.head == IRNode::Symbol(ASIN.to_string()) && inner.args.len() == 1 {
                return inner.args[0].clone();
            }
        }
        IRNode::Apply(Box::new(expr))
    })
}

/// Phase 31+33: Cos — even symmetry, arc-cancellation, π-multiple exact values.
fn cos_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 1 { return IRNode::Apply(Box::new(expr)); }
        let arg = &expr.args[0];
        // Rule 4 (Phase 33): π-multiple.
        if let Some((p, q)) = try_pi_multiple(arg) {
            if let Some((rp, rq)) = frac_mod(p, q, 2) {
                if let Some(val) = cos_pi_table(rp, rq) { return val; }
            }
        }
        if let Some(va) = to_numeric(arg) {
            if va == Numeric::Int(0) { return IRNode::Integer(1); }
            return IRNode::Float(va.to_f64().cos());
        }
        if !simplify { panic!("Cos requires a numeric argument: {expr}"); }
        if let IRNode::Apply(inner) = arg {
            // Rule 2 (Phase 31): even symmetry — cos(-x) = cos(x).
            if inner.head == IRNode::Symbol(NEG.to_string()) && inner.args.len() == 1 {
                return vm.eval(apply_node(COS, vec![inner.args[0].clone()]));
            }
            // Rule 3 (Phase 31): arc-cancellation — cos(acos(x)) = x.
            if inner.head == IRNode::Symbol(ACOS.to_string()) && inner.args.len() == 1 {
                return inner.args[0].clone();
            }
        }
        IRNode::Apply(Box::new(expr))
    })
}

/// Phase 31+33: Tan — odd symmetry, arc-cancellation, π-multiple exact values.
fn tan_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 1 { return IRNode::Apply(Box::new(expr)); }
        let arg = &expr.args[0];
        // Rule 4 (Phase 33): π-multiple.
        if let Some((p, q)) = try_pi_multiple(arg) {
            // tan(-q·π) = -tan(q·π): handle via sign.
            let (sign, p_abs) = if p < 0 { (-1i64, -p) } else { (1, p) };
            if let Some((rp, rq)) = frac_mod(p_abs, q, 1) {
                if let Some(val) = tan_pi_table(rp, rq) {
                    return if sign < 0 { apply_node(NEG, vec![val]) } else { val };
                }
                // rp/rq = 1/2 → undefined, fall through
            }
        }
        if let Some(va) = to_numeric(arg) {
            if va == Numeric::Int(0) { return IRNode::Integer(0); }
            return IRNode::Float(va.to_f64().tan());
        }
        if !simplify { panic!("Tan requires a numeric argument: {expr}"); }
        if let IRNode::Apply(inner) = arg {
            // Rule 2 (Phase 31): odd symmetry — tan(-x) = -tan(x).
            if inner.head == IRNode::Symbol(NEG.to_string()) && inner.args.len() == 1 {
                let inner_tan = vm.eval(apply_node(TAN, vec![inner.args[0].clone()]));
                return apply_node(NEG, vec![inner_tan]);
            }
            // Rule 3 (Phase 31): arc-cancellation — tan(atan(x)) = x.
            if inner.head == IRNode::Symbol(ATAN.to_string()) && inner.args.len() == 1 {
                return inner.args[0].clone();
            }
        }
        IRNode::Apply(Box::new(expr))
    })
}

/// Phase 31: Sinh — odd symmetry, arc-cancellation sinh(asinh(x)) = x.
fn sinh_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 1 { return IRNode::Apply(Box::new(expr)); }
        let arg = &expr.args[0];
        if let Some(va) = to_numeric(arg) {
            if va == Numeric::Int(0) { return IRNode::Integer(0); }
            return IRNode::Float(va.to_f64().sinh());
        }
        if !simplify { panic!("Sinh requires a numeric argument: {expr}"); }
        if let IRNode::Apply(inner) = arg {
            if inner.head == IRNode::Symbol(NEG.to_string()) && inner.args.len() == 1 {
                let inner_sinh = vm.eval(apply_node(SINH, vec![inner.args[0].clone()]));
                return apply_node(NEG, vec![inner_sinh]);
            }
            if inner.head == IRNode::Symbol(ASINH.to_string()) && inner.args.len() == 1 {
                return inner.args[0].clone();
            }
        }
        IRNode::Apply(Box::new(expr))
    })
}

/// Phase 31: Cosh — even symmetry, arc-cancellation cosh(acosh(x)) = x.
fn cosh_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 1 { return IRNode::Apply(Box::new(expr)); }
        let arg = &expr.args[0];
        if let Some(va) = to_numeric(arg) {
            if va == Numeric::Int(0) { return IRNode::Integer(1); }
            return IRNode::Float(va.to_f64().cosh());
        }
        if !simplify { panic!("Cosh requires a numeric argument: {expr}"); }
        if let IRNode::Apply(inner) = arg {
            if inner.head == IRNode::Symbol(NEG.to_string()) && inner.args.len() == 1 {
                return vm.eval(apply_node(COSH, vec![inner.args[0].clone()]));
            }
            if inner.head == IRNode::Symbol(ACOSH.to_string()) && inner.args.len() == 1 {
                return inner.args[0].clone();
            }
        }
        IRNode::Apply(Box::new(expr))
    })
}

/// Phase 31: Tanh — odd symmetry, arc-cancellation tanh(atanh(x)) = x.
fn tanh_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 1 { return IRNode::Apply(Box::new(expr)); }
        let arg = &expr.args[0];
        if let Some(va) = to_numeric(arg) {
            if va == Numeric::Int(0) { return IRNode::Integer(0); }
            return IRNode::Float(va.to_f64().tanh());
        }
        if !simplify { panic!("Tanh requires a numeric argument: {expr}"); }
        if let IRNode::Apply(inner) = arg {
            if inner.head == IRNode::Symbol(NEG.to_string()) && inner.args.len() == 1 {
                let inner_tanh = vm.eval(apply_node(TANH, vec![inner.args[0].clone()]));
                return apply_node(NEG, vec![inner_tanh]);
            }
            if inner.head == IRNode::Symbol(ATANH.to_string()) && inner.args.len() == 1 {
                return inner.args[0].clone();
            }
        }
        IRNode::Apply(Box::new(expr))
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

/// Phase 32: Atan — odd symmetry.
fn atan_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 1 { return IRNode::Apply(Box::new(expr)); }
        let arg = &expr.args[0];
        if let Some(va) = to_numeric(arg) {
            if va == Numeric::Int(0) { return IRNode::Integer(0); }
            return IRNode::Float(va.to_f64().atan());
        }
        if !simplify { panic!("Atan requires a numeric argument: {expr}"); }
        if let IRNode::Apply(inner) = arg {
            if inner.head == IRNode::Symbol(NEG.to_string()) && inner.args.len() == 1 {
                let inner_atan = vm.eval(apply_node(ATAN, vec![inner.args[0].clone()]));
                return apply_node(NEG, vec![inner_atan]);
            }
        }
        IRNode::Apply(Box::new(expr))
    })
}

/// Phase 32: Asin — odd symmetry.
fn asin_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 1 { return IRNode::Apply(Box::new(expr)); }
        let arg = &expr.args[0];
        if let Some(va) = to_numeric(arg) {
            if va == Numeric::Int(0) { return IRNode::Integer(0); }
            return IRNode::Float(va.to_f64().asin());
        }
        if !simplify { panic!("Asin requires a numeric argument: {expr}"); }
        if let IRNode::Apply(inner) = arg {
            if inner.head == IRNode::Symbol(NEG.to_string()) && inner.args.len() == 1 {
                let inner_asin = vm.eval(apply_node(ASIN, vec![inner.args[0].clone()]));
                return apply_node(NEG, vec![inner_asin]);
            }
        }
        IRNode::Apply(Box::new(expr))
    })
}

/// Phase 32: Acos — reflection identity acos(-x) = %pi − acos(x).
fn acos_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 1 { return IRNode::Apply(Box::new(expr)); }
        let arg = &expr.args[0];
        if let Some(va) = to_numeric(arg) {
            if va == Numeric::Int(1) { return IRNode::Integer(0); }
            return IRNode::Float(va.to_f64().acos());
        }
        if !simplify { panic!("Acos requires a numeric argument: {expr}"); }
        if let IRNode::Apply(inner) = arg {
            if inner.head == IRNode::Symbol(NEG.to_string()) && inner.args.len() == 1 {
                let inner_acos = vm.eval(apply_node(ACOS, vec![inner.args[0].clone()]));
                // acos(-x) = %pi - acos(x)
                return apply_node(SUB, vec![
                    IRNode::Symbol("%pi".to_string()),
                    inner_acos,
                ]);
            }
        }
        IRNode::Apply(Box::new(expr))
    })
}

/// Phase 32: Asinh — odd symmetry.
fn asinh_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 1 { return IRNode::Apply(Box::new(expr)); }
        let arg = &expr.args[0];
        if let Some(va) = to_numeric(arg) {
            if va == Numeric::Int(0) { return IRNode::Integer(0); }
            return IRNode::Float(va.to_f64().asinh());
        }
        if !simplify { panic!("Asinh requires a numeric argument: {expr}"); }
        if let IRNode::Apply(inner) = arg {
            if inner.head == IRNode::Symbol(NEG.to_string()) && inner.args.len() == 1 {
                let inner_asinh = vm.eval(apply_node(ASINH, vec![inner.args[0].clone()]));
                return apply_node(NEG, vec![inner_asinh]);
            }
        }
        IRNode::Apply(Box::new(expr))
    })
}

/// Acosh — numeric fold only (domain [1, ∞), no real symmetry rule).
fn acosh_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        single_trig(&expr, "Acosh", f64::acosh, &[(Numeric::Int(1), IRNode::Integer(0))], simplify)
    })
}

/// Phase 32: Atanh — odd symmetry.
fn atanh_handler(simplify: bool) -> Handler {
    std::sync::Arc::new(move |vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 1 { return IRNode::Apply(Box::new(expr)); }
        let arg = &expr.args[0];
        if let Some(va) = to_numeric(arg) {
            if va == Numeric::Int(0) { return IRNode::Integer(0); }
            return IRNode::Float(va.to_f64().atanh());
        }
        if !simplify { panic!("Atanh requires a numeric argument: {expr}"); }
        if let IRNode::Apply(inner) = arg {
            if inner.head == IRNode::Symbol(NEG.to_string()) && inner.args.len() == 1 {
                let inner_atanh = vm.eval(apply_node(ATANH, vec![inner.args[0].clone()]));
                return apply_node(NEG, vec![inner_atanh]);
            }
        }
        IRNode::Apply(Box::new(expr))
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

// ---------------------------------------------------------------------------
// Phase 34 — Weierstrass substitution for ∫ c/(a + b·sin(x)) dx and
// ∫ c/(a + b·cos(x)) dx with rational a, b satisfying a² > b² (a > 0 for cos).
// Mirrors Python symbolic-vm 0.59.0 and TypeScript symbolic-vm 0.7.0.
//
// Closed forms:
//   ∫ 1/(a + b·sin x) dx = (2/√(a²−b²)) · arctan((a·tan(x/2) + b)/√(a²−b²))
//   ∫ 1/(a + b·cos x) dx = (2/√(a²−b²)) · arctan(√((a−b)/(a+b)) · tan(x/2))
//
// Discriminant cases a² ≤ b² and a ≤ 0 for cos are deferred — they need
// sign analysis the assumption-free port cannot perform symbolically.
// ---------------------------------------------------------------------------

/// Convert an IRNode to a `RatC` rational if it's an Integer or Rational
/// literal.  Returns None for Float, Symbol, Apply.
fn node_to_rc(node: &IRNode) -> Option<RatC> {
    match node {
        IRNode::Integer(n) => Some((*n as i128, 1)),
        IRNode::Rational(n, d) => rc(*n as i128, *d as i128),
        _ => None,
    }
}

/// Build the IR for `√(rc)`, folding to a plain rational when both numerator
/// and denominator are perfect integer squares.  Wraps in `Sqrt` otherwise.
/// `rc` must be strictly positive — callers guard the discriminant first.
fn weierstrass_sqrt_fraction_ir(rc_val: RatC) -> IRNode {
    let (p, q) = rc_val;
    if p <= 0 || q <= 0 {
        // Defensive — produce a Sqrt of whatever rational we got.
        return apply_node(
            SQRT,
            vec![rc_to_ir(rc_val).unwrap_or(IRNode::Float(rc_val.0 as f64 / rc_val.1 as f64))],
        );
    }
    if let (Some(p_root), Some(q_root)) = (i128_sqrt(p), i128_sqrt(q)) {
        if let Some(ir) = rc_to_ir((p_root, q_root)) {
            return ir;
        }
    }
    // Not a perfect-square pair — emit Sqrt(rational).
    let inside = rc_to_ir(rc_val).unwrap_or(IRNode::Float(p as f64 / q as f64));
    apply_node(SQRT, vec![inside])
}

/// Phase 38: Parse a linear-in-`x` rational expression `α·x + β` and return
/// `(α, β)` with `α, β ∈ ℚ` and `α ≠ 0`.  Recognised shapes (any operand
/// ordering within commutative heads):
///
/// | Shape          | Returns   |
/// |----------------|-----------|
/// | `x`            | `(1, 0)`  |
/// | `α·x`          | `(α, 0)`  |
/// | `α·x + β`      | `(α, β)`  |
/// | `β + α·x`      | `(α, β)`  |
/// | `α·x − β`      | `(α, −β)` |
/// | `β − α·x`      | `(−α, β)` |
/// | `−(α·x + β)`   | `(−α, −β)`|
///
/// Returns `None` when the expression is not linear in `x` (e.g. `x²`,
/// `sin(x)`, pure constants, or nested nonlinear shapes).  `α = 0` is
/// filtered out so callers may rely on `α ≠ 0` throughout.
fn weierstrass_parse_linear_in_x(node: &IRNode, x: &str) -> Option<(RatC, RatC)> {
    let target = IRNode::Symbol(x.to_string());
    if node == &target {
        return Some(((1, 1), (0, 1)));
    }
    // Pure constants free of x are not linear-in-x (no x term).
    if !depends_on(node, x) {
        return None;
    }
    let IRNode::Apply(apply) = node else {
        return None;
    };
    // α·x — Mul(constant, x) in either order.  α = 0 is rejected.
    if apply.head == IRNode::Symbol(MUL.to_string()) && apply.args.len() == 2 {
        let (left, right) = (&apply.args[0], &apply.args[1]);
        if let Some(c_left) = node_to_rc(left) {
            if right == &target && c_left.0 != 0 {
                return Some((c_left, (0, 1)));
            }
        }
        if let Some(c_right) = node_to_rc(right) {
            if left == &target && c_right.0 != 0 {
                return Some((c_right, (0, 1)));
            }
        }
        return None;
    }
    // −(linear) — recurse and negate both coefficients.
    if apply.head == IRNode::Symbol(NEG.to_string()) && apply.args.len() == 1 {
        let inner = weierstrass_parse_linear_in_x(&apply.args[0], x)?;
        return Some((rc_neg(inner.0), rc_neg(inner.1)));
    }
    // ADD: constant + linear or linear + constant.
    if apply.head == IRNode::Symbol(ADD.to_string()) && apply.args.len() == 2 {
        let (left, right) = (&apply.args[0], &apply.args[1]);
        for (const_side, lin_side) in [(left, right), (right, left)] {
            let Some(c) = node_to_rc(const_side) else {
                continue;
            };
            let Some((alpha, beta)) = weierstrass_parse_linear_in_x(lin_side, x) else {
                continue;
            };
            return rc_add(beta, c).map(|sum| (alpha, sum));
        }
        return None;
    }
    // SUB: two cases depending on which side carries `x`.
    if apply.head == IRNode::Symbol(SUB.to_string()) && apply.args.len() == 2 {
        let (left, right) = (&apply.args[0], &apply.args[1]);
        // Case A: linear − constant → (α, β − c)
        if let Some(c_right) = node_to_rc(right) {
            if let Some((alpha, beta)) = weierstrass_parse_linear_in_x(left, x) {
                return rc_sub(beta, c_right).map(|d| (alpha, d));
            }
        }
        // Case B: constant − linear → (−α, c − β)
        if let Some(c_left) = node_to_rc(left) {
            if let Some((alpha, beta)) = weierstrass_parse_linear_in_x(right, x) {
                return rc_sub(c_left, beta).map(|d| (rc_neg(alpha), d));
            }
        }
    }
    None
}

/// Phase 38: Build the IR node for `α·x + β`, collapsing trivial cases so the
/// emitted `tan(arg/2)` carries the simplest equivalent argument.
///
/// - `α = 1, β = 0` → `x`     (bit-for-bit identical to the Phase 34 bare path)
/// - `α = 1, β ≠ 0` → `x + β`
/// - `β = 0, α ≠ 1` → `α·x`
/// - otherwise       → `α·x + β`
fn weierstrass_build_linear_arg_ir(alpha: RatC, beta: RatC, x: &str) -> Option<IRNode> {
    let alpha_is_one = alpha == (1, 1);
    let beta_is_zero = beta.0 == 0;
    let x_node = IRNode::Symbol(x.to_string());
    if alpha_is_one && beta_is_zero {
        return Some(x_node);
    }
    if beta_is_zero {
        return Some(apply_node(MUL, vec![rc_to_ir(alpha)?, x_node]));
    }
    if alpha_is_one {
        return Some(apply_node(ADD, vec![x_node, rc_to_ir(beta)?]));
    }
    let ax = apply_node(MUL, vec![rc_to_ir(alpha)?, x_node]);
    Some(apply_node(ADD, vec![ax, rc_to_ir(beta)?]))
}

/// Phase 38: match `c·sin(α·x + β)` / `c·cos(α·x + β)` (and the c=1 / α=1 /
/// β=0 degenerate variants) and return `(c, head_str, α, β)`.
///
/// Accepts both argument orders within `Mul` and unwraps a leading `Neg`.
/// The trig argument must be linear in `x` per
/// [`weierstrass_parse_linear_in_x`].  Supersedes the Phase 34 bare-`x`
/// predecessor `weierstrass_parse_const_times_trig_x`.
fn weierstrass_parse_const_times_trig_linear(
    node: &IRNode,
    x: &str,
) -> Option<(RatC, &'static str, RatC, RatC)> {
    if let IRNode::Apply(apply) = node {
        // Bare sin(arg) / cos(arg) — coefficient is 1.
        if apply.args.len() == 1 {
            let head_str = if apply.head == IRNode::Symbol(SIN.to_string()) {
                Some(SIN)
            } else if apply.head == IRNode::Symbol(COS.to_string()) {
                Some(COS)
            } else {
                None
            };
            if let Some(head) = head_str {
                if let Some((alpha, beta)) = weierstrass_parse_linear_in_x(&apply.args[0], x) {
                    return Some(((1, 1), head, alpha, beta));
                }
            }
        }
        // Mul(c, sin/cos(arg)) or Mul(sin/cos(arg), c)
        if apply.head == IRNode::Symbol(MUL.to_string()) && apply.args.len() == 2 {
            let (a, b) = (&apply.args[0], &apply.args[1]);
            for (const_side, trig_side) in [(a, b), (b, a)] {
                let Some(c) = node_to_rc(const_side) else {
                    continue;
                };
                let IRNode::Apply(trig) = trig_side else {
                    continue;
                };
                if trig.args.len() != 1 {
                    continue;
                }
                let head_str = if trig.head == IRNode::Symbol(SIN.to_string()) {
                    Some(SIN)
                } else if trig.head == IRNode::Symbol(COS.to_string()) {
                    Some(COS)
                } else {
                    None
                };
                let Some(head) = head_str else { continue };
                if let Some((alpha, beta)) = weierstrass_parse_linear_in_x(&trig.args[0], x) {
                    return Some((c, head, alpha, beta));
                }
            }
        }
        // Neg(inner) — recurse and negate `c`.
        if apply.head == IRNode::Symbol(NEG.to_string()) && apply.args.len() == 1 {
            if let Some((c, head, alpha, beta)) =
                weierstrass_parse_const_times_trig_linear(&apply.args[0], x)
            {
                return Some((rc_neg(c), head, alpha, beta));
            }
        }
    }
    None
}

/// Parse `a + b·sin(α·x+β)` / `a + b·cos(α·x+β)` (any operand order, plus the
/// SUB-headed variant) into `(a, b, trig_head_str, α, β)`.  Phase 38 generalises
/// the Phase 34 bare-`x` predecessor.
fn weierstrass_parse_a_plus_b_sincos(
    node: &IRNode,
    x: &str,
) -> Option<(RatC, RatC, &'static str, RatC, RatC)> {
    let IRNode::Apply(apply) = node else {
        return None;
    };
    if apply.args.len() != 2 {
        return None;
    }
    let (left, right) = (&apply.args[0], &apply.args[1]);
    if apply.head == IRNode::Symbol(ADD.to_string()) {
        for (const_side, trig_side) in [(left, right), (right, left)] {
            if let Some(a_rc) = node_to_rc(const_side) {
                if let Some((b_rc, head, alpha, beta)) =
                    weierstrass_parse_const_times_trig_linear(trig_side, x)
                {
                    return Some((a_rc, b_rc, head, alpha, beta));
                }
            }
        }
        return None;
    }
    if apply.head == IRNode::Symbol(SUB.to_string()) {
        // a − b·trig(...) = a + (−b)·trig(...)
        if let Some(a_left) = node_to_rc(left) {
            if let Some((b_rc, head, alpha, beta)) =
                weierstrass_parse_const_times_trig_linear(right, x)
            {
                return Some((a_left, rc_neg(b_rc), head, alpha, beta));
            }
        }
        // b·trig(...) − a = −a + b·trig(...)
        if let Some((b_left, head, alpha, beta)) =
            weierstrass_parse_const_times_trig_linear(left, x)
        {
            if let Some(a_right) = node_to_rc(right) {
                return Some((rc_neg(a_right), b_left, head, alpha, beta));
            }
        }
    }
    None
}

/// Phase 35: degenerate `a² = b²` Weierstrass cases.  Four sign
/// combinations × {SIN, COS} yield clean closed forms in `tan(x/2)`
/// without any `Sqrt` or `Atan` wrapper:
///
/// - sin, b ==  a : ``∫ c/(a + a·sin x) dx = -2c / (a · (tan(x/2) + 1))``
/// - sin, b == -a : ``∫ c/(a − a·sin x) dx =  2c / (a · (1 − tan(x/2)))``
/// - cos, b ==  a : ``∫ c/(a + a·cos x) dx =  c · tan(x/2) / a``
/// - cos, b == -a : ``∫ c/(a − a·cos x) dx = -c / (a · tan(x/2))``  (= -c·cot(x/2)/a)
///
/// Returns `None` when neither `b == a` nor `b == -a` (i.e. when the
/// caller has reached `disc == 0` via a path that the matcher can't
/// close), or when `a == 0` (the integrand `c/0` is not integrable).
fn try_weierstrass_degenerate(
    c: RatC,
    a: RatC,
    b: RatC,
    trig_head: &'static str,
    arg_node: &IRNode,
) -> Option<IRNode> {
    // `RatC` is in lowest terms; (n, d) with d > 0 and gcd(|n|, d) = 1.
    // a == 0 iff a.0 == 0.  Same shape check works for b.
    if a.0 == 0 {
        return None;
    }
    // Phase 38: `arg_node` is the IR for `α·x + β`; the inner factor `α`
    // has been pre-absorbed into `c` by the caller, so the closed form's
    // shape is identical to the bare-`x` case with `tan(arg/2)` in place
    // of `tan(x/2)`.
    let tan_half = apply_node(
        TAN,
        vec![apply_node(DIV, vec![arg_node.clone(), IRNode::Integer(2)])],
    );
    // Helper closures.
    let two_c = rc_mul(c, (2, 1))?;
    let neg_two_c = rc_neg(two_c);
    let neg_c = rc_neg(c);
    if trig_head == SIN {
        if b == a {
            // -2c / (a · (tan(x/2) + 1))
            let denom = apply_node(
                MUL,
                vec![rc_to_ir(a)?, apply_node(ADD, vec![tan_half, IRNode::Integer(1)])],
            );
            return Some(apply_node(DIV, vec![rc_to_ir(neg_two_c)?, denom]));
        }
        if b == rc_neg(a) {
            // 2c / (a · (1 − tan(x/2)))
            let denom = apply_node(
                MUL,
                vec![rc_to_ir(a)?, apply_node(SUB, vec![IRNode::Integer(1), tan_half])],
            );
            return Some(apply_node(DIV, vec![rc_to_ir(two_c)?, denom]));
        }
        return None;
    }
    // trig_head == COS
    if b == a {
        // c · tan(x/2) / a
        let numer = apply_node(MUL, vec![rc_to_ir(c)?, tan_half]);
        return Some(apply_node(DIV, vec![numer, rc_to_ir(a)?]));
    }
    if b == rc_neg(a) {
        // -c / (a · tan(x/2))
        let denom = apply_node(MUL, vec![rc_to_ir(a)?, tan_half]);
        return Some(apply_node(DIV, vec![rc_to_ir(neg_c)?, denom]));
    }
    None
}

/// Phase 36: Weierstrass log form for `a² < b²`.  Returns the closed
/// form with `log|·|` instead of `arctan(·)`.  Sin branch handles any
/// nonzero `a`; cos branch requires `b > |a|` strictly (the symmetric
/// `b < −|a|` case has the opposite sign pattern and is deferred).
fn try_weierstrass_log_form(
    c: RatC,
    a: RatC,
    b: RatC,
    trig_head: &'static str,
    arg_node: &IRNode,
) -> Option<IRNode> {
    // disc_sq = b² − a² > 0 (caller passed disc = a² − b² < 0).
    let b_sq = rc_mul(b, b)?;
    let a_sq = rc_mul(a, a)?;
    let disc_sq = rc_sub(b_sq, a_sq)?;
    if disc_sq.0 <= 0 {
        return None;
    }
    let sqrt_disc_ir = weierstrass_sqrt_fraction_ir(disc_sq);
    // Phase 38: `arg_node` is the IR for `α·x + β`; the inner factor
    // `α` has been pre-absorbed into `c` by the caller.
    let tan_half = apply_node(
        TAN,
        vec![apply_node(DIV, vec![arg_node.clone(), IRNode::Integer(2)])],
    );
    if trig_head == SIN {
        if a.0 == 0 {
            // 1/(b·sin x) — defer to the elementary table.
            return None;
        }
        // log|(a·tan(x/2) + b − D) / (a·tan(x/2) + b + D)|
        let a_tan = apply_node(MUL, vec![rc_to_ir(a)?, tan_half]);
        let a_tan_plus_b = apply_node(ADD, vec![a_tan, rc_to_ir(b)?]);
        let numer = apply_node(SUB, vec![a_tan_plus_b.clone(), sqrt_disc_ir.clone()]);
        let denom = apply_node(ADD, vec![a_tan_plus_b, sqrt_disc_ir.clone()]);
        let log_arg = apply_node("Abs", vec![apply_node(DIV, vec![numer, denom])]);
        let coef_ir = apply_node(DIV, vec![rc_to_ir(c)?, sqrt_disc_ir]);
        return Some(apply_node(MUL, vec![coef_ir, apply_node(LOG, vec![log_arg])]));
    }
    // COS branch — handles both b > |a| and b < −|a| (Phase 37 extension).
    //
    // The same expression log|(D + (b−a)·tan(x/2)) / (D − (b−a)·tan(x/2))|
    // is valid for both sign regimes because the inner rational is wrapped
    // in Abs: when (b−a) flips sign, the numerator and denominator of the
    // log argument swap (D − k·u and D + k·u), but |N/D'| = |D'/N| so the
    // absolute value collapses them to the same value.
    //
    // Caller already ensures b² > a² (disc < 0 entry); the only additional
    // precondition is a + b ≠ 0, which is automatic because b² > a² rules
    // out b = −a.
    let b_minus_a = rc_sub(b, a)?;
    // log|(D + (b−a)·tan(x/2)) / (D − (b−a)·tan(x/2))|
    let bma_tan = apply_node(MUL, vec![rc_to_ir(b_minus_a)?, tan_half]);
    let numer = apply_node(ADD, vec![sqrt_disc_ir.clone(), bma_tan.clone()]);
    let denom = apply_node(SUB, vec![sqrt_disc_ir.clone(), bma_tan]);
    let log_arg = apply_node("Abs", vec![apply_node(DIV, vec![numer, denom])]);
    let coef_ir = apply_node(DIV, vec![rc_to_ir(c)?, sqrt_disc_ir]);
    Some(apply_node(MUL, vec![coef_ir, apply_node(LOG, vec![log_arg])]))
}

/// Phase 34 + 38 entry point.  Returns the closed form when the integrand is
/// `c / (a + b·sin/cos(α·x + β))` with rational c, a, b, α, β (α ≠ 0) and the
/// relevant discriminant guard for each branch, or `None` otherwise.  When
/// `α = 1` and `β = 0`, this is bit-for-bit identical to the original Phase
/// 34/35/36/37 behaviour.
fn try_weierstrass_one_over_linear_trig(
    num: &IRNode,
    den: &IRNode,
    x: &str,
) -> Option<IRNode> {
    let c_in = node_to_rc(num)?;
    let (a_rc, b_rc, trig_head, alpha, beta) = weierstrass_parse_a_plus_b_sincos(den, x)?;
    // Phase 38: fold the inner substitution u = α·x + β (du = α·dx) into the
    // numerator constant once at entry: c ← c/α.  `α ≠ 0` is guaranteed by
    // `weierstrass_parse_linear_in_x`.  Every branch below uses the original
    // closed-form formulas with `tan(arg/2)` substituted for `tan(x/2)`.
    let c_rc = rc_div(c_in, alpha)?;
    let arg_node = weierstrass_build_linear_arg_ir(alpha, beta, x)?;
    // disc = a² − b².  Three sub-cases:
    //   disc > 0  → Phase 34 arctan form (below)
    //   disc == 0 → Phase 35 degenerate form (four sign combinations)
    //   disc < 0  → Phase 36/37 log form
    let a_sq = rc_mul(a_rc, a_rc)?;
    let b_sq = rc_mul(b_rc, b_rc)?;
    let disc = rc_sub(a_sq, b_sq)?;
    if disc.0 == 0 {
        return try_weierstrass_degenerate(c_rc, a_rc, b_rc, trig_head, &arg_node);
    }
    if disc.0 < 0 {
        // Phase 36/37: a² < b² → log form via partial fractions on the two
        // distinct real roots of the quadratic in tan(arg/2).
        return try_weierstrass_log_form(c_rc, a_rc, b_rc, trig_head, &arg_node);
    }
    let sqrt_disc_ir = weierstrass_sqrt_fraction_ir(disc);
    let tan_half = apply_node(
        TAN,
        vec![apply_node(DIV, vec![arg_node.clone(), IRNode::Integer(2)])],
    );
    let atan_arg = if trig_head == SIN {
        // (a·tan(x/2) + b) / √disc
        let a_ir = rc_to_ir(a_rc)?;
        let b_ir = rc_to_ir(b_rc)?;
        let top = apply_node(
            ADD,
            vec![apply_node(MUL, vec![a_ir, tan_half]), b_ir],
        );
        apply_node(DIV, vec![top, sqrt_disc_ir.clone()])
    } else {
        // cos branch — require a > 0 to avoid sign-flip.
        if a_rc.0 <= 0 {
            return None;
        }
        let ratio = rc_div(rc_sub(a_rc, b_rc)?, rc_add(a_rc, b_rc)?)?;
        if ratio.0 <= 0 {
            return None;
        }
        let sqrt_ratio_ir = weierstrass_sqrt_fraction_ir(ratio);
        apply_node(MUL, vec![sqrt_ratio_ir, tan_half])
    };
    // Outer coefficient: 2c / √disc
    let two_c = rc_mul(c_rc, (2, 1))?;
    let coef_ir = apply_node(DIV, vec![rc_to_ir(two_c)?, sqrt_disc_ir]);
    Some(apply_node(MUL, vec![coef_ir, apply_node(ATAN, vec![atan_arg])]))
}

fn integrate_handler() -> Handler {
    std::sync::Arc::new(move |vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 2 && expr.args.len() != 4 {
            panic!(
                "Integrate expects 2 or 4 arguments, got {}",
                expr.args.len()
            );
        }

        let f = expr.args[0].clone();
        let x = match &expr.args[1] {
            IRNode::Symbol(s) => s.clone(),
            _ => return IRNode::Apply(Box::new(expr)),
        };

        if expr.args.len() == 4 {
            if let Some(result) = complete_elliptic_first_kind(&f, &x, &expr.args[2], &expr.args[3]) {
                return vm.eval(result);
            }
            if let Some(result) = complete_elliptic_second_kind(&f, &x, &expr.args[2], &expr.args[3]) {
                return vm.eval(result);
            }
            if let Some(result) = complete_elliptic_third_kind(&f, &x, &expr.args[2], &expr.args[3]) {
                return vm.eval(result);
            }
            return IRNode::Apply(Box::new(expr));
        }

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

    if let Some(result) = incomplete_elliptic_first_kind(f, x) {
        return result;
    }

    if let Some(result) = incomplete_elliptic_second_kind(f, x) {
        return result;
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
        // Phase 34: Weierstrass substitution for c / (a + b·sin/cos(x))
        // when c, a, b are rational and a² > b² (and a > 0 for the cos case).
        (DIV, [c, denom]) if !depends_on(c, x) => {
            try_weierstrass_one_over_linear_trig(c, denom, x)
                .unwrap_or_else(|| apply_node(INTEGRATE, vec![f.clone(), IRNode::Symbol(x.to_string())]))
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
        // Phase 26: ∫ log(x)^n dx for integer n ≥ 2 (standalone, no poly factor).
        (POW, [base, exponent]) if is_log_of_x(base, x) => {
            match to_numeric(exponent) {
                Some(Numeric::Int(n)) if n >= 2 => poly_log_power_term(0, n as usize, x),
                _ => apply_node(INTEGRATE, vec![f.clone(), IRNode::Symbol(x.to_string())]),
            }
        }
        // Phase 27: ∫ sin(log(x)) dx and ∫ cos(log(x)) dx (k=0 direct forms).
        (SIN, [inner]) if is_log_of_x(inner, x) => trig_log_integral(SIN, 0, x),
        (COS, [inner]) if is_log_of_x(inner, x) => trig_log_integral(COS, 0, x),
        // Phase 26+27: Q(x) · log(x)^n or Q(x) · trig(log(x)) — both factors depend on x.
        // Phase 28: Q(x) · log(Q(x)) or Q(x) · atan(Q(x)) for non-linear Q (general IBP).
        (MUL, [a, b]) => try_log_power_product(a, b, x)
            .or_else(|| try_log_power_product(b, a, x))
            .or_else(|| try_trig_log_product(a, b, x))
            .or_else(|| try_trig_log_product(b, a, x))
            .or_else(|| try_log_poly_product(a, b, x))
            .or_else(|| try_log_poly_product(b, a, x))
            .or_else(|| try_atan_poly_product(a, b, x))
            .or_else(|| try_atan_poly_product(b, a, x))
            .unwrap_or_else(|| apply_node(INTEGRATE, vec![f.clone(), IRNode::Symbol(x.to_string())])),
        // Phase 28: bare ∫ log(Q(x)) dx and ∫ atan(Q(x)) dx for non-linear Q.
        // The simple arms (LOG, [inner]) and (ATAN, [inner]) above already captured
        // the x-equals-inner case; these arms fire when Q depends on x but is non-linear.
        (LOG, [q_ir]) if depends_on(q_ir, x) && !is_linear_in(q_ir, x) => {
            try_log_poly_product(f, &IRNode::Integer(1), x)
                .unwrap_or_else(|| apply_node(INTEGRATE, vec![f.clone(), IRNode::Symbol(x.to_string())]))
        }
        (ATAN, [q_ir]) if depends_on(q_ir, x) && !is_linear_in(q_ir, x) => {
            try_atan_poly_product(f, &IRNode::Integer(1), x)
                .unwrap_or_else(|| apply_node(INTEGRATE, vec![f.clone(), IRNode::Symbol(x.to_string())]))
        }
        _ => apply_node(INTEGRATE, vec![f.clone(), IRNode::Symbol(x.to_string())]),
    }
}

fn complete_elliptic_first_kind(
    f: &IRNode,
    x: &str,
    lower: &IRNode,
    upper: &IRNode,
) -> Option<IRNode> {
    if !to_numeric(lower).is_some_and(Numeric::is_zero) || !is_pi_over_two(upper) {
        return None;
    }
    elliptic_first_kind_modulus(f, x).map(|modulus| apply_node("EllipticK", vec![modulus]))
}

fn incomplete_elliptic_first_kind(f: &IRNode, x: &str) -> Option<IRNode> {
    elliptic_first_kind_modulus(f, x)
        .map(|modulus| apply_node("EllipticF", vec![IRNode::Symbol(x.to_string()), modulus]))
}

fn elliptic_first_kind_modulus(f: &IRNode, x: &str) -> Option<IRNode> {
    let radicand = match f {
        IRNode::Apply(apply) if apply.head == IRNode::Symbol(DIV.to_string()) => {
            let [numerator, denominator] = apply.args.as_slice() else {
                return None;
            };
            let IRNode::Apply(sqrt) = denominator else {
                return None;
            };
            let [radicand] = sqrt.args.as_slice() else {
                return None;
            };
            if to_numeric(numerator).is_some_and(Numeric::is_one)
                && sqrt.head == IRNode::Symbol(SQRT.to_string())
            {
                radicand
            } else {
                return None;
            }
        }
        IRNode::Apply(apply) if apply.head == IRNode::Symbol(POW.to_string()) => {
            let [base, exponent] = apply.args.as_slice() else {
                return None;
            };
            if to_numeric(exponent) == Some(Numeric::Rat(-1, 2)) {
                base
            } else {
                return None;
            }
        }
        _ => return None,
    };

    let IRNode::Apply(sub) = radicand else {
        return None;
    };
    let [constant, product] = sub.args.as_slice() else {
        return None;
    };
    if sub.head != IRNode::Symbol(SUB.to_string())
        || !to_numeric(constant).is_some_and(Numeric::is_one)
    {
        return None;
    }
    let IRNode::Apply(mul) = product else {
        return None;
    };
    let [left, right] = mul.args.as_slice() else {
        return None;
    };
    if mul.head != IRNode::Symbol(MUL.to_string()) {
        return None;
    }
    modulus_from_squared_factor(left, right, x)
        .or_else(|| modulus_from_squared_factor(right, left, x))
}

fn modulus_from_squared_factor(
    modulus_square: &IRNode,
    sine_square: &IRNode,
    x: &str,
) -> Option<IRNode> {
    let IRNode::Apply(sine_power) = sine_square else {
        return None;
    };
    let [sine, sine_exponent] = sine_power.args.as_slice() else {
        return None;
    };
    if sine_power.head != IRNode::Symbol(POW.to_string()) || sine_exponent != &IRNode::Integer(2) {
        return None;
    }
    let IRNode::Apply(sine_call) = sine else {
        return None;
    };
    let [inner] = sine_call.args.as_slice() else {
        return None;
    };
    if sine_call.head != IRNode::Symbol(SIN.to_string()) || inner != &IRNode::Symbol(x.to_string())
    {
        return None;
    }

    let IRNode::Apply(modulus_power) = modulus_square else {
        return None;
    };
    let [modulus, modulus_exponent] = modulus_power.args.as_slice() else {
        return None;
    };
    if modulus_power.head == IRNode::Symbol(POW.to_string())
        && modulus_exponent == &IRNode::Integer(2)
    {
        Some(modulus.clone())
    } else {
        None
    }
}

fn is_pi_over_two(node: &IRNode) -> bool {
    match node {
        IRNode::Float(value) => (*value - std::f64::consts::FRAC_PI_2).abs() < 1e-12,
        IRNode::Apply(apply) if apply.head == IRNode::Symbol(DIV.to_string()) => {
            let [numerator, denominator] = apply.args.as_slice() else {
                return false;
            };
            numerator == &IRNode::Symbol("%pi".to_string()) && denominator == &IRNode::Integer(2)
        }
        _ => false,
    }
}

/// Return `k` when `f = Sqrt(Sub(1, Mul(Pow(k,2), Pow(Sin(x),2))))`.
///
/// This matches the integrand `sqrt(1 - k^2 * sin(x)^2)` which is the
/// integrand for the complete and incomplete elliptic integrals of the
/// second kind.
fn elliptic_second_kind_radicand(f: &IRNode, x: &str) -> Option<IRNode> {
    let IRNode::Apply(sqrt) = f else { return None };
    if sqrt.head != IRNode::Symbol(SQRT.to_string()) { return None; }
    let [radicand] = sqrt.args.as_slice() else { return None };
    let IRNode::Apply(sub) = radicand else { return None };
    if sub.head != IRNode::Symbol(SUB.to_string()) { return None; }
    let [constant, product] = sub.args.as_slice() else { return None };
    if !to_numeric(constant).is_some_and(Numeric::is_one) { return None; }
    let IRNode::Apply(mul) = product else { return None };
    if mul.head != IRNode::Symbol(MUL.to_string()) { return None; }
    let [left, right] = mul.args.as_slice() else { return None };
    modulus_from_squared_factor(left, right, x)
        .or_else(|| modulus_from_squared_factor(right, left, x))
}

/// `∫₀^(π/2) sqrt(1-k²sin²θ)dθ` → `EllipticE(k)`
///
/// Recognises the complete elliptic integral of the second kind over the
/// canonical interval `[0, π/2]` and returns `EllipticE(k)`.
fn complete_elliptic_second_kind(
    f: &IRNode,
    x: &str,
    lower: &IRNode,
    upper: &IRNode,
) -> Option<IRNode> {
    if !to_numeric(lower).is_some_and(Numeric::is_zero) || !is_pi_over_two(upper) {
        return None;
    }
    elliptic_second_kind_radicand(f, x).map(|modulus| apply_node("EllipticE", vec![modulus]))
}

/// `∫ sqrt(1-k²sin²θ)dθ` → `EllipticE(θ, k)`
///
/// Recognises the incomplete elliptic integral of the second kind and
/// returns `EllipticE(θ, k)`.
fn incomplete_elliptic_second_kind(f: &IRNode, x: &str) -> Option<IRNode> {
    elliptic_second_kind_radicand(f, x).map(|modulus| {
        apply_node("EllipticE", vec![IRNode::Symbol(x.to_string()), modulus])
    })
}

/// Return `n` when `bracket = Add(1, Mul(n, Pow(Sin(x), 2)))`.
///
/// This matches the characteristic factor `(1 + n*sin(x)^2)` in the
/// denominator of the elliptic integral of the third kind.
fn extract_characteristic_n(bracket: &IRNode, x: &str) -> Option<IRNode> {
    let IRNode::Apply(add) = bracket else { return None };
    if add.head != IRNode::Symbol(ADD.to_string()) { return None; }
    let [a, b] = add.args.as_slice() else { return None };
    for (one_part, prod_part) in [(a, b), (b, a)] {
        if !to_numeric(one_part).is_some_and(Numeric::is_one) { continue; }
        let IRNode::Apply(mul) = prod_part else { continue };
        if mul.head != IRNode::Symbol(MUL.to_string()) { continue; }
        let [p1, p2] = mul.args.as_slice() else { continue };
        for (n_candidate, sin_sq) in [(p1, p2), (p2, p1)] {
            let IRNode::Apply(pow) = sin_sq else { continue };
            if pow.head != IRNode::Symbol(POW.to_string()) { continue; }
            let [sine, exp] = pow.args.as_slice() else { continue };
            if exp != &IRNode::Integer(2) { continue; }
            let IRNode::Apply(sin_call) = sine else { continue };
            if sin_call.head != IRNode::Symbol(SIN.to_string()) { continue; }
            let [inner] = sin_call.args.as_slice() else { continue };
            if inner != &IRNode::Symbol(x.to_string()) { continue; }
            if !depends_on(n_candidate, x) {
                return Some(n_candidate.clone());
            }
        }
    }
    None
}

/// Return `(n, k)` when `f = Div(1, Mul(bracket, Sqrt(Sub(1, Mul(Pow(k,2), Pow(Sin(x),2))))))`.
///
/// This matches the integrand `1/((1 + n*sin(x)^2) * sqrt(1 - k^2*sin(x)^2))`
/// for the complete elliptic integral of the third kind.
fn elliptic_third_kind_params(f: &IRNode, x: &str) -> Option<(IRNode, IRNode)> {
    let IRNode::Apply(div) = f else { return None };
    if div.head != IRNode::Symbol(DIV.to_string()) { return None; }
    let [numerator, denominator] = div.args.as_slice() else { return None };
    if !to_numeric(numerator).is_some_and(Numeric::is_one) { return None; }
    let IRNode::Apply(mul) = denominator else { return None };
    if mul.head != IRNode::Symbol(MUL.to_string()) { return None; }
    let [a, b] = mul.args.as_slice() else { return None };
    for (bracket, sqrt_term) in [(a, b), (b, a)] {
        let IRNode::Apply(sqrt) = sqrt_term else { continue };
        if sqrt.head != IRNode::Symbol(SQRT.to_string()) { continue; }
        let [radicand] = sqrt.args.as_slice() else { continue };
        let IRNode::Apply(sub) = radicand else { continue };
        if sub.head != IRNode::Symbol(SUB.to_string()) { continue; }
        let [constant, product] = sub.args.as_slice() else { continue };
        if !to_numeric(constant).is_some_and(Numeric::is_one) { continue; }
        let IRNode::Apply(prod_mul) = product else { continue };
        if prod_mul.head != IRNode::Symbol(MUL.to_string()) { continue; }
        let [f1, f2] = prod_mul.args.as_slice() else { continue };
        let k = modulus_from_squared_factor(f1, f2, x)
            .or_else(|| modulus_from_squared_factor(f2, f1, x));
        let Some(k) = k else { continue };
        let Some(n) = extract_characteristic_n(bracket, x) else { continue };
        return Some((n, k));
    }
    None
}

/// `∫₀^(π/2) 1/((1+n·sin²θ)·sqrt(1-k²sin²θ))dθ` → `EllipticPi(n, k)`
///
/// Recognises the complete elliptic integral of the third kind over the
/// canonical interval `[0, π/2]` and returns `EllipticPi(n, k)`.
fn complete_elliptic_third_kind(
    f: &IRNode,
    x: &str,
    lower: &IRNode,
    upper: &IRNode,
) -> Option<IRNode> {
    if !to_numeric(lower).is_some_and(Numeric::is_zero) || !is_pi_over_two(upper) {
        return None;
    }
    elliptic_third_kind_params(f, x)
        .map(|(n, k)| apply_node("EllipticPi", vec![n, k]))
}

// ---------------------------------------------------------------------------
// Phase 26 — log-power integration via IBP reduction
// ---------------------------------------------------------------------------

/// Returns ``true`` when ``node`` is ``Log(x)`` — i.e., the log of the bare
/// integration variable.  Used as a guard in the integrate match arms.
fn is_log_of_x(node: &IRNode, x: &str) -> bool {
    if let IRNode::Apply(a) = node {
        a.head == IRNode::Symbol(LOG.to_string())
            && a.args.len() == 1
            && a.args[0] == IRNode::Symbol(x.to_string())
    } else {
        false
    }
}

/// Extract polynomial coefficients from an IR expression in variable ``x``.
///
/// Returns ``Some(Vec<(degree, coefficient)>)`` sorted by ascending degree,
/// or ``None`` if ``expr`` is not a polynomial in ``x`` over the rationals.
///
/// Handles:
/// - constants free of x (degree 0)
/// - x itself (degree 1, coefficient 1)
/// - x^k for non-negative integer k
/// - c · f or f · c where c is free of x — scalar-multiply inner poly
/// - ADD and SUB of two polynomials — merge coefficient lists
/// - NEG of a polynomial — negate all coefficients
fn to_polynomial_coeffs(expr: &IRNode, x: &str) -> Option<Vec<(usize, IRNode)>> {
    if !depends_on(expr, x) {
        return Some(vec![(0, expr.clone())]);
    }
    if expr == &IRNode::Symbol(x.to_string()) {
        return Some(vec![(1, IRNode::Integer(1))]);
    }
    let IRNode::Apply(apply) = expr else {
        return None;
    };
    let IRNode::Symbol(head) = &apply.head else {
        return None;
    };
    match (head.as_str(), apply.args.as_slice()) {
        (POW, [base, exponent]) if base == &IRNode::Symbol(x.to_string()) => {
            if let Some(Numeric::Int(k)) = to_numeric(exponent) {
                if k >= 0 {
                    return Some(vec![(k as usize, IRNode::Integer(1))]);
                }
            }
            None
        }
        (MUL, [a, b]) if !depends_on(a, x) => {
            let mut poly = to_polynomial_coeffs(b, x)?;
            for (_, coef) in &mut poly {
                if to_numeric(a).is_some_and(Numeric::is_one) {
                    // no-op: coef stays the same
                } else {
                    *coef = apply_node(MUL, vec![a.clone(), coef.clone()]);
                }
            }
            Some(poly)
        }
        (MUL, [a, b]) if !depends_on(b, x) => {
            let mut poly = to_polynomial_coeffs(a, x)?;
            for (_, coef) in &mut poly {
                if to_numeric(b).is_some_and(Numeric::is_one) {
                    // no-op
                } else {
                    *coef = apply_node(MUL, vec![b.clone(), coef.clone()]);
                }
            }
            Some(poly)
        }
        (ADD, [a, b]) => {
            let mut poly_a = to_polynomial_coeffs(a, x)?;
            let poly_b = to_polynomial_coeffs(b, x)?;
            for (kb, vb) in poly_b {
                if let Some(entry) = poly_a.iter_mut().find(|(ka, _)| *ka == kb) {
                    entry.1 = apply_node(ADD, vec![entry.1.clone(), vb]);
                } else {
                    poly_a.push((kb, vb));
                }
            }
            poly_a.sort_by_key(|(k, _)| *k);
            Some(poly_a)
        }
        (SUB, [a, b]) => {
            let mut poly_a = to_polynomial_coeffs(a, x)?;
            let poly_b = to_polynomial_coeffs(b, x)?;
            for (kb, vb) in poly_b {
                if let Some(entry) = poly_a.iter_mut().find(|(ka, _)| *ka == kb) {
                    entry.1 = apply_node(SUB, vec![entry.1.clone(), vb]);
                } else {
                    poly_a.push((kb, apply_node(NEG, vec![vb])));
                }
            }
            poly_a.sort_by_key(|(k, _)| *k);
            Some(poly_a)
        }
        (NEG, [inner]) => {
            let poly = to_polynomial_coeffs(inner, x)?;
            Some(
                poly.into_iter()
                    .map(|(k, v)| (k, apply_node(NEG, vec![v])))
                    .collect(),
            )
        }
        _ => None,
    }
}

/// Phase 26: closed form of ``∫ x^k · log(x)^n dx``.
///
/// Uses the IBP reduction formula (u = log(x)^n, dv = x^k dx):
///
///   G_{k,0}(x) = x^(k+1)/(k+1)
///   G_{k,m}(x) = x^(k+1)/(k+1) · log(x)^m  −  m/(k+1) · G_{k,m-1}(x)
///
/// For k = 0, the base case simplifies to ``G_{0,0}(x) = x``.
fn poly_log_power_term(k: usize, n: usize, x: &str) -> IRNode {
    let kp1 = k + 1;
    let x_sym = IRNode::Symbol(x.to_string());
    let kp1_frac = make_rat(1, kp1 as i64);
    let log_node = apply_node(LOG, vec![x_sym.clone()]);

    // Base: G_{k,0}
    let mut acc: IRNode = if kp1 == 1 {
        x_sym.clone()
    } else {
        apply_node(
            MUL,
            vec![
                from_numeric(kp1_frac),
                apply_node(POW, vec![x_sym.clone(), IRNode::Integer(kp1 as i64)]),
            ],
        )
    };

    for m in 1..=n {
        let log_pow: IRNode = if m == 1 {
            log_node.clone()
        } else {
            apply_node(POW, vec![log_node.clone(), IRNode::Integer(m as i64)])
        };
        let first: IRNode = if kp1 == 1 {
            apply_node(MUL, vec![x_sym.clone(), log_pow])
        } else {
            apply_node(
                MUL,
                vec![
                    from_numeric(kp1_frac),
                    apply_node(
                        MUL,
                        vec![
                            apply_node(POW, vec![x_sym.clone(), IRNode::Integer(kp1 as i64)]),
                            log_pow,
                        ],
                    ),
                ],
            )
        };
        let n_coef = from_numeric(make_rat(m as i64, kp1 as i64));
        acc = apply_node(SUB, vec![first, apply_node(MUL, vec![n_coef, acc])]);
    }
    acc
}

/// Phase 26: ``∫ Q(x) · log(x)^n dx`` for integer n ≥ 2 via term-by-term IBP.
///
/// ``transcendental`` must be ``Pow(Log(x), n)`` with integer n ≥ 2.
/// ``poly_candidate`` must be a polynomial in x.
fn try_log_power_product(
    transcendental: &IRNode,
    poly_candidate: &IRNode,
    x: &str,
) -> Option<IRNode> {
    let IRNode::Apply(apply) = transcendental else {
        return None;
    };
    let IRNode::Symbol(head) = &apply.head else {
        return None;
    };
    if head.as_str() != POW || apply.args.len() != 2 {
        return None;
    }
    let log_node = &apply.args[0];
    let exp_node = &apply.args[1];
    if !is_log_of_x(log_node, x) {
        return None;
    }
    let n = match to_numeric(exp_node)? {
        Numeric::Int(n) if n >= 2 => n as usize,
        _ => return None,
    };

    let poly = to_polynomial_coeffs(poly_candidate, x)?;
    if poly.is_empty() {
        return None;
    }

    let pieces: Vec<IRNode> = poly
        .into_iter()
        .filter_map(|(k, coef)| {
            if to_numeric(&coef).is_some_and(Numeric::is_zero) {
                return None;
            }
            let term = poly_log_power_term(k, n, x);
            if to_numeric(&coef).is_some_and(Numeric::is_one) {
                Some(term)
            } else {
                Some(apply_node(MUL, vec![coef, term]))
            }
        })
        .collect();

    if pieces.is_empty() {
        return Some(IRNode::Integer(0));
    }
    Some(
        pieces
            .into_iter()
            .reduce(|acc, p| apply_node(ADD, vec![acc, p]))
            .unwrap(),
    )
}

// ---------------------------------------------------------------------------
// Phase 27 — trig(log(x)) integration via u = log(x) substitution
// ---------------------------------------------------------------------------
//
// The substitution u = log(x) converts:
//   ∫ xᵏ sin(log x) dx = ∫ e^((k+1)u) sin(u) du
//                       = x^(k+1) · ((k+1)sin(log x) − cos(log x)) / ((k+1)² + 1)
//
//   ∫ xᵏ cos(log x) dx = x^(k+1) · ((k+1)cos(log x) + sin(log x)) / ((k+1)² + 1)

/// Phase 27: closed form of ``∫ x^k · trig(log(x)) dx``.
///
/// ``trig_head`` is ``SIN`` or ``COS``; ``k`` is the integer power of x.
fn trig_log_integral(trig_head: &str, k: usize, x: &str) -> IRNode {
    let kp1 = k + 1;
    let denom = (kp1 * kp1 + 1) as i64;
    let log_x = apply_node(LOG, vec![IRNode::Symbol(x.to_string())]);
    let sin_log_x = apply_node(SIN, vec![log_x.clone()]);
    let cos_log_x = apply_node(COS, vec![log_x.clone()]);
    let x_pow: IRNode = if kp1 == 1 {
        IRNode::Symbol(x.to_string())
    } else {
        apply_node(
            POW,
            vec![IRNode::Symbol(x.to_string()), IRNode::Integer(kp1 as i64)],
        )
    };
    let kp1_ir = IRNode::Integer(kp1 as i64);
    let denom_ir = IRNode::Integer(denom);
    let numerator: IRNode = if trig_head == SIN {
        apply_node(
            SUB,
            vec![apply_node(MUL, vec![kp1_ir, sin_log_x]), cos_log_x],
        )
    } else {
        apply_node(
            ADD,
            vec![apply_node(MUL, vec![kp1_ir, cos_log_x]), sin_log_x],
        )
    };
    apply_node(DIV, vec![apply_node(MUL, vec![x_pow, numerator]), denom_ir])
}

/// Phase 27: ``∫ Q(x) · sin(log(x)) dx`` or ``∫ Q(x) · cos(log(x)) dx``.
///
/// ``transcendental`` must be ``Sin(Log(x))`` or ``Cos(Log(x))``.
/// ``poly_candidate`` must be a polynomial in x.
fn try_trig_log_product(
    transcendental: &IRNode,
    poly_candidate: &IRNode,
    x: &str,
) -> Option<IRNode> {
    let IRNode::Apply(apply) = transcendental else {
        return None;
    };
    let IRNode::Symbol(head) = &apply.head else {
        return None;
    };
    let trig_head = head.as_str();
    if trig_head != SIN && trig_head != COS {
        return None;
    }
    if apply.args.len() != 1 || !is_log_of_x(&apply.args[0], x) {
        return None;
    }

    let poly = to_polynomial_coeffs(poly_candidate, x)?;
    if poly.is_empty() {
        return None;
    }

    let pieces: Vec<IRNode> = poly
        .into_iter()
        .filter_map(|(k, coef)| {
            if to_numeric(&coef).is_some_and(Numeric::is_zero) {
                return None;
            }
            let term = trig_log_integral(trig_head, k, x);
            if to_numeric(&coef).is_some_and(Numeric::is_one) {
                Some(term)
            } else {
                Some(apply_node(MUL, vec![coef, term]))
            }
        })
        .collect();

    if pieces.is_empty() {
        return Some(IRNode::Integer(0));
    }
    Some(
        pieces
            .into_iter()
            .reduce(|acc, p| apply_node(ADD, vec![acc, p]))
            .unwrap(),
    )
}

// ---------------------------------------------------------------------------
// Phase 28 — General IBP: ∫ P(x)·log(Q(x)) dx  and  ∫ P(x)·atan(Q(x)) dx
// ---------------------------------------------------------------------------
//
// IBP formulas (u = transcendental, dv = P dx, v = R = ∫P):
//
//   ∫ P·log(Q) dx  =  R·log(Q) − ∫ R·Q′/Q dx
//   ∫ P·atan(Q) dx =  R·atan(Q) − ∫ R·Q′/(1+Q²) dx
//
// The residual integrals are rational functions.  We handle them with a
// limited rational integrator (`integrate_rational_simple_rp`) that covers:
//
//   Case A: remainder = c·D′  →  c·log(D)
//   Case B: remainder is constant, D = a₂x² + a₀ with rational √(a₀/a₂)
//           →  r₀/(a₂√(a₀/a₂))·atan(x/√(a₀/a₂))
//
// All polynomial arithmetic is done over exact rationals using a dense
// `RatPoly = Vec<RatC>` representation where index = monomial degree and
// `RatC = (i128, i128)` is a reduced rational (numerator, denominator).
// Using i128 gives enough headroom for the intermediate cross-multiplications
// in rc_add / rc_mul without overflow for any typical CAS polynomial.

/// GCD for u128 — Euclidean algorithm.
fn gcd128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Rational coefficient: (numerator, denominator), always in lowest terms with
/// denom > 0.  Zero is represented as (0, 1).
type RatC = (i128, i128);

const RC_ZERO: RatC = (0, 1);
const RC_ONE: RatC = (1, 1);

fn rc_is_zero((n, _): RatC) -> bool {
    n == 0
}

fn rc_is_one((n, d): RatC) -> bool {
    n == d
}

/// Reduce a (numer, denom) pair to lowest terms with positive denominator.
/// Returns `None` if `denom == 0`.
fn rc(numer: i128, denom: i128) -> Option<RatC> {
    if denom == 0 {
        return None;
    }
    let (numer, denom) = if denom < 0 {
        (-numer, -denom)
    } else {
        (numer, denom)
    };
    let g = gcd128(numer.unsigned_abs(), denom.unsigned_abs()) as i128;
    Some((numer / g, denom / g))
}

fn rc_neg((n, d): RatC) -> RatC {
    (-n, d)
}

fn rc_add((n1, d1): RatC, (n2, d2): RatC) -> Option<RatC> {
    // n1/d1 + n2/d2 = (n1·d2 + n2·d1) / (d1·d2)
    let numer = n1.checked_mul(d2)?.checked_add(n2.checked_mul(d1)?)?;
    let denom = d1.checked_mul(d2)?;
    rc(numer, denom)
}

fn rc_sub((n1, d1): RatC, (n2, d2): RatC) -> Option<RatC> {
    let numer = n1.checked_mul(d2)?.checked_sub(n2.checked_mul(d1)?)?;
    let denom = d1.checked_mul(d2)?;
    rc(numer, denom)
}

fn rc_mul((n1, d1): RatC, (n2, d2): RatC) -> Option<RatC> {
    rc(n1.checked_mul(n2)?, d1.checked_mul(d2)?)
}

fn rc_div((n1, d1): RatC, (n2, d2): RatC) -> Option<RatC> {
    if n2 == 0 {
        return None;
    }
    rc(n1.checked_mul(d2)?, d1.checked_mul(n2)?)
}

/// Convert a `RatC` to an `IRNode`.  Returns `None` if values overflow `i64`.
fn rc_to_ir((n, d): RatC) -> Option<IRNode> {
    if n == 0 {
        return Some(IRNode::Integer(0));
    }
    let n64 = i64::try_from(n).ok()?;
    if d == 1 {
        Some(IRNode::Integer(n64))
    } else {
        let d64 = i64::try_from(d).ok()?;
        Some(IRNode::Rational(n64, d64))
    }
}

/// Recursively evaluate a closed IR expression that contains only rational
/// constants and arithmetic (Add, Sub, Mul, Div, Neg) to a `RatC`.
///
/// The `to_polynomial_coeffs` helper sometimes returns coefficient nodes like
/// `Mul(Integer(2), Integer(1))` rather than a bare literal — this function
/// handles those compound forms.  Returns `None` for any non-rational node
/// (e.g. one containing a symbol).
fn eval_numeric_node(node: &IRNode) -> Option<RatC> {
    match node {
        IRNode::Integer(n) => rc(*n as i128, 1),
        IRNode::Rational(n, d) => rc(*n as i128, *d as i128),
        IRNode::Apply(apply) => {
            let IRNode::Symbol(head) = &apply.head else {
                return None;
            };
            match (head.as_str(), apply.args.as_slice()) {
                (MUL, [a, b]) => rc_mul(eval_numeric_node(a)?, eval_numeric_node(b)?),
                (DIV, [a, b]) => rc_div(eval_numeric_node(a)?, eval_numeric_node(b)?),
                (NEG, [a]) => Some(rc_neg(eval_numeric_node(a)?)),
                (ADD, [a, b]) => rc_add(eval_numeric_node(a)?, eval_numeric_node(b)?),
                (SUB, [a, b]) => rc_sub(eval_numeric_node(a)?, eval_numeric_node(b)?),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Dense rational polynomial: `p[k]` is the coefficient of x^k.
type RatPoly = Vec<RatC>;

/// Convert the sparse `(degree, IRNode)` output of `to_polynomial_coeffs` to a
/// dense `RatPoly`.  Returns `None` if any coefficient cannot be evaluated to
/// an exact rational.
fn rp_from_poly_vec(pairs: Vec<(usize, IRNode)>) -> Option<RatPoly> {
    if pairs.is_empty() {
        return Some(vec![]);
    }
    let max_deg = pairs.iter().map(|(k, _)| *k).max()?;
    let mut result = vec![RC_ZERO; max_deg + 1];
    for (deg, node) in pairs {
        result[deg] = eval_numeric_node(&node)?;
    }
    Some(result)
}

/// Effective degree: index of last nonzero coefficient.  Returns `None` for the
/// zero polynomial.
fn rp_deg(p: &[RatC]) -> Option<usize> {
    p.iter()
        .enumerate()
        .rev()
        .find(|(_, c)| !rc_is_zero(**c))
        .map(|(i, _)| i)
}

fn rp_is_zero(p: &[RatC]) -> bool {
    rp_deg(p).is_none()
}

/// Coefficient of x^`deg` (zero if out of range).
fn rp_coeff(p: &[RatC], deg: usize) -> RatC {
    p.get(deg).copied().unwrap_or(RC_ZERO)
}

/// Pointwise addition of two polynomials.
fn rp_add(a: &[RatC], b: &[RatC]) -> Option<RatPoly> {
    let n = a.len().max(b.len());
    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        result.push(rc_add(rp_coeff(a, i), rp_coeff(b, i))?);
    }
    Some(result)
}

/// Subtract polynomial `b` from `a`.
fn rp_sub_poly(a: &[RatC], b: &[RatC]) -> Option<RatPoly> {
    let n = a.len().max(b.len());
    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        result.push(rc_sub(rp_coeff(a, i), rp_coeff(b, i))?);
    }
    Some(result)
}

/// Multiply a polynomial by a scalar coefficient.  Returns empty vec for zero scalar.
fn rp_mul_scalar(p: &[RatC], c: RatC) -> Option<RatPoly> {
    if rc_is_zero(c) {
        return Some(vec![]);
    }
    p.iter().map(|&coef| rc_mul(coef, c)).collect()
}

/// Multiply a polynomial by x^`k` (degree shift / prepend k zeros).
fn rp_shift(p: &[RatC], k: usize) -> RatPoly {
    let mut result = vec![RC_ZERO; k];
    result.extend_from_slice(p);
    result
}

/// Multiply two polynomials.
fn rp_mul(a: &[RatC], b: &[RatC]) -> Option<RatPoly> {
    if a.is_empty() || b.is_empty() {
        return Some(vec![]);
    }
    let deg_a = rp_deg(a).unwrap_or(0);
    let deg_b = rp_deg(b).unwrap_or(0);
    let mut result = vec![RC_ZERO; deg_a + deg_b + 1];
    for (i, &ca) in a.iter().enumerate() {
        if rc_is_zero(ca) {
            continue;
        }
        for (j, &cb) in b.iter().enumerate() {
            if rc_is_zero(cb) {
                continue;
            }
            let prod = rc_mul(ca, cb)?;
            result[i + j] = rc_add(result[i + j], prod)?;
        }
    }
    Some(result)
}

/// Formal derivative of a polynomial (drops the constant term's contribution).
fn rp_deriv(p: &[RatC]) -> Option<RatPoly> {
    if p.len() <= 1 {
        return Some(vec![]);
    }
    // d/dx (c_k · x^k) = k · c_k · x^(k-1); result has one fewer degree
    p.iter()
        .enumerate()
        .skip(1)
        .map(|(i, &c)| rc_mul(c, (i as i128, 1)))
        .collect()
}

/// Antiderivative of a polynomial (integration constant = 0).
fn rp_integrate(p: &[RatC]) -> Option<RatPoly> {
    let mut result = vec![RC_ZERO]; // constant term = 0
    for (i, &c) in p.iter().enumerate() {
        result.push(rc_div(c, (i as i128 + 1, 1))?);
    }
    Some(result)
}

/// Polynomial long division: returns `(quotient, remainder)` such that
/// `num = quotient · denom + remainder` with `deg(remainder) < deg(denom)`.
///
/// Returns `None` if `denom` is the zero polynomial or if any arithmetic
/// overflows.
fn rp_div(num: &[RatC], denom: &[RatC]) -> Option<(RatPoly, RatPoly)> {
    let denom_deg = rp_deg(denom)?; // returns None for zero denom → propagates
    let denom_lead = rp_coeff(denom, denom_deg);
    let num_deg = match rp_deg(num) {
        Some(d) if d >= denom_deg => d,
        _ => {
            // deg(num) < deg(denom): quotient is zero, remainder is num
            return Some((vec![], num.to_vec()));
        }
    };

    let quot_size = num_deg - denom_deg + 1;
    let mut quotient = vec![RC_ZERO; quot_size];
    let mut remainder = num.to_vec();

    loop {
        let rem_deg = match rp_deg(&remainder) {
            Some(d) if d >= denom_deg => d,
            _ => break,
        };
        let c = rc_div(rp_coeff(&remainder, rem_deg), denom_lead)?;
        let shift = rem_deg - denom_deg;
        quotient[shift] = rc_add(quotient[shift], c)?;
        let term_scaled = rp_mul_scalar(&rp_shift(denom, shift), c)?;
        remainder = rp_sub_poly(&remainder, &term_scaled)?;
    }

    Some((quotient, remainder))
}

/// Convert a dense `RatPoly` to an IR expression in variable `x`.
/// Zero polynomial → `Integer(0)`.
fn rp_to_ir(p: &[RatC], x: &str) -> Option<IRNode> {
    let x_sym = IRNode::Symbol(x.to_string());
    let mut terms: Vec<IRNode> = Vec::new();

    for (deg, &coef) in p.iter().enumerate() {
        if rc_is_zero(coef) {
            continue;
        }
        let coef_ir = rc_to_ir(coef)?;

        let monomial: IRNode = match deg {
            0 => {
                terms.push(coef_ir);
                continue;
            }
            1 => x_sym.clone(),
            _ => apply_node(POW, vec![x_sym.clone(), IRNode::Integer(deg as i64)]),
        };

        if rc_is_one(coef) {
            terms.push(monomial);
        } else {
            terms.push(apply_node(MUL, vec![coef_ir, monomial]));
        }
    }

    if terms.is_empty() {
        return Some(IRNode::Integer(0));
    }

    Some(
        terms
            .into_iter()
            .reduce(|acc, t| apply_node(ADD, vec![acc, t]))
            .unwrap(),
    )
}

/// Check whether `r = c · d` for some rational `c`; returns `Some(c)` or `None`.
/// Both polynomials must be nonzero and of the same degree.
fn rp_proportional(r: &[RatC], d: &[RatC]) -> Option<RatC> {
    let r_deg = rp_deg(r)?;
    let d_deg = rp_deg(d)?;
    if r_deg != d_deg {
        return None;
    }
    let c = rc_div(rp_coeff(r, r_deg), rp_coeff(d, d_deg))?;
    // Verify every coefficient: c · d[i] == r[i]
    let max = r_deg;
    for i in 0..=max {
        let expected = rc_mul(c, rp_coeff(d, i))?;
        if expected != rp_coeff(r, i) {
            return None;
        }
    }
    Some(c)
}

/// Integer square root: returns `Some(k)` iff `n = k²`, else `None`.
fn i128_sqrt(n: i128) -> Option<i128> {
    if n < 0 {
        return None;
    }
    let k = (n as f64).sqrt() as i128;
    for candidate in [k.saturating_sub(1), k, k.saturating_add(1)] {
        if candidate >= 0 && candidate.checked_mul(candidate)? == n {
            return Some(candidate);
        }
    }
    None
}

/// Rational square root: returns `Some(√(n/d))` iff both numerator and
/// denominator are perfect squares, else `None`.
fn rc_sqrt((n, d): RatC) -> Option<RatC> {
    if n < 0 {
        return None;
    }
    rc(i128_sqrt(n)?, i128_sqrt(d)?)
}

/// Returns true iff `expr` is a non-constant polynomial of degree exactly 1
/// in `x` (i.e. of the form a·x + b with a ≠ 0).
fn is_linear_in(expr: &IRNode, x: &str) -> bool {
    let Some(pairs) = to_polynomial_coeffs(expr, x) else {
        return false;
    };
    matches!(pairs.iter().map(|(k, _)| *k).max(), Some(1))
}

/// Attempt to integrate `R(x) / D(x)` for the two cases that arise in
/// Phase 28 residuals:
///
/// - **Case A**: R = c·D′  →  c·log(D)
/// - **Case B**: R is a constant, D = a₂x²+a₀ (no odd-degree terms),
///              √(a₀/a₂) is rational  →  r₀/(a₂√(a₀/a₂))·atan(x/√(a₀/a₂))
///
/// Returns `None` if neither case applies (signals that Phase 28 falls through).
fn close_remainder_over_d(
    r: &[RatC],
    d: &[RatC],
    d_prime: &[RatC],
    d_ir: &IRNode,
    x: &str,
) -> Option<IRNode> {
    // Case A: R = c · D′  →  c · log(D)
    if !rp_is_zero(d_prime) {
        if let Some(c) = rp_proportional(r, d_prime) {
            let c_ir = rc_to_ir(c)?;
            let log_d = apply_node(LOG, vec![d_ir.clone()]);
            return Some(if rc_is_one(c) {
                log_d
            } else {
                apply_node(MUL, vec![c_ir, log_d])
            });
        }
    }

    // Case B: constant remainder over quadratic denominator with rational √
    if rp_deg(r) != Some(0) {
        return None;
    }
    if rp_deg(d) != Some(2) {
        return None;
    }

    let a2 = rp_coeff(d, 2);
    let a1 = rp_coeff(d, 1);
    let a0 = rp_coeff(d, 0);

    // No linear term (otherwise denominator has a real root and we'd need
    // partial fractions with irrational roots)
    if !rc_is_zero(a1) {
        return None;
    }

    // a₀/a₂ must be a positive rational perfect square
    let ba = rc_div(a0, a2)?;
    if ba.0 <= 0 {
        return None;
    }
    let sqrt_ba = rc_sqrt(ba)?;
    if rc_is_zero(sqrt_ba) {
        return None;
    }

    // ∫ r₀/(a₂x²+a₀) dx = r₀/(a₂·√(a₀/a₂)) · atan(x/√(a₀/a₂))
    let r0 = rp_coeff(r, 0);
    let denom_coef = rc_mul(a2, sqrt_ba)?;
    let coef = rc_div(r0, denom_coef)?;

    let coef_ir = rc_to_ir(coef)?;
    let sqrt_ba_ir = rc_to_ir(sqrt_ba)?;
    let x_sym = IRNode::Symbol(x.to_string());

    // Build atan(x / √(a₀/a₂)).  If √(a₀/a₂) = 1, simplify to atan(x).
    let atan_arg = if rc_is_one(sqrt_ba) {
        x_sym
    } else {
        apply_node(DIV, vec![x_sym, sqrt_ba_ir])
    };
    let atan_node = apply_node(ATAN, vec![atan_arg]);

    Some(if rc_is_one(coef) {
        atan_node
    } else {
        apply_node(MUL, vec![coef_ir, atan_node])
    })
}

/// Core rational function integrator (Phase 28 residuals).
///
/// Given `N / D` as `RatPoly` representations plus the original IR form of `D`
/// (for building log/atan nodes), integrates by:
///
/// 1. Polynomial long division: N = Q·D + R
/// 2. Integrate Q → polynomial antiderivative
/// 3. Close R/D using `close_remainder_over_d` (Cases A/B)
///
/// Returns `None` if the remainder cannot be closed in Cases A/B.
fn integrate_rational_simple_rp(
    num_rp: &[RatC],
    denom_rp: &[RatC],
    denom_ir: &IRNode,
    x: &str,
) -> Option<IRNode> {
    if rp_is_zero(denom_rp) {
        return None;
    }

    let d_prime_rp = rp_deriv(denom_rp)?;
    let (quotient, remainder) = rp_div(num_rp, denom_rp)?;

    // ∫ Q dx (polynomial antiderivative)
    let quot_integral_ir: Option<IRNode> = if rp_is_zero(&quotient) {
        None
    } else {
        let qi = rp_integrate(&quotient)?;
        Some(rp_to_ir(&qi, x)?)
    };

    // ∫ R/D dx (Cases A/B)
    let rem_integral_ir: Option<IRNode> = if rp_is_zero(&remainder) {
        None
    } else {
        Some(close_remainder_over_d(
            &remainder,
            denom_rp,
            &d_prime_rp,
            denom_ir,
            x,
        )?)
    };

    let terms: Vec<IRNode> = [quot_integral_ir, rem_integral_ir]
        .into_iter()
        .flatten()
        .collect();

    Some(match terms.len() {
        0 => IRNode::Integer(0),
        1 => terms.into_iter().next().unwrap(),
        _ => terms
            .into_iter()
            .reduce(|a, b| apply_node(ADD, vec![a, b]))
            .unwrap(),
    })
}

/// Phase 28: ``∫ P(x) · log(Q(x)) dx`` for non-linear polynomial Q.
///
/// IBP (u = log Q, dv = P dx):
///   ∫ P·log(Q) dx  =  R·log(Q) − ∫ R·Q′/Q dx
/// where R = ∫P (antiderivative, constant = 0).
///
/// `transcendental` must be `Log(Q)` and `poly_candidate` must be a
/// polynomial in x.  Linear Q is excluded (handled by earlier phases).
fn try_log_poly_product(
    transcendental: &IRNode,
    poly_candidate: &IRNode,
    x: &str,
) -> Option<IRNode> {
    let IRNode::Apply(apply) = transcendental else {
        return None;
    };
    let IRNode::Symbol(head) = &apply.head else {
        return None;
    };
    if head.as_str() != LOG || apply.args.len() != 1 {
        return None;
    }
    let q_ir = &apply.args[0];

    // Q must depend on x and be non-linear (linear Q goes to Phase 3)
    if !depends_on(q_ir, x) || is_linear_in(q_ir, x) {
        return None;
    }

    // Extract Q and P as rational polynomials
    let q_rp = rp_from_poly_vec(to_polynomial_coeffs(q_ir, x)?)?;
    let p_rp = rp_from_poly_vec(to_polynomial_coeffs(poly_candidate, x)?)?;

    // R = ∫P (constant = 0)
    let r_rp = rp_integrate(&p_rp)?;
    let r_ir = rp_to_ir(&r_rp, x)?;

    // Q′ = derivative of Q (exact rational arithmetic)
    let q_prime_rp = rp_deriv(&q_rp)?;

    // Residual numerator: N = R · Q′
    let n_rp = rp_mul(&r_rp, &q_prime_rp)?;

    // Integrate N / Q
    let residual = integrate_rational_simple_rp(&n_rp, &q_rp, q_ir, x)?;

    // Result: R · log(Q) − residual
    let main_term = apply_node(MUL, vec![r_ir, transcendental.clone()]);
    Some(apply_node(SUB, vec![main_term, residual]))
}

/// Phase 28: ``∫ P(x) · atan(Q(x)) dx`` for non-linear polynomial Q.
///
/// IBP (u = atan Q, dv = P dx):
///   ∫ P·atan(Q) dx  =  R·atan(Q) − ∫ R·Q′/(1+Q²) dx
/// where R = ∫P (antiderivative, constant = 0).
fn try_atan_poly_product(
    transcendental: &IRNode,
    poly_candidate: &IRNode,
    x: &str,
) -> Option<IRNode> {
    let IRNode::Apply(apply) = transcendental else {
        return None;
    };
    let IRNode::Symbol(head) = &apply.head else {
        return None;
    };
    if head.as_str() != ATAN || apply.args.len() != 1 {
        return None;
    }
    let q_ir = &apply.args[0];

    // Q must depend on x and be non-linear (linear Q left to future Phase 11)
    if !depends_on(q_ir, x) || is_linear_in(q_ir, x) {
        return None;
    }

    // Extract Q and P as rational polynomials
    let q_rp = rp_from_poly_vec(to_polynomial_coeffs(q_ir, x)?)?;
    let p_rp = rp_from_poly_vec(to_polynomial_coeffs(poly_candidate, x)?)?;

    // R = ∫P (constant = 0)
    let r_rp = rp_integrate(&p_rp)?;
    let r_ir = rp_to_ir(&r_rp, x)?;

    // Q′ = derivative of Q
    let q_prime_rp = rp_deriv(&q_rp)?;

    // Residual numerator: N = R · Q′
    let n_rp = rp_mul(&r_rp, &q_prime_rp)?;

    // Denominator: D = 1 + Q²
    let q_sq_rp = rp_mul(&q_rp, &q_rp)?;
    let one_rp: RatPoly = vec![RC_ONE];
    let d_rp = rp_add(&one_rp, &q_sq_rp)?;

    // D as IR: 1 + Q² (used inside log nodes constructed by close_remainder_over_d)
    let d_ir = apply_node(
        ADD,
        vec![
            IRNode::Integer(1),
            apply_node(POW, vec![q_ir.clone(), IRNode::Integer(2)]),
        ],
    );

    // Integrate N / D
    let residual = integrate_rational_simple_rp(&n_rp, &d_rp, &d_ir, x)?;

    // Result: R · atan(Q) − residual
    let main_term = apply_node(MUL, vec![r_ir, transcendental.clone()]);
    Some(apply_node(SUB, vec![main_term, residual]))
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

    if let Some(rewritten) = factor_multivariate_perfect_cube(input) {
        return rewritten;
    }

    if let Some(rewritten) = factor_multivariate_cubic_identity(input) {
        return rewritten;
    }

    if let Some(rewritten) = factor_multivariate_difference_of_squares(input) {
        return rewritten;
    }

    if let Some(rewritten) = factor_multivariate_perfect_square(input) {
        return rewritten;
    }

    if let Some(rewritten) = factor_multivariate_grouping(input) {
        return rewritten;
    }

    if let Some(rewritten) = factor_common_symbolic_term(input) {
        return vm.eval(rewritten);
    }

    fallback
}

/// Extract the greatest common integer coefficient and the intersection of
/// symbolic powers shared by every additive term.
///
/// This is the Rust equivalent of the Python / TypeScript
/// `_extract_multivariate_integer_content` / `extractCommonSymbolicFactor`
/// functions.  It handles both cases that the old symbolic-only version
/// handled *and* the integer-GCD case:
///
/// ```text
/// x^2·y − y          →  y · Factor(x^2 − 1)   = y·(x−1)·(x+1)
/// 2·x·y + 2·x·z      →  2·x · (y + z)
/// 2·x^2 + 4·x·y + 2·y^2 →  2 · Factor(x^2 + 2·x·y + y^2) = 2·(x+y)^2
/// ```
///
/// Returns `None` if the GCD of coefficients is 1 **and** there are no
/// common symbolic powers (i.e., nothing to pull out).
fn factor_common_symbolic_term(node: &IRNode) -> Option<IRNode> {
    let terms = additive_terms(node)?;
    if terms.len() < 2 {
        return None;
    }

    // Parse every term into (integer coefficient, symbolic power map).
    // If any term cannot be parsed this way, bail — another handler may
    // recognise the pattern.
    let parsed: Vec<(i64, HashMap<IRNode, usize>)> = terms
        .iter()
        .map(|term| term_integer_coefficient_and_powers(term))
        .collect::<Option<Vec<_>>>()?;

    // --- Integer GCD ---
    // Fold |coeff| values through GCD; if *all* coefficients are negative,
    // negate the result so the sign itself is factored out.
    let mut common_coefficient: i64 = 0;
    for (coeff, _) in &parsed {
        common_coefficient = integer_gcd(common_coefficient, coeff.abs());
    }
    if common_coefficient != 0 && parsed.iter().all(|(c, _)| *c < 0) {
        common_coefficient = -common_coefficient;
    }
    if common_coefficient == 0 {
        common_coefficient = 1;
    }

    // --- Common symbolic powers ---
    // Start with a clone of the first term's powers, then intersect with
    // each subsequent term, taking the minimum exponent at each variable.
    let mut common_powers = parsed[0].1.clone();
    for (_, powers) in &parsed[1..] {
        common_powers.retain(|base, exponent| {
            if let Some(other) = powers.get(base) {
                *exponent = (*exponent).min(*other);
                *exponent > 0
            } else {
                false
            }
        });
    }

    // Nothing to factor if coefficient is 1 and no common symbolic powers.
    if common_coefficient == 1 && common_powers.is_empty() {
        return None;
    }

    // --- Build the common factor ---
    let common_ir = term_from_coefficient_and_powers(common_coefficient, &common_powers);

    // --- Build residual: divide each term by the common factor ---
    let residual_terms: Vec<IRNode> = parsed
        .into_iter()
        .map(|(coeff, powers)| {
            let residual_coeff = coeff / common_coefficient;
            let residual_powers: HashMap<IRNode, usize> = powers
                .into_iter()
                .filter_map(|(base, exponent)| {
                    let shared = common_powers.get(&base).copied().unwrap_or(0);
                    let remaining = exponent - shared;
                    if remaining > 0 {
                        Some((base, remaining))
                    } else {
                        None
                    }
                })
                .collect();
            term_from_coefficient_and_powers(residual_coeff, &residual_powers)
        })
        .collect();

    let residual = add_nodes(residual_terms);
    Some(apply_node(
        MUL,
        vec![common_ir, maybe_factor_residual(residual)],
    ))
}

/// Build an IR term from an integer coefficient and a map of symbolic powers.
///
/// When `coefficient == 1` **and** there are powers, the `1` is dropped so
/// `term_from_coefficient_and_powers(1, {x: 2})` → `x^2` not `1·x^2`.
fn term_from_coefficient_and_powers(coefficient: i64, powers: &HashMap<IRNode, usize>) -> IRNode {
    let mut factors: Vec<IRNode> = Vec::new();
    if coefficient != 1 || powers.is_empty() {
        factors.push(IRNode::Integer(coefficient));
    }
    // Sort keys deterministically so tests can write exact assertions.
    let mut sorted: Vec<(&IRNode, usize)> = powers.iter().map(|(b, e)| (b, *e)).collect();
    sorted.sort_by_key(|(base, _)| base.to_string());
    for (base, exponent) in sorted {
        factors.push(power_to_ir(base.clone(), exponent));
    }
    multiply_nodes(factors)
}

/// Euclidean GCD over non-negative `i64` values.
fn integer_gcd(a: i64, b: i64) -> i64 {
    let mut x = a.abs();
    let mut y = b.abs();
    while y != 0 {
        let next = x % y;
        x = y;
        y = next;
    }
    x
}

/// Wrap `node` in `Factor(node)` if and only if it is a univariate integer
/// polynomial, enabling recursive factoring of the residual.
///
/// If the residual spans two or more variables — or is not a polynomial
/// at all — it is returned as-is so the caller does not produce an
/// unevaluated `Factor(...)` wrapper at the top level.
fn maybe_factor_residual(node: IRNode) -> IRNode {
    if let Some(variable) = find_single_variable(&node) {
        if ir_to_integer_poly(&node, &variable).is_some() {
            return apply_node(FACTOR, vec![node]);
        }
    }
    node
}

fn factor_multivariate_perfect_square(node: &IRNode) -> Option<IRNode> {
    let terms = additive_terms(node)?;
    if terms.len() != 3 {
        return None;
    }

    let mut squares = Vec::new();
    let mut cross = None;
    for term in &terms {
        let (coefficient, powers) = term_integer_coefficient_and_powers(term)?;
        if coefficient == 1 && powers.len() == 1 {
            let (base, exponent) = powers.iter().next()?;
            if *exponent == 2 {
                squares.push(base.clone());
                continue;
            }
        }
        if (coefficient == 2 || coefficient == -2) && powers.len() == 2 {
            let mut items = powers.iter();
            let (first, first_exponent) = items.next()?;
            let (second, second_exponent) = items.next()?;
            if *first_exponent == 1 && *second_exponent == 1 {
                cross = Some((coefficient, first.clone(), second.clone()));
                continue;
            }
        }
        return None;
    }

    if squares.len() != 2 {
        return None;
    }
    let (coefficient, cross_first, cross_second) = cross?;
    let square_keys: HashSet<IRNode> = squares.iter().cloned().collect();
    let cross_keys: HashSet<IRNode> = [cross_first, cross_second].into_iter().collect();
    if square_keys != cross_keys {
        return None;
    }

    let base = if coefficient > 0 {
        apply_node(ADD, vec![squares[0].clone(), squares[1].clone()])
    } else {
        apply_node(SUB, vec![squares[0].clone(), squares[1].clone()])
    };
    Some(apply_node(POW, vec![base, IRNode::Integer(2)]))
}

fn factor_multivariate_cubic_identity(node: &IRNode) -> Option<IRNode> {
    let terms = additive_terms(node)?;
    if terms.len() != 2 {
        return None;
    }

    let mut positive_cube = None;
    let mut negative_cube = None;
    for term in &terms {
        let (coefficient, powers) = term_integer_coefficient_and_powers(term)?;
        if powers.len() != 1 {
            return None;
        }

        let (base, exponent) = powers.iter().next()?;
        if *exponent != 3 {
            return None;
        }

        match coefficient {
            1 => positive_cube = Some(base.clone()),
            -1 => negative_cube = Some(base.clone()),
            _ => return None,
        }
    }

    if let Some(negative_cube) = negative_cube {
        let positive_cube = positive_cube?;
        return Some(apply_node(
            MUL,
            vec![
                apply_node(SUB, vec![positive_cube.clone(), negative_cube.clone()]),
                apply_node(
                    ADD,
                    vec![
                        apply_node(
                            ADD,
                            vec![
                                apply_node(POW, vec![positive_cube.clone(), IRNode::Integer(2)]),
                                apply_node(MUL, vec![positive_cube, negative_cube.clone()]),
                            ],
                        ),
                        apply_node(POW, vec![negative_cube, IRNode::Integer(2)]),
                    ],
                ),
            ],
        ));
    }

    let terms = additive_terms(node)?;
    let mut cubes = Vec::new();
    for term in &terms {
        let (coefficient, powers) = term_integer_coefficient_and_powers(term)?;
        if coefficient != 1 || powers.len() != 1 {
            return None;
        }
        let (base, exponent) = powers.iter().next()?;
        if *exponent != 3 {
            return None;
        }
        cubes.push(base.clone());
    }

    if cubes.len() != 2 {
        return None;
    }
    let first = cubes[0].clone();
    let second = cubes[1].clone();
    Some(apply_node(
        MUL,
        vec![
            apply_node(ADD, vec![first.clone(), second.clone()]),
            apply_node(
                ADD,
                vec![
                    apply_node(
                        ADD,
                        vec![
                            apply_node(POW, vec![first.clone(), IRNode::Integer(2)]),
                            apply_node(
                                MUL,
                                vec![
                                    IRNode::Integer(-1),
                                    apply_node(MUL, vec![first, second.clone()]),
                                ],
                            ),
                        ],
                    ),
                    apply_node(POW, vec![second, IRNode::Integer(2)]),
                ],
            ),
        ],
    ))
}

fn factor_multivariate_difference_of_squares(node: &IRNode) -> Option<IRNode> {
    let terms = additive_terms(node)?;
    if terms.len() != 2 {
        return None;
    }

    let mut positive_square = None;
    let mut negative_square = None;
    for term in &terms {
        let (coefficient, powers) = term_integer_coefficient_and_powers(term)?;
        if powers.len() != 1 {
            return None;
        }

        let (base, exponent) = powers.iter().next()?;
        if *exponent != 2 {
            return None;
        }

        match coefficient {
            1 => positive_square = Some(base.clone()),
            -1 => negative_square = Some(base.clone()),
            _ => return None,
        }
    }

    let positive_square = positive_square?;
    let negative_square = negative_square?;
    Some(apply_node(
        MUL,
        vec![
            apply_node(SUB, vec![positive_square.clone(), negative_square.clone()]),
            apply_node(ADD, vec![positive_square, negative_square]),
        ],
    ))
}

fn factor_multivariate_grouping(node: &IRNode) -> Option<IRNode> {
    let terms = additive_terms(node)?;
    if terms.len() != 4 {
        return None;
    }

    for first_index in 0..terms.len() - 1 {
        for second_index in first_index + 1..terms.len() {
            let grouped = vec![terms[first_index].clone(), terms[second_index].clone()];
            let first_common = common_symbolic_powers(&grouped);
            if first_common.is_empty() {
                continue;
            }

            let rest: Vec<IRNode> = terms
                .iter()
                .enumerate()
                .filter_map(|(index, term)| {
                    if index == first_index || index == second_index {
                        None
                    } else {
                        Some(term.clone())
                    }
                })
                .collect();
            let first_residual: Vec<IRNode> = grouped
                .iter()
                .map(|term| remove_common_factor(term, &first_common))
                .collect();
            let second_common = common_symbolic_powers(&rest);
            let second_residual: Vec<IRNode> = rest
                .iter()
                .map(|term| remove_common_factor(term, &second_common))
                .collect();
            if !same_two_terms(&first_residual, &second_residual) {
                continue;
            }

            let first_factor = powers_to_ir(&first_common);
            let second_factor = if second_common.is_empty() {
                IRNode::Integer(1)
            } else {
                powers_to_ir(&second_common)
            };
            return Some(apply_node(
                MUL,
                vec![
                    apply_node(ADD, vec![first_factor, second_factor]),
                    add_nodes(first_residual),
                ],
            ));
        }
    }

    None
}

/// Recognise `(a±b)^3` perfect-cube expansions.
///
/// The two identities handled are:
///
/// ```text
/// a^3 + 3·a^2·b + 3·a·b^2 + b^3  →  (a + b)^3   [sum cube]
/// a^3 − 3·a^2·b + 3·a·b^2 − b^3  →  (a − b)^3   [difference cube]
/// ```
///
/// Requires exactly four additive terms: two *pure-cube* terms (`|coeff|=1`,
/// one variable, exponent 3) and two *cross terms* (two variables, `|coeff|=3`).
/// Returns `None` on any mismatch so `factor_handler` can continue to the next
/// pattern.
fn factor_multivariate_perfect_cube(node: &IRNode) -> Option<IRNode> {
    let terms = additive_terms(node)?;
    if terms.len() != 4 {
        return None;
    }

    let mut pure_cubes: Vec<(i64, IRNode)> = Vec::new(); // (sign, base)
    let mut cross_terms: Vec<(i64, HashMap<IRNode, usize>)> = Vec::new();

    for term in &terms {
        let (coefficient, powers) = term_integer_coefficient_and_powers(term)?;
        match powers.len() {
            1 => {
                let (base, exponent) = powers.into_iter().next()?;
                if exponent == 3 && (coefficient == 1 || coefficient == -1) {
                    pure_cubes.push((coefficient, base));
                } else {
                    return None; // wrong exponent or coeff
                }
            }
            2 => {
                cross_terms.push((coefficient, powers));
            }
            _ => return None,
        }
    }

    if pure_cubes.len() != 2 || cross_terms.len() != 2 {
        return None;
    }

    // Identify a and b; derive sum vs. difference from pure-cube signs.
    let (a_node, b_node, is_sum) = {
        let (c0, ref b0) = pure_cubes[0];
        let (c1, ref b1) = pure_cubes[1];
        match (c0, c1) {
            (1, 1) => (b0.clone(), b1.clone(), true),
            (1, -1) => (b0.clone(), b1.clone(), false),
            (-1, 1) => (b1.clone(), b0.clone(), false),
            _ => return None,
        }
    };

    // Cross-term variable sets must equal exactly {a_node, b_node}.
    let variable_pair: HashSet<IRNode> = [a_node.clone(), b_node.clone()].into_iter().collect();
    for (_, powers) in &cross_terms {
        if powers.len() != 2 {
            return None;
        }
        let keys: HashSet<IRNode> = powers.keys().cloned().collect();
        if keys != variable_pair {
            return None;
        }
    }

    // Validate cross-term coefficients and exponent distributions.
    if is_sum {
        // Expect +3·a^2·b and +3·a·b^2 in any order.
        let mut found_a2b = false;
        let mut found_ab2 = false;
        for (coefficient, powers) in &cross_terms {
            let exp_a = powers.get(&a_node).copied().unwrap_or(0);
            let exp_b = powers.get(&b_node).copied().unwrap_or(0);
            match (*coefficient, exp_a, exp_b) {
                (3, 2, 1) => found_a2b = true,
                (3, 1, 2) => found_ab2 = true,
                _ => return None,
            }
        }
        if !found_a2b || !found_ab2 {
            return None;
        }
        Some(apply_node(
            POW,
            vec![apply_node(ADD, vec![a_node, b_node]), IRNode::Integer(3)],
        ))
    } else {
        // Expect −3·a^2·b and +3·a·b^2.
        // The sign on a^2·b flips because (a−b)^3 = a^3 − 3a^2b + 3ab^2 − b^3.
        let mut found_neg_a2b = false;
        let mut found_pos_ab2 = false;
        for (coefficient, powers) in &cross_terms {
            let exp_a = powers.get(&a_node).copied().unwrap_or(0);
            let exp_b = powers.get(&b_node).copied().unwrap_or(0);
            match (*coefficient, exp_a, exp_b) {
                (-3, 2, 1) => found_neg_a2b = true,
                (3, 1, 2) => found_pos_ab2 = true,
                _ => return None,
            }
        }
        if !found_neg_a2b || !found_pos_ab2 {
            return None;
        }
        Some(apply_node(
            POW,
            vec![apply_node(SUB, vec![a_node, b_node]), IRNode::Integer(3)],
        ))
    }
}

fn common_symbolic_powers(terms: &[IRNode]) -> HashMap<IRNode, usize> {
    let Some((first, rest)) = terms.split_first() else {
        return HashMap::new();
    };

    let mut common = term_factor_powers(first);
    for term in rest {
        let powers = term_factor_powers(term);
        common.retain(|base, exponent| {
            if let Some(other) = powers.get(base) {
                *exponent = (*exponent).min(*other);
                *exponent > 0
            } else {
                false
            }
        });
    }
    common
}

fn same_two_terms(left: &[IRNode], right: &[IRNode]) -> bool {
    left.len() == 2
        && right.len() == 2
        && ((left[0] == right[0] && left[1] == right[1])
            || (left[0] == right[1] && left[1] == right[0]))
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

fn term_integer_coefficient_and_powers(term: &IRNode) -> Option<(i64, HashMap<IRNode, usize>)> {
    let mut coefficient: i64 = 1;
    let mut powers = HashMap::new();
    for factor in multiplicative_factors(term) {
        absorb_factor_integer_coefficient_and_powers(factor, &mut coefficient, &mut powers)?;
    }
    Some((coefficient, powers))
}

fn absorb_factor_integer_coefficient_and_powers(
    factor: IRNode,
    coefficient: &mut i64,
    powers: &mut HashMap<IRNode, usize>,
) -> Option<()> {
    match factor {
        IRNode::Integer(value) => {
            *coefficient *= value;
            Some(())
        }
        IRNode::Apply(apply) if is_head_name(&apply.head, NEG) && apply.args.len() == 1 => {
            *coefficient *= -1;
            for inner_factor in multiplicative_factors(&apply.args[0]) {
                absorb_factor_integer_coefficient_and_powers(inner_factor, coefficient, powers)?;
            }
            Some(())
        }
        other => {
            let (base, exponent) = factor_base_power(other)?;
            *powers.entry(base).or_insert(0) += exponent;
            Some(())
        }
    }
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
    m.insert("Abs".to_string(), abs_handler(simplify));
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
