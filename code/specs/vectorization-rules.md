# Vectorization Rules

## Overview

Every Layer 1 domain core in the Rust statistics stack receives
**vectors** as input — never scalars. But "vector" means three
different things to the three frontends that will consume the cores:

- R recycles: `c(1, 2, 3, 4, 5, 6) + c(10, 20)` repeats the shorter
  end-to-end to match the longer.
- Excel array formulas broadcast: `{=A1:A6 + B1:B2}` aligns by shape
  and fills mismatched positions with `#N/A`.
- S follows R but with a few quirks (S's `cbind` recycles columns
  differently from R's).

If each domain core had to know which frontend was calling it, the
core would carry frontend complexity. Instead, we resolve shape *at
the frontend boundary* and hand the cores already-aligned vectors.
This spec defines what each frontend does and the contract a core can
rely on.

The rule, in one sentence:

> **Shape resolution is a frontend concern. By the time a domain
> core sees a vector, all recycling, broadcasting, and shape-error
> decisions have already been made.**

---

## Where It Fits

```
   r-runtime           spreadsheet-core         s-runtime
       │                      │                     │
       │ recycling            │ broadcasting        │ recycling-with-quirks
       ▼                      ▼                     ▼
       └──────────────────────┼─────────────────────┘
                              │
                              │  aligned vectors only
                              ▼
        ┌────────────────────────────────────────────┐
        │   Layer 1 cores (statistics-core, etc.)    │
        │   Receive vectors at known, equal shape.   │
        └────────────────────────────────────────────┘
```

**Used by:** every frontend before it dispatches to a domain core.

**Not used by:** domain cores themselves. Their function bodies do not
implement recycling; they assume aligned inputs and produce results
of derivable shape.

---

## R Recycling

The canonical R rule, reproduced from `r-vector.md` for reference:

1. For a binary element-wise operation, let `n = max(len(a), len(b))`.
2. Each operand is repeated to length `n`.
3. Warn if `n` is not a multiple of either input's length.

The recycling helper lives in `r-vector::recycle`. The future
`r-runtime` calls it before invoking a core function:

```rust
// In r-runtime, evaluating  a + b  where a, b are R vectors:
let (a_aligned, b_aligned) = r_vector::recycle_to_max(a, b);
math_core::add_elementwise(&a_aligned, &b_aligned)
```

Recycling applies to:

- All element-wise arithmetic (`+`, `-`, `*`, `/`, `^`, `%%`, `%/%`)
- All element-wise comparisons (`<`, `<=`, `==`, `!=`, `>=`, `>`)
- All element-wise logical operators (`&`, `|`)

Recycling does **not** apply to:

- Reductions (`sum`, `mean`, …) — they take one vector
- `match`, `%in%` — they take two vectors with their own length rules
- Function calls in general — argument-vector shapes are not recycled
  against each other; each function specifies its own contract

### Length-zero rule

If either operand has length 0, the result has length 0. R's most
notorious silent-empty trap.

### Length-one rule

If either operand has length 1, that single value is broadcast without
warning. This is the "scalar broadcasts" case and is the only
recycling that R does silently.

---

## Excel Broadcasting (Array Formulas)

Excel array formulas operate on rectangular ranges. The rules differ
from R's recycling in three important ways:

1. **Shape is rectangular** (rows × cols), not flat. Recycling means
   nothing; *broadcasting* aligns by both axes.
2. **Mismatch becomes `#N/A`**, not a warning + truncation. If the
   shapes don't broadcast, missing positions are filled with `#N/A`
   rather than recycling around.
3. **Length-one expansion is per-axis**, not flat. A 1-row range
   broadcasts row-wise across multiple rows; a 1-column broadcasts
   column-wise.

| Left shape | Right shape | Result shape | Behavior |
|------------|-------------|--------------|----------|
| 1×1        | n×m         | n×m          | Scalar broadcasts everywhere |
| n×1        | n×m         | n×m          | Column broadcasts across columns |
| 1×m        | n×m         | n×m          | Row broadcasts across rows |
| n×m        | n×m         | n×m          | Element-wise, no broadcast needed |
| n×m        | p×q (incompat) | max(n,p)×max(m,q) | Mismatched cells get `#N/A` |

`spreadsheet-core` implements this in its array-formula evaluator:

```rust
pub fn align_array(
    a: &CellArray,
    b: &CellArray,
) -> (CellArray, CellArray);
```

The aligned arrays then flatten into `Vector`s for dispatch into the
domain core, with `#N/A`-positioned cells materialized as the spreadsheet
NA sentinel (which translates to NA at the boundary — same as Excel
`=NA()`).

### Modern dynamic arrays (Excel 365)

Excel 365 introduced *dynamic array* spilling, where any formula can
return an array that auto-expands into adjacent cells. The broadcasting
rules above apply unchanged; what differs is the *output* — the result
flows into a cell range instead of being collapsed by an outer
aggregation. `spreadsheet-core` supports this via a `spill_target`
parameter on the formula evaluator; the array semantics are identical
to legacy `{}`-bracketed formulas.

### Implicit intersection

The legacy "implicit intersection" behavior (where `=A1:A10` in a
single cell collapses to the row-aligned scalar) is supported as a
compatibility mode flag. The default is dynamic-array mode (Excel 365
behavior).

---

## S Quirks

S-PLUS shares R's recycling rule for arithmetic but differs in two
places that bite when porting code:

1. `cbind` / `rbind`: in S, recycles columns/rows to the longest;
   in R, requires equal lengths or scalars. We follow R; the S
   frontend (future `s-runtime`) will detect the difference and
   either emulate or warn — TBD when that frontend is specced.
2. `apply` / `sapply` over uneven rows: S simplifies to a list-of-vectors;
   R simplifies to a matrix only if all rows have equal length. We
   follow R.

These differences live entirely in the S frontend. The cores see
recycled-or-broadcast vectors and don't know which frontend called.

---

## The Contract Cores Rely On

When a domain core function declares a signature like

```rust
pub fn add(a: &Double, b: &Double) -> Double { … }
```

it is allowed to assume:

1. `a.len() == b.len()` (already aligned by the frontend).
2. Both have the same `dim` attribute or both have none (already
   aligned).
3. NA propagation is element-wise: position-`i` NA in either input
   produces position-`i` NA in the output (per `na-semantics.md`).

If the frontend hands the core mismatched shapes, that is a frontend
bug. The core may panic in debug and return an error in release; it
does not attempt to recover.

This contract simplifies cores significantly. A naive
`add(a, b)` is:

```rust
pub fn add(a: &Double, b: &Double) -> Double {
    debug_assert_eq!(a.len(), b.len());
    let mut out = Double::na(a.len());
    for i in 0..a.len() {
        match (a.get(i), b.get(i)) {
            (Some(x), Some(y)) => out.set(i, Some(x + y)),
            _ => {} // already NA
        }
    }
    out
}
```

No recycling code, no shape juggling, no per-frontend branches.

---

## Shape Inference for Reductions

Reductions (`sum`, `mean`, etc.) collapse a vector to a scalar (technically
a length-1 vector). They take one input and have no shape-mismatch
problem. The contract:

- Input length 0 with `na_rm = true`: returns the function's empty-set
  identity (0 for `sum`, 1 for `prod`, NaN for `mean`/`var`/`sd`,
  `+Inf` for `min`, `-Inf` for `max`). See `na-semantics.md` for the
  full table.
- Input length 0 with `na_rm = false`: same as above; there are no
  NAs to propagate.
- Input has dim attribute (matrix): defaults to flattening (`sum(m)`
  sums everything). Per-axis reductions live on dedicated functions
  (`row_sums`, `col_sums`, etc.) that take the matrix in its native
  shape.

---

## Aggregate-with-Group Operations

`tapply`, `aggregate`, `by`, `dplyr::group_by` all share a shape:
*partition rows by a grouping vector, apply a reduction per partition,
return one row per group.*

Vectorization here is per-group, not per-element. The grouping vector
is recycled to the data's length under R's rules (so a length-2
grouping vector applied to a length-6 data vector produces three
groups of two each, with a warning if mismatched).

Implementation lives in `statistics-core::aggregate` (yet to be
specced). Both R and the future spreadsheet frontends route through
the same function.

---

## SIMD and Parallelism

Out of scope for this contract. The frontend hands the core aligned
vectors; the core may evaluate sequentially, with SIMD intrinsics, or
in parallel via `rayon`, all transparent to the caller. Numerical
results must be identical regardless of execution strategy (this is
non-trivial for floating-point reductions; see
`statistics-core.md` §Numerical Accuracy Contract for ULP tolerance
rules and the deterministic reduction order requirement).

---

## Test Vectors

Every frontend that vectorizes must pass:

1. R recycling: `c(1..6) + c(10, 20)` returns `c(11, 22, 13, 24, 15, 26)`
2. R warning: `c(1..5) + c(10, 20)` returns `c(11, 22, 13, 24, 15)` with one warning
3. Excel broadcast: `{1,2,3} + {10;20;30}` (row × column) returns 3×3 matrix
4. Excel mismatch: `{1,2,3} + {10,20}` returns 1×3 with `#N/A` in column 3
5. Length-zero: `c() + c(1, 2, 3)` returns length-zero
6. Scalar broadcast: `5 + c(1, 2, 3)` returns `c(6, 7, 8)` without warning
7. NA in either operand at position i produces NA at output position i
8. Aligned vectors round-trip through a domain core unchanged in shape

These tests live in `code/programs/<frontend>/vectorization-parity/`.

---

## Out of Scope

- Per-frontend implementation details (which crate computes which
  alignment) — covered in each frontend's own spec
- Non-rectangular array formulas (Excel allows ragged ranges via
  `LET`/`LAMBDA`; deferred to a follow-up)
- Tensor algebra (Einstein summation, etc.) — out of scope for the
  statistical stack; see future `tensor-core`
- Dataflow optimization across multi-step expressions — `r-runtime`
  may do this; the contract here is per-call

---

## References

- R Language Definition, §3.3.1 ("recycling rule")
- *The R Inferno*, ch. 1 ("Falling into the floating point trap" and
  "Recycling")
- Microsoft Excel function reference (Array Formulas; "Dynamic array
  formulas and spilled array behavior")
- Becker, Chambers, Wilks, *The New S Language* (1988), ch. 5
  (vectorization and recycling in S)
