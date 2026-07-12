//! CPU **reference** implementations of the core array operations.
//!
//! These compute exact results today. In a later MA item the same ops will run
//! through the planned `ComputeGraph` on the chosen backend (CPU/GPU) — the
//! lowering and dispatch decision already live in [`crate::accel`]. Keeping a
//! correct reference path here means every PR is usable while the execution
//! layer is built out.

use crate::value::Array;

/// A binary elementwise operator (`+ - * /`, etc.).
///
/// `Max`/`Min`/the six comparisons were added for MA-4e (`apl-runtime`): APL's
/// `⌈`/`⌊` (dyadic ceiling/floor mean "max"/"min", not the monadic rounding
/// meaning) and `= ≠ < ≤ ≥ >` are, like `+ - × ÷`, ordinary scalar dyadic
/// functions — so they plug into [`elementwise`]/[`reduce`]/[`scan`]/[`outer`]
/// for free, exactly as this enum's own doc history anticipated (see MA05 §2:
/// "`BinOp` ... extendable to `Max`/`Min`/comparison ops as APL requires
/// them"). Comparisons follow APL's boolean convention: `1.0` for true, `0.0`
/// for false (never a native `bool`, since the result must stay a plain `f64`
/// array element like every other value in this crate).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Max,
    Min,
    Eq,
    Ne,
    Lt,
    Le,
    Ge,
    Gt,
}

impl BinOp {
    fn apply(self, a: f64, b: f64) -> f64 {
        fn b2f(cond: bool) -> f64 {
            if cond {
                1.0
            } else {
                0.0
            }
        }
        match self {
            BinOp::Add => a + b,
            BinOp::Sub => a - b,
            BinOp::Mul => a * b,
            BinOp::Div => a / b,
            BinOp::Max => a.max(b),
            BinOp::Min => a.min(b),
            BinOp::Eq => b2f(a == b),
            BinOp::Ne => b2f(a != b),
            BinOp::Lt => b2f(a < b),
            BinOp::Le => b2f(a <= b),
            BinOp::Ge => b2f(a >= b),
            BinOp::Gt => b2f(a > b),
        }
    }
}

/// Elementwise binary op with scalar broadcasting. Either operand may be a
/// scalar; otherwise the shapes must match exactly. (Full NumPy/MATLAB
/// broadcasting follows in a later item.)
pub fn elementwise(op: BinOp, a: &Array, b: &Array) -> Result<Array, String> {
    let (ad, bd) = (a.data(), b.data());
    let data: Vec<f64> = match (a.is_scalar(), b.is_scalar()) {
        (true, _) => bd.iter().map(|&y| op.apply(ad[0], y)).collect(),
        (_, true) => ad.iter().map(|&x| op.apply(x, bd[0])).collect(),
        _ => {
            if a.shape() != b.shape() {
                return Err(format!(
                    "non-conformable arrays: {:?} vs {:?}",
                    a.shape(),
                    b.shape()
                ));
            }
            ad.iter().zip(bd).map(|(&x, &y)| op.apply(x, y)).collect()
        }
    };
    // Result takes the non-scalar operand's shape (or the scalar's if both are).
    let shape = if a.is_scalar() { b.shape() } else { a.shape() };
    Array::from_shape(data, shape.to_vec())
}

pub fn add(a: &Array, b: &Array) -> Result<Array, String> {
    elementwise(BinOp::Add, a, b)
}
pub fn sub(a: &Array, b: &Array) -> Result<Array, String> {
    elementwise(BinOp::Sub, a, b)
}
pub fn mul(a: &Array, b: &Array) -> Result<Array, String> {
    elementwise(BinOp::Mul, a, b)
}
pub fn div(a: &Array, b: &Array) -> Result<Array, String> {
    elementwise(BinOp::Div, a, b)
}

