//! `$` (shape/reshape), `i.` (index-generator/index-of), `,`
//! (ravel/catenate), `#` (tally/replicate), and `^` (exponential/power) — J's
//! five "bespoke" primitives (MA06 §4).
//!
//! Every other primitive verb (`+ - * % <. >. = ~: < > <: >:`) shares *one*
//! shape: a monadic and a dyadic meaning that both boil down to
//! `array_runtime::ops::BinOp` — see `eval.rs`'s `apply_monadic_scalar` /
//! `JFn::Atom`. These five do not fit that mould at all (each one's monadic
//! and dyadic meanings are unrelated to each other, and none is "an
//! elementwise scalar function"), so they get their own hand-rolled
//! implementations here — mirroring `apl-runtime::builtins`'s own `⍴`/`⍳`/`,`
//! (which are private to that crate and thus re-derived fresh here, not
//! reused), plus two genuinely new primitives (`#`, `^`) that have no APL
//! precedent at all.

use array_runtime::Array;

/// Upper bound on any array this crate allocates from a **runtime-computed**
/// size, or any work whose cost scales with a runtime-computed value —
/// monadic `i.n`'s `n`, dyadic `$`'s target element count, dyadic `,`'s
/// combined output length, dyadic `i.`'s `len(a)×len(b)` work, and dyadic
/// `#`'s total replicated-output length — checked *before* allocating or
/// scanning, so a crafted `i.2000000` is a clean `Err` instead of a
/// 2-million-element allocation. Same value, and the same "check before the
/// expensive work" discipline, as `apl-runtime::builtins::MAX_ARRAY_LENGTH`
/// (this repo's established cap for a user-controlled array length/work
/// product).
pub const MAX_ARRAY_LENGTH: usize = 1_000_000;

// ── $ shape / reshape ───────────────────────────────────────────────────────

/// Monadic `$` (shape-of): a scalar has **zero** dimensions, so its shape is
/// the *empty* vector (not a scalar!) — `$5` is `i.0`-shaped, a length-0
/// vector, exactly mirroring `Array::shape() == []` for a rank-0 value. A
/// vector `[n]` has shape `[n]` (a 1-element vector); a matrix `[r, c]` has
/// shape `[r, c]` (a 2-element vector).
pub fn shape(a: &Array) -> Array {
    let dims: Vec<f64> = a.shape().iter().map(|&d| d as f64).collect();
    Array::from_vec(dims)
}

