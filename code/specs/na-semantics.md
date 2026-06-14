# NA Semantics

## Overview

R has a built-in concept that no other mainstream programming language
gets quite right: a value that is **not present** but is still typed.
Not null. Not zero. Not NaN. A real arithmetic third state that says
"there is supposed to be a number here, we just don't have it." This
spec defines how that third state is represented in the Rust substrate,
how it propagates through operations, and how it interacts with the
Float-rung NaN that the Numeric Tower already provides.

The contract lives across two surfaces:

- The **per-type bit pattern** that marks a slot as NA. Owned by
  `r-vector`; this spec defines what those patterns are and the
  invariants they must satisfy.
- The **propagation rules** for arithmetic, comparison, and logical
  operations. Owned by every Layer 1 domain core; this spec defines
  the rules they all follow.

One contract, applied identically by every domain core, so that
`mean(c(1, NA, 3))` and `=AVERAGE(A1:A3)` (where A2 is NA — distinct
from "empty," see §NA vs Empty Cell) and the future R `lm` all give
the same answer.

---

## Where It Fits

```
   Domain cores (statistics-core, financial-core, math-core, …)
                          │
                          │  call r-vector ops; observe NA propagation
                          ▼
   ┌──────────────────────────────────────────────┐
   │   r-vector  (atomic types, indexing)         │
   │   Each element slot is value-or-NA.          │
   │   Per-type NA bit pattern owned here.        │
   └──────────────────────────┬───────────────────┘
                              │
                              ▼
                       numeric-tower
                       (NaN ≠ NA — distinct concepts)
```

**Used by:** every numerical or statistical crate. The rules below are
*the* rules; no domain core is allowed to invent its own.

**Not used by:** `spreadsheet-core` for empty cells. An empty cell is
a different sentinel with different propagation rules — see §NA vs
Empty Cell.

---

## The Three-Valued Logic

R's logical type takes three values: `TRUE`, `FALSE`, `NA`. Boolean
operations are extended to handle `NA` using the **strong Kleene** truth
tables, which agree with classical logic whenever an answer is
determined and yield `NA` when the answer would depend on the unknown.

### `&` (AND)

|         | TRUE  | FALSE | NA    |
|---------|-------|-------|-------|
| **TRUE**  | TRUE  | FALSE | NA    |
| **FALSE** | FALSE | FALSE | FALSE |
| **NA**    | NA    | FALSE | NA    |

Rule: `FALSE & x` is always `FALSE` (even if `x = NA`), because the
answer is determined regardless of what `x` is. Symmetric.

### `|` (OR)

|         | TRUE  | FALSE | NA    |
|---------|-------|-------|-------|
| **TRUE**  | TRUE  | TRUE  | TRUE  |
| **FALSE** | TRUE  | FALSE | NA    |
| **NA**    | TRUE  | NA    | NA    |

Rule: `TRUE | x` is always `TRUE`, mirror of above.

### `!` (NOT)

| x     | !x    |
|-------|-------|
| TRUE  | FALSE |
| FALSE | TRUE  |
| NA    | NA    |

### Equality and ordering

`NA == anything` (including `NA == NA`) is `NA`. The reasoning: equality
asks whether two values are the same; if either is unknown, the answer
is unknown. To test for NA, use `is_na(x)` — never `x == NA`.

`NA < anything`, `NA > anything`, `NA <= anything`, `NA >= anything`
are all `NA`. Same reasoning.

`NA != NA` is `NA` (not `FALSE`). This is the classic R footgun and
must be preserved.

---

## Per-Type Bit Patterns

Each atomic-vector element type has a designated NA bit pattern.
`r-vector` is responsible for never producing this pattern as a
*computed* value and for preserving the pattern across operations
that the propagation rules say should produce NA.

### Logical

`Logical` is a 3-state type stored as `i8`:

| Value | i8  |
|-------|-----|
| FALSE | 0   |
| TRUE  | 1   |
| NA    | -128 (`i8::MIN`) |

Any `i8` other than 0, 1, or -128 is invalid and indicates corruption.

### Integer

R's `integer` is fixed 32-bit signed. The NA pattern is `i32::MIN`
(`-2_147_483_648`). This means the integer type cannot represent
`-2_147_483_648` as a real value — that bit pattern is reserved.
Producing it as the result of arithmetic is an overflow error
distinct from NA, and `r-vector` rejects it. Note the distinction
from `numeric-tower::Number::Integer` which is arbitrary-precision
and has no NA — NA exists at the *vector* layer, not the scalar
arithmetic layer.

