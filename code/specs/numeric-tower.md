# Numeric Tower

## Overview

This spec defines the `Number` type that every numerical and statistical
crate in the Rust half of the repo agrees on. It is the type a spreadsheet
cell holds, the type an R atomic-double element carries, the type a
financial calculation accepts, and the type an `lm` coefficient comes back
as. Without a single answer to "what is a number," every domain crate
invents its own and the layers above the substrate cannot talk to each
other.

The tower is small and lives in one Rust crate, `numeric-tower`, at
`code/packages/rust/numeric-tower/`. It has no upstream Rust dependencies
beyond the standard library and `num-bigint`/`num-rational` (for the
exact-integer and exact-rational rungs). It is depended on by every Layer 1
domain core (statistics-core, financial-core, …) and by every Layer 0
companion crate (r-vector, data-frame).

This spec does **not** define how a number is *displayed* (that is a
spreadsheet cell-format concern) and does **not** define error behavior
for individual functions (that is each domain core's contract). It defines
representation, coercion, and arithmetic dispatch.

---

## Where It Fits

```
   statistics-core   financial-core   math-core   …          (Layer 1)
            ▲              ▲              ▲
            │              │              │
            └──────────────┴──────────────┘
                           │
                  ┌────────┴────────┐
                  │  numeric-tower  │   ← THIS SPEC
                  └────────┬────────┘
                           │
                           ▼
                std + num-bigint + num-rational
```

**Used by:**
- `r-vector` (the atomic-numeric vectors hold these as elements)
- `statistics-core` (every public function takes/returns these)
- `financial-core` (NPV, IRR; benefits from `Decimal` for monetary precision)
- `math-core`, `lookup-core`, `text-core` (coercion when crossing types)
- `spreadsheet-core` (cell value type wraps `Number` plus error sentinels)
- `cas-*` interop (the symbolic `IRInteger` / `IRRational` / `IRComplex`
  bridge to and from the tower's `Integer` / `Rational` / `Complex` rungs)

**Not used by:** `text-core`, `datetime-core` for their own values
(strings and dates are not `Number`s). They use `Number` only when an
arithmetic context coerces to or from one of them.

---

## The Five Rungs

The tower has exactly five rungs, ordered by *expressiveness* (each rung
can faithfully represent everything below it that is not lost in
conversion). Rust:

```rust
pub enum Number {
    Integer(Integer),     // arbitrary precision; wraps num_bigint::BigInt
    Rational(Rational),   // arbitrary precision; wraps num_rational::BigRational
    Float(f64),           // IEEE 754 binary64
    Complex(Complex),     // pair of f64 (re, im)
    Decimal(Decimal),     // base-10 fixed precision; see §Decimal below
}
```

The order is **not** the coercion lattice (see next section). It is the
order in which the rungs were chosen and the order in which the variants
are listed for stable serialization.

### Why these five and not others

- **Integer** is arbitrary precision because spreadsheet cells, factor
  codes, and R `integer` vectors all need it without overflow surprises.
  A separate fixed-width `i64` rung was rejected: the cost of one extra
  arithmetic dispatch case is tiny, and "integer overflow at row 5,
  column AB" is the kind of bug this tower exists to prevent.
- **Rational** because exact division is required for `lm` over rational
  data, for `cas-*` interop (the symbolic IR uses `Fraction` everywhere),
  and because monetary calculations during intermediate steps benefit
  from no-loss division.
- **Float** because most numerical algorithms (distributions, regression
  internals, optimizers) want IEEE 754 and because every interchange
  format the tower will eventually meet (CSV, JSON, R `RData`, Excel
  XLSX) defaults to it.
- **Complex** because R has it as an atomic type and the symbolic stack
  already uses Gaussian integers in `cas-complex`. Any statistical
  consumer can ignore it.
- **Decimal** because spreadsheet financial functions and tax/accounting
  calculations need radix-10 arithmetic where `0.1 + 0.2 == 0.3`. See
  `§Decimal`.

A `BigDecimal` (arbitrary precision base-10) was considered as a separate
rung. It is folded into `Decimal` with a precision parameter; see below.

---

## Coercion Lattice

When two rungs meet in a binary operation, the result lives on the
*join* — the lowest rung that can represent both faithfully without
loss. The lattice:

```
              Complex
              ╱     ╲
         Float       (no path)
        ╱     ╲
   Decimal   Rational
              │
           Integer
```

Read top-down for "more expressive." Joins:

| Left      | Right     | Join     |
|-----------|-----------|----------|
| Integer   | Integer   | Integer  |
| Integer   | Rational  | Rational |
| Integer   | Decimal   | Decimal  |
| Integer   | Float     | Float    |
| Integer   | Complex   | Complex  |
| Rational  | Decimal   | Decimal  |
| Rational  | Float     | Float    |
| Rational  | Complex   | Complex  |
| Decimal   | Float     | **Float** (lossy; flagged in §Lossy Coercions) |
| Decimal   | Complex   | Complex  |
| Float     | Complex   | Complex  |

**There is no path from Decimal to Rational.** A finite Decimal
(`0.1`) is exactly representable as `Rational(1, 10)`, but a `Rational`
with non-terminating decimal expansion (`1/3`) is not faithfully
representable as a finite-precision `Decimal`. We resolve the
asymmetric case by routing through Float and accepting the loss; users
who want exact rational answers must stay on the Rational/Integer
sub-lattice.

The `coerce_to_join(a: &Number, b: &Number) -> (Number, Number)`
function is the single entry point. Domain crates never inspect rung
tags directly to decide arithmetic; they call `coerce_to_join` and
match on the result.

---

## Lossy Coercions

A coercion is **lossy** if the join cannot represent the source value
exactly. Lossy coercions are permitted but recorded.

| From      | To        | Lossy when                                      |
|-----------|-----------|-------------------------------------------------|
| Integer   | Float     | abs(value) ≥ 2^53                               |
| Rational  | Float     | denominator is not a power of 2 small enough to be exact, OR magnitude exceeds f64 range |
| Rational  | Decimal   | non-terminating decimal expansion (e.g. 1/3)    |
| Decimal   | Float     | digits exceed f64 mantissa precision            |
| Decimal   | Rational  | n/a — always exact                              |
| Float     | anything  | NaN/Inf cannot leave the Float rung; coercion of NaN to Integer/Rational/Decimal is an error, not a silent zero |

The tower exposes a `try_coerce(n: &Number, target_rung: Rung) -> Result<Number, CoercionError>` function for callers that want the loss reported. The
internal `coerce_to_join` always succeeds (it picks the join precisely
to avoid this); the failure case only arises when a frontend asks for
"coerce this to integer."

---

## Arithmetic Dispatch

Each binary operation is implemented as

```
fn add(a: &Number, b: &Number) -> Number {
    let (a, b) = coerce_to_join(a, b);
    match (a, b) {
        (Integer(x), Integer(y)) => Integer(x + y),
        (Rational(x), Rational(y)) => Rational(x + y),
        (Float(x), Float(y)) => Float(x + y),
        (Decimal(x), Decimal(y)) => Decimal(x + y),
        (Complex(x), Complex(y)) => Complex(x + y),
        _ => unreachable!(), // coerce_to_join returns same-rung pairs
    }
}
```

The `_ => unreachable!()` arm is load-bearing: the type system does
not enforce that `coerce_to_join` returns same-rung pairs, so the
crate's invariant is `coerce_to_join(a, b)` ⟹ both outputs share a rung.
Tested by an exhaustive property-based suite over rung pairs.

The five operations the tower defines: `add`, `sub`, `mul`, `div`, `neg`.
Higher-order operations (`pow`, `sqrt`, `log`, `exp`, trig) live in
`math-core` and accept/return `Number`s; the tower stays small.

### Division on Integer

`div` on `Integer / Integer` lifts to `Rational` if the divisor does
not divide the dividend exactly. R does not do this (it returns Float);
spreadsheets do not do this (also Float). The tower's choice is
*Rational* for exactness, and frontends that want R-style floor or
spreadsheet-style coerce-to-Float wrap the result. This keeps the
tower itself opinion-free about which lossy step a frontend prefers.

### NaN, Infinity, signed zero

Live entirely on the `Float` rung. `Integer` / `Rational` / `Decimal`
have no infinity and no NaN. A `Float(NaN)` propagates through any
operation that touches it; this is **not** the same as NA (see
`na-semantics.md` — NaN means *the computation produced something
undefined*, NA means *we never had a value*). The two are distinct on
the wire, distinct in equality, and distinct in serialization.

`Float(0.0) == Float(-0.0)` is `true` for `Number::eq`, matching R
and IEEE 754 semantics in equality (but the bit-patterns differ in
serialization, and `Float::is_sign_negative` distinguishes them).

---

## Decimal

`Decimal` is base-10 fixed-precision. The Rust representation:

```rust
pub struct Decimal {
    coefficient: BigInt,   // signed
    exponent: i32,         // value = coefficient * 10^exponent
    precision: u32,        // significant digits; 0 means "as many as coefficient has"
}
```

This follows IEEE 754-2008 decimal arithmetic and Java `BigDecimal`. A
spreadsheet cell formatted as currency (`$1,234.56`) holds
`Decimal { coefficient: 123456, exponent: -2, precision: 6 }`.

Operations on `Decimal` use the *banker's rounding* mode (round
half-to-even) by default. The mode is overridable per-operation for
financial-core to use round-half-up where regulation requires it.

**`Decimal` is not a high-performance rung.** Operations are
`O(precision²)` in the worst case. Statistical algorithms should not
work in `Decimal`; they should coerce to `Float` at the boundary.
`Decimal` exists for spreadsheet cell values and financial outputs
where the answer must be "the same answer the calculator on the
accountant's desk gave."

---

## Equality and Ordering

Two `Number`s are equal iff they coerce to the same join *and* the
joined values are equal at that join. `Integer(2) == Float(2.0)` is
`true`. `Integer(2) == Rational(4, 2)` is `true`. `Float(NaN) == Float(NaN)`
is `false` (matches IEEE 754).

Ordering exists on the totally-ordered sub-lattice (everything except
Complex; partial-order on Complex would be a lie). Calling
`Number::partial_cmp` on a Complex with anything else returns `None`.

`hash(n: Number)` normalizes by coercing to the *highest exact* rung
(Integer if the value is integral; Rational if rational; Float
otherwise). This makes `Integer(2)` and `Rational(2, 1)` hash the same.
Required for use as `HashMap` keys in the symbol table of `r-runtime`
and the named-cell index of `spreadsheet-core`.

---

## Bridges

### `cas-*` Bridge

`cas-complex` / `cas-factor` / etc. use their own internal IR
(`IRInteger`, `IRRational`, `IRComplex`) for symbolic computation.
The bridge lives in `numeric-tower::cas` (feature-gated, so consumers
that don't pull in `cas-*` aren't forced to compile it):

```
Number::Integer(n)   ⇄  cas_simplify::IRInteger(n)
Number::Rational(r)  ⇄  cas_simplify::IRRational(r)
Number::Complex(c)   ⇄  cas_complex::IRComplex(c)
Number::Float(f)     →  cas_simplify::IRFloat(f)   [no inverse: cas-* prefers exact]
Number::Decimal(d)   →  cas_simplify::IRRational(d as ratio)
```

The bridge is one-way for Float and Decimal because the symbolic stack
has no Float/Decimal rung; converting back loses the symbolic identity
of the value.

### `r-vector` Bridge

`r-vector` atomic types `Logical`, `Integer`, `Double`, `Complex`,
`Character`, `Raw` hold elements at known rungs:

```
Vector<Logical>     elements are bool (or NA — see na-semantics)
Vector<Integer>     elements are i32 (R's integer is fixed 32-bit; the tower's
                    arbitrary-precision Integer is not the R atomic — see
                    r-vector.md for the rationale)
Vector<Double>      elements are Number::Float
Vector<Complex>     elements are Number::Complex
Vector<Character>   elements are String
Vector<Raw>         elements are u8
```

A `Vector<Double>` element is a `Number::Float` *value*, not a
`Number` *enum*. The vector knows its own rung; the variant tag is
not stored per element.

---

## NaN vs NA

This spec stays out of NA. See `na-semantics.md` for the contract.
The relationship in one line: **NaN belongs to the Float rung and
means an operation produced something undefined; NA is a per-vector
sentinel layered on top by `r-vector` and means a value was never
recorded.** The tower is unaware of NA.

---

## Out of Scope

- Display / formatting (cell-format concern; lives in spreadsheet-core)
- `pow`, `sqrt`, transcendentals (math-core)
- Date and time arithmetic (datetime-core; dates are not Numbers)
- Big-decimal arbitrary precision *with non-fixed scale* — folded into
  `Decimal { precision: u32 }`
- Posits / unum / fixed-point — out of scope; if added, a new rung
- Interval arithmetic — out of scope

---

## References

- IEEE 754-2008 (binary and decimal floating-point arithmetic)
- R Internals, §1.1 Vector types (atomic types and storage modes)
- The Java `BigDecimal` class (Decimal rung loosely follows)
- `cas-complex`, `cas-simplify` in this repo (existing exact-arithmetic IR)