/// Dyadic `$` (reshape): `a` is the new shape — a scalar or vector (rank ≤ 1)
/// of non-negative integers, itself capped at rank ≤ 2 (this crate's, and
/// `array_runtime::ops`'s, established ceiling; a longer shape is a clean
/// "not yet supported" error, the same honest-subset convention MA06 §4 uses
/// throughout). `b`'s elements are ravelled (see [`ravel`]) and then
/// cyclically repeated or truncated to fill the target shape's element
/// count — real J reshape semantics (a deliberate, natural design choice,
/// not an arbitrary invention: `2 3 $ 1 2` cycles `1 2` to `1 2 1 / 2 1 2`
/// in a real J session exactly as it does here).
pub fn reshape(a: &Array, b: &Array) -> Result<Array, String> {
    let dims = shape_vector(a)?;
    if dims.len() > 2 {
        return Err(format!(
            "$: reshape to rank > 2 is not yet supported (target shape {dims:?})"
        ));
    }
    // Checked multiplication: a crafted shape whose product overflows usize
    // must not wrap into a small count that spuriously slips past the cap
    // below (mirrors `array_runtime::value::Array::from_shape`'s own guard).
    let total: usize = dims
        .iter()
        .try_fold(1usize, |acc, &d| acc.checked_mul(d))
        .ok_or_else(|| format!("$: shape {dims:?} element count overflows usize"))?;
    if total > MAX_ARRAY_LENGTH {
        return Err(format!(
            "$: reshape target of {total} elements exceeds the cap of {MAX_ARRAY_LENGTH}"
        ));
    }
    let source = flatten(b);
    if total > 0 && source.is_empty() {
        return Err("$: cannot reshape an empty source into a non-empty shape".to_string());
    }
    // The cyclic fill is in ROW-MAJOR order (reshape fills the *last* axis
    // fastest, same convention as ravel) -- `filled[k]` is the value at
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
        // column-major instead of the row-major convention documented above.
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

/// Read dyadic `$`'s left argument: a scalar (treated as a 1-element shape)
/// or a vector of non-negative integers. Anything of rank > 1 is rejected
/// up front (a matrix cannot itself describe a shape).
fn shape_vector(a: &Array) -> Result<Vec<usize>, String> {
    if a.ndims() > 1 {
        return Err(format!(
            "$: shape argument must be a scalar or vector (got rank {})",
            a.ndims()
        ));
    }
    a.data()
        .iter()
        .map(|&x| {
            if x < 0.0 || x.fract() != 0.0 {
                Err(format!(
                    "$: shape elements must be non-negative integers, got {x}"
                ))
            } else {
                Ok(x as usize)
            }
        })
        .collect()
}

// ── i. index-generator / index-of ───────────────────────────────────────────

/// Monadic `i.` (index generator): `i.n` is the **0-based** vector
/// `[0, 1, …, n-1]`. Deliberately 0-based, unlike APL's `⍳n` (`[1, …, n]`,
/// see `apl-runtime::builtins::index_generator`) — MA06 §1 bullet 3 calls
/// this out as the single most safety-critical numeric difference between
/// the two frontends: `i.5` must be `0 1 2 3 4`, never `1 2 3 4 5`. `n` must
/// be a non-negative-integer-valued scalar; the result length is capped at
/// [`MAX_ARRAY_LENGTH`] *before* allocating.
pub fn index_generator(a: &Array) -> Result<Array, String> {
    if !a.is_scalar() {
        return Err("i.: monadic argument must be a scalar".to_string());
    }
    let x = a.data()[0];
    if x < 0.0 || x.fract() != 0.0 {
        return Err(format!(
            "i.: monadic argument must be a non-negative integer, got {x}"
        ));
    }
    let n = x as usize;
    if n > MAX_ARRAY_LENGTH {
        return Err(format!(
            "i.: {n} exceeds the cap of {MAX_ARRAY_LENGTH} elements"
        ));
    }
    Ok(Array::from_vec((0..n).map(|i| i as f64).collect()))
}

/// Dyadic `i.` (index-of): for every element of `b`, the **0-based** index
/// of its first occurrence in the vector `a` (plain `f64` equality — no
/// floating-point tolerance, matching every other comparison in this
/// crate), or `a`'s own tally (`a.len()`) if it does not occur at all — real
/// J's actual not-found convention, distinct from APL's `len() + 1` 1-based
/// sentinel (`apl-runtime::builtins::index_of`). The result has `b`'s shape.
pub fn index_of(a: &Array, b: &Array) -> Result<Array, String> {
    if a.ndims() > 1 {
        return Err(format!(
            "i.: left argument must be a scalar or vector (got rank {})",
            a.ndims()
        ));
    }
    // Each of `a`/`b` can independently reach MAX_ARRAY_LENGTH, but the work
    // done here is O(len(a) * len(b)) (a full linear scan of `a` per element
    // of `b`) -- capping each operand's *length* alone still permits up to
    // 10^12 comparisons. Cap the *product* before doing any scanning, the
    // same "check before the expensive work, not after" discipline as the
    // allocation caps elsewhere in this crate.
    match a.len().checked_mul(b.len()) {
        Some(work) if work <= MAX_ARRAY_LENGTH => {}
        _ => {
            return Err(format!(
                "i.: index-of over {} × {} elements exceeds the cap of {} comparisons",
                a.len(),
                b.len(),
                MAX_ARRAY_LENGTH
            ));
        }
    }
    let haystack = a.data();
    let out: Vec<f64> = b
        .data()
        .iter()
        .map(|&needle| {
            haystack
                .iter()
                .position(|&x| x == needle)
                .map(|i| i as f64) // 0-based -- no "+ 1" here (see doc comment)
                .unwrap_or(haystack.len() as f64) // not found -> a's tally
        })
        .collect();
    Array::from_shape(out, b.shape().to_vec())
}

// ── , ravel / catenate ──────────────────────────────────────────────────────

/// Flatten any (rank ≤ 2, this crate's ceiling) array to **row-major**
/// order — last axis varies fastest. `Array` itself stores data
/// **column**-major (`array-runtime`'s own `value.rs` doc comment: element
/// `(r, c)` lives at `c * nrows + r`), so a matrix must be walked "row, then
/// column" to produce true row-major ravel order — simply returning the raw
/// backing buffer would silently give *column*-major order instead. This
/// mirrors `apl-runtime::builtins::flatten`'s exact convention, kept
/// consistent across the two array-language frontends deliberately (MA06 §2:
/// same substrate, same `array-runtime`, so the same ravel order is the
/// natural choice rather than an arbitrary one).
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
        // stays at rank ≤ 2. Falling back to the raw buffer keeps this total
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
/// producing `[r, c1 + c2]`) — mirroring
/// `apl-runtime::builtins::catenate`'s exact shape rules (that function is
/// private to `apl-runtime`, so this is re-derived fresh here, not reused,
/// but kept behaviourally identical for consistency across the two
/// frontends). Any other rank combination — mismatched-row matrices, or a
/// matrix paired with a scalar/vector — is a clean "not yet supported"
/// error: an honestly disclosed subset restriction, the same convention
/// every other deferred construct in this language uses (MA06 §4's own
/// "Deferred" list).
pub fn catenate(a: &Array, b: &Array) -> Result<Array, String> {
    // Neither operand alone need be oversized for the *result* to be: both
    // can independently sit right at MAX_ARRAY_LENGTH, and a script that
    // repeatedly catenates a value with itself (`A=.A,A`) doubles the size
    // every line with no ceiling otherwise. Same cap and "check before
    // allocating" discipline as `i.`/dyadic `$` elsewhere in this crate.
    match a.len().checked_add(b.len()) {
        Some(total) if total <= MAX_ARRAY_LENGTH => {}
        _ => {
            return Err(format!(
                ",: catenate of {} and {} elements exceeds the cap of {} elements",
                a.len(),
                b.len(),
                MAX_ARRAY_LENGTH
            ));
        }
    }
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

// ── # tally / replicate ──────────────────────────────────────────────────────

/// Monadic `#` (tally): the item count along the leading axis. Genuinely new
/// relative to this repo's APL cut, which never added a tally primitive
/// (MA06 §4) — no template to copy, so the rule is spelled out directly: a
/// scalar has exactly **one** item (itself), a vector `[n]` has `n` items,
/// and a matrix `[r, c]` has `r` items (one per row — the leading axis).
/// Returned as a scalar `Array`.
pub fn tally(a: &Array) -> Array {
    let n = match *a.shape() {
        [] => 1,
        [n] => n,
        [r, _] => r,
        // Unreachable in practice (this crate's rank ≤ 2 ceiling, same as
        // every other primitive here) -- the first dimension is still the
        // most defensible "tally" if that ceiling is ever lifted.
        ref dims => dims.first().copied().unwrap_or(1),
    };
    Array::scalar(n as f64)
}

/// Dyadic `#` (copy/replicate): `x # y`, where `x` is a vector (or scalar,
/// for a length-1 `y`) of non-negative integer counts the same length as
/// `y`'s tally ([`tally`]). Each item of `y` is repeated `x[i]` times and the
/// results concatenated end to end; a count of `0` drops that item
/// entirely.
///
/// **Disclosed scope limit**: `y` is restricted to rank ≤ 1 (a scalar or
/// vector) — mirroring the same rank-limiting convention
/// `array_runtime::ops::outer` already uses for its own operands. A rank-2
/// `y` would need a genuinely different per-row replicate (repeating whole
/// rows, not scalar items), which this first cut does not attempt; it is
/// rejected here with a clear error rather than silently doing something
/// less general than a caller might expect, the same honest-subset
/// convention every other primitive in this crate follows.
pub fn replicate(x: &Array, y: &Array) -> Result<Array, String> {
    if y.ndims() > 1 {
        return Err(format!(
            "#: dyadic right argument must be a scalar or vector (rank ≤ 1), got rank {} — per-row replicate of a matrix is out of scope for this cut",
            y.ndims()
        ));
    }
    if x.ndims() > 1 {
        return Err(format!(
            "#: dyadic left argument (counts) must be a scalar or vector (rank ≤ 1), got rank {}",
            x.ndims()
        ));
    }
    let items = y.data();
    let counts = x.data();
    if counts.len() != items.len() {
        return Err(format!(
            "#: left argument's length must equal the right argument's tally ({} vs {})",
            counts.len(),
            items.len()
        ));
    }
    // Validate every count is a non-negative integer, and cap the total
    // output size *before* allocating -- same "check before the expensive
    // work" discipline as every other primitive here (a script that
    // replicates its own output over and over could otherwise grow without
    // bound).
    let mut parsed_counts = Vec::with_capacity(counts.len());
    let mut total: usize = 0;
    for &c in counts {
        if c < 0.0 || c.fract() != 0.0 {
            return Err(format!(
                "#: replicate counts must be non-negative integers, got {c}"
            ));
        }
        let c = c as usize;
        total = total
            .checked_add(c)
            .ok_or_else(|| "#: replicate output size overflows usize".to_string())?;
        parsed_counts.push(c);
    }
    if total > MAX_ARRAY_LENGTH {
        return Err(format!(
            "#: replicate output of {total} elements exceeds the cap of {MAX_ARRAY_LENGTH}"
        ));
    }
    let mut out = Vec::with_capacity(total);
    for (&val, &count) in items.iter().zip(parsed_counts.iter()) {
        for _ in 0..count {
            out.push(val);
        }
    }
    Ok(Array::from_vec(out))
}

// ── ^ exponential / power ────────────────────────────────────────────────────

/// Monadic `^` (natural exponential): `e` raised to each element of `y`.
/// Genuinely new relative to APL (this repo's APL cut has no `^` primitive
/// at all, and `array_runtime::ops::BinOp` has no `Pow` variant — MA06 §2
/// confirms this cut needs no new `array-runtime` substrate, so `^` is
/// implemented entirely locally in this crate rather than growing `BinOp`).
pub fn monadic_exp(a: &Array) -> Array {
    Array::from_shape(a.data().iter().map(|&v| v.exp()).collect(), a.shape().to_vec())
        .expect("monadic map preserves shape/length")
}

/// Dyadic `^` (power): `x` raised to the power `y`, elementwise
/// (`f64::powf`), with the *same* scalar-broadcast rule
/// `array_runtime::ops::elementwise` uses (either operand may be a scalar
/// broadcasting against the other's shape; otherwise shapes must match
/// exactly) — see [`elementwise_pow`] for the actual broadcast logic,
/// substituting `f64::powf` for `elementwise`'s `BinOp::apply` dispatch.
pub fn dyadic_pow(a: &Array, b: &Array) -> Result<Array, String> {
    elementwise_pow(a, b)
}

/// The `^`-specific elementwise combinator: mirrors
/// `array_runtime::ops::elementwise`'s exact broadcast-rule structure
/// (scalar⋅anything broadcasts, otherwise shapes must match exactly, result
/// takes the non-scalar operand's shape), substituting `f64::powf` in place
/// of `elementwise`'s `BinOp::apply` dispatch — `^` has no `BinOp` variant to
/// share (MA06 §2 explicitly scopes this cut to *not* need new
/// `array-runtime` substrate), so this small helper reimplements just enough
/// of that shape locally rather than growing `BinOp` for one caller.
fn elementwise_pow(a: &Array, b: &Array) -> Result<Array, String> {
    let (ad, bd) = (a.data(), b.data());
    let data: Vec<f64> = match (a.is_scalar(), b.is_scalar()) {
        (true, _) => bd.iter().map(|&y| ad[0].powf(y)).collect(),
        (_, true) => ad.iter().map(|&x| x.powf(bd[0])).collect(),
        _ => {
            if a.shape() != b.shape() {
                return Err(format!(
                    "^: non-conformable arrays: {:?} vs {:?}",
                    a.shape(),
                    b.shape()
                ));
            }
            ad.iter().zip(bd).map(|(&x, &y)| x.powf(y)).collect()
        }
    };
    let shape = if a.is_scalar() { b.shape() } else { a.shape() };
    Array::from_shape(data, shape.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- $ -----------------------------------------------------------------

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
        // 2 3 $ 1 2 -- cycles [1,2] to fill 6 elements, row-major fill order.
        let target = Array::from_vec(vec![2.0, 3.0]);
        let source = Array::from_vec(vec![1.0, 2.0]);
        let r = reshape(&target, &source).unwrap();
        assert_eq!(r.shape(), &[2, 3]);
        assert_eq!(r.get(0, 0), Some(1.0));
        assert_eq!(r.get(0, 1), Some(2.0));
        assert_eq!(r.get(0, 2), Some(1.0));
        assert_eq!(r.get(1, 0), Some(2.0));
        assert_eq!(r.get(1, 1), Some(1.0));
        assert_eq!(r.get(1, 2), Some(2.0));
    }

    #[test]
    fn reshape_truncates_a_longer_source() {
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

    // --- i. ------------------------------------------------------------------

    #[test]
    fn index_generator_is_zero_based_not_one_based() {
        // The single most safety-critical assertion in this whole crate
        // (MA06 §1 bullet 3): `i.5` is `[0,1,2,3,4]`, NEVER `[1,2,3,4,5]`.
        let r = index_generator(&Array::scalar(5.0)).unwrap();
        assert_eq!(r.data(), &[0.0, 1.0, 2.0, 3.0, 4.0]);
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
    fn index_of_is_zero_based_with_tally_not_found_sentinel() {
        let haystack = Array::from_vec(vec![10.0, 20.0, 30.0]);
        let needles = Array::from_vec(vec![20.0, 99.0, 10.0]);
        let r = index_of(&haystack, &needles).unwrap();
        // 20 is at 0-based index 1, 99 is not found (tally = 3), 10 is at
        // 0-based index 0 -- distinct from APL's 1-based [2, 4, 1].
        assert_eq!(r.data(), &[1.0, 3.0, 0.0]);
    }

    #[test]
    fn index_of_caps_the_work_product_before_scanning() {
        let n = 2000; // 2000 * 2000 = 4,000,000 > MAX_ARRAY_LENGTH
        let a = Array::from_vec(vec![0.0; n]);
        let b = Array::from_vec(vec![0.0; n]);
        assert!(index_of(&a, &b).is_err());
    }

    // --- , -----------------------------------------------------------------

    #[test]
    fn ravel_flattens_a_matrix_in_row_major_order() {
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
        let a = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap();
        let b = Array::from_rows(vec![vec![5.0], vec![6.0]]).unwrap();
        let r = catenate(&a, &b).unwrap();
        assert_eq!(r.shape(), &[2, 3]);
        assert_eq!(r.get(0, 0), Some(1.0));
        assert_eq!(r.get(0, 2), Some(5.0));
        assert_eq!(r.get(1, 2), Some(6.0));
    }

    #[test]
    fn catenate_rejects_mismatched_matrix_row_counts() {
        let a = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap();
        let b = Array::from_rows(vec![vec![5.0, 6.0]]).unwrap();
        assert!(catenate(&a, &b).is_err());
    }

    #[test]
    fn catenate_caps_combined_length_before_allocating() {
        let half = MAX_ARRAY_LENGTH / 2 + 1;
        let a = Array::from_vec(vec![0.0; half]);
        let b = Array::from_vec(vec![0.0; half]);
        assert!(catenate(&a, &b).is_err());
    }

    // --- # -------------------------------------------------------------------

    #[test]
    fn tally_of_scalar_vector_and_matrix() {
        assert_eq!(tally(&Array::scalar(7.0)).data(), &[1.0]);
        assert_eq!(tally(&Array::from_vec(vec![1.0, 2.0, 3.0])).data(), &[3.0]);
        let m = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]]).unwrap();
        assert_eq!(tally(&m).data(), &[3.0]); // 3 rows
    }

    #[test]
    fn replicate_repeats_and_drops_items() {
        let x = Array::from_vec(vec![2.0, 0.0, 3.0]);
        let y = Array::from_vec(vec![1.0, 2.0, 3.0]);
        let r = replicate(&x, &y).unwrap();
        assert_eq!(r.data(), &[1.0, 1.0, 3.0, 3.0, 3.0]);
    }

    #[test]
    fn replicate_of_a_scalar_y_with_scalar_count() {
        let x = Array::scalar(3.0);
        let y = Array::scalar(9.0);
        assert_eq!(replicate(&x, &y).unwrap().data(), &[9.0, 9.0, 9.0]);
    }

    #[test]
    fn replicate_rejects_rank_2_right_argument() {
        let x = Array::from_vec(vec![1.0, 1.0]);
        let y = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap();
        assert!(replicate(&x, &y).is_err());
    }

    #[test]
    fn replicate_rejects_mismatched_length() {
        let x = Array::from_vec(vec![1.0, 2.0]);
        let y = Array::from_vec(vec![1.0, 2.0, 3.0]);
        assert!(replicate(&x, &y).is_err());
    }

    #[test]
    fn replicate_rejects_negative_or_noninteger_counts() {
        let y = Array::from_vec(vec![1.0, 2.0]);
        assert!(replicate(&Array::from_vec(vec![-1.0, 1.0]), &y).is_err());
        assert!(replicate(&Array::from_vec(vec![1.5, 1.0]), &y).is_err());
    }

    #[test]
    fn replicate_caps_total_output_before_allocating() {
        let x = Array::from_vec(vec![(MAX_ARRAY_LENGTH + 1) as f64]);
        let y = Array::scalar(1.0);
        assert!(replicate(&x, &y).is_err());
    }

    // --- ^ -----------------------------------------------------------------

    #[test]
    fn monadic_exp_of_zero_is_one() {
        assert_eq!(monadic_exp(&Array::scalar(0.0)).data(), &[1.0]);
    }

    #[test]
    fn monadic_exp_is_elementwise() {
        let v = Array::from_vec(vec![0.0, 1.0]);
        let r = monadic_exp(&v);
        assert_eq!(r.data()[0], 1.0);
        assert!((r.data()[1] - std::f64::consts::E).abs() < 1e-9);
    }

    #[test]
    fn dyadic_pow_basic() {
        assert_eq!(dyadic_pow(&Array::scalar(2.0), &Array::scalar(3.0)).unwrap().data(), &[8.0]);
    }

    #[test]
    fn dyadic_pow_broadcasts_either_side() {
        let v = Array::from_vec(vec![1.0, 2.0, 3.0]);
        let s = Array::scalar(2.0);
        assert_eq!(dyadic_pow(&v, &s).unwrap().data(), &[1.0, 4.0, 9.0]);
        assert_eq!(dyadic_pow(&s, &v).unwrap().data(), &[2.0, 4.0, 8.0]);
    }

    #[test]
    fn dyadic_pow_rejects_mismatched_shapes() {
        let a = Array::from_vec(vec![1.0, 2.0]);
        let b = Array::from_vec(vec![1.0, 2.0, 3.0]);
        assert!(dyadic_pow(&a, &b).is_err());
    }
}
