# float-math (C++)

**Elementary floating-point functions, from scratch** — header-only, ISO C++17,
**no libm**, in namespace `ca::float_math`. Part of the
[CCPP02](../../../specs/CCPP02-os-platform-lane.md) lane (bucket A).

## Why

The pure-ISO lane links nothing, so a port calling `<cmath>`'s libm-backed
`std::sqrt`/`std::exp` would fail to link. This header supplies those functions
computed from first principles (only `+ - * /`, comparisons, IEEE-754 bit tricks
via `std::memcpy`), so a math-using C++ port depends on **this** instead of libm.
Identical under GCC, Clang, and MSVC. Companion to `trig` (sin/cos/tan/atan).

## What it provides

`ca::float_math::` — `isnan`/`isinf`/`isfinite`/`inf`/`nan`; `fabs`, `copysign`,
`floor`, `ceil`, `trunc`, `round`, `fmod`, `ldexp`, `frexp`; `sqrt`, `cbrt`,
`hypot`; `exp`, `expm1`, `log`, `log2`, `log10`, `log_base`; `pow`; `sinh`,
`cosh`, `tanh`. High-precision `constexpr` constants (`PI`, `E`, `LN2`, …).

```cpp
#include "float_math.hpp"
namespace fm = ca::float_math;
double r = fm::hypot(3.0, 4.0);         // 5.0
double l = fm::log(fm::exp(1.234));     // 1.234
```

## Method

Argument reduction (two-part `ln2` split / exponent decomposition) → short
Taylor/atanh series or a few Newton steps → exact power-of-two reconstruction.
Accuracy: **solid double precision (~1 ULP)**.

## Building

```sh
sh BUILD          # POSIX: g++ and/or clang++, via the shared iso-harness
```

Each compiler prints `N checks, 0 failed`. Verified under ASan + UBSan; the
committed tests are pure-ISO (golden constants + oracle-free identity sweeps),
matching the C sibling that was cross-checked against libm to ~1 ULP.