/// Matrix product `[m, k] · [k, n] → [m, n]` (column-major throughout).
pub fn matmul(a: &Array, b: &Array) -> Result<Array, String> {
    let (m, ka) = (a.nrows(), a.ncols());
    let (kb, n) = (b.nrows(), b.ncols());
    if ka != kb {
        return Err(format!(
            "matmul: inner dimensions disagree ({m}x{ka} · {kb}x{n})"
        ));
    }
    let (ad, bd) = (a.data(), b.data());
    // The output `[m, n]` can be far larger than either operand (e.g. an outer
    // product), so size it with checked arithmetic rather than risking a wrap.
    let out_len = m
        .checked_mul(n)
        .ok_or_else(|| format!("matmul: output {m}x{n} overflows usize"))?;
    let mut out = vec![0.0; out_len];
    for j in 0..n {
        for i in 0..m {
            let mut acc = 0.0;
            for p in 0..ka {
                acc += ad[p * m + i] * bd[j * kb + p]; // column-major indexing
            }
            out[j * m + i] = acc;
        }
    }
    Array::from_shape(out, vec![m, n])
}

/// Matrix transpose.
pub fn transpose(a: &Array) -> Array {
    let (m, n) = (a.nrows(), a.ncols());
    let ad = a.data();
    // The transpose has exactly as many elements as the input, so allocate from
    // `ad.len()` — that count already fit in memory, so it can't overflow even
    // if a malformed `Array` had `m * n != ad.len()`.
    let mut out = vec![0.0; ad.len()];
    for j in 0..n {
        for i in 0..m {
            out[i * n + j] = ad[j * m + i];
        }
    }
    Array::from_shape(out, vec![n, m]).expect("transpose preserves element count")
}

/// Whole-array reductions.
pub fn sum(a: &Array) -> f64 {
    a.data().iter().sum()
}
pub fn mean(a: &Array) -> f64 {
    if a.is_empty() {
        f64::NAN
    } else {
        sum(a) / a.len() as f64
    }
}
pub fn max(a: &Array) -> f64 {
    a.data().iter().copied().fold(f64::NEG_INFINITY, f64::max)
}
pub fn min(a: &Array) -> f64 {
    a.data().iter().copied().fold(f64::INFINITY, f64::min)
}

// --- AR-2: generalized reduce/scan/outer-product ---------------------------
//
// `sum`/`mean`/`max`/`min` above are each a *fixed*-operator, whole-array
// reduction. APL's `/` (reduce), `\` (scan), and `∘.` (outer product) are
// different: each is parameterized over an *arbitrary* dyadic function — in
// this crate, `BinOp` — so `+/v`, `-/v`, `×/v` (and later `⌈/v`, once `BinOp`
// grows a `Max` variant) are all the *same* kernel, not four bespoke ones.
// See `code/specs/MA05-apl-language.md` §2 for the motivating gap.
//
// These three functions are scoped identically to the rest of this file
// today: rank ≤ 2 (scalar/vector/matrix), matching `value.rs`'s documented
// "higher ranks are stored but only rank ≤ 2 ops are defined" convention —
// and CPU-reference only, exactly how `matmul`/`transpose` started before
// `exec::execute` wired them through the GPU-dispatch planner. Wiring these
// through `accel`/`exec` (so `apl-runtime`'s `+/`/`+\`/`∘.×` are
// GPU-dispatchable like matmul already is) is a natural follow-up, not
// required for AR-2 itself.

