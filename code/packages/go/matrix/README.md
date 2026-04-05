# Matrix (Go)

A pure-Go 2D matrix library for linear algebra. No external dependencies.
All public functions are wrapped in the Operations framework for automatic
timing, structured logging, and panic recovery.

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/matrix"

// Create matrices
M := matrix.New2D([][]float64{{1, 2}, {3, 4}})
Z := matrix.Zeros(3, 3)
I := matrix.Identity(3)
D := matrix.FromDiagonal([]float64{2, 3})

// Arithmetic
C, _ := A.Add(B)         // element-wise addition
C, _ = A.Subtract(B)     // element-wise subtraction
C = A.Scale(2.0)          // scalar multiplication
C, _ = A.Dot(B)           // matrix multiplication
C = A.Transpose()         // transpose

// Element access
v, _ := M.Get(0, 1)      // read element -> 2.0
N, _ := M.Set(0, 1, 99)  // new matrix with element replaced

// Reductions
M.Sum()           // sum of all elements -> 10.0
M.Mean()          // arithmetic mean -> 2.5
M.SumRows()       // column vector of row sums
M.SumCols()       // row vector of column sums
M.Min()           // smallest element
M.Max()           // largest element
M.Argmin()        // (row, col, err) of minimum
M.Argmax()        // (row, col, err) of maximum

// Element-wise math
M.Map(fn)         // apply function to every element
M.Sqrt()          // element-wise square root
M.Abs()           // element-wise absolute value
M.Pow(2.0)        // element-wise exponentiation

// Shape operations
M.Flatten()                   // 1 x n row vector
M.Reshape(rows, cols)         // reshape (total must match)
M.Row(i)                      // extract row i
M.Col(j)                      // extract column j
M.Slice(r0, r1, c0, c1)      // sub-matrix [r0..r1), [c0..c1)

// Equality
M.Equals(other)               // exact equality
M.Close(other, 1e-9)          // within tolerance
```

## Design Principles

1. **Immutable by default** -- methods return new matrices, never mutate.
2. **No external dependencies** -- pure Go math only.
3. **Consistent error handling** -- descriptive errors via Go idiom.
4. **Operations framework** -- all calls have timing and panic recovery.

## Running Tests

```bash
go test ./... -v -cover
```
