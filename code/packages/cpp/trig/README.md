# trig (C++)

## Full-range square root

The square-root API follows PHY00 across the full binary64 range. It preserves
negative zero, returns positive infinity and NaN unchanged, rejects negative
inputs with the lane-native error, normalizes by powers of four into `[0.25, 4)`,
and then applies at most 60 Newton steps without calling the host square-root
routine. Boundary behavior is checked against
[`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json).

Trigonometric functions **from first principles**, header-only in pure ISO
C++17 (namespace `ca::trig`) — a faithful port of the Rust `trig` crate. No
`<cmath>`, no libm: every value is computed from `+ - * /` and comparisons.

## Why

Understanding *how* a sine is calculated is more valuable than calling someone
else's black box. This library builds the functions from scratch:

| Function | Method |
|----------|--------|
| `sin` / `cos` | 20-term Maclaurin series after reducing the angle into `[-pi, pi]` |
| `sqrt` | Newton's (Babylonian) method — quadratic convergence |
| `tan` | `sin(x) / cos(x)`, saturating near the `cos = 0` poles |
| `atan` / `atan2` | Taylor series with two layers of range reduction |
| `radians` / `degrees` | the linear `*pi/180` and `*180/pi` conversions |

## Usage

```cpp
#include "trig.hpp"
namespace t = ca::trig;

double s = t::sin(t::PI / 6.0);   // 0.5
double c = t::cos(0.0);           // 1.0
double q = t::tan(t::PI / 4.0);   // 1.0
double a = t::atan2(1.0, 1.0);    // pi/4
double r = t::radians(180.0);     // pi
double root = t::sqrt(2.0);       // ~1.41421356  (throws std::domain_error on x < 0)
```

## Divergence from the Rust crate

Rust's `sqrt` **panics** on a negative input; this port throws
`std::domain_error`, the idiomatic C++ equivalent. Every other function is total
and returns a `double`, exactly like the Rust crate.

## Building

```sh
sh BUILD    # builds & runs the tests under every C++ compiler present
```

Pure ISO C++17, no `<cmath>`. Builds clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness).

## Where it fits

Part of the C/C++ port campaign mirroring the Rust learning packages — a
header-only companion to the [C `trig`](../../c/trig) library.
