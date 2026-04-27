//! Normalization of complex-number IR expressions.
//!
//! The canonical form of a complex number in the IR is `Add(real, Mul(imag, I))`
//! where `real` and `imag` are real-valued sub-expressions (free of `I`).
//! The [`complex_normalize`] function rewrites expressions into this form
//! by:
//!
//! 1. Distributing multiplication over addition (`(a+b·i)·(c+d·i)` → `(ac−bd) + (ad+bc)·i`).
//! 2. Simplifying `I²` → `−1`.
//! 3. Collecting real and imaginary parts into a single `Add`.
//!
//! **Scope:** handles `IRNode::Integer`, `IRNode::Float`, `IRNode::Rational`,
//! `Symbol("I")`, and `Apply` nodes with heads `Add`, `Sub`, `Mul`, `Neg`.
//! Expressions involving `Pow(I, n)` for integer `n` are simplified via
//! `i^n` cycling.  All other expressions are treated as opaque real values.
//!
//! # Internal representation
//!
//! Internally, [`complex_normalize`] returns a `(real, imag)` pair of
//! `IRNode`s (both in the original IR), then assembles the canonical form.
//! Zero-valued parts are suppressed (a zero real → pure imaginary; zero imag
//! → pure real; both zero → `Integer(0)`).

use symbolic_ir::{apply, int, sym, IRNode, ADD, MUL, NEG, SUB};

use crate::constants::IMAGINARY_UNIT;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Normalize `expr` into canonical `a + b·I` form.
///
/// Returns the expression with real and imaginary parts separated.
///
/// # Examples
///
/// ```rust
/// use cas_complex::complex_normalize;
/// use symbolic_ir::{apply, int, sym, ADD, MUL};
///
/// // Already canonical: 3 + 4*I
/// let expr = apply(sym(ADD), vec![int(3), apply(sym(MUL), vec![int(4), sym("I")])]);
/// let result = complex_normalize(&expr);
/// // result has the same structure since it's already in canonical form.
///
/// // I^2 → -1 (pure real)
/// use symbolic_ir::POW;
/// let i_sq = apply(sym(POW), vec![sym("I"), int(2)]);
/// assert_eq!(complex_normalize(&i_sq), int(-1));
/// ```
pub fn complex_normalize(expr: &IRNode) -> IRNode {
    let (re, im) = split_complex(expr);
    assemble(re, im)
}

// ---------------------------------------------------------------------------
// Internal: split into (real, imag) IRNode pair
// ---------------------------------------------------------------------------

