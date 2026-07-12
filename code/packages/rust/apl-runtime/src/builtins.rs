//! `⍴` (shape/reshape), `⍳` (index-generator/index-of), and `,`
//! (ravel/catenate) — APL's three "bespoke" primitives (MA05 §4).
//!
//! Every other primitive function atom (`+ - × ÷ ⌈ ⌊ = ≠ < ≤ ≥ >`) shares
//! *one* shape: a monadic and a dyadic meaning that both boil down to
//! `array_runtime::ops::BinOp` — see `eval.rs`'s `apply_monadic_scalar` /
//! `AplFn::Atom`. These three primitives do not fit that mould at all (their
//! monadic and dyadic meanings are unrelated to each other, and neither is
//! "an elementwise scalar function"), so they get their own bespoke
//! implementations here rather than being forced through `BinOp`.

use array_runtime::Array;

/// Upper bound on any array this crate allocates from a **runtime-computed**
/// size — monadic `⍳n`'s `n`, and dyadic `⍴`'s target element count — checked
/// *before* allocating, so a crafted `⍳2000000` is a clean `Err` instead of a
/// 2-million-element allocation. Same value and rationale as
/// `wolfram-runtime::builtins::MAX_LIST_LENGTH`: this repo's established cap
/// for a user-controlled array length (see that crate's `builtins.rs`).
pub const MAX_ARRAY_LENGTH: usize = 1_000_000;

// ── ⍴ shape / reshape ───────────────────────────────────────────────────────

/// Monadic `⍴` (shape-of): a scalar has **zero** dimensions, so its shape is
/// the *empty* vector (not a scalar!) — `⍴5` is `⍳0`-shaped, a length-0
/// vector, exactly mirroring `Array::shape() == []` for a rank-0 value. A
/// vector `[n]` has shape `[n]` (a 1-element vector); a matrix `[r, c]` has
/// shape `[r, c]` (a 2-element vector).
pub fn shape(a: &Array) -> Array {
    let dims: Vec<f64> = a.shape().iter().map(|&d| d as f64).collect();
    Array::from_vec(dims)
}

/// Dyadic `⍴` (reshape): `a` is the new shape — a scalar or vector (rank ≤ 1)
/// of non-negative integers, itself capped at rank ≤ 2 (this crate's, and
/// `array_runtime::ops`'s, established ceiling; a longer shape is a clean
/// "not yet supported" error, the same honest-subset convention MA05 §4 uses
/// throughout). `b`'s elements are ravelled (see [`ravel`]) and then
/// cyclically repeated or truncated to fill the target shape's element
/// count — APL's textbook reshape semantics.
pub fn reshape(a: &Array, b: &Array) -> Result<Array, String> {
    let dims = shape_vector(a)?;
    if dims.len() > 2 {
        return Err(format!(
            "⍴: reshape to rank > 2 is not yet supported (target shape {dims:?})"
        ));
    }
    // Checked multiplication: a crafted shape whose product overflows usize
    // must not wrap into a small count that spuriously slips past the cap
    // below (mirrors `array_runtime::value::Array::from_shape`'s own guard).
    let total: usize = dims
        .iter()
        .try_fold(1usize, |acc, &d| acc.checked_mul(d))
        .ok_or_else(|| format!("⍴: shape {dims:?} element count overflows usize"))?;
    if total > MAX_ARRAY_LENGTH {
        return Err(format!(
            "⍴: reshape target of {total} elements exceeds the cap of {MAX_ARRAY_LENGTH}"
        ));
    }
    let source = flatten(b);
    if total > 0 && source.is_empty() {
        return Err("⍴: cannot reshape an empty source into a non-empty shape".to_string());
    }
    // The cyclic fill is in ROW-MAJOR order (APL's reshape fills the *last*
    // axis fastest, same convention as ravel) -- `filled[k]` is the value at
    // logical row-major position `k`.
    // Safe: `total > 0` implies `source` is non-empty (checked above), and
    // when `total == 0` the range below is empty so the closure never runs.
    let filled: Vec<f64> = (0..total).map(|k| source[k % source.len()]).collect();
    match dims.as_slice() {
        // Rank ≤ 1: row-major and column-major coincide, so the fill is
        // already in the right order for `Array::from_shape` (which expects
        // column-major data).
        [] | [_] => Array::from_shape(filled, dims),
        // Rank 2: `Array::from_shape` expects COLUMN-major data
        // (`value.rs`: element `(r, c)` lives at `c * nrows + r`), so the
        // row-major `filled` sequence must be transposed into that layout —
        // handing `filled` straight to `from_shape` would silently reshape
        // column-major instead of APL's row-major convention.
        [r, c] => {
            let mut data = vec![0.0; total];
            for row in 0..*r {
                for col in 0..*c {
                    data[col * r + row] = filled[row * c + col];
                }
            }
            Array::from_shape(data, dims)
        }
        _ => unreachable!("dims.len() > 2 was already rejected above"),
    }
}

