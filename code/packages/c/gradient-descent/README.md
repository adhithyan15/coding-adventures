# gradient-descent (C)

One step of **stochastic gradient descent (SGD)** in pure ISO C17. A faithful
port of the Rust [`gradient-descent`](../../rust/gradient-descent) crate.

## What it does

SGD is the workhorse of machine-learning optimisation. Given model **weights**
and the **gradient** of the loss with respect to each weight, it nudges every
weight a small step *downhill* — toward a lower loss:

```
new_weight[i] = weight[i] - learning_rate * gradient[i]
```

The `learning_rate` (a small positive scalar like `0.01`) sets the step size:
too large and the optimiser overshoots; too small and it crawls. This routine
performs exactly one such update over the whole vector.

## API

```c
#include "gradient_descent.h"

double w[3]   = {1.0, -0.5, 2.0};
double g[3]   = {0.1, -0.2, 0.0};
double out[3];
gd_sgd(w, g, 3, 0.1, out);   /* out = {0.99, -0.48, 2.0} */
```

- `gd_sgd(weights, gradients, n, learning_rate, out)` — writes the update into
  `out` (caller-owned, may alias `weights` for an in-place step). Returns
  `GD_ERR_LENGTH` if `n == 0`. Allocation-free.

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.
