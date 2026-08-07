# loss-functions (C)

Machine-learning loss functions and their gradients, in pure ISO C17 — a
faithful port of the Rust `loss-functions` crate. **No libm**: the one
logarithm is computed from scratch.

## The functions

Each loss ships with its per-element derivative (the gradient training
descends):

| Loss | Formula | Use |
|------|---------|-----|
| **MSE** | `(1/n) Σ (t − p)²` | regression |
| **MAE** | `(1/n) Σ \|t − p\|` | robust regression |
| **BCE** | `−(1/n) Σ [t·ln p + (1−t)·ln(1−p)]` | binary classification |
| **CCE** | `−(1/n) Σ [t·ln p]` | multi-class classification |

Cross-entropy clamps each prediction to `[1e-7, 1 − 1e-7]` before the log, so
`ln(0) = −∞` never occurs.

## API

```c
#include "loss_functions.h"

double y_true[2] = {1.0, 0.0};
double y_pred[2] = {0.9, 0.1};

double loss;
if (loss_bce(y_true, 2, y_pred, 2, &loss) == LOSS_OK) {
    /* loss == 0.1053605 */
}

double grad[2];                                  /* caller-owned, length n */
loss_bce_derivative(y_true, 2, y_pred, 2, grad); /* {-0.625, 0.625} */
```

Each array carries its own length, so an unequal-length (or empty) call returns
`LOSS_ERR_LENGTH` — the same condition the Rust crate rejects.

## Divergence from the Rust crate

Rust returns `Result<_, &'static str>`; this port returns a `LossStatus` code
(`LOSS_OK` / `LOSS_ERR_LENGTH`). Scalar losses write to `*out`; gradients write
`n` values into a caller-provided `out`. The `ln` is computed by range reduction
+ an atanh series — no `<math.h>`.

## Building

```sh
sh BUILD    # builds & runs the tests under every C compiler present
```

Pure ISO C17, no `<math.h>`. Builds clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Where it fits

Part of the C/C++ port campaign. A companion to
[`activation-functions`](../activation-functions): activations are the forward
nonlinearity, these losses (and gradients) are what a network trains against.
