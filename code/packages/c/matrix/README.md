# matrix (C)

A small, dependency-free 2D matrix of `double`, in **pure ISO C17**. A faithful
port of the Rust [`matrix`](../../rust/matrix) crate.

## What it does

A `Mat` holds a row-major block of `rows * cols` doubles and supports:

- **Constructors** — `mat_new`, `mat_new_1d`, `mat_new_scalar`, `mat_zeros`,
  `mat_identity`, `mat_from_diagonal`, `mat_clone`.
- **Arithmetic** — `mat_add`, `mat_subtract`, `mat_add_scalar`, `mat_scale`,
  `mat_transpose`, `mat_dot`.
- **Element access** — `mat_get`, immutable `mat_set`.
- **Reductions** — `mat_sum`, `mat_mean`, `mat_min_val`, `mat_max_val`,
  `mat_sum_rows`, `mat_sum_cols`, `mat_argmin`, `mat_argmax`.
- **Element-wise math** — `mat_map`, `mat_sqrt`, `mat_abs`, `mat_pow`.
- **Shape** — `mat_flatten`, `mat_reshape`, `mat_row`, `mat_col`, `mat_slice`.
- **Comparison** — `mat_equals`, `mat_close`.

## How it fits the stack

Part of the C/C++ pure-ISO port lane. Every operation returns a **new** matrix
(the Rust crate is immutable by default), so inputs are never mutated.

## Design notes

- **Ownership.** Producers allocate a fresh matrix; on `MAT_OK` the out-param
  owns heap memory — release it with `mat_free`. On any error status the
  out-param is left empty (`data == NULL`), so `mat_free` on it is always safe,
  as is `mat_free(NULL)` and a repeated free.
- **Status codes, not exceptions.** Rust's `Result<Matrix, _>` becomes a
  `MatStatus` (`MAT_OK`, `MAT_ERR_DIM`, `MAT_ERR_BOUNDS`, `MAT_ERR_ALLOC`) with
  the result written through an out-parameter.
- **No libm.** `sqrt`, `abs`, and a general `pow` are computed from scratch, so
  the package needs no `<math.h>` and no `-lm`. They reproduce the Rust f64
  results to ~1e-12 relative.
- **Overflow-guarded allocation.** Every `rows * cols` element count is checked
  against `SIZE_MAX` before allocating.

## Usage

```c
#include "matrix.h"

double av[] = {1, 2, 3, 4};
Mat a, b, c;
mat_new(2, 2, av, &a);
mat_identity(2, &b);
if (mat_dot(&a, &b, &c) == MAT_OK) {
    /* c == a */
}
mat_free(&a);
mat_free(&b);
mat_free(&c);
```

## Building

```sh
sh BUILD           # POSIX: GCC and/or Clang via the shared iso-harness
```

The suite compiles under GCC, Clang, and MSVC with `-pedantic-errors` /
`/permissive-` and warnings-as-errors.