### Double

`Double` is f64. The NA pattern is a specific NaN payload:

```
NA_REAL = 0x7FF0_0000_0000_07A2  (R's choice; we reproduce it for
                                  cross-implementation parity)
```

Where 0x7A2 = 1954 is the year R was conceived (per the R FAQ;
preserved here for compatibility with R `RData` interchange).

This bit pattern *is* a NaN by IEEE 754, but it has a unique payload
that the `is_na` predicate distinguishes. Every other NaN bit pattern
is a "real" NaN that means *the computation produced something
undefined* — see §NA vs NaN.

### Complex

`Complex` is two f64s. NA is signaled by the real part holding
`NA_REAL` (the imaginary part is also set to `NA_REAL` for
consistency, but `is_na` only inspects the real part).

### Character

`Character` is `Option<String>` semantically; the `None` variant is
NA. There is no in-band string sentinel because `""` is a real
distinct value (the empty string).

### Raw

`Raw` is `u8` and **has no NA representation**. R's `raw` type does
not support NA. `r-vector` operations on `Raw` do not propagate NA;
attempting to store NA in a `Raw` slot is a type error caught at
construction.

---

## NA vs NaN

These are distinct in R, distinct in this spec, and both must
round-trip through the wire.

| Concept | Meaning | Belongs to | Created by | `is_na` | `is_nan` |
|---------|---------|------------|------------|---------|----------|
| NaN     | The computation produced something undefined (`0/0`, `log(-1)`) | Float rung of the Numeric Tower | Arithmetic | TRUE  | TRUE  |
| NA      | The value was never recorded; we don't know what it is | r-vector slot, every atomic type | Constructor / explicit assign | TRUE  | FALSE |

Note that `is_na(NaN)` is `TRUE`. This is R's behavior and is
deliberate: any "missing or undefined" check should accept both. To
distinguish, use `is_nan` on a Double element.

The distinction matters because:

- `mean(c(1, NaN, 3))` returns `NaN` (a real arithmetic outcome)
- `mean(c(1, NA, 3))` returns `NA` (the answer is unknown)
- `mean(c(1, NaN, 3), na.rm = TRUE)` still returns `NaN` (NaN is not removable; it is real)
- `mean(c(1, NA, 3), na.rm = TRUE)` returns `2` (NA is removable)

`r-vector` and every domain core preserves this distinction.

---

## NA vs Empty Cell

`spreadsheet-core` introduces a fourth state — the **empty cell**.
This is *not* NA. It is its own sentinel with its own propagation
rules:

| Concept | Source | In arithmetic | In text concat | In count | In comparison |
|---------|--------|---------------|----------------|----------|---------------|
| NA      | `r-vector` slot (R origin) | propagates to NA | propagates to NA | counted by `count_non_na` as 0 | yields NA |
| Empty cell | spreadsheet `=A1` where A1 is blank | coerces to 0 | coerces to "" | not counted by `COUNT` | coerces to 0 or "" |

Excel chose this behavior for practical reasons: it lets `=SUM(A1:A100)`
work over a partially-filled column without manually filtering. R chose
NA because statistical analysis demands the distinction between "we
didn't measure" and "we measured zero."

The two sentinels are *both* representable in `spreadsheet-core`'s cell
type, and a spreadsheet can hold an explicit `=NA()` (which behaves like
R's NA) or a blank cell (which behaves like Excel's empty). When
spreadsheet data is exported to an `r-vector`, blanks become NA — but
that is a frontend concern, not a substrate concern, and is handled at
the import boundary.

---

## Propagation Rules for Arithmetic

The default rule applied by every Layer 1 domain core:

> **If any input is NA, the output is NA, unless the function is
> documented as taking `na.rm = TRUE` and the caller passed it.**

Examples (where `vec` is shorthand for `Vector<Double>`):

