# trig (C++)

## Exact small arctangent

`atan` returns `x` unchanged when `|x| <= 2^-27`, before any half-angle
reduction. This exact binary64 identity preserves negative zero and both signs
of the minimum subnormal; the shared PHY00 fixture locks those boundaries.

## Full-range square root

The square-root API follows PHY00 across the full binary64 range. It preserves
negative zero, returns positive infinity and NaN unchanged, rejects negative
inputs with the lane-native error, normalizes by powers of four into `[0.25, 4)`,
and then applies at most 60 Newton steps without calling the host square-root
routine. Boundary behavior is checked against
[`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json).

Trigonometric functions **from first principles**, header-only in pure ISO
C++17 (namespace `ca::trig`). No `<cmath>`, no libm: `sin`, `cos`, `radians` and
`degrees` use only `+ - *` and comparisons; `tan`, `sqrt`, `atan` and `atan2`
add division.

The package follows the shared reference algorithm **TRIG-REF-1**
([PHY00a](../../../specs/PHY00a-trig-convergence.md)): every result matches the
bit-exact vectors in
[`trig-reference-v1`](../../../specs/fixtures/trig-reference-v1/README.md) bit for
bit, exactly as the Python, Rust and TypeScript ports do.

## Why

Understanding *how* a sine is calculated is more valuable than calling someone
else's black box. This library builds the functions from scratch:

| Function | Method |
|----------|--------|
| `sin` / `cos` | nearest-integer Cody-Waite reduction to `[-pi/4, pi/4]` (pi/2 stored as five pieces), then a ten-term Horner Maclaurin kernel on stored coefficients, quadrant picked by `k mod 4` |
| `sqrt` | Newton's (Babylonian) method — quadratic convergence |
| `tan` | `sin(x) / cos(x)`, saturating to `+-1e308` near the `cos = 0` poles (a pinned legacy convention) |
| `atan` / `atan2` | Taylor series with two layers of range reduction; `atan2` of a NaN is NaN |
| `radians` / `degrees` | one multiply by a bit-pinned constant (`pi/180`, `180/pi`) |

### Domain and accuracy

- `sin`, `cos`, `tan` are defined for `|x| <= 2^29` (about 5.4e8 radians). **NaN,
  both infinities and any larger argument return NaN**: a finite answer there
  would look right and be wrong.
- Within the domain `sin` and `cos` are within **3 ulp** of the exact value (2.38
  observed); `radians` and `degrees` within 1 ulp. `sin(-0.0)` is `-0.0`, and
  `|x| < 2^-27` returns `x` (and `cos` returns `1`).
- `tan`, `sqrt`, `atan` and `atan2` are pinned to the bit but make no accuracy
  claim (PHY00a 3.8, 3.9).

### No fused multiply-add

A header is compiled with the *consumer's* flags, so `trig.hpp` protects itself:
each compiler gets its own guard behind its own `#if` (`#pragma GCC optimize
("fp-contract=off")` on GCC, `_Pragma("STDC FP_CONTRACT OFF")` in each function on
clang, `#pragma fp_contract(off)` on MSVC), and `-ffast-math`, `-ffinite-math-only`
and extended-precision targets are compile errors. clang's own guard does not
survive an explicit `-ffp-contract=fast`: do not pass that when including this
header. `tools/run.sh` builds a fusion canary (guards on must pass the vectors,
guards off with `-ffp-contract=fast` must fail them) on x86-64 with FMA and on arm64.

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

## Tests

- `tests/trig_test.cpp`: hand-written example checks (1e-10 tolerance).
- `tests/conformance_test.cpp`: every row of the four `trig-reference-v1` groups, bit
  for bit; accuracy against the correctly rounded `cr` column; every embedded
  constant by its bits. Run from this directory (it finds the corpus relative to it).

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
