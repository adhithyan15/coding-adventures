# Matrix (Python)

A pure-Python 2D matrix library for linear algebra. No external dependencies --
only the standard library `math` module.

## Installation

```bash
pip install -e .
```

## Usage

```python
from matrix import Matrix

# Create matrices
M = Matrix([[1.0, 2.0], [3.0, 4.0]])
Z = Matrix.zeros(3, 3)
I = Matrix.identity(3)
D = Matrix.from_diagonal([2.0, 3.0])

# Arithmetic
A + B           # element-wise addition
A - B           # element-wise subtraction
A * 2.0         # scalar multiplication
A.dot(B)        # matrix multiplication
A.transpose()   # transpose

# Element access
M.get(0, 1)          # read element -> 2.0
M.set(0, 1, 99.0)    # new matrix with element replaced

# Reductions
M.sum()          # sum of all elements -> 10.0
M.mean()         # arithmetic mean -> 2.5
M.sum_rows()     # column vector of row sums
M.sum_cols()     # row vector of column sums
M.min()          # smallest element
M.max()          # largest element
M.argmin()       # (row, col) of minimum
M.argmax()       # (row, col) of maximum

# Element-wise math
M.map(fn)        # apply function to every element
M.sqrt()         # element-wise square root
M.abs()          # element-wise absolute value
M.pow(2.0)       # element-wise exponentiation

# Shape operations
M.flatten()              # 1 x n row vector
M.reshape(rows, cols)    # reshape (total must match)
M.row(i)                 # extract row i
M.col(j)                 # extract column j
M.slice(r0, r1, c0, c1)  # sub-matrix [r0..r1), [c0..c1)

# Equality
M.equals(other)               # exact equality
M.close(other, tolerance=1e-9)  # within tolerance
```

## Design Principles

1. **Immutable by default** -- methods return new matrices, never mutate.
2. **No external dependencies** -- pure Python math only.
3. **Consistent error handling** -- descriptive ValueError/IndexError.
4. **GPU-ready interface** -- designed for future backend swapping.

## Running Tests

```bash
python -m pytest tests/ -v
```
