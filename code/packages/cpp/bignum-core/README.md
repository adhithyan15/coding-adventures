# bignum-core (C++)

Arbitrary-precision signed integers (**BigInteger**) in pure ISO C++17,
header-only, in namespace `ca`. A faithful port of the `BigInteger` core of the
Rust `bignum-core` crate.

A `BigInteger` is sign-magnitude: a sign (−1 / 0 / +1) plus a magnitude stored as
little-endian base-2³² limbs with no trailing zero limb. All arithmetic uses
**32-bit limbs and a 64-bit accumulator** — no 128-bit integers.

- Add / subtract — column carry/borrow. Multiply — schoolbook `O(n·m)`.
- Divide — **Knuth's Algorithm D**, truncating toward zero; the remainder takes
  the dividend's sign (like C++ `/` `%`).
- `pow` (exponentiation by squaring) with a `try_pow` size guard, Euclid `gcd`,
  radix 2–36 parse / format.

## API

Value type with operator overloads; errors are exceptions.

```cpp
#include "bignum_core.hpp"
using ca::BigInteger;

BigInteger a = BigInteger::parse_radix("123456789012345678901234567890", 10);
BigInteger b = BigInteger::from_u64(1000);

BigInteger prod = a * b;
auto [q, r] = a.div_rem(b);         // truncating quotient + remainder
std::string s = prod.to_string();   // base-10
std::string hex = a.to_str_radix(16);
```

Operators `+ - * / % - == != < > <= >=` are provided. Division by zero throws
`std::domain_error`; `parse_radix` throws `ca::ParseBigIntError` (with a `kind`
and offending char); `try_pow` throws `ca::PowTooLargeError` when the projected
result exceeds the ceiling.

## BigDecimal — exact base-10 (`bignum_decimal.hpp`)

The `decimal` rung, built on `BigInteger`. A **`ca::BigDecimal`** is
`mantissa × 10^(-scale)` in canonical form, so `==` and `<` compare by value.
`+ - *` and `pow` are **exact**; division rounds to a stated number of places
under a `ca::RoundingMode` (`HalfEven`, `HalfUp`, `Floor`, …). Value semantics
throughout.

```cpp
#include "bignum_decimal.hpp"
using ca::BigDecimal;
using ca::RoundingMode;

BigDecimal sum = BigDecimal::parse("0.1") + BigDecimal::parse("0.2");
sum.to_string();                                 // "0.3", exactly

BigDecimal third = BigDecimal::parse("10")
    .div_round(BigDecimal::parse("3"), 4, RoundingMode::HalfEven); // "3.3333"
double d = third.to_f64();                       // labelled lossy exit
```

Every fallible operation offers **both** a throwing form (`parse` →
`ca::ParseDecimalError`, `div_round`/`from_parts` → `std::domain_error` /
`std::out_of_range`) and a non-throwing `try_parse` / `checked_div_round` /
`checked_from_parts` returning `std::optional`. `parse` enforces a strict
`MAX_SCALE` (10^6) budget on untrusted input so a tiny string cannot amplify into
a multi-gigabyte power of ten. `to_f64` goes through `std::strtod`, so no
`<cmath>`/libm is needed. This ports the integer core plus its decimal rung; the
crate's float rung builds on the same base.

## Portability

Pure ISO C++17 — compiles clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness). Standard library only.

## Development

```bash
# Compile and run the (Python-oracle-checked) tests under every C++ compiler.
sh BUILD
```
