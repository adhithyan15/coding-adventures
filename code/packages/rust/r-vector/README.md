# `r-vector`

The NA-aware vector substrate (Layer 0) for the Rust statistics / spreadsheet
stack.

R has a feature that most languages get wrong: a vector element can be *missing*,
and missing is not the same as wrong. `NA` is "we do not know this value";
`NaN` is "this computation had no answer". `mean(c(1, NA, 3))` should be able to
tell you it hit a hole in the data, while `mean(c(1, 0/0, 3))` is reporting a
broken arithmetic result. Collapse the two and you can no longer distinguish a
survey question nobody answered from a division by zero.

This crate keeps them distinct, and hands `statistics-core` and the rest of the
function catalog a vector type that carries that distinction correctly.

---

## How `NA` is represented

`NA_real_` is a **specific NaN payload**, not a separate tag byte:

```rust
pub const NA_REAL_BITS: u64 = 0x7ff0_0000_0000_07a2;
```

This is the trick R itself uses. Every IEEE-754 quiet NaN has 51 free payload
bits, so one particular bit pattern can be reserved to mean "missing" while every
*other* NaN keeps meaning "not a number". The consequences are worth being
explicit about:

- A `Double` vector is a plain `Vec<f64>` — no parallel mask, no `Option<f64>`
  per element, no extra allocation. Missingness rides along inside the value.
- `is_na_real(x)` compares `x.to_bits()` against the sentinel. It cannot use
  `==`, because NaN is not equal to itself.
- `is_nan_real(x)` is deliberately `x.is_nan() && !is_na_real(x)` — "a real NaN,
  and not our sentinel." That asymmetry is the whole point of the pair.

Hardware caveat inherited from R: an arithmetic operation on the NA payload is
not *guaranteed* by IEEE-754 to propagate that exact payload, so code that must
preserve NA across arithmetic checks for it rather than assuming it survives.

## What is in the box

| Item | Role |
|------|------|
| `Vector` | The small generic contract every vector kind implements (length, names) |
| `Double` | NA-aware `f64` vector — the workhorse |
| `Character` | String vector, where the missing/blank distinction is `Option<String>` |
| `Names` | Optional per-element names, length-checked against the vector at construction |
| `na_real` / `is_na_real` / `is_nan_real` | The sentinel and its two predicates |

`Double::to_number_at` is the bridge up to `numeric-tower`: it hands back a
`Number` for a present value and `None` for `NA`, so the exact-arithmetic layer
never has to know about the NaN-payload trick.

## Where it sits

```
numeric-tower   (the Number ladder)
      │
  r-vector      (this crate — NA-aware vectors)
      │
   ┌──┴────────┬─────────────┬──────────────┐
statistics-  math-core   datetime-core   financial-core  …
   core
   │
spreadsheet-core / s-runtime
```

Twelve crates depend on this one — `array-core`, `data-frame`, `database-core`,
`datetime-core`, `engineering-core`, `financial-core`, `lookup-core`,
`math-core`, `s-runtime`, `spreadsheet-core`, `statistics-core` and `text-core`.

## Usage

```rust
use r_vector::{is_na_real, is_nan_real, na_real, Double};

let v = Double::from_values(vec![1.0, na_real(), 3.0]);

assert!(is_na_real(v.data()[1]));          // missing
assert!(!is_nan_real(v.data()[1]));        // ... but NOT "not a number"
assert!(is_nan_real(f64::NAN));            // a genuine NaN is the other case
assert_eq!(v.to_number_at(1), None);       // NA has no exact Number
```

## Testing

```sh
cargo test -p r-vector -- --nocapture
```

## See also

- `numeric-tower` — the exact-arithmetic ladder below.
- `statistics-core` — the first consumer, and the reason the NA/NaN split exists.
