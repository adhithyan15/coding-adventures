# Matrix (TypeScript)

A pure TypeScript 2D matrix library with no external dependencies. Part of the coding-adventures monorepo.

## Installation

```bash
npm install
```

## Usage

```typescript
import { Matrix } from "./src/matrix";

// Construction
const M = new Matrix([[1, 2, 3], [4, 5, 6]]);
const Z = Matrix.zeros(3, 3);
const I = Matrix.identity(3);
const D = Matrix.fromDiagonal([2, 3, 4]);

// Arithmetic
M.add(other);        // element-wise addition (matrix or scalar)
M.subtract(other);   // element-wise subtraction
M.scale(2.0);        // scalar multiplication
M.dot(other);        // matrix multiplication

// Element access
M.get(0, 0);         // read element at (row, col)
M.set(0, 0, 99);     // new matrix with element replaced

// Reductions
M.sum();             // sum of all elements
M.sumRows();         // n x 1 column vector of row sums
M.sumCols();         // 1 x m row vector of column sums
M.mean();            // arithmetic mean
M.min();             // minimum element
M.max();             // maximum element
M.argmin();          // [row, col] of minimum
M.argmax();          // [row, col] of maximum

// Element-wise math
M.map(x => x * 2);  // apply function to every element
M.sqrt();            // element-wise square root
M.abs();             // element-wise absolute value
M.pow(2);            // element-wise exponentiation

// Shape operations
M.flatten();         // 1 x n row vector
M.reshape(3, 2);     // reshape (total elements must match)
M.row(0);            // extract row as 1 x cols matrix
M.col(0);            // extract column as rows x 1 matrix
M.slice(0, 2, 1, 3); // sub-matrix [r0..r1), [c0..c1)
M.transpose();       // swap rows and columns

// Equality
M.equals(other);     // exact element-wise comparison
M.close(other, 1e-9); // approximate comparison within tolerance
```

## Design Principles

1. **Immutable by default.** Every method returns a new Matrix; the original is never mutated.
2. **No external dependencies.** Only `Math.*` from the JavaScript standard library.
3. **Row-major storage.** `data[i][j]` is the element at row i, column j.
4. **Literate programming.** Source code includes inline explanations, examples, and diagrams.

## Running Tests

```bash
npx jest --verbose
```

## Package Structure

- `src/matrix.ts` -- Matrix class with all operations
- `tests/matrix.test.ts` -- Jest test suite (47 tests)
