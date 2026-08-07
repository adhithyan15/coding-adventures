# trig (C)

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
| `sin` / `cos` | 20-term Maclaurin series after reducing the angle into `[-pi, pi]` |
| `sqrt` | Newton's (Babylonian) method — quadratic convergence |
| `tan` | `sin(x) / cos(x)`, saturating near the `cos = 0` poles |
| `atan` / `atan2` | Taylor series with two layers of range reduction |
| `radians` / `degrees` | the linear `*pi/180` and `*180/pi` conversions |

Range reduction (`x mod 2pi`, truncation) is likewise implemented without libm,
so the package has zero external dependencies.

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
sh BUILD    # builds & runs the tests under every C compiler present
```

Pure ISO C17, no `<math.h>`. Builds clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Where it fits

Part of the C/C++ port campaign mirroring the Rust learning packages. A
companion to the numeric packages — where those do exact arithmetic, this shows
how the transcendental functions are approximated.
