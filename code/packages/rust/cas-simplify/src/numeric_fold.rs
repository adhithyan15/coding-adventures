//! Constant-folding inside `Add` and `Mul` arg lists.
//!
//! When canonicalization brings numeric literals together in a commutative
//! arg list, we collapse them into a single literal:
//!
//! ```text
//! Add(2, 3, x)  →  Add(5, x)
//! Mul(2, 3, x)  →  Mul(6, x)
//! ```
//!
//! ## Arithmetic precision
//!
//! Integer and rational arithmetic stays **exact** throughout.  Internally
//! the accumulator uses `i128` arithmetic for intermediate products (to avoid
//! overflow when multiplying two `i64` values together) and reduces back to
//! the smallest exact `IRNode` variant at the end:
//!
//! - Result denominator == 1 → `IRNode::Integer`
//! - Otherwise → `IRNode::Rational`
//!
//! A single `IRNode::Float` anywhere in the literal cluster contaminates
//! the entire fold to `f64`.  This matches Python's `Fraction | float`
//! promotion semantics.
//!
//! ## Scope of folding
//!
//! Only **direct** numeric children of an `Add` or `Mul` are folded in one
//! pass — nested heads are not reached.  This is intentional: the outer
//! fixed-point loop in `simplify` ensures that `Add(2, Mul(3, 4))` folds
//! the inner `Mul` first (giving `Add(2, 12)`) and then folds the outer `Add`
//! on the next pass.

use symbolic_ir::{IRApply, IRNode, ADD, MUL};

// ---------------------------------------------------------------------------
// Public entry
// ---------------------------------------------------------------------------

/// Recursively fold numeric literals inside `Add` and `Mul` arg lists.
///
/// All other node forms are returned unchanged.
pub fn numeric_fold(node: IRNode) -> IRNode {
    match node {
        IRNode::Apply(apply) => numeric_fold_apply(*apply),
        other => other,
    }
}

// ---------------------------------------------------------------------------
// Per-apply processing
// ---------------------------------------------------------------------------

fn numeric_fold_apply(node: IRApply) -> IRNode {
    // Recursively process every child first (bottom-up).
    let new_head = numeric_fold(node.head);
    let new_args: Vec<IRNode> = node.args.into_iter().map(numeric_fold).collect();

    if let IRNode::Symbol(ref name) = new_head {
        if name == ADD || name == MUL {
            let is_mul = name == MUL;
            let folded = fold_numerics(new_args, is_mul);

            // Singleton drop (may have been created by folding): Add(x) → x.
            if folded.len() == 1 {
                return folded.into_iter().next().unwrap();
            }

            // Empty container (shouldn't happen after canonical, but be safe).
            if folded.is_empty() {
                return if is_mul { IRNode::Integer(1) } else { IRNode::Integer(0) };
            }

            return IRNode::Apply(Box::new(IRApply { head: new_head, args: folded }));
        }
    }

    IRNode::Apply(Box::new(IRApply { head: new_head, args: new_args }))
}

// ---------------------------------------------------------------------------
// Numeric accumulator
// ---------------------------------------------------------------------------

/// Internal representation of a partial fold result.
///
/// We keep `i128` numerator/denominator to avoid overflow when multiplying
/// two `i64` values together mid-fold.  The result is reduced by GCD so the
/// denominator stays bounded.
#[derive(Clone)]
enum Acc {
    /// Exact rational `numer / denom` (always in reduced form, denom > 0).
    Rat(i128, i128),
    /// Float-contaminated: once a float is seen, the whole fold goes f64.
    Flt(f64),
}

impl Acc {
    /// Identity element for addition (`0`) or multiplication (`1`).
    fn identity(is_mul: bool) -> Self {
        if is_mul {
            Acc::Rat(1, 1)
        } else {
            Acc::Rat(0, 1)
        }
    }

    /// True iff this value equals the identity for the given operation.
    fn is_identity(&self, is_mul: bool) -> bool {
        match self {
            Acc::Rat(n, d) => {
                if is_mul {
                    *n == 1 && *d == 1
                } else {
                    *n == 0
                }
            }
            Acc::Flt(f) => {
                if is_mul {
                    *f == 1.0
                } else {
                    *f == 0.0
                }
            }
        }
    }

