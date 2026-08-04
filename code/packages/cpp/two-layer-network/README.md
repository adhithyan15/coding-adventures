# two-layer-network (C++)

A two-layer (one hidden layer) neural network with backpropagation, header-only
in pure ISO C++17 (namespace `ca::two_layer_network`) — a faithful port of the
Rust `two-layer-network` crate. **No `<cmath>` / libm**: the sigmoid uses a
from-scratch `e^x`.

## Why a hidden layer

A single dense layer cannot learn XOR (it's not linearly separable). One hidden
layer fixes that. Forward pass:

```
hidden_raw = inputs · W_ih + b_h ;  hidden     = activation(hidden_raw)
output_raw = hidden · W_ho + b_o ;  prediction = activation(output_raw)
```

`train_one_epoch` runs one full-batch mean-squared-error step, backpropagating
through both layers and returning every gradient plus the next parameters.

## Usage

```cpp
#include "two_layer_network.hpp"
namespace tln = ca::two_layer_network;
using tln::Matrix;

auto p = tln::xor_warm_start_parameters();      // a 2-2-1 XOR network
Matrix inputs = {{0,0}, {0,1}, {1,0}, {1,1}};

auto fp = tln::forward(inputs, p, tln::ActivationName::Sigmoid,
                       tln::ActivationName::Sigmoid);
// fp.predictions[1][0] ≈ 1 (XOR of 0,1), fp.predictions[0][0] ≈ 0

auto step = tln::train_one_epoch(inputs, Matrix{{0},{1},{1},{0}}, p, 0.5,
                                 tln::ActivationName::Sigmoid,
                                 tln::ActivationName::Sigmoid);
// step.next_parameters, step.loss, both layers' gradients …
```

Matrices are `std::vector<std::vector<double>>` (the same shape as the Rust
crate), so ragged rows are representable and validated.

## Divergence from the Rust crate

Rust returns `Result<_, String>`; this port throws `std::invalid_argument` with
the same message on a shape error. `e^x` is computed without `<cmath>`.

## Building

```sh
sh BUILD    # builds & runs the tests under every C++ compiler present
```

Pure ISO C++17, no `<cmath>`. Builds clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness).
