//! Type conversion between symbolic-ir `IRNode` and the numerical
//! types Layer-1 cores expose (`numeric_tower::Number`,
//! `r_vector::Double`, plain `f64`).
//!
//! Two directions:
//!
//! - **IR → numeric**: pull a numeric value out of an `IRNode`.
//!   `Integer` and `Rational` convert exactly; `Float` carries
//!   directly; anything else returns `None` so the caller can leave
//!   the expression symbolic.
//! - **numeric → IR**: wrap a result. `Number::Integer` becomes
//!   `IRNode::Integer`, `Number::Rational` becomes `IRNode::Rational`
//!   (in reduced form), `Number::Float` becomes `IRNode::Float`,
//!   etc. Complex and Decimal numbers become symbolic `Apply` nodes
//!   for now — explicit support comes when the symbolic VM grows
//!   first-class complex / decimal heads.
//!
//! NA carried via the r-vector NA bit-pattern survives the round-trip:
//! a `Float(NA_REAL)` becomes `Apply(Symbol("NA"), [])` symbolically,
//! and back the other way.

use numeric_tower::{Integer, Number, Rational};
use r_vector::{is_na_real, na_real, Double};
use symbolic_ir::{apply, sym, IRNode};

// `Integer` and `Rational` are aliases for `num_bigint::BigInt` and
// `num_rational::BigRational`. We use `ToPrimitive` for `to_i64`
// / `to_f64`.
use std::convert::TryFrom;

/// Attempt to extract a numeric value from an `IRNode`. Returns
/// `None` when the node is not a literal (symbol, string, or
/// not-yet-evaluated apply).
///
/// `Rational` is converted by dividing the f64 representations,
/// matching how spreadsheet / R callers expect rational input to be
/// usable in float-context math.
pub fn ir_to_f64(node: &IRNode) -> Option<f64> {
    match node {
        IRNode::Integer(n) => Some(*n as f64),
        IRNode::Rational(n, d) => Some(*n as f64 / *d as f64),
        IRNode::Float(f) => Some(*f),
        // NA encoded as the symbolic `NA()` head — also surface as the
        // r-vector NA bit-pattern when asked for a float.
        IRNode::Apply(boxed) => {
            if let IRNode::Symbol(name) = &boxed.head {
                if name == "NA" && boxed.args.is_empty() {
                    return Some(na_real());
                }
            }
            None
        }
        _ => None,
    }
}

/// Like `ir_to_f64` but returns `None` if the result would be NA.
/// Useful when a function does NOT want to propagate NA through (rare;
/// most do).
pub fn ir_to_f64_no_na(node: &IRNode) -> Option<f64> {
    let v = ir_to_f64(node)?;
    if is_na_real(v) {
        None
    } else {
        Some(v)
    }
}

/// Try to interpret a slice of IR args as a `Double` vector. Returns
/// `None` if any argument is symbolic (not yet resolvable to a
/// number). The resulting `Double` carries NA in the r-vector
/// bit-pattern.
pub fn args_to_double(args: &[IRNode]) -> Option<Double> {
    // Flatten one level of `List(...)` style arguments so callers can
    // pass either `Mean(1, 2, 3)` or `Mean(List(1, 2, 3))` and get the
    // same answer (matches Excel and Lotus where ranges are flattened
    // before reduction).
    let flat = flatten_one_level(args);
    let mut data: Vec<Option<f64>> = Vec::with_capacity(flat.len());
    for arg in &flat {
        match ir_to_f64(arg) {
            Some(v) if is_na_real(v) => data.push(None),
            Some(v) => data.push(Some(v)),
            None => return None,
        }
    }
    Some(Double::from_optional(data))
}

/// Flatten one level of `Apply(Symbol("List"), [...])` or
/// `Apply(Symbol("Range"), [...])` arguments. The current symbolic VM
/// uses `List` as the natural array head; spreadsheet ranges arrive
/// pre-flattened, but R-style and MACSYMA frontends pass `List(...)`
/// containers.
fn flatten_one_level(args: &[IRNode]) -> Vec<IRNode> {
    let mut out = Vec::with_capacity(args.len());
    for a in args {
        if let IRNode::Apply(boxed) = a {
            if let IRNode::Symbol(name) = &boxed.head {
                if name == "List" || name == "Range" || name == "Vector" {
                    for inner in &boxed.args {
                        out.push(inner.clone());
                    }
                    continue;
                }
            }
        }
        out.push(a.clone());
    }
    out
}