/// Read dyadic `⍴`'s left argument: a scalar (treated as a 1-element shape)
/// or a vector of non-negative integers. Anything of rank > 1 is rejected
/// up front (a matrix cannot itself describe a shape).
fn shape_vector(a: &Array) -> Result<Vec<usize>, String> {
    if a.ndims() > 1 {
        return Err(format!(
            "⍴: shape argument must be a scalar or vector (got rank {})",
            a.ndims()
        ));
    }
    a.data()
        .iter()
        .map(|&x| {
            if x < 0.0 || x.fract() != 0.0 {
                Err(format!(
                    "⍴: shape elements must be non-negative integers, got {x}"
                ))
            } else {
                Ok(x as usize)
            }
        })
        .collect()
}

// ── ⍳ index-generator / index-of ────────────────────────────────────────────

/// Monadic `⍳` (index generator): `⍳n` is the 1-based vector `[1, 2, …, n]`.
/// `n` must be a non-negative-integer-valued scalar; the result length is
/// capped at [`MAX_ARRAY_LENGTH`] *before* allocating.
pub fn index_generator(a: &Array) -> Result<Array, String> {
    if !a.is_scalar() {
        return Err("⍳: monadic argument must be a scalar".to_string());
    }
    let x = a.data()[0];
    if x < 0.0 || x.fract() != 0.0 {
        return Err(format!(
            "⍳: monadic argument must be a non-negative integer, got {x}"
        ));
    }
    let n = x as usize;
    if n > MAX_ARRAY_LENGTH {
        return Err(format!(
            "⍳: {n} exceeds the cap of {MAX_ARRAY_LENGTH} elements"
        ));
    }
    Ok(Array::from_vec((1..=n).map(|i| i as f64).collect()))
}

/// Dyadic `⍳` (index-of): for every element of `b`, the 1-based index of its
/// first occurrence in the vector `a` (plain `f64` equality — no floating
/// -point tolerance, matching every other comparison in this crate), or
/// `a.len() + 1` if it does not occur at all ("not found"). The result has
/// `b`'s shape.
pub fn index_of(a: &Array, b: &Array) -> Result<Array, String> {
    if a.ndims() > 1 {
        return Err(format!(
            "⍳: left argument must be a scalar or vector (got rank {})",
            a.ndims()
        ));
    }
    let haystack = a.data();
    let out: Vec<f64> = b
        .data()
        .iter()
        .map(|&needle| {
            haystack
                .iter()
                .position(|&x| x == needle)
                .map(|i| (i + 1) as f64)
                .unwrap_or((haystack.len() + 1) as f64)
        })
        .collect();
    Array::from_shape(out, b.shape().to_vec())
}

// ── , ravel / catenate ──────────────────────────────────────────────────────

/// Flatten any (rank ≤ 2, this crate's ceiling) array to **row-major**
/// order — last axis varies fastest. `Array` itself stores data
/// **column**-major (`value.rs`'s own doc comment: element `(r, c)` lives at
/// `c * nrows + r`), so a matrix must be walked "row, then column" to
/// produce true row-major ravel order — simply returning the raw backing
/// buffer would silently give *column*-major order instead.
fn flatten(a: &Array) -> Vec<f64> {
    match *a.shape() {
        [] | [_] => a.data().to_vec(),
        [r, c] => {
            let mut out = Vec::with_capacity(r * c);
            for row in 0..r {
                for col in 0..c {
                    out.push(a.get(row, col).expect("row/col in bounds"));
                }
            }
            out
        }
        // Unreachable in practice: every value this evaluator can construct
        // stays at rank ≤ 2 (see `display`'s matching comment in
        // `value.rs`). Falling back to the raw buffer keeps this total
        // rather than panicking if that ever changes.
        _ => a.data().to_vec(),
    }
}

/// Monadic `,` (ravel): flatten `a` to a 1-D vector in row-major order.
pub fn ravel(a: &Array) -> Array {
    Array::from_vec(flatten(a))
}

