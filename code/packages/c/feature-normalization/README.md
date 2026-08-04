# feature-normalization (C)

Column-wise feature scaling in pure ISO C17 — a faithful port of the Rust
`feature-normalization` crate. Two classic scalers for putting a data matrix's
columns on comparable scales before feeding a model:

| Scaler | Formula (per column) | Learns |
|--------|----------------------|--------|
| **StandardScaler** (z-score) | `z = (x - mean) / stddev` | mean, population stddev |
| **MinMaxScaler** (unit range) | `u = (x - min) / (max - min)` | min, max |

Each is a two-step **fit / transform**: `fit` learns the per-column statistics
from a training matrix; `transform` applies them to that matrix or any other of
the same width. A column with zero spread maps to `0.0` (no divide-by-zero).

## Matrix representation

Matrices are a flat **row-major** array of `nrows * ncols` doubles — element
`(r, c)` at `data[r*ncols + c]`. Because the width is explicit, ragged rows
can't occur; a single `FN_ERR_EMPTY` covers `nrows == 0` or `ncols == 0`.

## API

```c
#include "feature_normalization.h"

double data[9] = {1000,3,1, 1500,4,0, 2000,5,1};   /* 3 rows x 3 cols */

FnStandardScaler s;
if (fn_fit_standard_scaler(data, 3, 3, &s) == FN_OK) {
    double out[9];
    fn_transform_standard(data, 3, 3, &s, out);    /* caller-owned buffer */
    /* out[0] == -1.2247..., out[3] == 0, out[6] == +1.2247... */
    fn_standard_scaler_free(&s);
}
```

`fn_transform_*` writes into a caller-provided `out` buffer (also
`nrows*ncols`). Every `fn_fit_*` that returns `FN_OK` must be paired with the
matching `fn_*_free`.

## Divergence from the Rust crate

Rust returns `Result<_, &'static str>`; this port returns an `FnStatus` code
(`FN_OK` / `FN_ERR_EMPTY` / `FN_ERR_WIDTH_MISMATCH` / `FN_ERR_NOMEM`). The
population standard deviation (divide by `n`, not `n-1`) matches Rust. `sqrt` is
computed by Newton's method — no `<math.h>`.

## Building

```sh
sh BUILD    # builds & runs the tests under every C compiler present
```

Pure ISO C17. Builds clean under GCC, Clang, and MSVC with `-pedantic-errors` /
`/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).
