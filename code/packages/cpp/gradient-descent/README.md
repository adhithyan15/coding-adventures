# gradient-descent (C++)

One step of **stochastic gradient descent (SGD)**, header-only, ISO C++17. A
faithful port of the Rust [`gradient-descent`](../../rust/gradient-descent)
crate, in namespace `ca::gradient_descent`.

## What it does

SGD is the workhorse of machine-learning optimisation. Given model **weights**
and the **gradient** of the loss with respect to each weight, it nudges every
weight a small step *downhill*:

```
new_weight[i] = weight[i] - learning_rate * gradient[i]
```

The `learning_rate` sets the step size. This function performs exactly one such
update over the whole vector.

## API

```cpp
#include "gradient_descent.hpp"
namespace gd = ca::gradient_descent;

std::vector<double> w = {1.0, -0.5, 2.0};
std::vector<double> g = {0.1, -0.2, 0.0};
std::vector<double> next = gd::sgd(w, g, 0.1);   // {0.99, -0.48, 2.0}
```

- `sgd(weights, gradients, learning_rate)` — returns the updated weights.
  Throws `GradientDescentError` if the vectors differ in length or are empty
  (matching the Rust crate's error condition).

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.
