//! Q's 16 primitive verbs (MA11 §4), split the same way `j-runtime`'s own
//! `builtins.rs` splits J's: the ones whose *dyadic* meaning is an ordinary
//! elementwise scalar function reuse `array_runtime::ops::BinOp` directly
//! (see [`Prim::to_binop`] in `eval.rs`); the rest (`!` `,` `#` `_` `~` --
//! **exactly five**, the same count as J's own five bespoke primitives
//! `$`/`i.`/`,`/`#`/`^`, a pleasant structural echo of MA11 §2's own framing
//! that Q "maps directly onto the kernels APL/J already reused") get
//! hand-rolled monadic+dyadic logic here.
//!
//! Unlike J, several of Q's *BinOp-mappable* primitives (`+`, `*`, `&`, `|`)
//! have a **monadic** meaning that is *not* itself an elementwise scalar
//! map (flip/transpose, first, where, reverse) -- so this module also grows
//! bespoke monadic-only implementations for those four, even though their
//! *dyadic* meaning is plain `ops::elementwise`. This is a genuine
//! structural difference from J (whose 12 `BinOp`-mappable atoms all had
//! *uniformly* elementwise monadic meanings, MA06 §4) that falls straight
//! out of Q's own primitive table (MA11 §4) rather than being invented
//! here.

use array_runtime::{ops, ops::BinOp, Array};

/// Upper bound on any array this crate allocates from a **runtime-computed**
/// size, or any work whose cost scales with a runtime-computed value --
/// monadic `!n`'s `n`, dyadic `#`'s (take) target element count, dyadic
/// `,`'s (join) combined output length -- checked *before* allocating or
/// scanning, so a crafted `!2000000` is a clean `Err` instead of a
/// 2-million-element allocation. Same value, and the same "check before the
/// expensive work" discipline, as `j-runtime::builtins::MAX_ARRAY_LENGTH`/
/// `apl-runtime::builtins::MAX_ARRAY_LENGTH` (this repo's established cap
/// for a user-controlled array length/work product).
pub const MAX_ARRAY_LENGTH: usize = 1_000_000;

/// One of Q's 16 primitive verb glyphs (MA11 §4's full table). Kept as a
/// small `Copy` enum (not the raw token) so error messages can name the
/// actual glyph (`"!"`, not `"BANG"`) via [`glyph`], mirroring
/// `j-runtime::eval::NonScalarAtom`'s identical role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prim {
    Plus,
    Minus,
    Star,
    Percent,
    Bang,
    Comma,
    Hash,
    Underscore,
    Amp,
    Pipe,
    Tilde,
    Eq,
    Ne,
    Lt,
    Le,
    Ge,
    Gt,
}

