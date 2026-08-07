# hash-functions (C++)

**Non-cryptographic hash functions**, header-only, pure ISO C++17. A faithful
port of the Rust [`hash-functions`](../../rust/hash-functions) crate, in
namespace `ca::hash_functions`.

## What it does

The classic fast table hashes plus two quality-analysis helpers:

| function | width | notes |
|----------|-------|-------|
| `fnv1a_32` / `fnv1a_64` | 32 / 64 | FNV-1a: xor-then-multiply |
| `djb2`                  | 64      | Bernstein's `hash*33 + c` |
| `polynomial_rolling[_with_params]` | 64 | Rabin–Karp `Σ cᵢ·baseⁱ mod m` |
| `murmur3_32[_with_seed]` | 32     | Murmur3, good avalanche |
| `siphash_2_4`           | 64      | keyed PRF, resists hash-flooding |

> These are **not** cryptographic. For collision resistance use the crypto
> digests this repo also ports ([sha256](../sha256), [sha1](../sha1),
> [md5](../md5), [hmac](../hmac)).

Validated against the Rust crate's own known-answer vectors.

## Design

- The Rust `HashFunction` trait becomes an abstract base `HashFunction` with one
  concrete `final` struct per algorithm (`Fnv1a32`, `Fnv1a64`, `Djb2`,
  `PolynomialRolling{base,modulus}`, `Murmur3_32{seed}`, `SipHash24{key}`), each
  with `hash()` and `output_bits()` — usable polymorphically through a base
  reference (the trait-object analog).
- Rust's `u128` in polynomial rolling is replaced by an exact, overflow-safe
  `mulmod` (no 128-bit type), matching results for any 64-bit modulus.
- The analysis helpers are function templates generic over the hash callable,
  mirroring the Rust generics. `avalanche_score` takes a caller-supplied fill
  callable in place of Rust's `getrandom` (OS entropy — no pure-ISO equivalent).

## API

```cpp
#include "hash_functions.hpp"
using namespace ca::hash_functions;

std::uint32_t h = fnv1a_32(data, len);
std::uint32_t hs = hash_str_fnv1a_32("hello");           // 1335831723

PolynomialRolling poly;                                  // base 31, mod 2^61-1
std::uint64_t p = poly.hash(data, len);
std::uint32_t bits = poly.output_bits();                 // 64

std::array<std::uint8_t, 16> key{};
std::uint64_t s = siphash_2_4(data, len, key);

double chi2 = distribution_test(
    [](const std::uint8_t* d, std::size_t n){ return fnv1a_64(d, n); },
    std::vector<std::string_view>{"a", "b", "c"}, 8);
```

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.
