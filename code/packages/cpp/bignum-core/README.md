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
result exceeds the ceiling. This ports the integer core; the crate's decimal /
float rungs build on it.

## Portability

Pure ISO C++17 — compiles clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness). Standard library only.

## Development

```bash
# Compile and run the (Python-oracle-checked) tests under every C++ compiler.
sh BUILD
```
