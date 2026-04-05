# Matrix (Swift)

Pure-Swift 2D matrix type with arithmetic and linear-algebra operations.

## What it does

Provides a `Matrix` struct (value semantics, `Equatable`, `Sendable`) with the following operations:

### Base Operations

| Operation | API | Notes |
|-----------|-----|-------|
| Create all-zeros | `Matrix.zeros(rows:cols:)` | Static factory |
| Create from 2D array | `Matrix(from2D:)` | Deep-copies input |
| Create from 1D array | `Matrix(from1D:)` | Creates a 1 x n row vector |
| Create from scalar | `Matrix(scalar:)` | Creates a 1 x 1 matrix |
| Subscript access | `matrix[row, col]` | 0-based |
| Named access | `get(row:col:)` / `set(row:col:value:)` | Throws on out-of-bounds |
| Element-wise add | `add(_:)` or `+` | Throws if shapes differ |
| Add scalar | `addScalar(_:)` | Every element += s |
| Element-wise subtract | `subtract(_:)` or `-` | Throws if shapes differ |
| Scale | `scale(_:)` or `*` | Every element *= s |
| Transpose | `transpose()` | Returns new n x m matrix |
| Matrix multiply | `dot(_:)` | self.cols must equal other.rows |

### Extension Operations (ML03)

| Operation | API | Notes |
|-----------|-----|-------|
| Sum | `sum()` | Sum of all elements |
| Sum rows | `sumRows()` | m x 1 column vector |
| Sum cols | `sumCols()` | 1 x n row vector |
| Mean | `mean()` | Arithmetic mean |
| Min / Max | `min()` / `max()` | Scalar min/max |
| Argmin / Argmax | `argmin()` / `argmax()` | (row, col) tuple |
| Map | `map(_:)` | Apply closure to every element |
| Sqrt | `sqrt()` | Element-wise square root |
| Abs | `abs()` | Element-wise absolute value |
| Pow | `pow(_:)` | Element-wise exponentiation |
| Flatten | `flatten()` | 1 x n row vector |
| Reshape | `reshape(rows:cols:)` | Must preserve total elements |
| Row / Col | `row(_:)` / `col(_:)` | Extract single row/col |
| Slice | `slice(r0:r1:c0:c1:)` | Sub-matrix [r0..<r1), [c0..<c1) |
| Equals | `==` (Equatable) | Exact equality |
| Close | `close(_:tolerance:)` | Within tolerance (default 1e-9) |
| Identity | `Matrix.identity(n:)` | n x n identity matrix |
| From diagonal | `Matrix.fromDiagonal(_:)` | Diagonal matrix from values |

## How it fits in the stack

This is the Swift mirror of `code/packages/rust/matrix` and `code/packages/go/matrix`. It is a pure-math leaf package with no dependencies outside the Swift standard library and Foundation.

In a machine-learning pipeline this module underpins forward passes (weight x activation dot products), gradient accumulation, and layer-output buffering.

## Usage

```swift
import Matrix

// Build a 2x3 weight matrix.
let W = Matrix(from2D: [
    [0.1, 0.2, 0.3],
    [0.4, 0.5, 0.6],
])

// Build a 3x1 input column vector.
let x = Matrix(from1D: [1.0, 2.0, 3.0]).transpose()

// Forward pass: W . x  ->  2x1 result
let out = try W.dot(x)
print(out[0, 0])  // 0.1*1 + 0.2*2 + 0.3*3 = 1.4
print(out[1, 0])  // 0.4*1 + 0.5*2 + 0.6*3 = 3.2

// Reductions
let A = Matrix(from2D: [[1, 2], [3, 4]])
print(A.sum())    // 10.0
print(A.mean())   // 2.5

// Operators
let B = A + A        // element-wise addition
let C = A * 2.0      // scalar multiplication
```

## Running the tests

```bash
swift test --verbose
```

Requires Swift 6.0+.
