# Matrix (Ruby)

A pure-Ruby 2D matrix library for linear algebra. No external dependencies --
only Ruby's built-in `Math` module.

## Usage

```ruby
require_relative "lib/matrix_ml"

# Create matrices
m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
z = Matrix.zeros(3, 3)
i = Matrix.identity(3)
d = Matrix.from_diagonal([2.0, 3.0])

# Arithmetic
a + b           # element-wise addition
a - b           # element-wise subtraction
a * 2.0         # scalar multiplication
a.dot(b)        # matrix multiplication
a.transpose     # transpose

# Element access
m.get(0, 1)          # read element -> 2.0
m.set(0, 1, 99.0)    # new matrix with element replaced

# Reductions
m.sum          # sum of all elements -> 10.0
m.mean         # arithmetic mean -> 2.5
m.sum_rows     # column vector of row sums
m.sum_cols     # row vector of column sums
m.min          # smallest element
m.max          # largest element
m.argmin       # [row, col] of minimum
m.argmax       # [row, col] of maximum

# Element-wise math
m.map_elements { |v| v * 2 }   # apply block to every element
m.sqrt         # element-wise square root
m.abs          # element-wise absolute value
m.pow(2.0)     # element-wise exponentiation

# Shape operations
m.flatten                   # 1 x n row vector
m.reshape(rows, cols)       # reshape (total must match)
m.row(i)                    # extract row i
m.col(j)                    # extract column j
m.slice(r0, r1, c0, c1)    # sub-matrix [r0..r1), [c0..c1)

# Equality
m.equals(other)                  # exact equality
m.close(other, tolerance = 1e-9)  # within tolerance
```

## Design Principles

1. **Immutable by default** -- methods return new matrices, never mutate.
2. **No external dependencies** -- pure Ruby math only.
3. **Consistent error handling** -- descriptive ArgumentError/IndexError.
4. **GPU-ready interface** -- designed for future backend swapping.

## Running Tests

```bash
ruby test/test_matrix.rb -v
```
