# polynomial (C)

A pure ISO **C17** library for polynomial arithmetic over the reals — add,
subtract, multiply, long division, evaluation, and GCD. A faithful port of the
Rust `polynomial` crate.

It compiles clean under **GCC, Clang, and MSVC** with
`-std=c17 -pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive-
/W4 /WX` on MSVC), via the shared [`iso-harness`](../iso-harness/). No compiler
extensions, no third-party dependencies — **not even libm**.

## Representation

A polynomial is a **little-endian** array of `double` coefficients: `p[i]` is the
coefficient of `xⁱ`, so `[1, 2, 3]` means `1 + 2x + 3x²`. The zero polynomial is
the empty array (length 0). Trailing near-zero coefficients are stripped by
`normalize` (a coefficient with magnitude ≤ `DBL_EPSILON·1e6` counts as zero).

## API

```c
#include "polynomial.h"

double a[] = {1, 2, 3};   /* 1 + 2x + 3x^2 */
double b[] = {2, 1};      /* 2 + x */
double out[8];

size_t n = poly_multiply(a, 3, b, 2, out);      /* (1+2x+3x^2)(2+x) */
double v = poly_evaluate(a, 3, 2.0);            /* 17 */

double q[8], r[8]; size_t ql, rl;
poly_divmod(a, 3, b, 2, q, &ql, r, &rl);        /* a = b*q + r */
```

| Function | Output-buffer capacity |
| --- | --- |
| `poly_normalize` | `n` |
| `poly_add` / `poly_subtract` | `max(na, nb)` |
| `poly_multiply` | `na + nb - 1` |
| `poly_divmod` / `poly_divide` / `poly_modulo` | `nd` (dividend length) each |
| `poly_gcd` | `max(na, nb)` |
| `poly_degree` / `poly_evaluate` | — |

Each op returns the length of the normalized result. `poly_divmod` returns 0 on
a zero divisor (the crate panics).

## Implementation notes

- **No libm.** The only non-arithmetic operation is an absolute value, done
  manually (`x < 0 ? -x : x`); the zero threshold uses `DBL_EPSILON` from
  `<float.h>`. So no `-lm` linkage is needed.
- **Caller buffers.** Everything writes into caller-provided storage; only
  `poly_gcd` allocates internal scratch (and frees it), guarded against `size_t`
  overflow.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests use integer coefficients (so results are exact `double` values) and check
the crate's identities: `(1+2x)(3+4x) = 3+10x+8x²`, the long-division worked
example with reconstruction (`divisor·q + r = dividend`), Horner evaluation, and
that `gcd(x²−1, x−1)` is a scalar multiple of `x−1`.
