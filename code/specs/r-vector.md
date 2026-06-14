# R-Vector

## Overview

In R, *every* value is a vector. There is no scalar type. `1` and
`c(1)` are indistinguishable; both are length-one numeric vectors.
This decision shapes the rest of the language: arithmetic broadcasts,
indexing has four flavors, and missing data is a per-element first-class
concern. The Rust crate `r-vector` provides the foundational type that
makes this design possible — a typed, NA-aware, named, recyclable
sequence — used by every Layer 1 statistical or numerical core in the
repo and by the future `r-runtime` and `s-runtime` frontends.

This spec defines the crate's data structures, indexing modes, recycling
rule, and the bridge to the `numeric-tower`. Per-element NA bit patterns
and propagation rules live in `na-semantics.md`; this spec assumes that
contract and focuses on the container.

The crate lives at `code/packages/rust/r-vector/`. Its only Rust
dependencies are the standard library, `numeric-tower`, and (transitively
via numeric-tower) `num-bigint` and `num-rational`.

---

## Where It Fits

```
   r-runtime (future)        statistics-core         spreadsheet-core
        │                          │                        │
        │  AST → vectors           │  function inputs       │  range → vector
        ▼                          ▼                        ▼
   ┌────────────────────────────────────────────────────────────┐
   │                       r-vector                             │
   │   Vector<T>  +  atomic types  +  NA  +  names  +  indexing │
   └──────────────────────────┬─────────────────────────────────┘
                              │
                              ▼
                       numeric-tower
```

**Used by:** statistics-core (every function takes `&Vector<…>`);
spreadsheet-core (cell ranges materialize as vectors before being passed
to dispatch); future r-runtime and s-runtime; data-frame (a data frame
is a list of equal-length `Vector`s).

**Not used by:** text-core for non-vector string operations; datetime-core
for scalar-only date arithmetic. Both wrap their results in vectors at
the boundary.

---

## Atomic Types

R has six atomic types. The crate exposes one Rust type per atomic:

| R atomic   | Rust storage         | NA representation       | Notes |
|------------|----------------------|-------------------------|-------|
| `logical`  | `Vec<i8>`            | `i8::MIN` (-128)        | Tri-state: 0, 1, NA |
| `integer`  | `Vec<i32>`           | `i32::MIN`              | Fixed 32-bit; reserved bit pattern is NA |
| `double`   | `Vec<f64>`           | `NA_REAL` NaN payload   | The IEEE 754 base of R numerics |
| `complex`  | `Vec<(f64, f64)>`    | `(NA_REAL, NA_REAL)`    | Real + imaginary |
| `character`| `Vec<Option<String>>` | `None`                 | Empty string `""` is a real value, not NA |
| `raw`      | `Vec<u8>`            | none (no NA support)    | Bytes; cannot hold NA |

Each gets its own concrete Rust type so monomorphization specializes
arithmetic and the NA check is inlined:

```rust
pub struct Logical { data: Vec<i8>,         names: Option<Names>, attrs: Attrs }
pub struct Integer { data: Vec<i32>,        names: Option<Names>, attrs: Attrs }
pub struct Double  { data: Vec<f64>,        names: Option<Names>, attrs: Attrs }
pub struct Complex { data: Vec<(f64, f64)>, names: Option<Names>, attrs: Attrs }
pub struct Character { data: Vec<Option<String>>, names: Option<Names>, attrs: Attrs }
pub struct Raw     { data: Vec<u8>,         names: Option<Names>, attrs: Attrs }
```

A trait `Vector` unifies them for code that does not care about element
type:

```rust
pub trait Vector {
    type Element;
    fn len(&self) -> usize;
    fn is_na(&self, i: usize) -> bool;
    fn names(&self) -> Option<&Names>;
    fn attr(&self, key: &str) -> Option<&Attr>;
    fn type_name(&self) -> &'static str;  // "logical" | "integer" | …
}
```

The trait is small on purpose. Most operations live on the concrete
types where the compiler can specialize.

### Why `i32`-fixed for `Integer` when `numeric-tower::Integer` is arbitrary

R's `integer` is a 32-bit signed type. We faithfully reproduce that
because R `RData` files, R's C API, and R's coercion rules all assume
it. The arbitrary-precision `numeric-tower::Integer` is a distinct
concept used at the *scalar arithmetic* layer, not the vector layer.

This is the same separation as in R itself: R's `integer` overflows to
NA with a warning, but R's internal `bignum` (in package `gmp`) does
not. The vector type matches the language; the tower's exact integer is
for everything else.