| Call | Result |
|------|--------|
| `add(NA, 5)` | NA |
| `mean(vec[1, NA, 3])` | NA |
| `mean(vec[1, NA, 3], na.rm = TRUE)` | 2 |
| `mean(vec[NA, NA, NA])` | NA |
| `mean(vec[NA, NA, NA], na.rm = TRUE)` | NaN (Float rung; mean of empty set) |
| `sum(vec[NA, NA, NA], na.rm = TRUE)` | 0 (sum of empty set is 0 by convention) |
| `prod(vec[NA, NA, NA], na.rm = TRUE)` | 1 (product of empty set is 1 by convention) |
| `length(vec[1, NA, 3])` | 3 (length is structural — counts NAs) |
| `count_non_na(vec[1, NA, 3])` | 2 |

The "empty after na.rm" cases (`mean`, `var`, `sd` returning NaN; `sum`/`prod`
returning identity) follow R exactly.

---

## The `na.rm` Convention

Every reduction function in `statistics-core` (any function that
collapses a vector to a scalar) accepts a final parameter
`na_rm: bool` (Rust signature; Excel/R bind it differently at the
frontend):

```rust
pub fn mean(x: &Vector<Double>, na_rm: bool) -> Number { … }
```

- Default value at the Rust API: `false`. Callers must opt in.
- Excel binding: `=AVERAGE(...)` always sets `na_rm = true` and
  treats `=NA()` cells as removable (matching Excel's "ignore NA"
  policy as of Excel 2010+). `=AVERAGEA(...)` sets `na_rm = false`
  and treats NA as part of the data, returning `#N/A`.
- R binding: defaults to `na.rm = FALSE`; user passes `na.rm = TRUE`
  to opt in.

Functions that are not reductions (element-wise operations like
`add`, `log`, `sqrt`) do not take `na_rm`. They always propagate.

---

## NA in Indexing

Indexing a vector with an NA index produces an NA element in the
output:

```
x <- c(10, 20, 30)
x[NA_integer_]   # → NA
x[c(1, NA, 3)]   # → c(10, NA, 30)
```

This is structural NA propagation: the *position* of the NA in the
index becomes an NA in the output, regardless of `x`'s contents at
that position.

Logical indexing with NA also produces NA at that position:

```
x[c(TRUE, NA, TRUE)]  # → c(10, NA, 30)
```

---

## Serialization

| Format | NA representation |
|--------|-------------------|
| R `RData` (binary) | The native bit patterns above |
| CSV out | Configurable string; default `"NA"` |
| CSV in | Configurable list; default `["", "NA", "N/A"]` |
| JSON out | `null` |
| JSON in | `null` → NA |
| Spreadsheet xlsx | `#N/A` error sentinel for explicit NA; blank cell for empty |

Serialization libraries are out of this spec's scope; they live in
`r-data-io` (future). The contract here is: an NA round-trips as an
NA through any of these formats, and an empty-cell sentinel does
not get conflated with NA except at the spreadsheet-import boundary.

---

## Test Vectors

Every implementation of `r-vector` and every domain core function
must pass a parity-test suite that exercises:

1. All four element types (Logical, Integer, Double, Character) with
   NA in every position of a 3-element vector
2. Strong-Kleene tables for `&`, `|`, `!`, `==`, `!=`, `<`, `<=`,
   `>`, `>=` over `{TRUE, FALSE, NA}` (9 cells per binary op)
3. Reduction parity: `sum/mean/var/sd/min/max/prod` with all-NA,
   one-NA, no-NA inputs, both with and without `na_rm`
4. Empty-after-`na_rm` cases (NaN for mean/var/sd, 0 for sum, 1 for
   prod)
5. NA vs NaN distinction: `is_na` is true for both; `is_nan` is
   true only for NaN
6. NA-in-index propagation for all four index modes (positive,
   negative, logical, by-name)

The full test corpus lives in `r-vector/tests/na_parity.rs` and is
referenced by every domain core's CI.

---

## Out of Scope

- The bit-level representation invariants of `r-vector` (those live
  in `r-vector.md`)
- Per-function NA behavior of every statistics-core function (those
  live in `statistics-core.md`'s function catalog)
- The CSV / JSON / xlsx import-export contract (future `r-data-io`
  spec)
- POSIXct / Date NA (datetime-core; same rules but the bit pattern is
  in time domain)

---

## References

- R Language Definition, §3.3.1 NULL and NA
- R Internals, §1.1 (NA bit patterns)
- *The Statistical Sleuth*, ch. 1 (the philosophical case for NA as a
  first-class arithmetic state)
- Strong Kleene three-valued logic (Kleene 1938)