/// Split `expr` into `(real_part, imag_part)` as `IRNode`s.
///
/// Both parts are returned as IR expressions representing real-valued
/// quantities (no `I` appears in them).
pub(crate) fn split_complex(expr: &IRNode) -> (IRNode, IRNode) {
    match expr {
        // Numeric literals are purely real.
        IRNode::Integer(_) | IRNode::Rational(_, _) | IRNode::Float(_) => {
            (expr.clone(), int(0))
        }

        // The imaginary unit symbol I: re=0, im=1
        IRNode::Symbol(name) if name == IMAGINARY_UNIT => (int(0), int(1)),

        // Any other symbol is treated as a real-valued atom.
        IRNode::Symbol(_) | IRNode::Str(_) => (expr.clone(), int(0)),

        IRNode::Apply(a) => {
            let head_name = match &a.head {
                IRNode::Symbol(s) => s.as_str(),
                _ => return (expr.clone(), int(0)),
            };

            match head_name {
                // Add(a, b, c, …): split each child and sum real+imag parts.
                "Add" => {
                    let mut re = int(0);
                    let mut im = int(0);
                    for arg in &a.args {
                        let (ar, ai) = split_complex(arg);
                        re = add_ir(re, ar);
                        im = add_ir(im, ai);
                    }
                    (re, im)
                }

                // Sub(a, b): split and subtract.
                "Sub" if a.args.len() == 2 => {
                    let (ar, ai) = split_complex(&a.args[0]);
                    let (br, bi) = split_complex(&a.args[1]);
                    (sub_ir(ar, br), sub_ir(ai, bi))
                }

                // Neg(a): negate both parts.
                "Neg" if a.args.len() == 1 => {
                    let (ar, ai) = split_complex(&a.args[0]);
                    (neg_ir(ar), neg_ir(ai))
                }

                // Mul(a, b, …): multiply complex numbers pairwise.
                // (a+bi)·(c+di) = (ac−bd) + (ad+bc)·i
                "Mul" => {
                    let mut re = int(1);
                    let mut im = int(0);
                    for arg in &a.args {
                        let (ar, ai) = split_complex(arg);
                        // new_re = re*ar − im*ai
                        // new_im = re*ai + im*ar
                        let new_re = sub_ir(mul_ir(re.clone(), ar.clone()), mul_ir(im.clone(), ai.clone()));
                        let new_im = add_ir(mul_ir(re, ai), mul_ir(im, ar));
                        re = new_re;
                        im = new_im;
                    }
                    (re, im)
                }

                // Pow(I, n) for integer n: cycle i^n through 1, i, -1, -i.
                "Pow" if a.args.len() == 2 => {
                    if a.args[0] == sym(IMAGINARY_UNIT) {
                        if let IRNode::Integer(n) = &a.args[1] {
                            return i_power(*n);
                        }
                    }
                    // Generic Pow: treat as opaque real.
                    (expr.clone(), int(0))
                }

                // Unknown head: treat as opaque real.
                _ => (expr.clone(), int(0)),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// i^n cycle: 1, i, -1, -i, 1, ...
// ---------------------------------------------------------------------------

/// Compute `I^n` for integer `n`, cycling through the four values.
///
/// ```text
/// n mod 4 = 0 → 1      (re=1, im=0)
/// n mod 4 = 1 → i      (re=0, im=1)
/// n mod 4 = 2 → -1     (re=-1, im=0)
/// n mod 4 = 3 → -i     (re=0, im=-1)
/// ```
///
/// Negative exponents use the same cycle (i^−1 = −i, etc.).
fn i_power(n: i64) -> (IRNode, IRNode) {
    // Normalise n to [0, 4) using Euclidean remainder.
    let r = n.rem_euclid(4);
    match r {
        0 => (int(1), int(0)),   // 1
        1 => (int(0), int(1)),   // i
        2 => (int(-1), int(0)),  // -1
        3 => (int(0), int(-1)),  // -i
        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// Assemble (real, imag) → canonical IRNode
// ---------------------------------------------------------------------------

/// Build the canonical `a + b·I` form from `(real, imag)` parts.
///
/// Suppresses zero terms:
/// - `imag = 0` → `real`
/// - `real = 0` → `Mul(imag, I)` (or just `I` when `imag = 1`)
/// - both zero → `Integer(0)`
pub(crate) fn assemble(re: IRNode, im: IRNode) -> IRNode {
    let re_zero = is_zero(&re);
    let im_zero = is_zero(&im);

    match (re_zero, im_zero) {
        (true, true) => int(0),
        (false, true) => re,
        (true, false) => im_term(im),
        (false, false) => apply(sym(ADD), vec![re, im_term(im)]),
    }
}

/// Build the imaginary-part term: `b·I` (or just `I` when `b = 1`).
fn im_term(im: IRNode) -> IRNode {
    if im == int(1) {
        sym(IMAGINARY_UNIT)
    } else {
        apply(sym(MUL), vec![im, sym(IMAGINARY_UNIT)])
    }
}

// ---------------------------------------------------------------------------
// Arithmetic helpers — build IR expressions
// ---------------------------------------------------------------------------
// These helpers produce flat, un-simplified IR.  Zero-cancellation is
// handled by `is_zero` checks rather than a full simplifier.

fn is_zero(n: &IRNode) -> bool {
    matches!(n, IRNode::Integer(0))
}

fn is_one(n: &IRNode) -> bool {
    matches!(n, IRNode::Integer(1))
}

fn add_ir(a: IRNode, b: IRNode) -> IRNode {
    if is_zero(&a) {
        return b;
    }
    if is_zero(&b) {
        return a;
    }
    // Numeric folding for integers
    if let (IRNode::Integer(x), IRNode::Integer(y)) = (&a, &b) {
        return int(x + y);
    }
    apply(sym(ADD), vec![a, b])
}

fn sub_ir(a: IRNode, b: IRNode) -> IRNode {
    if is_zero(&b) {
        return a;
    }
    if is_zero(&a) {
        return neg_ir(b);
    }
    // Numeric folding
    if let (IRNode::Integer(x), IRNode::Integer(y)) = (&a, &b) {
        return int(x - y);
    }
    apply(sym(SUB), vec![a, b])
}

fn neg_ir(a: IRNode) -> IRNode {
    if is_zero(&a) {
        return int(0);
    }
    if let IRNode::Integer(n) = &a {
        return int(-n);
    }
    apply(sym(NEG), vec![a])
}

fn mul_ir(a: IRNode, b: IRNode) -> IRNode {
    if is_zero(&a) || is_zero(&b) {
        return int(0);
    }
    if is_one(&a) {
        return b;
    }
    if is_one(&b) {
        return a;
    }
    // Numeric folding
    if let (IRNode::Integer(x), IRNode::Integer(y)) = (&a, &b) {
        return int(x * y);
    }
    apply(sym(MUL), vec![a, b])
}
