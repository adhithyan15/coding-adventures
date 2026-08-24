# `numeric-tower`

The shared `Number` type at the bottom of the spreadsheet / R statistics stack.

Spreadsheets and R both have to answer an awkward question: *what is a number?*
`1`, `1/3`, `1.5`, `0.10` (exactly ten cents, not `0.1000000000000000055…`), and
`3 + 4i` are all "numbers" a user can type into a cell, and they do not obey the
same arithmetic. Pick `f64` for all of them and `0.1 + 0.2 == 0.3` is false; pick
rationals for all of them and `sqrt(2)` has nowhere to live.

The classic answer — borrowed from Scheme's numeric tower — is a *ladder* of
representations with automatic promotion. That is what this crate is: the five
rungs from `code/specs/numeric-tower.md`, plus the coercion and arithmetic entry
points every domain crate above shares.

---

## The five rungs

| `Rung` | `Number` variant | Backing type | Exact? | What it is for |
|--------|------------------|--------------|--------|----------------|
| `Integer` | `Number::Integer` | `BigInt` | yes | Counts, indices, whole-number cell values — unbounded, so `2^200` is not an overflow |
| `Rational` | `Number::Rational` | `BigRational` | yes | `1/3` stays `1/3` instead of decaying to `0.333…` |
| `Decimal` | `Number::Decimal` | `Decimal` (`BigInt` units + base-10 `scale`) | yes | Money. `0.10` is ten cents, and `0.1 + 0.2` is exactly `0.3` |
| `Float` | `Number::Float` | `f64` | no | Transcendentals — `sqrt`, `ln`, `sin`. Where exactness has to be given up |
| `Complex` | `Number::Complex` | `Complex { re, im }` | no | The Excel `IMSUM`/`IMPRODUCT`/`IM*` family |

## Promotion

Every binary operation first calls `coerce_to_join`, which lifts both operands to
the *same* rung — the one `join_rung` picks:

```
Complex  >  Float  >  Decimal  >  Rational  >  Integer
   (most general)                      (most exact)
```

The rule is "promote to the more general rung, never demote," because only that
direction cannot lose information: an `Integer` is always representable as a
`Float`, but rounding a `Float` back to an `Integer` throws digits away. So
`Integer + Rational → Rational` and `Rational + Float → Float`.

Demotion is still available, but it is *fallible* and explicit —
`try_coerce(&n, target)` returns `Err(CoercionError::…)` naming exactly why the
value would not survive the trip (`NonIntegralRational`, `FractionalFloat`,
`ComplexToReal`, `NonIntegralDecimal`). Callers that genuinely want the lossy
version ask for it by name with `to_f64_lossy`.

## Where it sits

```
                     numeric-tower  (this crate — the Number type)
                            │
                       r-vector      (NA-aware vectors)
                            │
   ┌──────────┬─────────────┼──────────────┬──────────────┐
   │          │             │              │              │
statistics-  math-core   datetime-core  financial-   engineering-
   core                                    core          core
   │          │             │              │              │
   └──────────┴──────┬──────┴──────────────┴──────────────┘
                     │
             spreadsheet-core / s-runtime   (the frontends)
```

Thirteen crates depend on this one — `array-core`, `data-frame`,
`database-core`, `datetime-core`, `engineering-core`, `financial-core`,
`lookup-core`, `math-core`, `r-vector`, `s-runtime`, `spreadsheet-core`,
`statistics-core` and `text-core` — which is precisely why it is worth keeping
green: a change here is felt by the entire function catalog.

## Usage

```rust
use num_bigint::BigInt;
use num_rational::BigRational;
use numeric_tower::{add, try_coerce, Number, Rung};

let third = Number::Rational(BigRational::new(BigInt::from(1), BigInt::from(3)));

// Same rung on both sides: exactness is preserved. 1/3 + 1/3 = 2/3,
// not 0.6666666666666666.
let exact = add(&third, &third);

// Mixing rungs promotes to the more general one, here Float.
let approx = add(&third, &Number::Float(0.5));

// Demotion is fallible and says why: 2/3 is not an integer.
assert!(try_coerce(&exact, Rung::Integer).is_err());
```

## Testing

```sh
cargo test -p numeric-tower -- --nocapture
```

## See also

- `code/specs/numeric-tower.md` — the specification these five rungs implement.
- `r-vector` — the NA-aware vector layer directly above.
