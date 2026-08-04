# float-math (C)

**Elementary floating-point functions, from scratch** — pure ISO C17, **no
libm**. Part of the [CCPP02](../../../specs/CCPP02-os-platform-lane.md) lane
(bucket A: computable from scratch, no OS, no external libraries).

## Why

The pure-ISO lane links nothing — so a port that called libm's `sqrt`/`exp`/`log`
would fail to link. This library supplies those functions computed from first
principles (only `+ - * /`, comparisons, and IEEE-754 bit tricks via `memcpy`),
so a math-using port depends on **this** instead of libm. It compiles
identically under GCC, Clang, and MSVC. Companion to the `trig` crate (which
covers sin/cos/tan/atan); this one covers roots, exponentials, logarithms,
powers, and hyperbolics.

## What it provides

- **classification** — `fm_isnan`/`isinf`/`isfinite`, `fm_inf`/`nan`;
- **sign / rounding / remainder** — `fabs`, `copysign`, `floor`, `ceil`,
  `trunc`, `round`, `fmod`, `ldexp`, `frexp` (all exact);
- **roots** — `sqrt`, `cbrt`, `hypot` (overflow-safe);
- **exp / log** — `exp`, `expm1`, `log`, `log2`, `log10`, `log_base`;
- **power** — `pow` (exact integer-exponent path; `exp(y·ln x)` otherwise);
- **hyperbolics** — `sinh`, `cosh`, `tanh`.

```c
#include "float_math.h"
double r = fm_hypot(3.0, 4.0);          /* 5.0 */
double p = fm_pow(2.0, 0.5);            /* sqrt 2, ~1.41421356 */
double l = fm_log(fm_exp(1.234));       /* 1.234 */
```

## Method

Each function reduces its argument into a small range (via a two-part `ln2`
split, or an exponent/mantissa decomposition), approximates there with a short
Taylor/atanh series or a few Newton steps, then reconstructs with an exact power
of two. Accuracy is **solid double precision (~1 ULP)**.

## Building

```sh
sh BUILD          # POSIX: gcc and/or clang, via the shared iso-harness
```

Each compiler prints `N checks, 0 failed`. Verified under ASan + UBSan. The
committed tests are pure-ISO (golden constants + oracle-free identity sweeps:
`exp(log x) == x`, `cosh² − sinh² == 1`, …); accuracy was separately
cross-checked against the platform libm over tens of millions of random inputs
(~1 ULP), an oracle kept local since this lane forbids libm.
