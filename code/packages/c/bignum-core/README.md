# bignum-core (C)

Arbitrary-precision signed integers (**BigInteger**) in pure ISO C17. A faithful
port of the `BigInteger` core of the Rust `bignum-core` crate.

A `BigInteger` is sign-magnitude: a sign (−1 / 0 / +1) plus a magnitude stored as
little-endian base-2³² limbs with no trailing zero limb (zero is the empty
magnitude — never a "−0"). All arithmetic uses **32-bit limbs and a 64-bit
accumulator**, so the code needs no 128-bit integers.

- Add / subtract — column carry/borrow methods.
- Multiply — schoolbook `O(n·m)`.
- Divide — **Knuth's Algorithm D** (TAOCP §4.3.1), long division in base 2³².
  Truncates toward zero; the remainder takes the dividend's sign (like C `/` `%`).
- Also: `pow` (exponentiation by squaring) with a `try_pow` size guard, Euclid
  `gcd`, and radix 2–36 parse / format.

## API

Every operation returns a **new heap `BigInteger *`** the caller frees with
`bigint_free` (NULL on allocation failure). Fallible operations return a
`BigIntStatus`.

```c
#include "bignum_core.h"

BigInteger *a = NULL, *b = NULL, *q = NULL, *r = NULL;
bigint_parse_radix("123456789012345678901234567890", 10, &a, NULL);
b = bigint_from_u64(1000);

BigInteger *prod = bigint_mul(a, b);        /* a * b */
bigint_div_rem(a, b, &q, &r);               /* truncating quotient + remainder */

char *s = bigint_to_string(prod);           /* base-10, malloc'd */
free(s);
bigint_free(a); bigint_free(b); bigint_free(q); bigint_free(r); bigint_free(prod);
```

`bigint_try_pow` refuses an oversized result up front (returns
`BIGINT_POW_TOO_LARGE` without allocating) so a hostile exponent can't OOM the
process.

## BigDecimal — exact base-10 (`bignum_decimal.h`)

The `decimal` rung, built on `BigInteger`. A **BigDecimal** is
`mantissa × 10^(-scale)` in canonical form (the mantissa never ends in a `0`
digit; zero is `(0, 0)`), so equal values print and compare identically. `+ − ×`
and `^` are **exact**; division rounds to a stated number of places under one of
seven `DecRoundingMode`s (`HALF_EVEN`, `HALF_UP`, `FLOOR`, …).

```c
#include "bignum_decimal.h"

BigDecimal *a = NULL, *b = NULL, *sum = NULL;
dec_parse("0.1", &a);
dec_parse("0.2", &b);
dec_add(a, b, &sum);              /* exact: no 0.30000000000000004 */
char *s = dec_to_string(sum);    /* "0.3" */

BigDecimal *third = NULL;
dec_div_round(dec_one(), b, 4, DEC_ROUND_HALF_EVEN, &third); /* 1/0.2 → "5" */

free(s);
dec_free(a); dec_free(b); dec_free(sum); dec_free(third);
```

`dec_parse` enforces a strict `DEC_MAX_SCALE` (10^6) budget on untrusted input,
so a few-byte string like `"1e-2000000000"` cannot force a multi-gigabyte
power-of-ten materialization in a later `+`/`cmp`/`to_string`. Where the Rust
`from_parts`/`div_round` panic (scale past the internal ceiling; divide by zero),
this port returns a `DecStatus` instead — a library must not abort its host. The
lossy `dec_to_f64` goes through the value's decimal string and `strtod`, so no
`<math.h>` is needed.

## Portability

Pure ISO C17 — compiles clean under GCC, Clang, and MSVC with `-pedantic-errors`
/ `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Development

```bash
# Compile and run the (Python-oracle-checked) tests under every C compiler.
sh BUILD
```
