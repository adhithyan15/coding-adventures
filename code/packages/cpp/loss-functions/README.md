# loss-functions (C++)

Machine-learning loss functions and their gradients, header-only in pure ISO
C++17 (namespace `ca::loss_functions`) — a faithful port of the Rust
`loss-functions` crate. **No `<cmath>` / libm**: the one logarithm is computed
from scratch.

## The functions

| Loss | Formula | Use |
|------|---------|-----|
| **MSE** | `(1/n) Σ (t − p)²` | regression |
| **MAE** | `(1/n) Σ \|t − p\|` | robust regression |
| **BCE** | `−(1/n) Σ [t·ln p + (1−t)·ln(1−p)]` | binary classification |
| **CCE** | `−(1/n) Σ [t·ln p]` | multi-class classification |

Cross-entropy clamps each prediction to `[1e-7, 1 − 1e-7]` before the log.

## Usage

```cpp
#include "loss_functions.hpp"
namespace lf = ca::loss_functions;

std::vector<double> y_true = {1.0, 0.0};
std::vector<double> y_pred = {0.9, 0.1};

double loss = lf::bce(y_true, y_pred);          // 0.1053605
auto   grad = lf::bce_derivative(y_true, y_pred); // {-0.625, 0.625}
```

Mismatched-length or empty inputs throw `std::invalid_argument` — the same
condition the Rust crate rejects.

## Divergence from the Rust crate

Rust returns `Result<_, &'static str>`; this port throws `std::invalid_argument`.
Scalar losses return `double`; gradients return `std::vector<double>`. The `ln`
is computed by range reduction + an atanh series — no `<cmath>`.

## Building

```sh
sh BUILD    # builds & runs the tests under every C++ compiler present
```

Pure ISO C++17, no `<cmath>`. Builds clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness).
