//! TrigReduce identities for powers and simple products.
//!
//! The reduction identities rewrite powers of trig functions into multiple-angle
//! expressions, removing powers where this package has explicit formulas:
//!
//! ```text
//! sin²(x) = (1 − cos(2x)) / 2  =  Mul(1/2, Sub(1, Cos(Mul(2, x))))
//! cos²(x) = (1 + cos(2x)) / 2  =  Mul(1/2, Add(1, Cos(Mul(2, x))))
//! sin³(x) = (3sin(x) − sin(3x)) / 4
//! cos³(x) = (3cos(x) + cos(3x)) / 4
//! sin(x)cos(x) = sin(2x) / 2
//! ```
//!
//! ## Scope
//!
//! `trig_reduce` applies hard-coded exact formulas for `Sin(x)^n` and
//! `Cos(x)^n` where `2 <= n <= 6`, plus the product-to-sum identity for
//! `Sin(x) * Cos(x)` with the same argument. Powers above 6 are left as powers.
//!
//! `power_reduce` remains available as a compatibility wrapper around
//! `trig_reduce`.

use symbolic_ir::{apply, int, rat, sym, IRNode, ADD, COS, MUL, POW, SIN, SUB};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Walk `expr` and reduce supported trig powers/products to multiple-angle forms.
///
/// Applies hard-coded formulas for `Pow(Sin(…), n)` and `Pow(Cos(…), n)`
/// where `2 <= n <= 6`, and rewrites `Mul(Sin(x), Cos(x))` to
/// `Mul(1/2, Sin(Mul(2, x)))`.
///
/// All other expressions are recursed into unchanged.
///
/// # Examples
///
/// ```rust
/// use cas_trig::trig_reduce;
/// use symbolic_ir::{apply, int, rat, sym, ADD, COS, MUL, POW, SIN, SUB};
///
/// // sin²(x) → (1 − cos(2x)) / 2
/// let sin_sq = apply(sym(POW), vec![
///     apply(sym(SIN), vec![sym("x")]),
///     int(2),
/// ]);
/// let reduced = trig_reduce(&sin_sq);
/// let expected = apply(sym(MUL), vec![
///     rat(1, 2),
///     apply(sym(SUB), vec![int(1), apply(sym(COS), vec![
///         apply(sym(MUL), vec![int(2), sym("x")])])])
/// ]);
/// assert_eq!(reduced, expected);
/// ```
pub fn trig_reduce(expr: &IRNode) -> IRNode {
    match expr {
        IRNode::Apply(a) => {
            let head = match &a.head {
                IRNode::Symbol(s) => s.as_str(),
                _ => {
                    let reduced: Vec<_> = a.args.iter().map(trig_reduce).collect();
                    return apply(a.head.clone(), reduced);
                }
            };

            let reduced_args: Vec<_> = a.args.iter().map(trig_reduce).collect();

            if head == POW && reduced_args.len() == 2 {
                if let IRNode::Integer(n) = &reduced_args[1] {
                    if *n >= 2 {
                        if let Some(reduced) = reduce_power(&reduced_args[0], *n) {
                            return reduced;
                        }
                    }
                }
            }

            if head == MUL {
                if let Some(reduced) = reduce_sin_cos_product(&reduced_args) {
                    return reduced;
                }
            }

            apply(a.head.clone(), reduced_args)
        }
        _ => expr.clone(),
    }
}

/// Compatibility alias for callers that imported the original sin²/cos² API.
pub fn power_reduce(expr: &IRNode) -> IRNode {
    trig_reduce(expr)
}

// ---------------------------------------------------------------------------
// Reduction builders
// ---------------------------------------------------------------------------

fn reduce_power(base: &IRNode, n: i64) -> Option<IRNode> {
    if let Some(inner) = extract_sin_arg(base) {
        return sin_power(inner, n);
    }
    if let Some(inner) = extract_cos_arg(base) {
        return cos_power(inner, n);
    }
    None
}

fn sin_power(inner: &IRNode, n: i64) -> Option<IRNode> {
    match n {
        2 => Some(sin_squared(inner)),
        3 => Some(frac(
            apply(
                sym(SUB),
                vec![
                    apply(sym(MUL), vec![int(3), apply(sym(SIN), vec![inner.clone()])]),
                    sin_nx(3, inner),
                ],
            ),
            4,
        )),
        4 => Some(frac(
            apply(
                sym(ADD),
                vec![
                    apply(
                        sym(SUB),
                        vec![int(3), apply(sym(MUL), vec![int(4), cos_nx(2, inner)])],
                    ),
                    cos_nx(4, inner),
                ],
            ),
            8,
        )),
        5 => Some(frac(
            apply(
                sym(ADD),
                vec![
                    apply(
                        sym(SUB),
                        vec![
                            apply(
                                sym(MUL),
                                vec![int(10), apply(sym(SIN), vec![inner.clone()])],
                            ),
                            apply(sym(MUL), vec![int(5), sin_nx(3, inner)]),
                        ],
                    ),
                    sin_nx(5, inner),
                ],
            ),
            16,
        )),
        6 => Some(frac(
            apply(
                sym(SUB),
                vec![
                    apply(
                        sym(ADD),
                        vec![
                            apply(
                                sym(SUB),
                                vec![int(10), apply(sym(MUL), vec![int(15), cos_nx(2, inner)])],
                            ),
                            apply(sym(MUL), vec![int(6), cos_nx(4, inner)]),
                        ],
                    ),
                    cos_nx(6, inner),
                ],
            ),
            32,
        )),
        _ => None,
    }
}

