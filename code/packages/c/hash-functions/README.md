# hash-functions (C)

**Non-cryptographic hash functions**, implemented from scratch in pure ISO C17.
A faithful port of the Rust [`hash-functions`](../../rust/hash-functions) crate.

## What it does

A grab-bag of the classic fast table hashes, plus two quality-analysis helpers:

| function | width | notes |
|----------|-------|-------|
| `hf_fnv1a_32` / `hf_fnv1a_64` | 32 / 64 | FNV-1a: xor-then-multiply, tiny and fast |
| `hf_djb2`                     | 64      | Bernstein's `hash*33 + c` string hash |
| `hf_polynomial_rolling[_with_params]` | 64 | Rabin–Karp `Σ cᵢ·baseⁱ mod m` |
| `hf_murmur3_32[_with_seed]`   | 32      | Murmur3, good avalanche |
| `hf_siphash_2_4`              | 64      | keyed PRF, resists hash-flooding |

> These are **not** cryptographic. For collision resistance use the crypto
> digests this repo also ports ([sha256](../sha256), [sha1](../sha1),
> [md5](../md5), [hmac](../hmac)).

Every result is validated against the Rust crate's own known-answer vectors
(e.g. `fnv1a_32("") == 0x811C9DC5`, `siphash_2_4("", 0..15) == 0x726FDB47DD0E0E31`).

## The `u128` question

Rust's polynomial rolling computes `(hash*base + byte) % modulus` in `u128` to
avoid overflow. C has no portable 128-bit integer (`__int128` is a GNU
extension), so this port uses an exact, overflow-safe modular multiply
(`mulmod`, binary double-and-add over an overflow-safe `addmod`). Results match
the Rust crate bit-for-bit for **any** 64-bit modulus.

## API

```c
#include "hash_functions.h"

uint32_t h = hf_fnv1a_32((const uint8_t *)"hello", 5);   /* 1335831723 */
uint64_t s = hf_siphash_2_4(data, len, key16);

/* The trait, as a tagged value: */
HfHashFunction poly = hf_new_polynomial_rolling();       /* base 31, mod 2^61-1 */
uint64_t p = hf_hash(&poly, data, len);
uint32_t bits = hf_output_bits(&poly);                   /* 64 */
```

- Free functions per algorithm (`hf_fnv1a_32/64`, `hf_djb2`,
  `hf_polynomial_rolling[_with_params]`, `hf_murmur3_32[_with_seed]`,
  `hf_siphash_2_4`) plus string helpers (`hf_hash_str_fnv1a_32`,
  `hf_hash_str_siphash`).
- `HfHashFunction` + `hf_new_*` constructors + `hf_hash` / `hf_output_bits` — the
  Rust `HashFunction` trait folded into one tagged struct.
- Analysis: `hf_avalanche_score` (bit-flip sensitivity; the caller supplies the
  byte source, since Rust's `getrandom` is OS entropy with no pure-ISO
  equivalent) and `hf_distribution_test` (chi-square uniformity).

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.
