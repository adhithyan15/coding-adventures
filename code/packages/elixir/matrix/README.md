# Matrix (Elixir)

A pure Elixir 2D matrix library with no external dependencies. Part of the coding-adventures monorepo.

## Usage

```elixir
# Construction
m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
z = Matrix.zeros(3, 3)
i = Matrix.identity(3)
d = Matrix.from_diagonal([2, 3])

# Arithmetic
Matrix.add(a, b)          # element-wise addition
Matrix.subtract(a, b)     # element-wise subtraction
Matrix.add_scalar(m, 2.0) # broadcast scalar addition
Matrix.scale(m, 2.0)      # scalar multiplication
Matrix.dot(a, b)          # matrix multiplication

# Element access
Matrix.get(m, 0, 0)       # read element at (row, col)
Matrix.set(m, 0, 0, 99.0) # new matrix with element replaced

# Reductions
Matrix.sum(m)              # sum of all elements
Matrix.sum_rows(m)         # n x 1 column vector of row sums
Matrix.sum_cols(m)         # 1 x m row vector of column sums
Matrix.mean(m)             # arithmetic mean
Matrix.min_val(m)          # minimum element
Matrix.max_val(m)          # maximum element
Matrix.argmin(m)           # {row, col} of minimum
Matrix.argmax(m)           # {row, col} of maximum

# Element-wise math
Matrix.map_elements(m, &(&1 * 2))  # apply function to every element
Matrix.matrix_sqrt(m)              # element-wise square root
Matrix.matrix_abs(m)               # element-wise absolute value
Matrix.matrix_pow(m, 2)            # element-wise exponentiation

# Shape operations
Matrix.flatten(m)              # 1 x n row vector
Matrix.reshape(m, 3, 2)       # reshape (total elements must match)
Matrix.get_row(m, 0)          # extract row as 1 x cols matrix
Matrix.get_col(m, 0)          # extract column as rows x 1 matrix
Matrix.matrix_slice(m, 0, 2, 1, 3)  # sub-matrix [r0..r1), [c0..c1)
Matrix.transpose(m)           # swap rows and columns

# Equality
Matrix.equals(a, b)           # exact element-wise comparison
Matrix.close(a, b, 1.0e-9)   # approximate comparison within tolerance
```

## Design Principles

1. **Immutable by default.** All functions return new Matrix structs (Elixir data is always immutable).
2. **No external dependencies.** Only `:math` from Erlang's standard library.
3. **Functional style.** Module functions rather than methods on objects.
4. **Literate programming.** Source code includes `@doc` annotations with examples and explanations.

## Running Tests

```bash
mix test --trace
```

## Package Structure

- `lib/matrix.ex` -- Matrix module with all operations
- `test/matrix_test.exs` -- ExUnit test suite (42 tests)