impl Prim {
    /// The ASCII spelling of this glyph, for error messages. `Ne` spells
    /// `<>` -- Q's own not-equal (MA11 §4), never `~=`/`#`.
    pub fn glyph(self) -> &'static str {
        match self {
            Prim::Plus => "+",
            Prim::Minus => "-",
            Prim::Star => "*",
            Prim::Percent => "%",
            Prim::Bang => "!",
            Prim::Comma => ",",
            Prim::Hash => "#",
            Prim::Underscore => "_",
            Prim::Amp => "&",
            Prim::Pipe => "|",
            Prim::Tilde => "~",
            Prim::Eq => "=",
            Prim::Ne => "<>",
            Prim::Lt => "<",
            Prim::Le => "<=",
            Prim::Ge => ">=",
            Prim::Gt => ">",
        }
    }

    /// The `array_runtime::ops::BinOp` this primitive's **dyadic** meaning
    /// maps onto, if any. Exactly 12 of the 16 primitives map onto one of
    /// `BinOp`'s 12 variants (one glyph per variant, a happy 1:1
    /// correspondence): `+ - * % & | = <> < <= >= >`. The remaining 5
    /// (`! , # _ ~`) have no elementwise-scalar dyadic meaning at all --
    /// `None` here, dispatched to bespoke logic instead (see
    /// [`apply_dyadic_prim`]).
    pub fn to_binop(self) -> Option<BinOp> {
        match self {
            Prim::Plus => Some(BinOp::Add),
            Prim::Minus => Some(BinOp::Sub),
            Prim::Star => Some(BinOp::Mul),
            Prim::Percent => Some(BinOp::Div),
            Prim::Amp => Some(BinOp::Min),
            Prim::Pipe => Some(BinOp::Max),
            Prim::Eq => Some(BinOp::Eq),
            Prim::Ne => Some(BinOp::Ne),
            Prim::Lt => Some(BinOp::Lt),
            Prim::Le => Some(BinOp::Le),
            Prim::Ge => Some(BinOp::Ge),
            Prim::Gt => Some(BinOp::Gt),
            Prim::Bang | Prim::Comma | Prim::Hash | Prim::Underscore | Prim::Tilde => None,
        }
    }

    /// Whether **monadic** `f'x` (each) has a well-defined, non-redundant
    /// meaning for this primitive in this cut's flat, dense-numeric-only
    /// value model -- true only for the four primitives whose *monadic*
    /// meaning is itself an ordinary per-element scalar map (`-` negate,
    /// `%` reciprocal, `_` floor, `~` not). See this module's own top doc
    /// comment and [`crate::eval`]'s `QFn::Each` doc comment for the full
    /// rationale: this repo's value model has no nested/boxed list type, so
    /// "apply per element" only has a meaning distinct from "apply
    /// directly" when the direct application *isn't already* elementwise --
    /// which, for every other primitive here (`+` flip, `*` first, `!` til,
    /// `,` enlist, `#` tally, `&` where, `|` reverse -- monadically -- and
    /// `!` `,` `#` `_` `~` dyadically), is never the case.
    pub fn each_monadic_supported(self) -> bool {
        matches!(self, Prim::Minus | Prim::Percent | Prim::Underscore | Prim::Tilde)
    }

    /// Whether **dyadic** `x f'y` (each) has a well-defined, non-redundant
    /// meaning -- true exactly for the `BinOp`-mappable primitives, whose
    /// ordinary dyadic meaning (`ops::elementwise`) is *already* per-element,
    /// making `each` degenerate to the identical computation as plain
    /// dyadic application. See [`each_monadic_supported`](Self::each_monadic_supported)'s
    /// doc comment for the shared rationale.
    pub fn each_dyadic_supported(self) -> bool {
        self.to_binop().is_some()
    }
}

// ── Monadic dispatch ────────────────────────────────────────────────────────

/// Apply `p` monadically. `Eq`/`Ne`/`Lt`/`Le`/`Ge`/`Gt` have no monadic
/// meaning in Q either (MA11 §4 lists them as dyadic-only comparisons) --
/// a clean, explicit error rather than silently picking a behavior, mirroring
/// `j-runtime::eval::apply_monadic_scalar`'s identical treatment of its own
/// six comparison atoms.
pub fn apply_monadic_prim(p: Prim, a: &Array) -> Result<Array, String> {
    match p {
        Prim::Plus => Ok(flip(a)),
        Prim::Minus => Ok(negate(a)),
        Prim::Star => first(a),
        Prim::Percent => Ok(reciprocal(a)),
        Prim::Bang => til(a),
        Prim::Comma => Ok(enlist(a)),
        Prim::Hash => Ok(tally(a)),
        Prim::Underscore => Ok(floor_monadic(a)),
        Prim::Amp => where_indices(a),
        Prim::Pipe => reverse(a),
        Prim::Tilde => Ok(not_(a)),
        Prim::Eq | Prim::Ne | Prim::Lt | Prim::Le | Prim::Ge | Prim::Gt => {
            Err(format!("q-runtime: no monadic form for {}", p.glyph()))
        }
    }
}

/// Apply `p` dyadically.
pub fn apply_dyadic_prim(p: Prim, a: &Array, b: &Array) -> Result<Array, String> {
    match p {
        Prim::Plus => ops::elementwise(BinOp::Add, a, b),
        Prim::Minus => ops::elementwise(BinOp::Sub, a, b),
        Prim::Star => ops::elementwise(BinOp::Mul, a, b),
        Prim::Percent => ops::elementwise(BinOp::Div, a, b),
        Prim::Amp => ops::elementwise(BinOp::Min, a, b),
        Prim::Pipe => ops::elementwise(BinOp::Max, a, b),
        Prim::Eq => ops::elementwise(BinOp::Eq, a, b),
        Prim::Ne => ops::elementwise(BinOp::Ne, a, b),
        Prim::Lt => ops::elementwise(BinOp::Lt, a, b),
        Prim::Le => ops::elementwise(BinOp::Le, a, b),
        Prim::Ge => ops::elementwise(BinOp::Ge, a, b),
        Prim::Gt => ops::elementwise(BinOp::Gt, a, b),
        // MA11 §4: "dyadic `!` (dict creation, and its other real overloads)
        // is deferred" -- explicitly out of scope, never silently
        // misinterpreted as something else.
        Prim::Bang => Err(
            "q-runtime: dyadic ! (dict creation) is not yet implemented -- explicitly deferred, MA11 §4".to_string(),
        ),
        Prim::Comma => join(a, b),
        Prim::Hash => take(a, b),
        Prim::Underscore => drop_(a, b),
        Prim::Tilde => Ok(match_(a, b)),
    }
}