    /// Convert to `f64` (for float-contaminated operations).
    fn as_f64(&self) -> f64 {
        match self {
            Acc::Rat(n, d) => *n as f64 / *d as f64,
            Acc::Flt(f) => *f,
        }
    }

    /// Combine `self` with `val` via addition or multiplication.
    fn combine(self, val: Acc, is_mul: bool) -> Acc {
        match (self, val) {
            // Float contamination: either operand float → float result.
            (Acc::Flt(a), v) => {
                let b = v.as_f64();
                Acc::Flt(if is_mul { a * b } else { a + b })
            }
            (s, Acc::Flt(b)) => {
                let a = s.as_f64();
                Acc::Flt(if is_mul { a * b } else { a + b })
            }
            // Both rational: stay exact.
            (Acc::Rat(an, ad), Acc::Rat(bn, bd)) => {
                if is_mul {
                    // (an/ad) * (bn/bd) = (an*bn) / (ad*bd)
                    rat_reduce(an * bn, ad * bd)
                } else {
                    // (an/ad) + (bn/bd) = (an*bd + bn*ad) / (ad*bd)
                    rat_reduce(an * bd + bn * ad, ad * bd)
                }
            }
        }
    }

    /// Convert to the smallest IR numeric literal.
    fn into_irnode(self) -> IRNode {
        match self {
            Acc::Flt(f) => IRNode::Float(f),
            Acc::Rat(n, d) => {
                // Cast back to i64: safe because all input values were i64 and
                // GCD reduction only makes values smaller.
                IRNode::rational(n as i64, d as i64)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Fold helpers
// ---------------------------------------------------------------------------

/// Fold all numeric literals in `args` into one, leaving non-literal args in
/// their original relative order.
///
/// If no literals are present, returns `args` (via the `other` accumulation)
/// unchanged.  If the folded literal equals the identity *and* there are
/// non-literal args, the literal is dropped — it's redundant.
fn fold_numerics(args: Vec<IRNode>, is_mul: bool) -> Vec<IRNode> {
    let mut acc = Acc::identity(is_mul);
    let mut saw_literal = false;
    let mut other: Vec<IRNode> = Vec::new();

    for arg in args {
        match node_to_acc(&arg) {
            Some(val) => {
                saw_literal = true;
                acc = acc.combine(val, is_mul);
            }
            None => other.push(arg),
        }
    }

    if !saw_literal {
        // Nothing to fold — return what we collected (== original args).
        return other;
    }

    // If the accumulated value is the identity AND there are non-literal args,
    // drop it — the identity is redundant (canonical/identity rules will clean
    // up any remaining Add(x, 0) on the next pass).
    if acc.is_identity(is_mul) && !other.is_empty() {
        return other;
    }

    // Prepend the folded literal so canonical's sort key puts it first.
    let mut result = vec![acc.into_irnode()];
    result.extend(other);
    result
}

/// Extract a numeric value from `node`, or return `None`.
fn node_to_acc(node: &IRNode) -> Option<Acc> {
    match node {
        IRNode::Integer(n) => Some(Acc::Rat(*n as i128, 1)),
        IRNode::Rational(n, d) => Some(Acc::Rat(*n as i128, *d as i128)),
        IRNode::Float(f) => Some(Acc::Flt(*f)),
        _ => None,
    }
}

/// Reduce a rational to lowest terms with a positive denominator.
fn rat_reduce(n: i128, d: i128) -> Acc {
    if d == 0 {
        // Shouldn't happen in well-formed expressions; surface as NaN float.
        return Acc::Flt(f64::NAN);
    }
    let g = gcd(n.unsigned_abs(), d.unsigned_abs()) as i128;
    let n = n / g;
    let d = d / g;
    // Canonical form: denominator always positive.
    if d < 0 {
        Acc::Rat(-n, -d)
    } else {
        Acc::Rat(n, d)
    }
}

/// Euclidean GCD for unsigned 128-bit integers.
fn gcd(a: u128, b: u128) -> u128 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}
