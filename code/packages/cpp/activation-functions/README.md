# activation-functions (C++)

Neural-network activation functions and their derivatives, header-only in pure
ISO C++17 (namespace `ca::activation_functions`) — a faithful port of the Rust
`activation-functions` crate. **No `<cmath>` / libm**: the transcendentals
(`e^x`, `tanh`, `ln(1+x)`) are computed from scratch.

## The functions

| Activation | `f(x)` | `f'(x)` |
|-----------|--------|---------|
| linear | `x` | `1` |
| sigmoid | `1 / (1 + e^-x)` | `f(x)(1 - f(x))` |
| relu | `max(0, x)` | `x > 0 ? 1 : 0` |
| leaky_relu | `x>0 ? x : 0.01x` | `x > 0 ? 1 : 0.01` |
| tanh | `tanh(x)` | `1 - tanh(x)^2` |
| softplus | `ln(1 + e^x)` | `sigmoid(x)` |

`sigmoid` saturates to 0/1 outside ±709; `softplus` uses the stable
`ln(1 + e^-|x|) + max(x, 0)`. Every function is total.

## How the transcendentals are built (libm-free)

- **`e^x`** — Cody-Waite range reduction `x = k·ln2 + r`, Taylor series for
  `e^r`, exact `2^k` scale.
- **`ln(1+x)`** — `2·atanh(u)` with `u = x/(2+x)`.
- **`tanh`** — `(1 - e^-2|x|)/(1 + e^-2|x|)`, odd-extended and saturated.

Results match `std::exp` / `std::tanh` / `std::log1p` to within ~1e-12.

## Usage

```cpp
#include "activation_functions.hpp"
namespace af = ca::activation_functions;

double y  = af::sigmoid(1.0);             // 0.7310585786300049
double dy = af::sigmoid_derivative(1.0);  // 0.19661193324148185
double s  = af::softplus(0.0);            // ln 2 = 0.6931471805599453
```

## Building

```sh
sh BUILD    # builds & runs the tests under every C++ compiler present
```

Pure ISO C++17, no `<cmath>`. Builds clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness).