// ── + flip ───────────────────────────────────────────────────────────────

/// Monadic `+` (flip): transposes a matrix (rank 2); identity for a scalar
/// or vector (rank <= 1). Real Q's monadic `+` is really about flipping a
/// column-dictionary/table into row-major orientation (and vice versa) --
/// this cut's value model has no tables (MA11 §4 defers them wholesale), so
/// "transpose the rank-2 case, identity otherwise" is this crate's own
/// considered, disclosed reading of "flip" restricted to the dense-numeric
/// subset actually in scope, reusing `array_runtime::ops::transpose`
/// directly (the *same* AR-2 kernel APL/J already share, per MA11 §2's own
/// "zero new substrate" finding).
pub fn flip(a: &Array) -> Array {
    if a.ndims() == 2 {
        ops::transpose(a)
    } else {
        a.clone()
    }
}

// ── - negate / % reciprocal / _ floor (elementwise monadic maps) ──────────

pub fn negate(a: &Array) -> Array {
    map_elementwise(a, |v| -v)
}

pub fn reciprocal(a: &Array) -> Array {
    map_elementwise(a, |v| 1.0 / v)
}

pub fn floor_monadic(a: &Array) -> Array {
    map_elementwise(a, f64::floor)
}

/// Not (monadic `~`): `1` for `0`, `0` for anything nonzero -- matching
/// MA11 §4's "comparisons/logic produce/accept plain 0/1 numerics" (no
/// native boolean type in this cut).
pub fn not_(a: &Array) -> Array {
    map_elementwise(a, |v| if v == 0.0 { 1.0 } else { 0.0 })
}

fn map_elementwise(a: &Array, f: impl Fn(f64) -> f64) -> Array {
    Array::from_shape(a.data().iter().map(|&v| f(v)).collect(), a.shape().to_vec())
        .expect("elementwise map preserves shape/length")
}

// ── * first ──────────────────────────────────────────────────────────────

/// Monadic `*` (first): the first item along the leading axis -- a scalar's
/// first item is itself; a vector's first item is its element 0 (a scalar
/// result); a matrix's first item is its row 0 (a vector result). Deferring
/// to Q's own real "first item of a list" semantics rather than APL/J's
/// unrelated monadic `*` (sign) -- MA11 §4 is explicit that Q's `*` is
/// "first / multiply", a completely different pairing from J's
/// "sign / multiply".
pub fn first(a: &Array) -> Result<Array, String> {
    match *a.shape() {
        [] => Ok(a.clone()),
        [n] => {
            if n == 0 {
                return Err("*: first of an empty vector is undefined".to_string());
            }
            Ok(Array::scalar(a.data()[0]))
        }
        [r, c] => {
            if r == 0 {
                return Err("*: first of an empty matrix is undefined".to_string());
            }
            let row: Vec<f64> = (0..c).map(|col| a.get(0, col).expect("in bounds")).collect();
            Ok(Array::from_vec(row))
        }
        _ => Err("*: first is only supported for rank <= 2".to_string()),
    }
}

// ── ! til ────────────────────────────────────────────────────────────────

/// Monadic `!` (til): the **0-based** vector `[0, 1, ..., n-1]` -- matches
/// J's own `i.`, NOT APL's 1-based `⍳` (MA11 §4's own explicit callout).
/// `n` must be a non-negative-integer-valued scalar; the result length is
/// capped at [`MAX_ARRAY_LENGTH`] *before* allocating.
pub fn til(a: &Array) -> Result<Array, String> {
    if !a.is_scalar() {
        return Err("!: monadic argument (til) must be a scalar".to_string());
    }
    let x = a.data()[0];
    if x < 0.0 || x.fract() != 0.0 {
        return Err(format!(
            "!: monadic argument must be a non-negative integer, got {x}"
        ));
    }
    let n = x as usize;
    if n > MAX_ARRAY_LENGTH {
        return Err(format!("!: {n} exceeds the cap of {MAX_ARRAY_LENGTH} elements"));
    }
    Ok(Array::from_vec((0..n).map(|i| i as f64).collect()))
}