/// Wrap a `Number` back into an `IRNode`. The conversion is
/// exact-preserving: `Number::Integer` → `IRNode::Integer`,
/// `Number::Rational` → `IRNode::Rational` (in reduced form),
/// `Number::Float` → `IRNode::Float`, `Number::Complex` →
/// `Apply(Symbol("Complex"), [re, im])`, `Number::Decimal` →
/// `Float(decimal.to_f64())` (lossy; documented).
pub fn number_to_ir(n: Number) -> IRNode {
    match n {
        Number::Integer(big) => big_int_to_ir(&big),
        Number::Rational(r) => rational_to_ir(&r),
        Number::Float(f) => {
            if is_na_real(f) {
                apply(sym("NA"), vec![])
            } else {
                IRNode::Float(f)
            }
        }
        Number::Complex(c) => apply(
            sym("Complex"),
            vec![IRNode::Float(c.re), IRNode::Float(c.im)],
        ),
        Number::Decimal(d) => IRNode::Float(d.to_f64()),
    }
}

fn big_int_to_ir(big: &Integer) -> IRNode {
    // BigInt has a TryFrom<&BigInt> for i64 via the num-bigint crate's
    // implementation (i64::try_from(&BigInt) returns Result). We use
    // that to avoid pulling in num-traits explicitly.
    match i64::try_from(big) {
        Ok(v) => IRNode::Integer(v),
        // Falls back to a lossy f64 approximation; the symbolic VM has
        // no big-integer rung yet, so this is the best we can do.
        Err(_) => IRNode::Float(big_int_to_f64(big)),
    }
}

fn rational_to_ir(r: &Rational) -> IRNode {
    let num = r.numer();
    let den = r.denom();
    match (i64::try_from(num), i64::try_from(den)) {
        (Ok(n), Ok(d)) => IRNode::rational(n, d),
        _ => IRNode::Float(big_int_to_f64(num) / big_int_to_f64(den)),
    }
}

/// Loss-tolerant `BigInt` → `f64`. Falls through `num-bigint`'s
/// public formatting when no built-in conversion is available; we go
/// via parsing the decimal string to keep this crate light on
/// dependencies (no extra `num-traits` import).
fn big_int_to_f64(big: &Integer) -> f64 {
    big.to_string().parse::<f64>().unwrap_or(f64::INFINITY)
}

/// Plain f64 → IRNode. NA becomes `Apply(Symbol("NA"), [])`.
pub fn f64_to_ir(v: f64) -> IRNode {
    if is_na_real(v) {
        apply(sym("NA"), vec![])
    } else {
        IRNode::Float(v)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ir_to_f64_for_each_literal_form() {
        assert_eq!(ir_to_f64(&IRNode::Integer(42)), Some(42.0));
        assert_eq!(ir_to_f64(&IRNode::Float(3.14)), Some(3.14));
        let r = IRNode::rational(1, 4);
        assert_eq!(ir_to_f64(&r), Some(0.25));
    }

    #[test]
    fn ir_to_f64_returns_none_for_symbols() {
        assert!(ir_to_f64(&sym("x")).is_none());
        assert!(ir_to_f64(&IRNode::Str("hello".into())).is_none());
    }

    #[test]
    fn na_apply_round_trips() {
        let na_node = apply(sym("NA"), vec![]);
        let v = ir_to_f64(&na_node).unwrap();
        assert!(is_na_real(v));
        let back = f64_to_ir(v);
        assert_eq!(back, na_node);
    }

    #[test]
    fn args_to_double_flattens_list_wrapper() {
        let list = apply(
            sym("List"),
            vec![IRNode::Integer(1), IRNode::Integer(2), IRNode::Integer(3)],
        );
        let d = args_to_double(&[list]).unwrap();
        use r_vector::Vector as _;
        assert_eq!(d.len(), 3);
    }

    #[test]
    fn args_to_double_propagates_na() {
        use r_vector::Vector as _;
        let args = vec![IRNode::Integer(1), apply(sym("NA"), vec![]), IRNode::Integer(3)];
        let d = args_to_double(&args).unwrap();
        assert!(d.is_na(1));
    }

    #[test]
    fn args_to_double_returns_none_if_any_symbolic() {
        let args = vec![IRNode::Integer(1), sym("x")];
        assert!(args_to_double(&args).is_none());
    }

    #[test]
    fn number_to_ir_preserves_type() {
        // Integer.
        let n = Number::Integer(numeric_tower::Integer::from(42_i64));
        assert_eq!(number_to_ir(n), IRNode::Integer(42));
        // Float.
        assert_eq!(number_to_ir(Number::Float(3.14)), IRNode::Float(3.14));
    }

    #[test]
    fn number_to_ir_na_becomes_na_apply() {
        let na_value = Number::Float(na_real());
        assert_eq!(number_to_ir(na_value), apply(sym("NA"), vec![]));
    }
}