/// Fold `a` with `op`, along its *last* axis, producing a rank-reduced
/// result:
/// - a scalar has nothing to fold and reduces to itself;
/// - a vector `[n]` folds to a scalar (`op` applied pairwise, left to right —
///   `reduce(Add, v)` is `((v[0] + v[1]) + v[2]) + …`);
/// - a matrix `[r, c]` folds **each row** across its `c` columns, producing a
///   vector `[r]` (one folded value per row) — this is APL's *default-axis*
///   `F/M`, matching how `matmul`/`ops::sum` already treat "rows × columns"
///   as the matrix's logical shape regardless of the column-major backing
///   store.
///
/// Unlike [`sum`] (which has a built-in identity, `0`, so an empty array
/// sums to `0`), `reduce` is generic over *any* `BinOp` — `Mul`'s identity is
/// `1`, not `0` — so guessing an identity for an arbitrary, possibly future
/// op would be silently wrong for some of them. An empty axis (`n == 0` or
/// `c == 0`) is therefore a clean error instead of a guess.
pub fn reduce(op: BinOp, a: &Array) -> Result<Array, String> {
    match *a.shape() {
        [] => Ok(a.clone()),
        [n] => {
            if n == 0 {
                return Err("reduce: cannot fold an empty vector (no identity element for an arbitrary BinOp)".to_string());
            }
            let d = a.data();
            let folded = d[1..].iter().fold(d[0], |acc, &x| op.apply(acc, x));
            Ok(Array::scalar(folded))
        }
        [r, c] => {
            if c == 0 {
                return Err(
                    "reduce: cannot fold an empty row (no identity element for an arbitrary BinOp)"
                        .to_string(),
                );
            }
            let d = a.data();
            let mut out = vec![0.0; r];
            for (row, slot) in out.iter_mut().enumerate() {
                // Column-major: element (row, col) lives at `col * r + row`.
                let mut acc = d[row];
                for col in 1..c {
                    acc = op.apply(acc, d[col * r + row]);
                }
                *slot = acc;
            }
            Array::from_shape(out, vec![r])
        }
        _ => Err(format!(
            "reduce: rank > 2 not yet supported (shape {:?})",
            a.shape()
        )),
    }
}

/// The same fold as [`reduce`], but keeping **every** intermediate result
/// instead of only the last — the output has the same shape as `a`:
/// - a scalar scans to itself;
/// - a vector `[n]` scans to a vector `[n]`, where element `i` is the fold of
///   `a[0..=i]` (a running total, for `op = Add`);
/// - a matrix `[r, c]` scans **each row** independently across its columns,
///   producing a same-shape `[r, c]` matrix.
///
/// An empty axis is not an error here (unlike `reduce`): there is simply
/// nothing to scan, and the (empty) output shape already says so.
pub fn scan(op: BinOp, a: &Array) -> Result<Array, String> {
    match *a.shape() {
        [] => Ok(a.clone()),
        [n] => {
            let d = a.data();
            let mut out = Vec::with_capacity(n);
            let mut acc: Option<f64> = None;
            for &x in d {
                acc = Some(match acc {
                    None => x,
                    Some(prev) => op.apply(prev, x),
                });
                out.push(acc.expect("just set"));
            }
            Array::from_shape(out, vec![n])
        }
        [r, c] => {
            let d = a.data();
            let mut out = vec![0.0; d.len()];
            for row in 0..r {
                let mut acc: Option<f64> = None;
                for col in 0..c {
                    let x = d[col * r + row]; // column-major
                    acc = Some(match acc {
                        None => x,
                        Some(prev) => op.apply(prev, x),
                    });
                    out[col * r + row] = acc.expect("just set");
                }
            }
            Array::from_shape(out, vec![r, c])
        }
        _ => Err(format!(
            "scan: rank > 2 not yet supported (shape {:?})",
            a.shape()
        )),
    }
}