// ── , enlist / join ──────────────────────────────────────────────────────

/// Monadic `,` (enlist): "ensure this value is list-shaped." A scalar
/// (rank 0) is wrapped into a length-1 vector; a value already at rank >= 1
/// is returned unchanged.
///
/// **Disclosed simplification**: real Q's enlist always wraps its argument
/// in exactly *one more* level of list nesting, even when the argument is
/// already a list (`,1 2 3` produces a 1-item list *containing* the
/// 3-element list `1 2 3` -- a strictly deeper nesting than `1 2 3` itself,
/// distinguishable by `count each`). This cut's value model
/// (`array_runtime::Array`, dense and *flat* -- MA11 §4: "arrays only") has
/// no boxed/nested representation at all, so that extra nesting level
/// simply cannot be represented here. "Ensure rank >= 1, otherwise
/// identity" is the closest sound approximation available within this
/// value model, and is exact for enlist's single most common use (wrapping
/// a bare scalar into a 1-element list, e.g. `,5` == `1#5`).
pub fn enlist(a: &Array) -> Array {
    if a.is_scalar() {
        Array::from_vec(vec![a.data()[0]])
    } else {
        a.clone()
    }
}

/// Dyadic `,` (join/catenate): supports scalar-scalar, scalar-vector,
/// vector-scalar, vector-vector (all producing a vector), and
/// matrix-matrix-with-equal-row-counts (column catenate, producing
/// `[r, c1 + c2]`) -- re-derived fresh here (not shared with `j-runtime`,
/// whose identical-shaped `catenate` is private to that crate), but kept
/// behaviourally consistent across the two frontends by design, mirroring
/// the same "same substrate, same natural convention" reasoning MA11 §2
/// gives for reusing AR-2's kernels. Any other rank combination is a clean
/// "not yet supported" error.
pub fn join(a: &Array, b: &Array) -> Result<Array, String> {
    match a.len().checked_add(b.len()) {
        Some(total) if total <= MAX_ARRAY_LENGTH => {}
        _ => {
            return Err(format!(
                ",: join of {} and {} elements exceeds the cap of {MAX_ARRAY_LENGTH} elements",
                a.len(),
                b.len()
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
                    ",: matrix join needs equal row counts ({} vs {})",
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
        (ra, rb) => Err(format!(",: join of rank {ra} and rank {rb} is not yet supported")),
    }
}

// ── # tally / take ───────────────────────────────────────────────────────

/// Monadic `#` (count/tally): the item count along the leading axis --
/// scalar has exactly **one** item, a vector `[n]` has `n`, and a matrix
/// `[r, c]` has `r` (one per row).
pub fn tally(a: &Array) -> Array {
    let n = match *a.shape() {
        [] => 1,
        [n] => n,
        [r, _] => r,
        ref dims => dims.first().copied().unwrap_or(1),
    };
    Array::scalar(n as f64)
}

/// Dyadic `#` (take): `x#y` takes `|x|` items from `y`, **cycling** if `y`
/// is shorter than needed -- from the front if `x >= 0`, from the *end* if
/// `x < 0` (real Q's actual take semantics; a genuinely different meaning
/// from J's dyadic `#`, which is *replicate*, not take -- MA11 §4 is
/// explicit that Q's `#` is "count / take").
///
/// `y` is scoped to rank <= 1 (a scalar or vector), mirroring
/// `j-runtime::builtins::replicate`'s identical rank-limiting convention
/// for its own dyadic right operand.
pub fn take(x: &Array, y: &Array) -> Result<Array, String> {
    if !x.is_scalar() {
        return Err("#: dyadic left argument (take count) must be a scalar".to_string());
    }
    if y.ndims() > 1 {
        return Err(format!(
            "#: dyadic right argument must be a scalar or vector (rank <= 1), got rank {}",
            y.ndims()
        ));
    }
    let n = x.data()[0];
    if n.fract() != 0.0 {
        return Err(format!("#: take count must be an integer, got {n}"));
    }
    let count = n.abs() as usize;
    if count > MAX_ARRAY_LENGTH {
        return Err(format!(
            "#: take of {count} elements exceeds the cap of {MAX_ARRAY_LENGTH}"
        ));
    }
    let src = y.data();
    if count == 0 {
        return Ok(Array::from_vec(vec![]));
    }
    if src.is_empty() {
        return Err("#: cannot take a nonzero count from an empty array".to_string());
    }
    let out: Vec<f64> = if n >= 0.0 {
        (0..count).map(|i| src[i % src.len()]).collect()
    } else {
        // Negative count: the last `count` items of the infinite cyclic
        // repetition of `y`, ending exactly at `y`'s own last element --
        // equivalent to taking `count` from the front of the *reversed*
        // source, then reversing that result back.
        let mut rev = src.to_vec();
        rev.reverse();
        let mut out: Vec<f64> = (0..count).map(|i| rev[i % rev.len()]).collect();
        out.reverse();
        out
    };
    Ok(Array::from_vec(out))
}

// ── _ floor (monadic, above) / drop (dyadic) ────────────────────────────

/// Dyadic `_` (drop): `x _ y` drops `|x|` items from `y` -- from the front
/// if `x >= 0`, from the end if `x < 0` -- with **no** cycling (unlike take
/// above): dropping more items than `y` has simply empties it. `y` is
/// scoped to rank <= 1, mirroring [`take`]'s identical restriction.
pub fn drop_(x: &Array, y: &Array) -> Result<Array, String> {
    if !x.is_scalar() {
        return Err("_: dyadic left argument (drop count) must be a scalar".to_string());
    }
    if y.ndims() > 1 {
        return Err(format!(
            "_: dyadic right argument must be a scalar or vector (rank <= 1), got rank {}",
            y.ndims()
        ));
    }
    let n = x.data()[0];
    if n.fract() != 0.0 {
        return Err(format!("_: drop count must be an integer, got {n}"));
    }
    let src = y.data();
    let len = src.len();
    let k = (n.abs() as usize).min(len);
    let out: Vec<f64> = if n >= 0.0 {
        src[k..].to_vec()
    } else {
        src[..len - k].to_vec()
    };
    Ok(Array::from_vec(out))
}

// ── & where / min ────────────────────────────────────────────────────────

/// Monadic `&` (where): the indices (0-based) of every nonzero element, per
/// MA11 §4's own literal wording ("monadic: indices of nonzero elements").
/// Scoped to rank <= 1 (scalar or vector), mirroring this crate's other
/// rank-limited bespoke primitives.
///
/// **Disclosed simplification**: real Q's monadic `&` actually generalizes
/// this to non-boolean non-negative-integer *counts* (`&2 0 3` produces
/// `0 0 1 1 1`, replicating index `i` exactly `x[i]` times, of which
/// "indices of nonzero elements" is only the special case where every count
/// is 0 or 1). This crate follows MA11 §4's own literal spec text rather
/// than real Q's fuller generalization, since the spec explicitly commits
/// to the narrower reading.
pub fn where_indices(a: &Array) -> Result<Array, String> {
    if a.ndims() > 1 {
        return Err(format!(
            "&: monadic argument (where) must be a scalar or vector, got rank {}",
            a.ndims()
        ));
    }
    let idx: Vec<f64> = a
        .data()
        .iter()
        .enumerate()
        .filter(|(_, &v)| v != 0.0)
        .map(|(i, _)| i as f64)
        .collect();
    Ok(Array::from_vec(idx))
}

// ── | reverse / max ──────────────────────────────────────────────────────

/// Monadic `|` (reverse): reverses element order for a vector; reverses row
/// order (each row's own column order stays intact) for a matrix; a scalar
/// reverses to itself.
pub fn reverse(a: &Array) -> Result<Array, String> {
    match *a.shape() {
        [] => Ok(a.clone()),
        [_] => {
            let mut d = a.data().to_vec();
            d.reverse();
            Ok(Array::from_vec(d))
        }
        [r, c] => {
            let mut data = vec![0.0; r * c];
            for row in 0..r {
                let src_row = r - 1 - row;
                for col in 0..c {
                    data[col * r + row] = a.get(src_row, col).expect("in bounds");
                }
            }
            Array::from_shape(data, vec![r, c])
        }
        _ => Err("|: reverse is only supported for rank <= 2".to_string()),
    }
}

// ── ~ not (monadic, above) / match (dyadic) ─────────────────────────────

/// Dyadic `~` (match): deep equality -- same shape *and* every element
/// exactly equal (plain `f64 ==`, no floating-point tolerance, matching
/// every other comparison in this crate) -- producing a single scalar `1`
/// or `0`, **not** an elementwise array (a genuine, deliberate difference
/// from every other dyadic primitive in this crate, which are all
/// elementwise: match answers one yes/no question about the whole pair of
/// values, exactly mirroring real Q's own `~` semantics).
pub fn match_(a: &Array, b: &Array) -> Array {
    let eq = a.shape() == b.shape() && a.data() == b.data();
    Array::scalar(if eq { 1.0 } else { 0.0 })
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- + flip --------------------------------------------------------

    #[test]
    fn flip_transposes_a_matrix() {
        let m = Array::from_rows(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]).unwrap();
        let t = flip(&m);
        assert_eq!(t.shape(), &[3, 2]);
        assert_eq!(t.get(0, 0), Some(1.0));
        assert_eq!(t.get(2, 1), Some(6.0));
    }

    #[test]
    fn flip_is_identity_for_scalar_and_vector() {
        assert_eq!(flip(&Array::scalar(5.0)).data(), &[5.0]);
        let v = Array::from_vec(vec![1.0, 2.0, 3.0]);
        assert_eq!(flip(&v).data(), v.data());
    }

    // --- - negate / % reciprocal / _ floor ------------------------------

    #[test]
    fn negate_flips_sign_elementwise() {
        assert_eq!(negate(&Array::from_vec(vec![1.0, -2.0, 0.0])).data(), &[-1.0, 2.0, -0.0]);
    }

    #[test]
    fn reciprocal_is_elementwise() {
        assert_eq!(reciprocal(&Array::from_vec(vec![1.0, 2.0, 4.0])).data(), &[1.0, 0.5, 0.25]);
    }

    #[test]
    fn floor_monadic_rounds_down() {
        assert_eq!(floor_monadic(&Array::scalar(3.8)).data(), &[3.0]);
        assert_eq!(floor_monadic(&Array::scalar(-3.2)).data(), &[-4.0]);
    }

    // --- ~ not -----------------------------------------------------------

    #[test]
    fn not_flips_zero_and_nonzero() {
        assert_eq!(not_(&Array::from_vec(vec![0.0, 1.0, 5.0, -1.0])).data(), &[1.0, 0.0, 0.0, 0.0]);
    }

    // --- * first -----------------------------------------------------------

    #[test]
    fn first_of_scalar_vector_and_matrix() {
        assert_eq!(first(&Array::scalar(9.0)).unwrap().data(), &[9.0]);
        assert_eq!(
            first(&Array::from_vec(vec![7.0, 8.0, 9.0])).unwrap().data(),
            &[7.0]
        );
        let m = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap();
        assert_eq!(first(&m).unwrap().data(), &[1.0, 2.0]);
    }

    #[test]
    fn first_of_empty_is_an_error() {
        assert!(first(&Array::from_vec(vec![])).is_err());
    }

    // --- ! til -------------------------------------------------------------

    #[test]
    fn til_is_zero_based_not_one_based() {
        // THE single most safety-critical assertion for this primitive
        // (MA11 §4): `!5` is `[0,1,2,3,4]`, never APL's 1-based `[1..5]`.
        assert_eq!(til(&Array::scalar(5.0)).unwrap().data(), &[0.0, 1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn til_of_zero_is_empty() {
        assert!(til(&Array::scalar(0.0)).unwrap().is_empty());
    }

    #[test]
    fn til_rejects_negative_and_noninteger() {
        assert!(til(&Array::scalar(-1.0)).is_err());
        assert!(til(&Array::scalar(2.5)).is_err());
    }

    #[test]
    fn til_caps_n_before_allocating() {
        let huge = Array::scalar((MAX_ARRAY_LENGTH + 1) as f64);
        assert!(til(&huge).is_err());
    }

    // --- , enlist / join -----------------------------------------------------

    #[test]
    fn enlist_wraps_a_scalar_into_a_one_element_vector() {
        assert_eq!(enlist(&Array::scalar(5.0)).data(), &[5.0]);
        assert_eq!(enlist(&Array::scalar(5.0)).shape(), &[1]);
    }

    #[test]
    fn enlist_of_an_already_list_shaped_value_is_unchanged() {
        let v = Array::from_vec(vec![1.0, 2.0, 3.0]);
        assert_eq!(enlist(&v).data(), v.data());
    }

    #[test]
    fn join_scalar_and_vector_combinations() {
        assert_eq!(join(&Array::scalar(1.0), &Array::scalar(2.0)).unwrap().data(), &[1.0, 2.0]);
        let v = Array::from_vec(vec![2.0, 3.0]);
        assert_eq!(join(&Array::scalar(1.0), &v).unwrap().data(), &[1.0, 2.0, 3.0]);
        assert_eq!(join(&v, &Array::scalar(4.0)).unwrap().data(), &[2.0, 3.0, 4.0]);
    }

    #[test]
    fn join_matrices_with_equal_rows_concatenates_columns() {
        let a = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap();
        let b = Array::from_rows(vec![vec![5.0], vec![6.0]]).unwrap();
        let r = join(&a, &b).unwrap();
        assert_eq!(r.shape(), &[2, 3]);
        assert_eq!(r.get(0, 2), Some(5.0));
        assert_eq!(r.get(1, 2), Some(6.0));
    }

    #[test]
    fn join_rejects_mismatched_matrix_row_counts() {
        let a = Array::from_rows(vec![vec![1.0, 2.0]]).unwrap();
        let b = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap();
        assert!(join(&a, &b).is_err());
    }

    #[test]
    fn join_caps_combined_length_before_allocating() {
        let half = MAX_ARRAY_LENGTH / 2 + 1;
        let a = Array::from_vec(vec![0.0; half]);
        let b = Array::from_vec(vec![0.0; half]);
        assert!(join(&a, &b).is_err());
    }

    // --- # tally / take -------------------------------------------------

    #[test]
    fn tally_of_scalar_vector_and_matrix() {
        assert_eq!(tally(&Array::scalar(7.0)).data(), &[1.0]);
        assert_eq!(tally(&Array::from_vec(vec![1.0, 2.0, 3.0])).data(), &[3.0]);
        let m = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]]).unwrap();
        assert_eq!(tally(&m).data(), &[3.0]);
    }

    #[test]
    fn take_positive_count_cycles_a_shorter_source() {
        // 5#1 2 3 -> [1,2,3,1,2] (cycled).
        let y = Array::from_vec(vec![1.0, 2.0, 3.0]);
        let r = take(&Array::scalar(5.0), &y).unwrap();
        assert_eq!(r.data(), &[1.0, 2.0, 3.0, 1.0, 2.0]);
    }

    #[test]
    fn take_positive_count_truncates_a_longer_source() {
        let y = Array::from_vec(vec![1.0, 2.0, 3.0, 4.0]);
        let r = take(&Array::scalar(2.0), &y).unwrap();
        assert_eq!(r.data(), &[1.0, 2.0]);
    }

    #[test]
    fn take_negative_count_takes_from_the_end_and_cycles() {
        // -2#1 2 3 -> last 2 elements -> [2,3].
        let y = Array::from_vec(vec![1.0, 2.0, 3.0]);
        assert_eq!(take(&Array::scalar(-2.0), &y).unwrap().data(), &[2.0, 3.0]);
        // -5#1 2 3 -> cycled from the end -> [2,3,1,2,3].
        assert_eq!(
            take(&Array::scalar(-5.0), &y).unwrap().data(),
            &[2.0, 3.0, 1.0, 2.0, 3.0]
        );
    }

    #[test]
    fn take_zero_count_is_empty_even_from_an_empty_source() {
        let empty = Array::from_vec(vec![]);
        assert_eq!(take(&Array::scalar(0.0), &empty).unwrap().data(), &[] as &[f64]);
    }

    #[test]
    fn take_rejects_nonzero_count_from_empty_source() {
        let empty = Array::from_vec(vec![]);
        assert!(take(&Array::scalar(3.0), &empty).is_err());
    }

    #[test]
    fn take_caps_count_before_allocating() {
        let y = Array::from_vec(vec![1.0]);
        assert!(take(&Array::scalar((MAX_ARRAY_LENGTH + 1) as f64), &y).is_err());
    }

    #[test]
    fn take_rejects_rank_2_source() {
        let m = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap();
        assert!(take(&Array::scalar(2.0), &m).is_err());
    }

    // --- _ drop ------------------------------------------------------------

    #[test]
    fn drop_positive_count_drops_from_the_front() {
        let y = Array::from_vec(vec![1.0, 2.0, 3.0, 4.0]);
        assert_eq!(drop_(&Array::scalar(2.0), &y).unwrap().data(), &[3.0, 4.0]);
    }

    #[test]
    fn drop_negative_count_drops_from_the_end() {
        let y = Array::from_vec(vec![1.0, 2.0, 3.0, 4.0]);
        assert_eq!(drop_(&Array::scalar(-2.0), &y).unwrap().data(), &[1.0, 2.0]);
    }

    #[test]
    fn drop_more_than_length_empties_without_erroring() {
        let y = Array::from_vec(vec![1.0, 2.0]);
        assert!(drop_(&Array::scalar(10.0), &y).unwrap().is_empty());
        assert!(drop_(&Array::scalar(-10.0), &y).unwrap().is_empty());
    }

    // --- & where -------------------------------------------------------------

    #[test]
    fn where_indices_of_nonzero_elements() {
        let v = Array::from_vec(vec![0.0, 1.0, 1.0, 0.0, 1.0]);
        assert_eq!(where_indices(&v).unwrap().data(), &[1.0, 2.0, 4.0]);
    }

    #[test]
    fn where_indices_rejects_rank_2() {
        let m = Array::from_rows(vec![vec![1.0, 0.0]]).unwrap();
        assert!(where_indices(&m).is_err());
    }

    // --- | reverse -----------------------------------------------------------

    #[test]
    fn reverse_of_vector() {
        let v = Array::from_vec(vec![1.0, 2.0, 3.0]);
        assert_eq!(reverse(&v).unwrap().data(), &[3.0, 2.0, 1.0]);
    }

    #[test]
    fn reverse_of_scalar_is_itself() {
        assert_eq!(reverse(&Array::scalar(9.0)).unwrap().data(), &[9.0]);
    }

    #[test]
    fn reverse_of_matrix_reverses_row_order() {
        let m = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]]).unwrap();
        let r = reverse(&m).unwrap();
        assert_eq!(r.get(0, 0), Some(5.0));
        assert_eq!(r.get(0, 1), Some(6.0));
        assert_eq!(r.get(2, 0), Some(1.0));
    }

    // --- ~ match -------------------------------------------------------------

    #[test]
    fn match_true_for_deeply_equal_arrays() {
        let a = Array::from_vec(vec![1.0, 2.0, 3.0]);
        let b = Array::from_vec(vec![1.0, 2.0, 3.0]);
        assert_eq!(match_(&a, &b).data(), &[1.0]);
    }

    #[test]
    fn match_false_for_different_values_or_shapes() {
        let a = Array::from_vec(vec![1.0, 2.0, 3.0]);
        let b = Array::from_vec(vec![1.0, 2.0, 4.0]);
        assert_eq!(match_(&a, &b).data(), &[0.0]);

        let c = Array::from_vec(vec![1.0, 2.0]);
        assert_eq!(match_(&a, &c).data(), &[0.0]);
    }

    // --- Prim / to_binop / glyph --------------------------------------------

    #[test]
    fn exactly_twelve_primitives_map_onto_a_binop() {
        let all = [
            Prim::Plus, Prim::Minus, Prim::Star, Prim::Percent, Prim::Bang, Prim::Comma,
            Prim::Hash, Prim::Underscore, Prim::Amp, Prim::Pipe, Prim::Tilde, Prim::Eq,
            Prim::Ne, Prim::Lt, Prim::Le, Prim::Ge, Prim::Gt,
        ];
        let count = all.iter().filter(|p| p.to_binop().is_some()).count();
        assert_eq!(count, 12);
    }

    #[test]
    fn not_equal_glyph_is_angle_brackets_not_tilde_equals() {
        // The single easiest glyph to get wrong in this whole crate
        // (MA11 §4's own callout): Q's not-equal is `<>`, never `~=`
        // (MATLAB/Scilab's spelling) and never `#` (which is count/take).
        assert_eq!(Prim::Ne.glyph(), "<>");
    }
}
