# ML03 — Matrix Extensions

## Overview

This spec extends the existing ML03 Matrix package with element access,
reductions, element-wise math, shape operations, and comparison methods.
These operations are required by ST01 (Statistics) and bring the Matrix
closer to NumPy expressiveness while remaining a pure, dependency-free
implementation in each language.

All operations are added to the existing Matrix class/struct in each of
the 9 language packages. This spec also creates the Swift matrix package
(the 9th language) with full parity — both base operations and extensions.

Existing methods (zeros, add, subtract, scale, transpose, dot) remain
unchanged.

## Design Principles

1. **Immutable by default.** Methods return a new Matrix, never mutate.
2. **No external dependencies.** Only language-native math (sqrt, abs, pow).
3. **Consistent error handling.** Out-of-bounds, shape mismatches, and
   invalid reshape dimensions raise errors using each language's idiom.
4. **GPU-ready interface.** Methods designed so a future trait/interface
   split (CPU vs GPU backend) can swap implementation without changing
   call sites.

## Interface Contract

### Element Access

| Method | Signature | Description |
|--------|-----------|-------------|
| `get` | `(row: int, col: int) -> float` | Element at (row, col). Raises on out-of-bounds. |
| `set` | `(row: int, col: int, value: float) -> Matrix` | New matrix with element replaced. |

### Reductions

| Method | Signature | Description |
|--------|-----------|-------------|
| `sum` | `() -> float` | Sum of all elements. |
| `sum_rows` | `() -> Matrix` | Column vector (n×1): each row's sum. |
| `sum_cols` | `() -> Matrix` | Row vector (1×m): each column's sum. |
| `mean` | `() -> float` | Arithmetic mean of all elements. |
| `min` | `() -> float` | Minimum element value. |
| `max` | `() -> float` | Maximum element value. |
| `argmin` | `() -> (int, int)` | (row, col) of minimum. First occurrence. |
| `argmax` | `() -> (int, int)` | (row, col) of maximum. First occurrence. |

### Element-wise Math

| Method | Signature | Description |
|--------|-----------|-------------|
| `map` | `(fn: float -> float) -> Matrix` | Apply function to every element. |
| `sqrt` | `() -> Matrix` | Element-wise square root. |
| `abs` | `() -> Matrix` | Element-wise absolute value. |
| `pow` | `(exp: float) -> Matrix` | Element-wise exponentiation. |

### Shape Operations

| Method | Signature | Description |
|--------|-----------|-------------|
| `flatten` | `() -> Matrix` | Returns 1×n matrix (row vector). |
| `reshape` | `(rows: int, cols: int) -> Matrix` | Reshape. rows*cols must equal total elements. |
| `row` | `(i: int) -> Matrix` | Row i as 1×cols matrix. |
| `col` | `(j: int) -> Matrix` | Column j as rows×1 matrix. |
| `slice` | `(r0: int, r1: int, c0: int, c1: int) -> Matrix` | Sub-matrix [r0..r1), [c0..c1). |

### Equality and Comparison

| Method | Signature | Description |
|--------|-----------|-------------|
| `equals` | `(other: Matrix) -> bool` | Exact element-wise equality. |
| `close` | `(other: Matrix, tol: float) -> bool` | Within tolerance. Default tol=1e-9. |

### Factory Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `identity` | `(n: int) -> Matrix` | n×n identity matrix. |
| `from_diagonal` | `(values: list[float]) -> Matrix` | Diagonal matrix from values. |

## Worked Example

```text
M = Matrix([[1.0, 2.0, 3.0],
            [4.0, 5.0, 6.0]])

M.get(1, 2)        -> 6.0
M.sum()             -> 21.0
M.sum_rows()        -> Matrix([[6.0], [15.0]])
M.sum_cols()        -> Matrix([[5.0, 7.0, 9.0]])
M.mean()            -> 3.5
M.min()             -> 1.0
M.argmax()          -> (1, 2)
M.sqrt()            -> Matrix([[1.0, 1.414, 1.732],
                               [2.0, 2.236, 2.449]])
M.row(0)            -> Matrix([[1.0, 2.0, 3.0]])
M.flatten()         -> Matrix([[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]])
M.reshape(3, 2)     -> Matrix([[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]])
M.slice(0, 2, 1, 3) -> Matrix([[2.0, 3.0], [5.0, 6.0]])

Matrix.identity(3)       -> [[1,0,0],[0,1,0],[0,0,1]]
Matrix.from_diagonal([2, 3]) -> [[2,0],[0,3]]

M.close(M.sqrt().pow(2.0), 1e-9) -> true
```

## Parity Test Vectors

All 9 languages must produce identical results for:

- **sum/mean:** `[[1,2],[3,4]]` -> sum=10.0, mean=2.5
- **sum_rows/sum_cols:** `[[1,2],[3,4]]` -> rows=[[3],[7]], cols=[[4,6]]
- **identity dot:** `identity(3).dot(M) == M` for any 3×n M
- **flatten/reshape roundtrip:** `M.flatten().reshape(M.rows, M.cols) == M`
- **close after sqrt/pow:** `M.close(M.sqrt().pow(2.0), 1e-9)` -> true

## Package Matrix

| Language | Package Directory | File(s) to modify/create |
|----------|-------------------|--------------------------|
| Python | `code/packages/python/matrix/` | `src/matrix/matrix.py` |
| Go | `code/packages/go/matrix/` | `matrix.go` |
| Ruby | `code/packages/ruby/matrix/` | `lib/matrix_ml.rb` |
| TypeScript | `code/packages/typescript/matrix/` | `src/matrix.ts` |
| Rust | `code/packages/rust/matrix/` | `src/lib.rs` |
| Elixir | `code/packages/elixir/matrix/` | `lib/matrix.ex` |
| Lua | `code/packages/lua/matrix/` | `src/coding_adventures/matrix/init.lua` |
| Perl | `code/packages/perl/matrix/` | `lib/CodingAdventures/Matrix.pm` |
| Swift | `code/packages/swift/matrix/` | **NEW** — full package with base + extensions |

**Dependencies:** None. Pure math only. Extends ML03.
