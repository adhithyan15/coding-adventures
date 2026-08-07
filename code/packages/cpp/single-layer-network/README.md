# single-layer-network (C++)

A single dense neural-network layer with batch gradient descent, header-only in
pure ISO C++17 (namespace `ca::single_layer_network`) — a faithful port of the
Rust `single-layer-network` crate. **No `<cmath>` / libm**: the sigmoid uses a
from-scratch `e^x`.

## What it does

One dense layer maps `input_count` features to `output_count` outputs:

```
prediction[o] = activate( bias[o] + Σ_i input[i] · weight[i][o] )
```

Training is full-batch mean-squared-error gradient descent. Activations are
**Linear** and **Sigmoid**.

## Usage

```cpp
#include "single_layer_network.hpp"
namespace sln = ca::single_layer_network;
using sln::Matrix;

sln::SingleLayerNetwork net(3, 2, sln::ActivationName::Linear);
auto history = net.fit(
    Matrix{{0,0,1}, {1,2,1}, {2,1,1}},
    Matrix{{1,-1}, {3,2}, {4,1}}, 0.05, 500);
// history.back().loss < history.front().loss

Matrix pred = net.predict(Matrix{{1,1,1}});   // 1 × 2
```

Matrices are `std::vector<std::vector<double>>` (the same shape as the Rust
crate), so ragged rows are representable and validated. The free functions
`predict_with_parameters` / `train_one_epoch_with_matrices` /
`fit_single_layer_network` mirror the crate's API.

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
