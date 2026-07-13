# activation-functions (C)

Neural-network activation functions and their derivatives, in pure ISO C17 — a
faithful port of the Rust `activation-functions` crate. **No libm**: the
transcendental helpers (`e^x`, `tanh`, `ln(1+x)`) are computed from scratch.

## The functions

Each activation ships with its derivative (what backpropagation multiplies
through):

| Activation | `f(x)` | `f'(x)` |
|-----------|--------|---------|
| linear | `x` | `1` |
| sigmoid | `1 / (1 + e^-x)` | `f(x)(1 - f(x))` |
| relu | `max(0, x)` | `x > 0 ? 1 : 0` |
| leaky_relu | `x>0 ? x : 0.01x` | `x > 0 ? 1 : 0.01` |
| tanh | `tanh(x)` | `1 - tanh(x)^2` |
| softplus | `ln(1 + e^x)` | `sigmoid(x)` |

`sigmoid` saturates to 0/1 outside ±709, and `softplus` uses the numerically
stable `ln(1 + e^-|x|) + max(x, 0)` — so every function is total.

## How the transcendentals are built (libm-free)

- **`e^x`** — Cody-Waite range reduction `x = k·ln2 + r` (`|r| ≤ ln2/2`), then
  a Taylor series for `e^r` and an exact `2^k` scale.
- **`ln(1+x)`** — `2·atanh(u)` with `u = x/(2+x)` (no near-1 cancellation).
- **`tanh`** — `(1 - e^-2|x|)/(1 + e^-2|x|)`, odd-extended and saturated.

Results match the C standard library to within ~1e-12 (the tolerance the Rust
crate's own tests use).

## API

```c
#include "activation_functions.h"

double y  = af_sigmoid(1.0);              /* 0.7310585786300049 */
double dy = af_sigmoid_derivative(1.0);   /* 0.19661193324148185 */
double s  = af_softplus(0.0);             /* ln 2 = 0.6931471805599453 */
```

All functions are `double -> double`, total, with no error path — matching the
Rust crate.

## Building

```sh
sh BUILD    # builds & runs the tests under every C compiler present
```

Pure ISO C17, no `<math.h>`. Builds clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Where it fits

Part of the C/C++ port campaign mirroring the Rust learning packages. A
companion to [`trig`](../trig): where that builds sine/cosine from series, this
builds the exponential-family activations a neural network needs.