When a `Vector<Integer>` overflows during arithmetic, the result is NA
with a flag set on the result vector's attributes (`overflow = TRUE`).
This matches R.

---

## NA

NA is per-type and per-element. The bit-pattern table is in
`na-semantics.md`. Three crate-level invariants:

1. Every constructor of every atomic type checks for and rejects an
   accidentally-produced NA bit pattern in non-NA slots. (Specifically:
   `i32::MIN` cannot enter an `Integer` vector except as an NA; the
   `NA_REAL` payload cannot enter a `Double` vector except as an NA.)
2. Every operation that the propagation rules say should produce NA
   *does* produce NA at exactly the right slots, never spreading
   beyond.
3. `is_na(i)` is `O(1)` and inlines to a single comparison or pattern
   match.

The crate exposes one universal `is_na` predicate per atomic type plus
a generic version through the `Vector` trait.

---

## Names Attribute

A vector may have an optional `names` attribute:

```rust
pub struct Names { values: Vec<String> }   // length must match data
```

When present, `names.len() == data.len()`. Names are unique-or-not at
the user's choice; R does not enforce uniqueness, and neither do we
(but see §Indexing for what happens when `x["a"]` matches multiple).

Names are preserved across operations that preserve shape:

| Operation | Names preserved? |
|-----------|------------------|
| Element-wise arithmetic on a single named vector | Yes — names of the LHS |
| Element-wise arithmetic on two named vectors | Yes — names of the longer (or LHS if same length) |
| Reduction (sum, mean, …) | No — result is unnamed scalar |
| Indexing | Yes — derived from indexed-into vector |
| Concatenation `c(a, b)` | Yes — both sets concatenated, with `""` filling unnamed positions |

---

## Attributes (other)

Every vector carries an attribute bag:

```rust
pub struct Attrs { entries: Vec<(String, AttrValue)> }
pub enum AttrValue { Logical(Logical), Integer(Integer), Double(Double),
                     Complex(Complex), Character(Character), Raw(Raw),
                     List(List) }
```

`names` is conceptually an attribute (`x@names`) but is given a typed
slot for performance because every operation queries it.

Reserved attribute keys recognized across the repo:

| Key       | Meaning                                                 | Owner |
|-----------|---------------------------------------------------------|-------|
| `dim`     | Vector is implicitly a matrix/array (e.g. `c(2, 3)` for 2×3) | r-vector |
| `dimnames`| Per-axis names for a matrix/array                       | r-vector |
| `class`   | S3 class chain                                          | r-runtime |
| `levels`  | Factor levels (ordered or unordered)                    | r-runtime (factor type) |
| `tsp`     | Time-series start, end, frequency                       | statistics-core::ts |
| `comment` | Free-form annotation                                    | r-runtime |
| `overflow`| Set to `TRUE` if any Integer arithmetic overflowed     | r-vector |

User-defined attributes (any string key not on this list) are stored
verbatim and round-trip through serialization.

---

## Indexing

Four index modes, all supported by every atomic type. The Rust API:

```rust
impl<V: Vector> V {
    fn index_positive(&self, idx: &Integer) -> V::Output;
    fn index_negative(&self, idx: &Integer) -> V::Output;
    fn index_logical(&self, idx: &Logical) -> V::Output;
    fn index_named(&self, idx: &Character) -> V::Output;
}
```

A higher-level `index(i: &Index)` dispatches by index kind. `Index` is
an enum with the four variants plus a fifth for the empty index `x[]`
(which returns `x` unchanged).

### Positive indexing

Indices are 1-based (R convention; the language we are emulating).
`x[c(1, 3)]` returns positions 1 and 3. Position 0 is silently dropped.
Out-of-range positions yield NA in the output, with the original NA
distinction preserved.

Internally the crate uses 0-based offsets; the 1-based-to-0-based shift
happens at the API boundary in a single helper. This avoids the entire
crate carrying a +1/-1 footgun.

### Negative indexing

`x[c(-1, -3)]` removes positions 1 and 3. Mixing positive and negative
in the same index vector is an error (R behavior). Index 0 is silently
dropped.

### Logical indexing

`x[c(TRUE, FALSE, TRUE)]` returns positions where the index is `TRUE`.
The index is recycled to `length(x)` if shorter (see §Recycling). NA
in the index produces NA in the output at that position
(`na-semantics.md`, §NA in Indexing).

