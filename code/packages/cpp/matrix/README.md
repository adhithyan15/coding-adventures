# matrix (C++)

A small, dependency-free 2D matrix of `double`, **header-only** in pure ISO
C++17 (namespace `ca::matrix`). A faithful port of the Rust
[`matrix`](../../rust/matrix) crate.

## What it does

`ca::matrix::Matrix` stores elements as `std::vector<std::vector<double>>` (a
vector of row vectors, exactly like the Rust `Vec<Vec<f64>>`) and supports:

- **Constructors** — `Matrix(data)`, `Matrix::new_1d`, `Matrix::new_scalar`,
  `Matrix::zeros`, `Matrix::identity`, `Matrix::from_diagonal`.
- **Arithmetic** — `add`, `subtract`, `add_scalar`, `scale`, `transpose`,
  `dot`.
- **Element access** — `get`, immutable `set`.
- **Reductions** — `sum`, `mean`, `min_val`, `max_val`, `sum_rows`, `sum_cols`,
  `argmin`, `argmax`.
- **Element-wise math** — `map`, `sqrt`, `abs_val`, `pow_val`.
- **Shape** — `flatten`, `reshape`, `row`, `col`, `slice`.
- **Comparison** — `equals`, `close`.

## How it fits the stack

Part of the C/C++ pure-ISO port lane. Every method returns a **new** `Matrix`
and never mutates `*this` (the Rust crate is immutable by default).

## Design notes

- **Exceptions, not `Result`.** Rust's `Result<Matrix, _>` becomes a throw:
  `std::invalid_argument` for dimension mismatches, `std::out_of_range` for a
  bad index.
- **No `<cmath>`.** `sqrt` and a general `pow` are computed from scratch (the
  pure-ISO build links no math library); they match the Rust f64 results to
  ~1e-12 relative.
- **Header-only.** `#include "matrix.hpp"` and go — no separate translation
  unit to build.

## Usage

```cpp
#include "matrix.hpp"
using ca::matrix::Matrix;

Matrix a({{1, 2}, {3, 4}});
Matrix i = Matrix::identity(2);
Matrix c = a.dot(i);   // c == a
double top_left = c.get(0, 0);
```

## Building

```sh
sh BUILD           # POSIX: g++ and/or clang++ via the shared iso-harness
```

The suite compiles under GCC, Clang, and MSVC with `-pedantic-errors` /
`/permissive-` and warnings-as-errors.
