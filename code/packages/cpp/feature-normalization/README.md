# feature-normalization (C++)

Column-wise feature scaling, header-only in pure ISO C++17 (namespace
`ca::feature_normalization`) — a faithful port of the Rust
`feature-normalization` crate. Two classic scalers:

| Scaler | Formula (per column) | Learns |
|--------|----------------------|--------|
| **StandardScaler** (z-score) | `z = (x - mean) / stddev` | mean, population stddev |
| **MinMaxScaler** (unit range) | `u = (x - min) / (max - min)` | min, max |

Each is a two-step **fit / transform**: `fit_*` learns per-column statistics
from a training matrix; `transform_*` applies them. A column with zero spread
maps to `0.0` (no divide-by-zero).

## Usage

```cpp
#include "feature_normalization.hpp"
namespace fn = ca::feature_normalization;

fn::Matrix data = {{1000,3,1}, {1500,4,0}, {2000,5,1}};

fn::StandardScaler s = fn::fit_standard_scaler(data);
fn::Matrix z = fn::transform_standard(data, s);
// z[0][0] == -1.2247..., z[1][0] == 0, z[2][0] == +1.2247...

fn::Matrix u = fn::transform_min_max(data, fn::fit_min_max_scaler(data));
// u == {{0,0,1}, {0.5,0.5,0}, {1,1,1}}
```

Matrices are `std::vector<std::vector<double>>` (the same shape as the Rust
crate), so ragged rows are representable and validated.

## Divergence from the Rust crate

Rust returns `Result<_, &'static str>`; this port throws `std::invalid_argument`
with the same message (empty matrix, ragged rows, or a width mismatch). The
population standard deviation (divide by `n`, not `n-1`) matches Rust. `sqrt` is
computed by Newton's method — no `<cmath>`.

## Building

```sh
sh BUILD    # builds & runs the tests under every C++ compiler present
```

Pure ISO C++17. Builds clean under GCC, Clang, and MSVC with `-pedantic-errors`
/ `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness).
