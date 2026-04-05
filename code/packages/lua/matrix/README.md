# coding-adventures-matrix (Lua)

Pure-Lua 2D matrix type with arithmetic and linear-algebra operations.

## What it does

Provides a `Matrix` value (a plain Lua table with `rows`, `cols`, and `data` fields) and the following operations:

### Base Operations

| Operation | Function | Notes |
|-----------|----------|-------|
| Create all-zeros | `zeros(rows, cols)` | |
| Create from 2D table | `new_2d(data)` | Deep-copies input |
| Create from 1D table | `new_1d(data)` | Creates a 1×n row vector |
| Create from scalar | `new_scalar(val)` | Creates a 1×1 matrix |
| Element access | `get(mat, i, j)` / `set(mat, i, j, val)` | 1-based; set returns new matrix |
| Element-wise add | `add(A, B)` | Returns error if shapes differ |
| Add scalar | `add_scalar(A, s)` | Every element += s |
| Element-wise subtract | `subtract(A, B)` | Returns error if shapes differ |
| Scale | `scale(A, s)` | Every element *= s |
| Transpose | `transpose(A)` | Returns new m×n → n×m matrix |
| Matrix multiply | `dot(A, B)` | A.cols must equal B.rows |

### Extension Operations (ML03)

| Operation | Function | Notes |
|-----------|----------|-------|
| Sum | `sum(mat)` | Sum of all elements |
| Sum rows | `sum_rows(mat)` | n×1 column vector |
| Sum cols | `sum_cols(mat)` | 1×m row vector |
| Mean | `mean(mat)` | Arithmetic mean |
| Min / Max | `min(mat)` / `max(mat)` | Scalar min/max |
| Argmin / Argmax | `argmin(mat)` / `argmax(mat)` | (row, col) of extremum |
| Map | `map(mat, fn)` | Apply fn to every element |
| Sqrt | `sqrt(mat)` | Element-wise square root |
| Abs | `abs(mat)` | Element-wise absolute value |
| Pow | `pow(mat, exp)` | Element-wise exponentiation |
| Flatten | `flatten(mat)` | 1×n row vector |
| Reshape | `reshape(mat, rows, cols)` | Must preserve total elements |
| Row / Col | `row(mat, i)` / `col(mat, j)` | Extract single row/col |
| Slice | `slice(mat, r0, r1, c0, c1)` | Sub-matrix [r0..r1), [c0..c1) |
| Equals | `equals(A, B)` | Exact equality |
| Close | `close(A, B, tol)` | Within tolerance (default 1e-9) |
| Identity | `identity(n)` | n×n identity matrix |
| From diagonal | `from_diagonal(values)` | Diagonal matrix from values |

## How it fits in the stack

This package is the Lua mirror of `code/packages/perl/matrix` and `code/packages/elixir/matrix`. It is a pure-math leaf package with no dependencies outside Lua's standard library.

In a machine-learning pipeline this module underpins forward passes (weight × activation dot products), gradient accumulation, and layer-output buffering.

## Usage

```lua
local M = require("coding_adventures.matrix")

-- Build a 2×3 weight matrix.
local W = M.new_2d({
    {0.1, 0.2, 0.3},
    {0.4, 0.5, 0.6},
})

-- Build a 3×1 input column vector.
local x = M.transpose(M.new_1d({1.0, 2.0, 3.0}))

-- Forward pass: W · x  →  2×1 result
local out, err = M.dot(W, x)
if err then error(err) end

print(M.get(out, 1, 1))  -- 0.1*1 + 0.2*2 + 0.3*3 = 1.4
print(M.get(out, 2, 1))  -- 0.4*1 + 0.5*2 + 0.6*3 = 3.2
```

## Running the tests

```
cd tests && busted . --verbose --pattern=test_
```

Requires [busted](https://olivinelabs.com/busted/) (`luarocks install busted`).