/// Dyadic `,` (catenate): supports scalar⋅scalar, scalar⋅vector,
/// vector⋅scalar, vector⋅vector (all producing a vector), and
/// matrix⋅matrix-with-equal-row-counts (horizontal/last-axis catenate,
/// producing `[r, c1 + c2]`). Any other rank combination — mismatched-row
/// matrices, or a matrix paired with a scalar/vector — is a clean "not yet
/// supported" error: an honestly disclosed subset restriction, the same
/// convention every other deferred construct in this language uses (MA05
/// §4's own "Deferred" list).
pub fn catenate(a: &Array, b: &Array) -> Result<Array, String> {
    match (a.ndims(), b.ndims()) {
        (0, 0) => Ok(Array::from_vec(vec![a.data()[0], b.data()[0]])),
        (0, 1) => {
            let mut out = vec![a.data()[0]];
            out.extend_from_slice(b.data());
            Ok(Array::from_vec(out))
        }
        (1, 0) => {
            let mut out = a.data().to_vec();
            out.push(b.data()[0]);
            Ok(Array::from_vec(out))
        }
        (1, 1) => {
            let mut out = a.data().to_vec();
            out.extend_from_slice(b.data());
            Ok(Array::from_vec(out))
        }
        (2, 2) => {
            if a.nrows() != b.nrows() {
                return Err(format!(
                    ",: matrix catenate needs equal row counts ({} vs {})",
                    a.nrows(),
                    b.nrows()
                ));
            }
            let (r, ca, cb) = (a.nrows(), a.ncols(), b.ncols());
            let mut data = vec![0.0; r * (ca + cb)];
            for row in 0..r {
                for col in 0..ca {
                    data[col * r + row] = a.get(row, col).expect("in bounds");
                }
                for col in 0..cb {
                    data[(ca + col) * r + row] = b.get(row, col).expect("in bounds");
                }
            }
            Array::from_shape(data, vec![r, ca + cb])
        }
        (ra, rb) => Err(format!(
            ",: catenate of rank {ra} and rank {rb} is not yet supported"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ⍴ ---------------------------------------------------------------

    #[test]
    fn shape_of_scalar_is_empty_vector_not_a_scalar() {
        let s = shape(&Array::scalar(7.0));
        assert_eq!(s.shape(), &[0]);
        assert!(s.is_empty());
    }

    #[test]
    fn shape_of_vector_and_matrix() {
        assert_eq!(shape(&Array::from_vec(vec![1.0, 2.0, 3.0])).data(), &[3.0]);
        let m = Array::from_rows(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]).unwrap();
        assert_eq!(shape(&m).data(), &[2.0, 3.0]);
    }

    #[test]
    fn reshape_cycles_a_shorter_source() {
        // 2 3⍴1 2 -- cycles [1,2] to fill 6 elements, row-major fill order.
        let target = Array::from_vec(vec![2.0, 3.0]);
        let source = Array::from_vec(vec![1.0, 2.0]);
        let r = reshape(&target, &source).unwrap();
        assert_eq!(r.shape(), &[2, 3]);
        // Row-major fill: row0 = [1,2,1], row1 = [2,1,2].
        assert_eq!(r.get(0, 0), Some(1.0));
        assert_eq!(r.get(0, 1), Some(2.0));
        assert_eq!(r.get(0, 2), Some(1.0));
        assert_eq!(r.get(1, 0), Some(2.0));
        assert_eq!(r.get(1, 1), Some(1.0));
        assert_eq!(r.get(1, 2), Some(2.0));
    }

    #[test]
    fn reshape_truncates_a_longer_source() {
        // 2 2⍴1 2 3 4 5 6 -- only the first 4 elements are used.
        let target = Array::from_vec(vec![2.0, 2.0]);
        let source = Array::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let r = reshape(&target, &source).unwrap();
        assert_eq!(r.shape(), &[2, 2]);
        assert_eq!(r.get(0, 0), Some(1.0));
        assert_eq!(r.get(0, 1), Some(2.0));
        assert_eq!(r.get(1, 0), Some(3.0));
        assert_eq!(r.get(1, 1), Some(4.0));
    }

    #[test]
    fn reshape_rejects_rank_above_2_target() {
        let target = Array::from_vec(vec![2.0, 2.0, 2.0]);
        let source = Array::from_vec(vec![1.0]);
        assert!(reshape(&target, &source).is_err());
    }

    #[test]
    fn reshape_of_empty_source_into_nonempty_target_is_an_error() {
        let target = Array::from_vec(vec![3.0]);
        let empty = Array::from_vec(vec![]);
        assert!(reshape(&target, &empty).is_err());
    }

    #[test]
    fn reshape_caps_target_element_count_before_allocating() {
        let target = Array::from_vec(vec![(MAX_ARRAY_LENGTH + 1) as f64]);
        let source = Array::from_vec(vec![1.0]);
        assert!(reshape(&target, &source).is_err());
    }

    // --- ⍳ ---------------------------------------------------------------

    #[test]
    fn index_generator_produces_one_based_run() {
        let r = index_generator(&Array::scalar(5.0)).unwrap();
        assert_eq!(r.data(), &[1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn index_generator_of_zero_is_empty() {
        let r = index_generator(&Array::scalar(0.0)).unwrap();
        assert!(r.is_empty());
    }

    #[test]
    fn index_generator_rejects_negative_and_noninteger() {
        assert!(index_generator(&Array::scalar(-1.0)).is_err());
        assert!(index_generator(&Array::scalar(2.5)).is_err());
    }

    #[test]
    fn index_generator_caps_n_before_allocating() {
        let huge = Array::scalar((MAX_ARRAY_LENGTH + 1) as f64);
        assert!(index_generator(&huge).is_err());
    }

    #[test]
    fn index_of_finds_and_reports_not_found() {
        let haystack = Array::from_vec(vec![10.0, 20.0, 30.0]);
        let needles = Array::from_vec(vec![20.0, 99.0, 10.0]);
        let r = index_of(&haystack, &needles).unwrap();
        // 20 is at index 2, 99 is not found (len+1 = 4), 10 is at index 1.
        assert_eq!(r.data(), &[2.0, 4.0, 1.0]);
    }

    // --- , -----------------------------------------------------------------

    #[test]
    fn ravel_flattens_a_matrix_in_row_major_order() {
        // [[1,2,3],[4,5,6]] ravels to [1,2,3,4,5,6] (row-major), even though
        // the backing store is column-major [1,4,2,5,3,6].
        let m = Array::from_rows(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]).unwrap();
        assert_eq!(ravel(&m).data(), &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn ravel_of_scalar_and_vector_is_a_noop_reshape() {
        assert_eq!(ravel(&Array::scalar(9.0)).data(), &[9.0]);
        let v = Array::from_vec(vec![1.0, 2.0]);
        assert_eq!(ravel(&v).data(), &[1.0, 2.0]);
    }

    #[test]
    fn catenate_scalar_and_scalar() {
        let r = catenate(&Array::scalar(1.0), &Array::scalar(2.0)).unwrap();
        assert_eq!(r.data(), &[1.0, 2.0]);
    }

    #[test]
    fn catenate_scalar_and_vector_prepends_or_appends() {
        let v = Array::from_vec(vec![2.0, 3.0]);
        assert_eq!(
            catenate(&Array::scalar(1.0), &v).unwrap().data(),
            &[1.0, 2.0, 3.0]
        );
        assert_eq!(
            catenate(&v, &Array::scalar(4.0)).unwrap().data(),
            &[2.0, 3.0, 4.0]
        );
    }

    #[test]
    fn catenate_vector_and_vector() {
        let a = Array::from_vec(vec![1.0, 2.0]);
        let b = Array::from_vec(vec![3.0, 4.0, 5.0]);
        assert_eq!(catenate(&a, &b).unwrap().data(), &[1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn catenate_matrices_with_equal_rows_concatenates_columns() {
        let a = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap(); // 2x2
        let b = Array::from_rows(vec![vec![5.0], vec![6.0]]).unwrap(); // 2x1
        let r = catenate(&a, &b).unwrap();
        assert_eq!(r.shape(), &[2, 3]);
        assert_eq!(r.get(0, 0), Some(1.0));
        assert_eq!(r.get(0, 1), Some(2.0));
        assert_eq!(r.get(0, 2), Some(5.0));
        assert_eq!(r.get(1, 2), Some(6.0));
    }

    #[test]
    fn catenate_rejects_mismatched_matrix_row_counts() {
        let a = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap(); // 2x2
        let b = Array::from_rows(vec![vec![5.0, 6.0]]).unwrap(); // 1x2
        assert!(catenate(&a, &b).is_err());
    }

    #[test]
    fn catenate_rejects_matrix_with_vector() {
        let a = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap();
        let v = Array::from_vec(vec![1.0, 2.0]);
        assert!(catenate(&a, &v).is_err());
        assert!(catenate(&v, &a).is_err());
    }
}
