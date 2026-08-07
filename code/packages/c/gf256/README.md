# gf256 (C)

A pure ISO **C17** implementation of **Galois Field GF(2⁸)** arithmetic — the
finite field of 256 elements behind Reed-Solomon codes, QR codes, and AES. A
faithful port of the Rust `gf256` crate.

It compiles clean under **GCC, Clang, and MSVC** with
`-std=c17 -pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive-
/W4 /WX` on MSVC), via the shared [`iso-harness`](../iso-harness/). No compiler
extensions, no third-party dependencies.

## The field in one paragraph

Each byte is a polynomial over GF(2) of degree ≤ 7. **Addition is XOR** (in
characteristic 2, `1 + 1 = 0`, so subtraction is the same operation).
**Multiplication** multiplies the polynomials and reduces the result modulo an
irreducible degree-8 polynomial — `0x11D` for Reed-Solomon, `0x11B` for AES.

## Two interfaces

```c
#include "gf256.h"

/* Module-level: the default Reed-Solomon field (0x11D), via log/antilog tables. */
gf256_multiply(3, 7);          /* 9  = (x+1)(x^2+x+1) = x^3+1 */
gf256_inverse(a);              /* a * inverse(a) == 1 */
gf256_divide(9, 3);            /* 7 */

/* A field parameterised by any polynomial — e.g. AES (0x11B): */
gf256_field aes = gf256_field_new(0x11B);
gf256_field_multiply(&aes, 0x57, 0x83);   /* 0xC1 (FIPS-197 example) */
gf256_field_inverse(&aes, 0x53);          /* 0xCA */
```

| Default field (0x11D) | Parameterised field |
| --- | --- |
| `gf256_add` / `gf256_subtract` | `gf256_field_add` / `_subtract` |
| `gf256_multiply` / `gf256_divide` | `gf256_field_multiply` / `_divide` |
| `gf256_power` / `gf256_inverse` | `gf256_field_power` / `_inverse` |

## Implementation notes

- **Log/antilog tables** for the default field turn multiplication into two
  lookups and an add (`a·b = ALOG[(LOG[a] + LOG[b]) mod 255]`). They are built
  once, lazily, on first use.
- **Russian-peasant (shift-and-XOR) multiplication** for the parameterised
  field — table-free and correct for any irreducible polynomial (the log-table
  trick needs `g = 2` to be a primitive element, which fails for AES's 0x11B).
- Division by zero and the inverse of zero return `0` (the Rust crate panics).
- The lazy table build is single-threaded (the crate uses `OnceLock`; pure ISO C
  has no portable one-time-init primitive) — noted in the header.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests pin known values in both fields (RS `multiply(3,7)=9`; AES
`multiply(0x57,0x83)=0xC1`, `inverse(0x53)=0xCA`) and verify the algebraic laws
— `a·inverse(a)=1` and `divide(multiply(a,b),b)=a` over full sweeps.

## Where it fits

A foundational primitive for the `code/packages/c` crypto/coding set: a future
`aes` port uses `gf256_field(0x11B)` for its S-box and MixColumns, and
Reed-Solomon codes use the default 0x11D field.
