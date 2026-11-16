# trig (C)

## Converged on TRIG-REF-1 (0.2.0)

`trig_sin`, `trig_cos`, `trig_radians` and `trig_degrees` follow the reference
algorithm of [PHY00a](../../../specs/PHY00a-trig-convergence.md) **bit for bit**, and
`trig_tan`, `trig_sqrt`, `trig_atan` and `trig_atan2` are pinned to the same vectors. All
62,154 rows of
[`trig-reference-v1`](../../../specs/fixtures/trig-reference-v1/README.md) are checked by
`tests/conformance_test.c` (it decodes each hex bit pattern by a bit cast and compares result
bits as integers; a NaN result matches any NaN).

* **Reduction.** `t = x * (2/pi)`, `k = (t + MAGIC) - MAGIC` rounds to the nearest integer
  with one add and one subtract (`MAGIC = 1.5 * 2^52`), then five Cody-Waite subtractions
  `r = r - k*PIO2_i` remove `k` quarter-turns. The first four pieces of pi/2 have at most
  24 bits, so their products with `|k| < 2^29` are exact. No `floor`, `fmod` or integer
  cast of an unbounded value is used. The quadrant is `(int)k & 3`.
* **Kernels.** Ten-term Horner polynomials on the stored coefficients `1/3!, 1/5!, ...`
  (sin) and `1/4!, 1/6!, ...` (cos); no division and no per-term recurrence.
* **Accuracy.** Within **3 ulp** of the correctly rounded value on `|x| <= 2^29` (worst
  observed 2 steps against the `cr` column; 2.38 ulp against the exact value). `radians` and
  `degrees` are one multiply by a pre-divided constant, within 1 ulp. `tan` makes no accuracy
  claim.
* **Domain.** `trig_sin`, `trig_cos` and `trig_tan` return **NaN** for NaN, the infinities
  and `|x| > 2^29` (about 5.4e8 radians). Reduce long-running phases yourself (in cycles)
  before calling.
* **Edges.** `trig_sin(-0.0)` is `-0.0`. `|x| < 2^-27` returns `x` (sin) or `1` (cos)
  unchanged. `trig_atan2` returns NaN if either argument is NaN. `trig_tan` keeps its
  pinned `+-1e308` pole sentinel; the `atan2` branch conventions
  (`atan2(-0, -1) = +pi`, `atan2(0, 0) = 0`) are pinned, not IEEE's.
* **No fused multiply-add.** Fusing `a*b + c` changes the result bits, so the source
  switches contraction off per compiler (`#pragma STDC FP_CONTRACT OFF` on clang,
  `#pragma GCC optimize("fp-contract=off")` on GCC, `#pragma fp_contract(off)` on MSVC),
  and refuses to compile under `-ffast-math` or when `FLT_EVAL_METHOD != 0` (32-bit x87).
  Do not build it with clang's `-ffp-contract=fast`; the command-line flag overrides the
  pragma, and the vectors then fail. Supported targets are x86-64 (SSE2) and arm64.

## Exact small arctangent

`trig_atan` returns `x` unchanged when `|x| <= 2^-27`, before any half-angle
reduction. This exact binary64 identity preserves negative zero and both signs
of the minimum subnormal; the shared PHY00 fixture locks those boundaries.

## Full-range square root

The square-root API follows PHY00 across the full binary64 range. It preserves
negative zero, returns positive infinity and NaN unchanged, rejects negative
inputs with the lane-native error, normalizes by powers of four into `[0.25, 4)`,
and then applies at most 60 Newton steps without calling the host square-root
routine. Boundary behavior is checked against
[`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json).

Trigonometric functions **from first principles**, in pure ISO C17 — a faithful
port of the Rust `trig` crate. No `<math.h>`, no libm: every value is computed
from `+ - * /` and comparisons.

## Why

Understanding *how* a sine is calculated is more valuable than calling someone
else's black box. This library builds `sin`, `cos`, `tan`, `sqrt`, `atan`, and
`atan2` from scratch:

| Function | Method |
|----------|--------|
| `sin` / `cos` | Cody-Waite reduction to `[-pi/4, pi/4]`, then ten-term Horner Maclaurin kernels (TRIG-REF-1) |
| `sqrt` | Newton's (Babylonian) method — quadratic convergence |
| `tan` | `sin(x) / cos(x)`, saturating near the `cos = 0` poles |
| `atan` / `atan2` | Taylor series with two layers of range reduction |
| `radians` / `degrees` | one multiply by the pre-divided constants `pi/180` and `180/pi` |

Range reduction is likewise implemented without libm, so the package has zero
external dependencies.

## API

```c
#include "trig.h"

double s = trig_sin(TRIG_PI / 6.0);   /* 0.5  */
double c = trig_cos(0.0);             /* 1.0  */
double t = trig_tan(TRIG_PI / 4.0);   /* 1.0  */
double a = trig_atan2(1.0, 1.0);      /* pi/4 */
double r = trig_radians(180.0);       /* pi   */

double root;
if (trig_sqrt(2.0, &root) == TRIG_OK) { /* root ~ 1.41421356 */ }
```

## Divergence from the Rust crate

Rust's `sqrt` **panics** on a negative input. This port returns a status code
instead: `trig_sqrt` writes the root to an out-parameter and returns `TRIG_OK`,
or returns `TRIG_ERR_DOMAIN` (leaving `*out` untouched) for `x < 0`. Every other
function is total and returns a `double` directly, exactly like the Rust crate.

## Building

```sh
sh BUILD    # builds & runs the unit tests and the TRIG-REF-1 conformance test
            # under every C compiler present
```

Pure ISO C17, no `<math.h>`. Builds clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Where it fits

Part of the C/C++ port campaign mirroring the Rust learning packages. A
companion to the numeric packages — where those do exact arithmetic, this shows
how the transcendental functions are approximated.