/// Outer product: apply `op` to every pair `(aᵢ, bⱼ)`, producing a result of
/// rank `rank(a) + rank(b)`:
/// - scalar ⊗ scalar → scalar (`op(a, b)`);
/// - scalar ⊗ vector `[n]` (or vector ⊗ scalar) → vector `[n]` (the scalar
///   broadcasts, exactly like [`elementwise`]'s scalar case);
/// - vector `[m]` ⊗ vector `[n]` → matrix `[m, n]`, element `(i, j) =
///   op(a[i], b[j])`. `Kernel::MatMul` is the `op = Mul`-**then-sum-reduce**
///   special case of this (matrix product *is* `+/¨ v∘.×w` in APL terms) —
///   the product alone, with no summing, is what's new here.
///
/// Scoped to `rank(a) ≤ 1` and `rank(b) ≤ 1` — the vector⊗vector case already
/// reaches this crate's rank-2 ceiling (see `value.rs`), so a higher-rank
/// operand is a clean error pending the N-D generalization this crate's docs
/// already flag as future work, rather than silently-wrong output.
pub fn outer(op: BinOp, a: &Array, b: &Array) -> Result<Array, String> {
    match (a.shape(), b.shape()) {
        ([], []) => Ok(Array::scalar(op.apply(a.data()[0], b.data()[0]))),
        ([], [n]) => {
            let x = a.data()[0];
            let out: Vec<f64> = b.data().iter().map(|&y| op.apply(x, y)).collect();
            Array::from_shape(out, vec![*n])
        }
        ([m], []) => {
            let y = b.data()[0];
            let out: Vec<f64> = a.data().iter().map(|&x| op.apply(x, y)).collect();
            Array::from_shape(out, vec![*m])
        }
        ([m], [n]) => {
            let (ad, bd) = (a.data(), b.data());
            let out_len = m
                .checked_mul(*n)
                .ok_or_else(|| format!("outer: output {m}x{n} overflows usize"))?;
            let mut out = vec![0.0; out_len];
            for j in 0..*n {
                for i in 0..*m {
                    out[j * m + i] = op.apply(ad[i], bd[j]); // column-major
                }
            }
            Array::from_shape(out, vec![*m, *n])
        }
        _ => Err(format!(
            "outer: operands of rank > 1 not yet supported (shapes {:?}, {:?})",
            a.shape(),
            b.shape()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Array;

    #[test]
    fn elementwise_equal_shapes() {
        let a = Array::from_vec(vec![1.0, 2.0, 3.0]);
        let b = Array::from_vec(vec![10.0, 20.0, 30.0]);
        assert_eq!(add(&a, &b).unwrap().data(), &[11.0, 22.0, 33.0]);
        assert_eq!(sub(&b, &a).unwrap().data(), &[9.0, 18.0, 27.0]);
        assert_eq!(mul(&a, &b).unwrap().data(), &[10.0, 40.0, 90.0]);
        assert_eq!(div(&b, &a).unwrap().data(), &[10.0, 10.0, 10.0]);
    }

    #[test]
    fn scalar_broadcasts_either_side() {
        let v = Array::from_vec(vec![1.0, 2.0, 3.0]);
        let s = Array::scalar(2.0);
        assert_eq!(mul(&s, &v).unwrap().data(), &[2.0, 4.0, 6.0]); // scalar lhs
        assert_eq!(add(&v, &s).unwrap().data(), &[3.0, 4.0, 5.0]); // scalar rhs
                                                                   // The result keeps the array operand's shape, not the scalar's.
        assert_eq!(add(&v, &s).unwrap().shape(), &[3]);
    }

    #[test]
    fn two_scalars_stay_scalar() {
        let r = add(&Array::scalar(2.0), &Array::scalar(40.0)).unwrap();
        assert!(r.is_scalar());
        assert_eq!(r.data(), &[42.0]);
    }

    #[test]
    fn nonconformable_is_an_error() {
        let a = Array::from_vec(vec![1.0, 2.0]);
        let b = Array::from_vec(vec![1.0, 2.0, 3.0]);
        assert!(add(&a, &b).is_err());
    }

    #[test]
    fn nan_and_inf_propagate() {
        let a = Array::from_vec(vec![1.0, 0.0]);
        let b = Array::from_vec(vec![0.0, 0.0]);
        let q = div(&a, &b).unwrap();
        assert!(q.data()[0].is_infinite());
        assert!(q.data()[1].is_nan());
    }

    #[test]
    fn matmul_identity_and_product() {
        // [[1,2],[3,4]] · I == itself.
        let a = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap();
        assert_eq!(matmul(&a, &Array::eye(2)).unwrap().data(), a.data());

        // [[1,2],[3,4]] · [[5,6],[7,8]] = [[19,22],[43,50]].
        let b = Array::from_rows(vec![vec![5.0, 6.0], vec![7.0, 8.0]]).unwrap();
        let c = matmul(&a, &b).unwrap();
        assert_eq!(c.shape(), &[2, 2]);
        assert_eq!(c.get(0, 0), Some(19.0));
        assert_eq!(c.get(0, 1), Some(22.0));
        assert_eq!(c.get(1, 0), Some(43.0));
        assert_eq!(c.get(1, 1), Some(50.0));
    }

    #[test]
    fn matmul_nonsquare() {
        // [2x3] · [3x1] -> [2x1].
        let a = Array::from_rows(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]).unwrap();
        let x = Array::from_rows(vec![vec![1.0], vec![0.0], vec![-1.0]]).unwrap();
        let y = matmul(&a, &x).unwrap();
        assert_eq!(y.shape(), &[2, 1]);
        assert_eq!(y.data(), &[-2.0, -2.0]);
    }

    #[test]
    fn matmul_dimension_mismatch_errors() {
        let a = Array::from_rows(vec![vec![1.0, 2.0]]).unwrap(); // 1x2
        let b = Array::from_rows(vec![vec![1.0, 2.0]]).unwrap(); // 1x2
        assert!(matmul(&a, &b).is_err());
    }

    #[test]
    fn transpose_swaps_axes() {
        let a = Array::from_rows(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]).unwrap();
        let t = transpose(&a);
        assert_eq!(t.shape(), &[3, 2]);
        assert_eq!(t.get(0, 0), Some(1.0));
        assert_eq!(t.get(2, 1), Some(6.0));
        // Transpose is an involution.
        assert_eq!(transpose(&t).data(), a.data());
    }

    #[test]
    fn reductions() {
        let a = Array::from_vec(vec![2.0, 4.0, 6.0, 8.0]);
        assert_eq!(sum(&a), 20.0);
        assert_eq!(mean(&a), 5.0);
        assert_eq!(max(&a), 8.0);
        assert_eq!(min(&a), 2.0);
    }

    #[test]
    fn mean_of_empty_is_nan() {
        let empty = Array::from_shape(vec![], vec![0, 0]).unwrap();
        assert!(mean(&empty).is_nan());
    }

    // --- reduce ----------------------------------------------------------

    #[test]
    fn reduce_scalar_is_itself() {
        let s = Array::scalar(7.0);
        assert_eq!(reduce(BinOp::Add, &s).unwrap().data(), &[7.0]);
    }

    #[test]
    fn reduce_vector_folds_to_scalar() {
        let v = Array::from_vec(vec![1.0, 2.0, 3.0, 4.0]);
        // +/v = ((1+2)+3)+4 = 10
        let r = reduce(BinOp::Add, &v).unwrap();
        assert!(r.is_scalar());
        assert_eq!(r.data(), &[10.0]);
        // ×/v = ((1×2)×3)×4 = 24
        assert_eq!(reduce(BinOp::Mul, &v).unwrap().data(), &[24.0]);
    }

    #[test]
    fn reduce_matrix_folds_each_row_across_columns() {
        // [[1,2,3],[4,5,6]] (2x3): row 0 -> 1+2+3=6, row 1 -> 4+5+6=15.
        let m = Array::from_rows(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]).unwrap();
        let r = reduce(BinOp::Add, &m).unwrap();
        assert_eq!(r.shape(), &[2]);
        assert_eq!(r.data(), &[6.0, 15.0]);
    }

    #[test]
    fn reduce_empty_vector_or_row_is_an_error_not_a_guessed_identity() {
        let empty_vec = Array::from_vec(vec![]);
        assert!(reduce(BinOp::Add, &empty_vec).is_err());
        assert!(reduce(BinOp::Mul, &empty_vec).is_err());

        let empty_row_matrix = Array::from_shape(vec![], vec![2, 0]).unwrap();
        assert!(reduce(BinOp::Add, &empty_row_matrix).is_err());
    }

    #[test]
    fn reduce_rejects_rank_above_2() {
        let cube = Array::from_shape(vec![0.0; 8], vec![2, 2, 2]).unwrap();
        assert!(reduce(BinOp::Add, &cube).is_err());
    }

    // --- scan --------------------------------------------------------------

    #[test]
    fn scan_scalar_is_itself() {
        let s = Array::scalar(9.0);
        assert_eq!(scan(BinOp::Add, &s).unwrap().data(), &[9.0]);
    }

    #[test]
    fn scan_vector_keeps_every_running_fold() {
        let v = Array::from_vec(vec![1.0, 2.0, 3.0, 4.0]);
        // +\v = [1, 1+2, 1+2+3, 1+2+3+4] = [1, 3, 6, 10]
        let s = scan(BinOp::Add, &v).unwrap();
        assert_eq!(s.shape(), &[4]);
        assert_eq!(s.data(), &[1.0, 3.0, 6.0, 10.0]);
    }

    #[test]
    fn scan_matrix_scans_each_row_independently() {
        // [[1,2,3],[4,5,6]]: row 0 running sums [1,3,6], row 1 [4,9,15].
        let m = Array::from_rows(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]).unwrap();
        let s = scan(BinOp::Add, &m).unwrap();
        assert_eq!(s.shape(), &[2, 3]);
        assert_eq!(s.get(0, 0), Some(1.0));
        assert_eq!(s.get(0, 1), Some(3.0));
        assert_eq!(s.get(0, 2), Some(6.0));
        assert_eq!(s.get(1, 0), Some(4.0));
        assert_eq!(s.get(1, 1), Some(9.0));
        assert_eq!(s.get(1, 2), Some(15.0));
    }

    #[test]
    fn scan_empty_vector_is_empty_not_an_error() {
        let empty = Array::from_vec(vec![]);
        let s = scan(BinOp::Add, &empty).unwrap();
        assert_eq!(s.shape(), &[0]);
        assert!(s.is_empty());
    }

    #[test]
    fn scan_rejects_rank_above_2() {
        let cube = Array::from_shape(vec![0.0; 8], vec![2, 2, 2]).unwrap();
        assert!(scan(BinOp::Add, &cube).is_err());
    }

    // --- outer ---------------------------------------------------------

    #[test]
    fn outer_scalar_scalar_is_scalar() {
        let r = outer(BinOp::Mul, &Array::scalar(6.0), &Array::scalar(7.0)).unwrap();
        assert!(r.is_scalar());
        assert_eq!(r.data(), &[42.0]);
    }

    #[test]
    fn outer_scalar_vector_broadcasts() {
        let v = Array::from_vec(vec![1.0, 2.0, 3.0]);
        let r = outer(BinOp::Mul, &Array::scalar(10.0), &v).unwrap();
        assert_eq!(r.shape(), &[3]);
        assert_eq!(r.data(), &[10.0, 20.0, 30.0]);

        let r2 = outer(BinOp::Mul, &v, &Array::scalar(10.0)).unwrap();
        assert_eq!(r2.data(), &[10.0, 20.0, 30.0]);
    }

    #[test]
    fn outer_vector_vector_is_rank_sum_matrix() {
        // [1,2,3] outer-x [10,100] = [[10,100],[20,200],[30,300]] (3x2).
        let a = Array::from_vec(vec![1.0, 2.0, 3.0]);
        let b = Array::from_vec(vec![10.0, 100.0]);
        let r = outer(BinOp::Mul, &a, &b).unwrap();
        assert_eq!(r.shape(), &[3, 2]);
        assert_eq!(r.get(0, 0), Some(10.0));
        assert_eq!(r.get(0, 1), Some(100.0));
        assert_eq!(r.get(1, 0), Some(20.0));
        assert_eq!(r.get(1, 1), Some(200.0));
        assert_eq!(r.get(2, 0), Some(30.0));
        assert_eq!(r.get(2, 1), Some(300.0));
    }

    #[test]
    fn outer_add_matches_manual_pairwise_sums() {
        let a = Array::from_vec(vec![1.0, 2.0]);
        let b = Array::from_vec(vec![100.0, 200.0, 300.0]);
        let r = outer(BinOp::Add, &a, &b).unwrap();
        assert_eq!(r.shape(), &[2, 3]);
        for i in 0..2 {
            for j in 0..3 {
                assert_eq!(r.get(i, j), Some(a.data()[i] + b.data()[j]));
            }
        }
    }

    #[test]
    fn outer_rejects_rank_above_1_operands() {
        let m = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap();
        let v = Array::from_vec(vec![1.0, 2.0]);
        assert!(outer(BinOp::Add, &m, &v).is_err());
        assert!(outer(BinOp::Add, &v, &m).is_err());
    }

    #[test]
    fn outer_empty_operand_is_empty_result() {
        let empty = Array::from_vec(vec![]);
        let v = Array::from_vec(vec![1.0, 2.0, 3.0]);
        let r = outer(BinOp::Mul, &empty, &v).unwrap();
        assert_eq!(r.shape(), &[0, 3]);
        assert!(r.is_empty());
    }

    // --- Max/Min/comparisons (added for MA-4e) ----------------------------

    #[test]
    fn max_and_min_are_elementwise() {
        let a = Array::from_vec(vec![1.0, 5.0, 3.0]);
        let b = Array::from_vec(vec![4.0, 2.0, 3.0]);
        assert_eq!(elementwise(BinOp::Max, &a, &b).unwrap().data(), &[4.0, 5.0, 3.0]);
        assert_eq!(elementwise(BinOp::Min, &a, &b).unwrap().data(), &[1.0, 2.0, 3.0]);
    }

    #[test]
    fn comparisons_produce_apl_style_boolean_1_and_0() {
        let a = Array::from_vec(vec![1.0, 2.0, 3.0]);
        let b = Array::from_vec(vec![3.0, 2.0, 1.0]);
        assert_eq!(elementwise(BinOp::Eq, &a, &b).unwrap().data(), &[0.0, 1.0, 0.0]);
        assert_eq!(elementwise(BinOp::Ne, &a, &b).unwrap().data(), &[1.0, 0.0, 1.0]);
        assert_eq!(elementwise(BinOp::Lt, &a, &b).unwrap().data(), &[1.0, 0.0, 0.0]);
        assert_eq!(elementwise(BinOp::Le, &a, &b).unwrap().data(), &[1.0, 1.0, 0.0]);
        assert_eq!(elementwise(BinOp::Ge, &a, &b).unwrap().data(), &[0.0, 1.0, 1.0]);
        assert_eq!(elementwise(BinOp::Gt, &a, &b).unwrap().data(), &[0.0, 0.0, 1.0]);
    }

    #[test]
    fn reduce_and_scan_and_outer_work_with_max() {
        let v = Array::from_vec(vec![3.0, 7.0, 2.0, 9.0, 4.0]);
        assert_eq!(reduce(BinOp::Max, &v).unwrap().data(), &[9.0]);
        assert_eq!(scan(BinOp::Max, &v).unwrap().data(), &[3.0, 7.0, 7.0, 9.0, 9.0]);
        let r = outer(BinOp::Max, &Array::from_vec(vec![1.0, 5.0]), &Array::from_vec(vec![3.0, 2.0])).unwrap();
        assert_eq!(r.shape(), &[2, 2]);
        assert_eq!(r.get(0, 0), Some(3.0)); // max(1,3)
        assert_eq!(r.get(0, 1), Some(2.0)); // max(1,2)
        assert_eq!(r.get(1, 0), Some(5.0)); // max(5,3)
        assert_eq!(r.get(1, 1), Some(5.0)); // max(5,2)
    }
}