### Named indexing

`x["a"]` returns elements whose name is `"a"`. If multiple names match,
returns the first (R behavior); to get all matches, the user converts
to logical-index manually. Unmatched names produce NA in the output.

---

## Recycling

When a binary operation receives two vectors of different lengths, the
shorter is *recycled* — repeated end-to-end — to match the longer. This
is R's most distinctive (and most dangerous) feature.

```
c(1, 2, 3, 4, 5, 6) + c(10, 20)
```

becomes

```
c(1, 2, 3, 4, 5, 6) + c(10, 20, 10, 20, 10, 20)  =  c(11, 22, 13, 24, 15, 26)
```

Rule:

1. Let `n = max(len(a), len(b))`.
2. Each operand is repeated to length `n` (the longer is repeated zero
   times — i.e. unchanged).
3. **If `n` is not a multiple of either input's length, emit a warning.**
   The operation still proceeds with truncated recycling, but the
   warning ("longer object length is not a multiple of shorter object
   length") flags a likely bug.

Recycling is implemented in `r-vector::recycle` and is invoked by
domain cores explicitly:

```rust
pub fn recycle_to_max<A: Vector, B: Vector>(a: &A, b: &B) -> (A, B);
```

The function returns owned (or cheaply cloned) recycled copies. For
zero-copy paths, `RecycledIter` provides an iterator interface that
indexes modulo length without allocating.

Recycling rule subtleties:

- If either operand has length 0, the result has length 0.
- If either operand has length 1, that single value is broadcast to
  the other's length without warning.
- Recycling does **not** apply to dispatch: `mean(c(1, 2, 3))` returns
  a length-1 vector whose contents the receiving function decides;
  there is no recycling at the function-call boundary.

Spreadsheet array formulas use *broadcasting*, not recycling — see
`vectorization-rules.md`.

---

## Coercion Lattice

When two atomic types meet, the *less expressive* coerces upward.
R's lattice (we reproduce it):

```
   character
       │
   complex
       │
    double
       │
   integer
       │
   logical
```

Joins for binary operations:

| Left      | Right     | Join     |
|-----------|-----------|----------|
| Logical   | Integer   | Integer  |
| Logical   | Double    | Double   |
| Integer   | Double    | Double   |
| Double    | Complex   | Complex  |
| Anything  | Character | Character|

`raw` does not participate in arithmetic coercion; mixing `raw` with
non-`raw` is a type error.

`Vector::coerce(target_type)` produces a copy at the requested type or
an error if the conversion is invalid. Coercion of NA at any rung
yields NA at the new rung. Coercion of out-of-range values (e.g.
`integer(2147483648)`) yields NA with the overflow attribute set.

Character coercion deserves a note: `as.character(c(0.1 + 0.2))`
returns `"0.3"`, not `"0.30000000000000004"`. R uses the shortest
decimal that round-trips. We match this via the `ryu` crate
(or equivalent `f64`-shortest-decimal algorithm). Tested against R's
`format` output.

---

## Element Access

```rust
impl Double {
    pub fn get(&self, i: usize) -> Option<f64>;        // None if NA
    pub fn get_raw(&self, i: usize) -> f64;            // Returns NA bit-pattern as-is
    pub fn set(&mut self, i: usize, value: Option<f64>);
    pub fn iter(&self) -> impl Iterator<Item = Option<f64>>;
    pub fn iter_raw(&self) -> impl Iterator<Item = f64>;
}
```

`get` returns `Option` so the caller cannot accidentally use an NA bit
pattern as a real value. `get_raw` is the escape hatch for crates that
want bit-level access (e.g. zero-copy serialization to `RData`).

The same shape exists on every atomic type, with the appropriate
element type substituted for `f64`.

---

## Construction

```rust
impl Double {
    pub fn from_iter<I: IntoIterator<Item = Option<f64>>>(iter: I) -> Self;
    pub fn of(values: &[f64]) -> Self;       // No NAs
    pub fn na(n: usize) -> Self;             // All NA, length n
    pub fn empty() -> Self;
    pub fn singleton(v: f64) -> Self;
}
```

The `from_iter` constructor is the only one that can produce NA slots.
`of` rejects (panics in debug, returns error in release) any input
containing an NA bit pattern, because that pattern would be ambiguous.

---

## Bridge to numeric-tower

Each atomic type can lift to a `Number` for scalar contexts:

```rust
impl Double {
    pub fn to_scalar(&self) -> Result<Option<Number>, NotScalar> {
        if self.len() != 1 { return Err(NotScalar); }
        match self.get(0) {
            Some(v) => Ok(Some(Number::Float(v))),
            None    => Ok(None),  // NA
        }
    }
}
```

Length-1 vectors can be used as scalars; longer vectors cannot. This
is the boundary at which `mean(c(1, 2, 3))` returns a length-1 vector
that a frontend then unboxes to a scalar for display.

The reverse direction:

```rust
impl Double {
    pub fn from_scalar(n: Number) -> Result<Self, IncompatibleRung> {
        match n {
            Number::Float(f)   => Ok(Self::singleton(f)),
            Number::Integer(i) => Ok(Self::singleton(i.to_f64())),
            Number::Rational(r)=> Ok(Self::singleton(r.to_f64())),
            Number::Complex(_) => Err(IncompatibleRung),
            Number::Decimal(d) => Ok(Self::singleton(d.to_f64())),
        }
    }
}
```

A `Number::Complex` cannot become a `Vector<Double>`; the caller must
coerce to `Vector<Complex>` first.

---

## Implicit Matrix / Array

A vector with the `dim` attribute is implicitly a matrix or array:

```rust
let m = Double::of(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0])
    .with_dim(&[2, 3]);   // 2 rows × 3 cols, column-major
```

R uses column-major storage. We follow.

This crate provides the storage; matrix arithmetic (multiplication,
solve, decompositions) lives in `statistics-core::linalg`. The `dim`
attribute is the only matrix concept r-vector exposes.

---

## Equality

`Vector` equality is element-wise. Two vectors are `==` if:

1. They have the same atomic type
2. Same length
3. Same `dim` attribute (or both absent)
4. Same names (or both absent)
5. Each non-NA element is equal at the join rung
6. Each NA position aligns with an NA position in the other

R has separate `==` (element-wise, returns a logical vector with NA
where either side is NA) and `identical` (whole-vector strict
equality). We expose both:

```rust
pub fn eq_elementwise(&self, other: &Self) -> Logical;  // R's ==
pub fn identical(&self, other: &Self) -> bool;          // R's identical()
```

`identical` returns `false` (not NA) for NA-vs-non-NA mismatches.
`eq_elementwise` returns NA at those positions. Both are needed.

---

## Memory and Performance

- Vectors own their data (`Vec<T>` storage). No reference-counted
  sharing in this crate.
- R uses copy-on-modify semantics at the language level; `r-runtime`
  layers that on top via `Rc<Vector>` with deep-clone-on-write. The
  crate itself is value-typed.
- Element access is `O(1)`. Indexing produces a new vector; we do not
  return views (Rust lifetimes make views painful, and the cost of a
  copy is acceptable for the educational scope).
- Recycling does not allocate when used through `RecycledIter`.

---

## Test Vectors

Every implementation must pass:

1. NA round-trip: every atomic type, NA in every slot, survives
   construct → serialize → deserialize → query
2. Strong-Kleene tables for `&`, `|`, `!` over `Logical`
3. Recycling: `c(1..6) + c(10, 20)` produces `c(11, 22, 13, 24, 15, 26)`
4. Recycling warning: `c(1..5) + c(10, 20)` produces `c(11, 22, 13, 24, 15)` and emits one warning
5. All four index modes on a 5-element vector with named slots
6. Coercion: `Logical → Integer → Double → Complex → Character` round-trips for both real values and NA at each rung
7. Names preservation across element-wise ops
8. `dim` attribute round-trips through indexing

The `tests/` directory contains all of these as separate test files.
Cross-language parity tests (against the existing Python `stats`
package's vector helpers and against actual R output) live in
`code/programs/<lang>/r-vector-parity/`.

---

## Out of Scope

- Factors (categorical type) — own crate, depends on r-vector
- Data frames — own crate, depends on r-vector
- Lists (heterogeneous typed) — folded into r-vector as `List` later
- Lazy evaluation of vector expressions (e.g. delayed computation in
  `lm(y ~ x)`) — that's r-runtime's job
- SIMD optimizations — out of scope for v1; the substrate is correct
  before it is fast

---

## References

- R Language Definition, ch. 2 (vector structures) and §3.3 (NA, NULL)
- R Internals, §1.1 (atomic types and storage modes)
- *The R Inferno* (Burns 2011), ch. 1 (recycling pitfalls)
- Wickham, *Advanced R*, ch. 3 (vectors), ch. 4 (subsetting)