fn cos_power(inner: &IRNode, n: i64) -> Option<IRNode> {
    match n {
        2 => Some(cos_squared(inner)),
        3 => Some(frac(
            apply(
                sym(ADD),
                vec![
                    apply(sym(MUL), vec![int(3), apply(sym(COS), vec![inner.clone()])]),
                    cos_nx(3, inner),
                ],
            ),
            4,
        )),
        4 => Some(frac(
            apply(
                sym(ADD),
                vec![
                    apply(
                        sym(ADD),
                        vec![int(3), apply(sym(MUL), vec![int(4), cos_nx(2, inner)])],
                    ),
                    cos_nx(4, inner),
                ],
            ),
            8,
        )),
        5 => Some(frac(
            apply(
                sym(ADD),
                vec![
                    apply(
                        sym(ADD),
                        vec![
                            apply(
                                sym(MUL),
                                vec![int(10), apply(sym(COS), vec![inner.clone()])],
                            ),
                            apply(sym(MUL), vec![int(5), cos_nx(3, inner)]),
                        ],
                    ),
                    cos_nx(5, inner),
                ],
            ),
            16,
        )),
        6 => Some(frac(
            apply(
                sym(ADD),
                vec![
                    apply(
                        sym(ADD),
                        vec![
                            apply(
                                sym(ADD),
                                vec![int(10), apply(sym(MUL), vec![int(15), cos_nx(2, inner)])],
                            ),
                            apply(sym(MUL), vec![int(6), cos_nx(4, inner)]),
                        ],
                    ),
                    cos_nx(6, inner),
                ],
            ),
            32,
        )),
        _ => None,
    }
}

/// `sin²(inner)` → `Mul(1/2, Sub(1, Cos(Mul(2, inner))))`
fn sin_squared(inner: &IRNode) -> IRNode {
    frac(apply(sym(SUB), vec![int(1), cos_nx(2, inner)]), 2)
}

/// `cos²(inner)` → `Mul(1/2, Add(1, Cos(Mul(2, inner))))`
fn cos_squared(inner: &IRNode) -> IRNode {
    frac(apply(sym(ADD), vec![int(1), cos_nx(2, inner)]), 2)
}

fn reduce_sin_cos_product(args: &[IRNode]) -> Option<IRNode> {
    if args.len() < 2 {
        return None;
    }

    let mut sin_arg: Option<&IRNode> = None;
    let mut cos_arg: Option<&IRNode> = None;
    let mut other = Vec::new();

    for arg in args {
        if sin_arg.is_none() {
            if let Some(inner) = extract_sin_arg(arg) {
                sin_arg = Some(inner);
                continue;
            }
        }
        if cos_arg.is_none() {
            if let Some(inner) = extract_cos_arg(arg) {
                cos_arg = Some(inner);
                continue;
            }
        }
        other.push(arg.clone());
    }

    let (Some(sin_arg), Some(cos_arg)) = (sin_arg, cos_arg) else {
        return None;
    };
    if sin_arg != cos_arg {
        return None;
    }

    let sin_2x_half = frac(sin_nx(2, sin_arg), 2);
    if other.is_empty() {
        return Some(sin_2x_half);
    }

    other.push(sin_2x_half);
    Some(apply(sym(MUL), other))
}

fn sin_nx(n: i64, inner: &IRNode) -> IRNode {
    apply(sym(SIN), vec![apply(sym(MUL), vec![int(n), inner.clone()])])
}

fn cos_nx(n: i64, inner: &IRNode) -> IRNode {
    apply(sym(COS), vec![apply(sym(MUL), vec![int(n), inner.clone()])])
}

fn frac(numerator: IRNode, denominator: i64) -> IRNode {
    apply(sym(MUL), vec![rat(1, denominator), numerator])
}

// ---------------------------------------------------------------------------
// Pattern helpers
// ---------------------------------------------------------------------------

/// If `node` is `Sin(inner)`, return `Some(inner)`.
fn extract_sin_arg(node: &IRNode) -> Option<&IRNode> {
    if let IRNode::Apply(a) = node {
        if matches!(&a.head, IRNode::Symbol(s) if s.as_str() == "Sin") && a.args.len() == 1 {
            return Some(&a.args[0]);
        }
    }
    None
}

/// If `node` is `Cos(inner)`, return `Some(inner)`.
fn extract_cos_arg(node: &IRNode) -> Option<&IRNode> {
    if let IRNode::Apply(a) = node {
        if matches!(&a.head, IRNode::Symbol(s) if s.as_str() == "Cos") && a.args.len() == 1 {
            return Some(&a.args[0]);
        }
    }
    None
}
