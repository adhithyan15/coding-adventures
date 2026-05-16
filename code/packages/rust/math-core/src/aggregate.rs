//! Cross-array aggregate functions.
//!
//! Currently exposes Excel's `SUMPRODUCT`, which is the workhorse for many
//! spreadsheet patterns (weighted sums, conditional counts via boolean
//! arrays, dot products). Other aggregates (`SUMIF`, `SUMIFS`, `AGGREGATE`)
//! live above this layer because they involve predicate machinery the
//! spreadsheet adapter owns.

use crate::{MathError, MathResult};
use numeric_tower::Number;
use r_vector::{is_na_real, na_real, Double};

/// Excel `SUMPRODUCT(a1, a2, ..., aN)`. Element-wise multiply across all
/// arrays, then sum.
///
/// Rules:
/// * All arrays must have the same length, else `BadParameter`.
/// * The zero-argument case returns `0` by convention.
/// * If any element of any array at position `i` is NA, the corresponding
///   product contributes NA — and following spreadsheet `na_rm = false`
///   semantics, the entire sum becomes NA. (Spreadsheet frontends can wrap
///   this with their own NA handling if they want `na_rm = true`.)
/// * Kahan summation keeps long products numerically stable.
pub fn sumproduct(arrays: &[&Double]) -> MathResult<Number> {
    if arrays.is_empty() {
        return Ok(Number::Float(0.0));
    }
    let n = arrays[0].len();
    for (idx, a) in arrays.iter().enumerate() {
        if a.len() != n {
            return Err(MathError::BadParameter {
                name: "arrays",
                value: format!("array {idx} has length {} (expected {n})", a.len()),
            });
        }
    }
    // Kahan summation; if any product is NA, short-circuit to NA result.
    let mut sum = 0.0_f64;
    let mut compensation = 0.0_f64;
    for i in 0..n {
        let mut product = 1.0_f64;
        let mut na_seen = false;
        for a in arrays {
            // Safe: bounds-checked above.
            let v = a.get_value(i).expect("bounds checked");
            if is_na_real(v) {
                na_seen = true;
                break;
            }
            product *= v;
        }
        if na_seen {
            return Ok(Number::Float(na_real()));
        }
        let y = product - compensation;
        let t = sum + y;
        compensation = (t - sum) - y;
        sum = t;
    }
    Ok(Number::Float(sum))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract(n: Number) -> f64 {
        match n {
            Number::Float(v) => v,
            other => panic!("expected float, got {other:?}"),
        }
    }

    #[test]
    fn sumproduct_two_arrays() {
        let a = Double::from_values(vec![1.0, 2.0, 3.0, 4.0]);
        let b = Double::from_values(vec![5.0, 6.0, 7.0, 8.0]);
        // 1*5 + 2*6 + 3*7 + 4*8 = 5 + 12 + 21 + 32 = 70
        assert_eq!(extract(sumproduct(&[&a, &b]).unwrap()), 70.0);
    }

    #[test]
    fn sumproduct_single_array_is_sum() {
        let a = Double::from_values(vec![1.0, 2.0, 3.0]);
        assert_eq!(extract(sumproduct(&[&a]).unwrap()), 6.0);
    }

    #[test]
    fn sumproduct_empty_is_zero() {
        assert_eq!(extract(sumproduct(&[]).unwrap()), 0.0);
    }

    #[test]
    fn sumproduct_three_arrays() {
        let a = Double::from_values(vec![1.0, 2.0]);
        let b = Double::from_values(vec![3.0, 4.0]);
        let c = Double::from_values(vec![5.0, 6.0]);
        // 1*3*5 + 2*4*6 = 15 + 48 = 63
        assert_eq!(extract(sumproduct(&[&a, &b, &c]).unwrap()), 63.0);
    }

    #[test]
    fn sumproduct_length_mismatch() {
        let a = Double::from_values(vec![1.0, 2.0]);
        let b = Double::from_values(vec![1.0, 2.0, 3.0]);
        assert!(matches!(
            sumproduct(&[&a, &b]).unwrap_err(),
            MathError::BadParameter { .. }
        ));
    }

    #[test]
    fn sumproduct_propagates_na() {
        let a = Double::from_values(vec![1.0, na_real(), 3.0]);
        let b = Double::from_values(vec![4.0, 5.0, 6.0]);
        match sumproduct(&[&a, &b]).unwrap() {
            Number::Float(v) => assert!(is_na_real(v)),
            other => panic!("expected float NA, got {other:?}"),
        }
    }
}
